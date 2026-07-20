use std::process::Command;
use std::path::Path;
use crate::update::error::{UpdateError, Result};

/// 다운로드한 NSIS 인스톨러를 무음 모드(/S)로 구동하고 현재 프로세스를 자연스럽게 종료해.
pub fn run_silent_installer(installer_path: &Path) -> Result<()> {
    if !installer_path.exists() {
        return Err(UpdateError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "인스톨러 파일을 찾을 수 없습니다.",
        )));
    }

    // Windows 환경에서 락을 우회하기 위해 현재 바이너리는 이미 백업 단계에서 .exe.old로 이름이 바뀌었으므로
    // 인스톨러는 충돌 없이 DeepFileX.exe를 덮어쓸 수 있어.
    let status = Command::new(installer_path)
        .arg("/S")
        .spawn();

    match status {
        Ok(_) => {
            // 인스톨러가 백그라운드로 안전하게 스폰되었으므로, 새 버전을 위해 현재 프로세스는 즉시 종료해.
            std::process::exit(0);
        }
        Err(e) => {
            Err(UpdateError::InstallerFailed(-1))
        }
    }
}
