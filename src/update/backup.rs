use std::fs;
use std::path::{Path, PathBuf};
use crate::update::error::{UpdateError, Result};

/// 백업 폴더 루트 경로 반환
pub fn get_backup_root() -> PathBuf {
    let home_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
    std::path::PathBuf::from(home_dir)
        .join("Documents")
        .join("DeepFileX")
        .join("backups")
}

/// 실행 파일과 설정을 지정된 버전 식별자와 타임스탬프로 백업해.
pub fn create_backup(version: &str) -> Result<String> {
    let backup_id = format!("{}_{}", version, chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let backup_dir = get_backup_root().join(&backup_id);
    fs::create_dir_all(&backup_dir)?;

    let current_exe = std::env::current_exe()?;
    let dest_exe = backup_dir.join("DeepFileX.exe");

    // 하드링크 우선 시도, 실패 시 일반 복사로 폴백
    if let Err(_) = fs::hard_link(&current_exe, &dest_exe) {
        fs::copy(&current_exe, &dest_exe)?;
    }

    // 설정 파일이 존재할 경우 같이 백업
    let config_path = super::config::UpdateConfig::get_config_path();
    if config_path.exists() {
        let dest_config = backup_dir.join("update_config.json");
        let _ = fs::copy(&config_path, &dest_config);
    }

    Ok(backup_id)
}

/// 백업 유지 정책: 가장 오래된 백업을 제거하여 최대 3개까지만 보관해.
pub fn enforce_retention_policy() -> Result<()> {
    let root = get_backup_root();
    if !root.exists() {
        return Ok(());
    }
    
    let mut entries = Vec::new();
    if let Ok(dir_entries) = fs::read_dir(root) {
        for entry in dir_entries.filter_map(std::result::Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        entries.push((path, modified));
                    }
                }
            }
        }
    }
    
    // 수정 시간 기준 정렬 (오래된 것 순)
    entries.sort_by_key(|x| x.1);
    
    // 3개 이상인 경우 오래된 것 삭제
    if entries.len() > 3 {
        let to_remove = entries.len() - 3;
        for i in 0..to_remove {
            let _ = fs::remove_dir_all(&entries[i].0);
        }
    }
    
    Ok(())
}
