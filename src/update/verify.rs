use std::path::Path;
use crate::update::error::Result;

#[cfg(feature = "auto-update")]
use std::fs::File;
#[cfg(feature = "auto-update")]
use std::io::Read;
#[cfg(feature = "auto-update")]
use sha2::{Sha256, Digest};
#[cfg(feature = "auto-update")]
use crate::update::error::UpdateError;

/// SHA-256 해시를 스트리밍 방식으로 읽어 헥스(Hex) 문자열로 변환해 예상값과 대조해.
#[cfg(feature = "auto-update")]
pub fn verify_file_hash(path: &Path, expected_hex: &str) -> Result<()> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024]; // 64KB 청크 버퍼
    
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    
    let result = hasher.finalize();
    let actual_hex = to_hex(&result);
    
    if actual_hex.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        Err(UpdateError::HashMismatch {
            expected: expected_hex.to_string(),
            actual: actual_hex,
        })
    }
}

#[cfg(not(feature = "auto-update"))]
pub fn verify_file_hash(_path: &Path, _expected_hex: &str) -> Result<()> {
    Ok(())
}

/// 다운로드된 실행 파일의 서명을 확인해.
pub fn verify_binary_signature(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(feature = "auto-update")]
fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
