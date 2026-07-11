use std::path::Path;
use windows_sys::Win32::UI::Shell::{
    ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_INVOKEIDLIST,
};
use windows_sys::Win32::Foundation::HWND;

pub fn show_file_properties<P: AsRef<Path>>(path: P) {
    let path_buf = path.as_ref().to_path_buf();
    std::thread::spawn(move || {
        let path_str = path_buf.to_string_lossy().replace("/", "\\");
        let wide_path: Vec<u16> = path_str
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let verb_wide: Vec<u16> = "properties"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let mut sei = unsafe { std::mem::zeroed::<SHELLEXECUTEINFOW>() };
        sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        sei.fMask = SEE_MASK_INVOKEIDLIST;
        sei.hwnd = 0 as HWND;
        sei.lpVerb = verb_wide.as_ptr();
        sei.lpFile = wide_path.as_ptr();
        sei.nShow = 1; // SW_SHOWNORMAL

        unsafe {
            ShellExecuteExW(&mut sei);
        }
    });
}

pub fn open_file_location<P: AsRef<Path>>(path: P) {
    let path_buf = path.as_ref().to_path_buf();
    std::thread::spawn(move || {
        let path_str = path_buf.to_string_lossy().replace("/", "\\");
        use std::os::windows::process::CommandExt;
        if path_buf.is_dir() {
            let _ = std::process::Command::new("explorer.exe")
                .raw_arg(format!(r#""{}""#, path_str))
                .spawn();
        } else {
            let _ = std::process::Command::new("explorer.exe")
                .raw_arg(format!(r#"/select,"{}""#, path_str))
                .spawn();
        }
    });
}

pub fn open_file<P: AsRef<Path>>(path: P) {
    let path_buf = path.as_ref().to_path_buf();
    std::thread::spawn(move || {
        let path_str = path_buf.to_string_lossy().replace("/", "\\");
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", &path_str])
            .spawn();
    });
}
