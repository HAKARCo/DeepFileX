pub mod error;
pub mod manifest;
pub mod config;
pub mod verify;
pub mod download;
pub mod backup;
pub mod rollback;
pub mod state;
pub mod installer;

use std::sync::{Arc, Mutex};
use std::thread;
use std::path::PathBuf;
use crate::update::error::{UpdateError, Result};
pub use crate::update::state::{UpdateState, UpdateContext};
pub use crate::update::config::UpdateConfig;
pub use crate::update::manifest::Channel;

#[cfg(feature = "auto-update")]
const MANIFEST_URL: &str = "https://raw.githubusercontent.com/deepfilex/releases/main/update-manifest.json";

/// 백그라운드 업데이트 상태 검사 및 자동/수동 흐름 기동
#[cfg(feature = "auto-update")]
pub fn trigger_update_check(
    context: Arc<Mutex<UpdateContext>>,
    config: UpdateConfig,
) {
    // 이미 업데이트 진행 중인지 파악
    {
        let mut lock = context.lock().unwrap();
        if lock.state != UpdateState::Idle 
            && lock.state != UpdateState::Failed 
            && lock.state != UpdateState::Complete 
            && lock.state != UpdateState::RolledBack 
        {
            return;
        }
        lock.transition(UpdateState::Checking);
    }

    thread::spawn(move || {
        if let Err(e) = run_update_flow(context.clone(), &config) {
            let mut lock = context.lock().unwrap();
            lock.set_error(e.to_string());
        }
    });
}

#[cfg(feature = "auto-update")]
fn run_update_flow(
    context: Arc<Mutex<UpdateContext>>,
    config: &UpdateConfig,
) -> Result<()> {
    // 1. 매니페스트 JSON 다운로드
    let manifest_str = download::download_manifest(MANIFEST_URL)?;
    let manifest = manifest::Manifest::from_json(&manifest_str)?;
    
    // 2. 사용자 설정 채널 정보 조회
    let channel_info = match manifest.channels.get(&config.channel) {
        Some(info) => info,
        None => return Err(UpdateError::ManifestInvalid(format!("{:?} 채널 정보가 존재하지 않습니다.", config.channel))),
    };

    // 3. 버전 비교 (CARGO_PKG_VERSION 기반)
    let current_version_str = env!("CARGO_PKG_VERSION");
    let current_version = manifest::Version::parse(current_version_str)
        .map_err(|e| UpdateError::ManifestInvalid(e))?;
    
    if channel_info.current_version <= current_version {
        // 업데이트 필요 없음 (최신 버전임)
        let mut lock = context.lock().unwrap();
        lock.transition(UpdateState::UpToDate);
        return Ok(());
    }

    // 4. 플랫폼에 부합하는 다운로드 아티팩트 선별 (Windows x64)
    let artifact = match channel_info.platforms.get("windows-x64") {
        Some(art) => art,
        None => return Err(UpdateError::ManifestInvalid("윈도우 x64 바이너리 경로가 제공되지 않았습니다.".to_string())),
    };

    // 상태를 다운로드 중으로 변경
    {
        let mut lock = context.lock().unwrap();
        lock.new_version = Some(channel_info.current_version.to_string());
        lock.transition(UpdateState::Downloading);
    }

    // 다운로드 대상 경로 설정
    let home_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
    let temp_dir = std::path::PathBuf::from(home_dir)
        .join("Documents")
        .join("DeepFileX")
        .join("updates");
    
    let dest_path = temp_dir.join(format!("DeepFileX_upgrade_{}.exe", channel_info.current_version.to_string()));

    // 5. 바이너리 다운로드
    let context_clone = context.clone();
    download::download_binary(&artifact.url, &dest_path, artifact.size, move |progress| {
        let mut lock = context_clone.lock().unwrap();
        lock.set_progress(progress.downloaded, progress.total);
    })?;

    // 6. 무결성 및 해시 체크
    {
        let mut lock = context.lock().unwrap();
        lock.transition(UpdateState::Verifying);
    }
    verify::verify_file_hash(&dest_path, &artifact.sha256)?;
    verify::verify_binary_signature(&dest_path)?;

    // 7. 현재 바이너리 백업 단계 진입 (설치 준비)
    {
        let mut lock = context.lock().unwrap();
        lock.transition(UpdateState::Installing);
    }
    
    // 백업 생성
    let backup_id = backup::create_backup(current_version_str)?;
    {
        let mut lock = context.lock().unwrap();
        lock.backup_id = Some(backup_id.clone());
    }
    
    // 최대 백업 수 제한 조치 (3개)
    let _ = backup::enforce_retention_policy();

    // 8. 무음 인스톨러 구동 후 현재 프로세스 종료
    if let Err(e) = installer::run_silent_installer(&dest_path) {
        // 인스톨러 실행 자체가 실패했을 시 백업에서 원래대로 롤백 복원
        if let Err(rollback_err) = rollback::execute_rollback(&backup_id) {
            return Err(UpdateError::RollbackFailed(format!(
                "인스톨러 구동 실패 후 백업 복구 실패: {}",
                rollback_err
            )));
        }
        return Err(UpdateError::InstallerFailed(-1));
    }

    Ok(())
}

/// `"auto-update"` 피처가 꺼져있을 때의 No-op 더미 구현체
#[cfg(not(feature = "auto-update"))]
pub fn trigger_update_check(
    _context: Arc<Mutex<UpdateContext>>,
    _config: UpdateConfig,
) {
    // No-op
}
