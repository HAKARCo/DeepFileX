use std::fs;
use crate::update::error::{UpdateError, Result};
use crate::update::backup::get_backup_root;

/// 지정된 백업 ID를 사용해 이전 버전의 실행 파일 및 설정을 복구해.
pub fn execute_rollback(backup_id: &str) -> Result<()> {
    let backup_dir = get_backup_root().join(backup_id);
    if !backup_dir.exists() {
        return Err(UpdateError::RollbackFailed(format!("백업 폴더가 존재하지 않음: {}", backup_id)));
    }

    let backup_exe = backup_dir.join("DeepFileX.exe");
    if !backup_exe.exists() {
        return Err(UpdateError::RollbackFailed("백업 폴더 내 실행 파일이 존재하지 않음".to_string()));
    }

    let current_exe = std::env::current_exe()?;
    let old_exe = current_exe.with_extension("exe.old");

    // Windows 실행 중 바이너리 교체 기법:
    // 1. 현재 실행 파일을 .old로 이름 변경 (Windows는 실행 중이어도 이름 변경을 허용함)
    if current_exe.exists() {
        let _ = fs::remove_file(&old_exe); // 기존 old가 남아있으면 삭제
        fs::rename(&current_exe, &old_exe)?;
    }

    // 2. 백업 폴더의 실행 파일을 원래 경로에 복사
    if let Err(e) = fs::copy(&backup_exe, &current_exe) {
        // 복사 실패 시 복구 롤백
        if old_exe.exists() {
            let _ = fs::rename(&old_exe, &current_exe);
        }
        return Err(UpdateError::RollbackFailed(format!("바이너리 롤백 복사 실패: {}", e)));
    }

    // 3. 백업 설정 파일이 있다면 원래 위치에 덮어씌움
    let backup_config = backup_dir.join("update_config.json");
    if backup_config.exists() {
        let config_path = super::config::UpdateConfig::get_config_path();
        let _ = fs::copy(&backup_config, &config_path);
    }

    // .old 파일 삭제 시도 (실패해도 무방함)
    let _ = fs::remove_file(&old_exe);

    Ok(())
}
