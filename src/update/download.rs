#[cfg(feature = "auto-update")]
use std::fs::{self, OpenOptions};
#[cfg(feature = "auto-update")]
use std::io::{self, Read, Write, Seek, SeekFrom};
#[cfg(feature = "auto-update")]
use std::path::Path;
#[cfg(feature = "auto-update")]
use reqwest::blocking::Client;
#[cfg(feature = "auto-update")]
use reqwest::header::RANGE;
use crate::update::error::{UpdateError, Result};

#[cfg(feature = "auto-update")]
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub total: u64,
    pub downloaded: u64,
}

#[cfg(feature = "auto-update")]
pub fn download_manifest(url: &str) -> Result<String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let resp = client.get(url).send()?;
    if !resp.status().is_success() {
        return Err(UpdateError::Http(format!("Server response error: {}", resp.status())));
    }
    let body = resp.text()?;
    Ok(body)
}

#[cfg(feature = "auto-update")]
pub fn download_binary<F>(
    url: &str,
    dest_path: &Path,
    expected_size: u64,
    mut progress_cb: F,
) -> Result<()>
where
    F: FnMut(DownloadProgress),
{
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(600)) // 10분 타임아웃
        .build()?;
        
    let part_path = dest_path.with_extension("part");
    
    // 이어받기 지원: 기존 .part 파일의 크기 확인
    let mut downloaded = 0u64;
    let mut file = if part_path.exists() {
        let meta = fs::metadata(&part_path)?;
        downloaded = meta.len();
        
        if downloaded >= expected_size {
            // 이미 완료되었거나 데이터 오염이 의심되는 경우 삭제 후 다시 다운로드
            fs::remove_file(&part_path)?;
            downloaded = 0;
            OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&part_path)?
        } else {
            let mut f = OpenOptions::new()
                .write(true)
                .open(&part_path)?;
            f.seek(SeekFrom::End(0))?;
            f
        }
    } else {
        // 부모 디렉토리가 없으면 생성
        if let Some(parent) = part_path.parent() {
            fs::create_dir_all(parent)?;
        }
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&part_path)?
    };

    let mut req = client.get(url);
    if downloaded > 0 {
        req = req.header(RANGE, format!("bytes={}-", downloaded));
    }
    
    let mut resp = req.send()?;
    
    // 206 Partial Content 혹은 200 OK 검사
    if downloaded > 0 && resp.status() != reqwest::StatusCode::PARTIAL_CONTENT {
        // 서버가 이어받기를 지원하지 않는 경우 처음부터 재시도
        downloaded = 0;
        file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&part_path)?;
        let req_new = client.get(url);
        resp = req_new.send()?;
    }
    
    if !resp.status().is_success() {
        return Err(UpdateError::Http(format!("Binary download failed: {}", resp.status())));
    }

    let mut buffer = [0u8; 64 * 1024]; // 64KB 단위 청크
    
    loop {
        let n = resp.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n])?;
        downloaded += n as u64;
        
        progress_cb(DownloadProgress {
            total: expected_size,
            downloaded,
        });
    }
    
    file.flush()?;
    drop(file);
    
    // 다운로드 완료 시 최종 파일명으로 원자적 변경
    fs::rename(&part_path, dest_path)?;
    
    Ok(())
}
