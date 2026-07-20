#![allow(clippy::type_complexity, clippy::collapsible_if)]

pub mod ntfs;
pub mod crypto;
pub mod db;
pub mod parser;
pub mod update;

pub fn scan_volume(volume: &str) -> Result<Vec<(u64, String, bool)>, String> {
    // 1. Try with GENERIC_READ first (necessary for FSCTL_QUERY_USN_JOURNAL validation)
    let handle_res = ntfs::open_volume_with_access(volume, 0x80000000);
    let handle = match handle_res {
        Ok(h) => h,
        Err(_) => {
            // 2. Fallback to GENERIC_READ | GENERIC_WRITE
            match ntfs::open_volume_with_access(volume, 0x80000000 | 0x40000000) {
                Ok(h) => h,
                Err(_) => {
                    // 3. Ultimate Fallback to FILE_ANY_ACCESS (0)
                    ntfs::open_volume_with_access(volume, 0)?
                }
            }
        }
    };
    
    // 2. Try scanning the MFT
    let scan_res = ntfs::scan_mft(handle);
    let entries = match scan_res {
        Ok(e) => {
            ntfs::close_volume(handle);
            e
        }
        Err(err_msg) => {
            ntfs::close_volume(handle);
            // If failure was due to inactive USN journal, retry by creating it
            if err_msg.contains("FSCTL_QUERY_USN_JOURNAL failed") {
                println!("USN Journal not active. Attempting to create/activate USN Journal...");
                if let Ok(write_handle) = ntfs::open_volume_with_access(volume, 0x80000000 | 0x40000000) {
                    if ntfs::create_usn_journal(write_handle).is_ok() {
                        println!("USN Journal created successfully. Retrying MFT scan...");
                        if let Ok(new_read_handle) = ntfs::open_volume_with_access(volume, 0) {
                            if let Ok(entries) = ntfs::scan_mft(new_read_handle) {
                                ntfs::close_volume(new_read_handle);
                                return Ok(ntfs::get_absolute_paths(&entries));
                            }
                            ntfs::close_volume(new_read_handle);
                        }
                    }
                    ntfs::close_volume(write_handle);
                }
            }
            return Err(err_msg);
        }
    };

    let paths = ntfs::get_absolute_paths(&entries);
    Ok(paths)
}

pub fn query_journal_info(volume: &str) -> Result<(u64, i64), String> {
    let handle = ntfs::open_volume(volume)?;
    let info = ntfs::query_journal_info(handle)?;
    ntfs::close_volume(handle);
    Ok(info)
}

pub fn read_journal(volume: &str, start_usn: i64, journal_id: u64) -> Result<(Vec<(i64, u64, u64, String, u32, bool)>, i64), String> {
    let handle = ntfs::open_volume(volume)?;
    let (entries, next_usn) = ntfs::read_journal(handle, start_usn, journal_id)?;
    ntfs::close_volume(handle);

    let result = entries
        .into_iter()
        .map(|e| (e.usn, e.file_id, e.parent_id, e.name, e.reason, e.is_dir))
        .collect();

    Ok((result, next_usn))
}

pub fn encrypt_data(data: &[u8], password: &str, salt: &[u8], iv: &[u8]) -> Result<Vec<u8>, String> {
    let iv_arr: &[u8; 12] = iv.try_into().map_err(|_| "IV must be exactly 12 bytes".to_string())?;
    let key = crypto::derive_key(password, salt);
    let cipher = crypto::encrypt_aes_gcm(data, &key, iv_arr)?;
    Ok(cipher)
}

pub fn decrypt_data(encrypted_data: &[u8], password: &str, salt: &[u8], iv: &[u8]) -> Result<Vec<u8>, String> {
    let iv_arr: &[u8; 12] = iv.try_into().map_err(|_| "IV must be exactly 12 bytes".to_string())?;
    let key = crypto::derive_key(password, salt);
    let plain = crypto::decrypt_aes_gcm(encrypted_data, &key, iv_arr)?;
    Ok(plain)
}

pub fn compress_data(data: &[u8], level: Option<i32>) -> Result<Vec<u8>, String> {
    let comp = crypto::compress_zstd(data, level.unwrap_or(3))?;
    Ok(comp)
}

pub fn decompress_data(data: &[u8]) -> Result<Vec<u8>, String> {
    let decomp = crypto::decompress_zstd(data)?;
    Ok(decomp)
}

pub fn walk_directory(root: &str) -> Vec<(String, u64, u64, bool)> {
    let mut results = Vec::new();
    let walker = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name == "Windows" || name == "Program Files" || name == "Program Files (x86)"
                        || name == "$Recycle.Bin" || name == "System Volume Information"
                        || name == "AppData" || name == "node_modules" || name == ".git"
                        || name == "target" || name == ".cargo" || name == ".rustup"
                        || name == "Local" || name == "Roaming" || name.starts_with('.')
                    {
                        return false;
                    }
                }
            }
            true
        });

    for entry in walker.filter_map(Result::ok) {
        let path_str = entry.path().to_string_lossy().into_owned();
        let is_dir = entry.file_type().is_dir();
        let mut size = 0;
        let mut mtime = 0;
        if !is_dir {
            if let Ok(meta) = entry.metadata() {
                size = meta.len();
                if let Ok(modified) = meta.modified() {
                    if let Ok(dur) = modified.duration_since(std::time::SystemTime::UNIX_EPOCH) {
                        mtime = dur.as_secs();
                    }
                }
            }
        }
        results.push((path_str, size, mtime, is_dir));
    }
    results
}


