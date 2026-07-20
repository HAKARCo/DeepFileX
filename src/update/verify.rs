use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use sha2::{Sha256, Digest};
use crate::update::error::{UpdateError, Result};

/// SHA-256 해시를 스트리밍 방식으로 읽어 헥스(Hex) 문자열로 변환해 예상값과 대조해.
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

/// 다운로드된 실행 파일의 서명을 확인해.
/// 현재 단계에서는 인증서 비용 부담으로 인해 서명 검증을 선택적 Bypass 처리하며,
/// 향후 서명 적용 시 WinVerifyTrust API 호출 코드를 활성화할 수 있도록 스켈레톤 구조를 유지해.
pub fn verify_binary_signature(_path: &Path) -> Result<()> {
    // TODO: EV 코드 서명 인증서 구매 후 활성화 예정
    // let status = unsafe { WinVerifyTrust(...) };
    Ok(())
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
