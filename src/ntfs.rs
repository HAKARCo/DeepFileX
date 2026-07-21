use std::collections::HashMap;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::ptr::null_mut;
use std::slice;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, GetLastError};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    FILE_FLAG_NO_BUFFERING, FILE_FLAG_WRITE_THROUGH,
};
use windows_sys::Win32::System::IO::DeviceIoControl;

// Windows NTFS/USN 제어 코드 상수 정의 (WinIoCtl.h)
const FSCTL_QUERY_USN_JOURNAL: u32 = 0x000900F4;
const FSCTL_ENUM_USN_DATA: u32 = 0x000900B3;
const FSCTL_READ_USN_JOURNAL: u32 = 0x000900BB;

#[derive(Clone)]
pub struct UsnChangeEntry {
    pub usn: i64,
    pub file_id: u64,
    pub parent_id: u64,
    pub name: String,
    pub reason: u32,
    pub is_dir: bool,
}

#[repr(C)]
struct ReadUsnJournalDataV0 {
    start_usn: i64,
    reason_mask: u32,
    return_only_on_close: u32,
    timeout: u64,
    bytes_to_wait: u64,
    usn_journal_id: u64,
}

#[derive(Clone)]
pub struct FileEntry {
    pub file_id: u64,
    pub parent_id: u64,
    pub name: String,
    pub is_dir: bool,
}

#[repr(C)]
struct UsnRecordV2Header {
    record_length: u32,
    major_version: u16,
    minor_version: u16,
    file_reference_number: u64,
    parent_file_reference_number: u64,
    usn: i64,
    time_stamp: i64,
    reason: u32,
    source_info: u32,
    security_id: u32,
    file_attributes: u32,
    file_name_length: u16,
    file_name_offset: u16,
}

#[repr(C)]
struct UsnJournalDataV0 {
    usn_journal_id: u64,
    first_usn: i64,
    next_usn: i64,
    lowest_valid_usn: i64,
    max_usn: i64,
    maximum_size: u64,
    allocation_delta: u64,
}

#[repr(C)]
struct MftEnumDataV0 {
    start_file_reference_number: u64,
    low_usn: i64,
    high_usn: i64,
}

const FSCTL_CREATE_USN_JOURNAL: u32 = 0x000900E7;

#[repr(C)]
struct CreateUsnJournalData {
    maximum_size: u64,
    allocation_delta: u64,
}

pub fn open_volume(volume: &str) -> Result<HANDLE, String> {
    open_volume_with_access(volume, 0) // Default to 0 (FILE_ANY_ACCESS) for max compatibility
}

pub fn open_volume_with_access(volume: &str, access: u32) -> Result<HANDLE, String> {
    let volume_wide: Vec<u16> = volume
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // FILE_SHARE_READ=1, FILE_SHARE_WRITE=2, FILE_SHARE_DELETE=4
    let handle = unsafe {
        CreateFileW(
            volume_wide.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | 4, // Include FILE_SHARE_DELETE
            null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH,
            0,
        )
    };

    if handle == INVALID_HANDLE_VALUE {
        Err(format!(
            "Failed to open volume handle. LastError: {}",
            unsafe { GetLastError() }
        ))
    } else {
        Ok(handle)
    }
}

pub fn create_usn_journal(handle: HANDLE) -> Result<(), String> {
    let mut create_data = CreateUsnJournalData {
        maximum_size: 0x2000000, // 32MB
        allocation_delta: 0x400000, // 4MB
    };
    let mut bytes_returned = 0;
    let success = unsafe {
        DeviceIoControl(
            handle,
            FSCTL_CREATE_USN_JOURNAL,
            &mut create_data as *mut _ as *mut _,
            std::mem::size_of::<CreateUsnJournalData>() as u32,
            null_mut(),
            0,
            &mut bytes_returned,
            null_mut(),
        )
    };
    if success == 0 {
        Err(format!("FSCTL_CREATE_USN_JOURNAL failed. Error: {}", unsafe { GetLastError() }))
    } else {
        Ok(())
    }
}

pub fn close_volume(handle: HANDLE) {
    if handle != INVALID_HANDLE_VALUE {
        unsafe { CloseHandle(handle) };
    }
}

pub fn scan_mft(handle: HANDLE) -> Result<Vec<FileEntry>, String> {
    let mut usn_journal_data = UsnJournalDataV0 {
        usn_journal_id: 0,
        first_usn: 0,
        next_usn: 0,
        lowest_valid_usn: 0,
        max_usn: 0,
        maximum_size: 0,
        allocation_delta: 0,
    };
    
    let mut bytes_returned: u32 = 0;

    // 1. Query USN Journal
    let success = unsafe {
        DeviceIoControl(
            handle,
            FSCTL_QUERY_USN_JOURNAL,
            null_mut(),
            0,
            &mut usn_journal_data as *mut _ as *mut _,
            std::mem::size_of::<UsnJournalDataV0>() as u32,
            &mut bytes_returned,
            null_mut(),
        )
    };

    if success == 0 {
        return Err(format!("FSCTL_QUERY_USN_JOURNAL failed. Error: {}", unsafe { GetLastError() }));
    }

    // 2. Enumerate MFT files using FSCTL_ENUM_USN_DATA
    let mut mft_enum_data = MftEnumDataV0 {
        start_file_reference_number: 0,
        low_usn: 0,
        high_usn: usn_journal_data.next_usn,
    };

    let mut buffer = vec![0u8; 64 * 1024]; // 64KB Direct I/O buffer
    let mut entries = Vec::with_capacity(500_000); // 50만개 사전 용량 할당으로 힙 재할당 병목 소거

    loop {
        let mut bytes_returned: u32 = 0;
        let success = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_ENUM_USN_DATA,
                &mut mft_enum_data as *mut _ as *mut _,
                std::mem::size_of::<MftEnumDataV0>() as u32,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
                &mut bytes_returned,
                null_mut(),
            )
        };

        if success == 0 {
            let err = unsafe { GetLastError() };
            if err == 38 { // ERROR_HANDLE_EOF (End of File)
                break;
            }
            return Err(format!("FSCTL_ENUM_USN_DATA failed. Error: {}", err));
        }

        if bytes_returned < 8 {
            break;
        }

        // Parse buffer of USN records
        let next_start_frn = u64::from_ne_bytes(buffer[0..8].try_into().unwrap());
        mft_enum_data.start_file_reference_number = next_start_frn;

        let mut offset = 8; // The first 8 bytes contains the next starting file reference number
        while offset < bytes_returned as usize {
            let record = unsafe { &*(buffer.as_ptr().add(offset) as *const UsnRecordV2Header) };
            if record.record_length == 0 {
                break;
            }

            if record.major_version == 2 {
                let name_start = offset + record.file_name_offset as usize;
                let name_len = record.file_name_length as usize;
                
                if name_start + name_len <= bytes_returned as usize {
                    let name_raw = unsafe {
                        slice::from_raw_parts(
                            buffer.as_ptr().add(name_start) as *const u16,
                            name_len / 2,
                        )
                    };
                    
                    let name = OsString::from_wide(name_raw).to_string_lossy().into_owned();
                    let is_dir = (record.file_attributes & 0x00000010) != 0; // FILE_ATTRIBUTE_DIRECTORY
                    
                    entries.push(FileEntry {
                        file_id: record.file_reference_number & 0x0000FFFFFFFFFFFF,
                        parent_id: record.parent_file_reference_number & 0x0000FFFFFFFFFFFF,
                        name,
                        is_dir,
                    });
                }
            }

            offset += record.record_length as usize;
        }
    }

    Ok(entries)
}

pub fn get_absolute_paths(entries: &[FileEntry]) -> Vec<(u64, String, bool)> {
    let mut id_map = HashMap::with_capacity(entries.len());
    for entry in entries {
        id_map.insert(entry.file_id, entry);
    }

    let mut result = Vec::with_capacity(entries.len());
    // Cache absolute path strings only for directories to save memory (roughly 10% of total entries)
    let mut path_cache: HashMap<u64, String> = HashMap::with_capacity(entries.len() / 10);

    // Recursively build absolute path using memoization to eliminate redundant parent walks
    fn get_path(
        id: u64,
        id_map: &HashMap<u64, &FileEntry>,
        path_cache: &mut HashMap<u64, String>,
        visited: &mut std::collections::HashSet<u64>,
    ) -> Option<String> {
        if id == 5 { // 5 = Root directory
            return Some(String::new());
        }
        if id == 0 {
            return None;
        }
        if let Some(cached) = path_cache.get(&id) {
            return Some(cached.clone());
        }

        // Prevent infinite path traversal loop
        if !visited.insert(id) {
            return None;
        }

        let entry = id_map.get(&id)?;
        
        let parent_path = if entry.parent_id == id || entry.parent_id == 0 {
            String::new()
        } else {
            get_path(entry.parent_id, id_map, path_cache, visited).unwrap_or_default()
        };

        let abs_path = if parent_path.is_empty() {
            entry.name.clone()
        } else {
            format!("{}\\{}", parent_path, entry.name)
        };

        // Cache path ONLY if the entry is a directory (files don't have children to query)
        if entry.is_dir {
            path_cache.insert(id, abs_path.clone());
        }

        visited.remove(&id);
        Some(abs_path)
    }

    let mut visited = std::collections::HashSet::new();

    for entry in entries {
        if entry.name.is_empty() || entry.name == "$MFT" || entry.name == "$LogFile" || entry.name == "$Bitmap"
            || entry.name == "$Volume" || entry.name == "$AttrDef" || entry.name == "$Boot"
            || entry.name == "$BadClus" || entry.name == "$Secure" || entry.name == "$UpCase"
            || entry.name == "$Extend" || entry.name == "$MFTMirr"
        {
            continue;
        }

        if let Some(abs_path) = get_path(entry.file_id, &id_map, &mut path_cache, &mut visited) {
            result.push((entry.file_id, abs_path, entry.is_dir));
        }
    }

    result
}

pub fn query_journal_info(handle: HANDLE) -> Result<(u64, i64), String> {
    let mut usn_journal_data = UsnJournalDataV0 {
        usn_journal_id: 0,
        first_usn: 0,
        next_usn: 0,
        lowest_valid_usn: 0,
        max_usn: 0,
        maximum_size: 0,
        allocation_delta: 0,
    };
    let mut bytes_returned = 0;
    let success = unsafe {
        DeviceIoControl(
            handle,
            FSCTL_QUERY_USN_JOURNAL,
            null_mut(),
            0,
            &mut usn_journal_data as *mut _ as *mut _,
            std::mem::size_of::<UsnJournalDataV0>() as u32,
            &mut bytes_returned,
            null_mut(),
        )
    };
    if success == 0 {
        return Err(format!("Query USN Journal failed. Error: {}", unsafe { GetLastError() }));
    }
    Ok((usn_journal_data.usn_journal_id, usn_journal_data.next_usn))
}

pub fn read_journal(handle: HANDLE, start_usn: i64, journal_id: u64) -> Result<(Vec<UsnChangeEntry>, i64), String> {
    let mut read_data = ReadUsnJournalDataV0 {
        start_usn,
        reason_mask: 0xFFFFFFFF,
        return_only_on_close: 0,
        timeout: 0,
        bytes_to_wait: 0,
        usn_journal_id: journal_id,
    };

    let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
    let mut bytes_returned = 0;

    let success = unsafe {
        DeviceIoControl(
            handle,
            FSCTL_READ_USN_JOURNAL,
            &mut read_data as *mut _ as *mut _,
            std::mem::size_of::<ReadUsnJournalDataV0>() as u32,
            buffer.as_mut_ptr() as *mut _,
            buffer.len() as u32,
            &mut bytes_returned,
            null_mut(),
        )
    };

    if success == 0 {
        return Err(format!("FSCTL_READ_USN_JOURNAL failed. Error: {}", unsafe { GetLastError() }));
    }

    let mut entries = Vec::new();
    if bytes_returned < 8 {
        return Ok((entries, start_usn));
    }

    // The first 8 bytes contains the next starting USN number
    let next_usn = i64::from_ne_bytes(buffer[0..8].try_into().unwrap());

    let mut offset = 8;
    while offset < bytes_returned as usize {
        let record = unsafe { &*(buffer.as_ptr().add(offset) as *const UsnRecordV2Header) };
        if record.record_length == 0 {
            break;
        }

        if record.major_version == 2 {
            let name_start = offset + record.file_name_offset as usize;
            let name_len = record.file_name_length as usize;

            if name_start + name_len <= bytes_returned as usize {
                let name_raw = unsafe {
                    slice::from_raw_parts(
                        buffer.as_ptr().add(name_start) as *const u16,
                        name_len / 2,
                    )
                };
                let name = OsString::from_wide(name_raw).to_string_lossy().into_owned();
                let is_dir = (record.file_attributes & 0x00000010) != 0;

                entries.push(UsnChangeEntry {
                    usn: record.usn,
                    file_id: record.file_reference_number,
                    parent_id: record.parent_file_reference_number,
                    name,
                    reason: record.reason,
                    is_dir,
                });
            }
        }
        offset += record.record_length as usize;
    }

    Ok((entries, next_usn))
}
