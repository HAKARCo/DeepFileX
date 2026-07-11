use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;
use quick_xml::Reader;
use quick_xml::events::Event;
use std::os::windows::ffi::OsStrExt;
use winapi::um::libloaderapi::{LoadLibraryW, GetProcAddress, FreeLibrary};

fn load_pdfium() -> Result<pdfium_render::prelude::Pdfium, String> {
    use std::fs::write;
    
    let temp_dir = std::env::temp_dir();
    let dll_path = temp_dir.join("DeepFileX_pdfium.dll");
    
    // Write embedded bytes if not exists or if size differs
    let dll_bytes = include_bytes!("pdfium.dll");
    let needs_write = if !dll_path.exists() {
        true
    } else {
        std::fs::metadata(&dll_path)
            .map(|m| m.len() != dll_bytes.len() as u64)
            .unwrap_or(true)
    };
    
    if needs_write {
        if let Err(e) = write(&dll_path, dll_bytes) {
            return Err(format!("Failed to dump embedded pdfium.dll: {}", e));
        }
    }
    
    // Bind directly to the written file path
    let bindings = pdfium_render::prelude::Pdfium::bind_to_library(&dll_path)
        .map_err(|e| format!("Failed to bind to pdfium.dll: {:?}", e))?;

    Ok(pdfium_render::prelude::Pdfium::new(bindings))
}

pub fn extract_text_from_pdf<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let pdfium = load_pdfium()?;
    
    let doc_path = path.as_ref().to_string_lossy().to_string();
    let document = pdfium.load_pdf_from_file(&doc_path, None)
        .map_err(|e| format!("Failed to load PDF document: {:?}", e))?;
        
    let mut full_text = String::new();
    for page in document.pages().iter() {
        if let Ok(text) = page.text() {
            full_text.push_str(&text.all());
            full_text.push('\n');
        }
    }
    
    Ok(full_text)
}

pub fn extract_text_from_docx<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut doc_file = archive.by_name("word/document.xml").map_err(|e| e.to_string())?;
    
    let mut content = Vec::new();
    doc_file.read_to_end(&mut content).map_err(|e| e.to_string())?;
    
    let mut reader = Reader::from_reader(&content[..]);
    let mut buf = Vec::new();
    let mut text = String::new();
    let mut in_t = false;
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"w:t" => {
                in_t = true;
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"w:t" => {
                in_t = false;
            }
            Ok(Event::Text(e)) if in_t => {
                if let Ok(t) = e.unescape() {
                    text.push_str(&t);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.to_string()),
            _ => {}
        }
        buf.clear();
    }
    Ok(text)
}

pub fn extract_text_from_xlsx<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    
    let mut doc_file = match archive.by_name("xl/sharedStrings.xml") {
        Ok(f) => f,
        Err(_) => return Ok(String::new()),
    };
    
    let mut content = Vec::new();
    doc_file.read_to_end(&mut content).map_err(|e| e.to_string())?;
    
    let mut reader = Reader::from_reader(&content[..]);
    let mut buf = Vec::new();
    let mut text = String::new();
    let mut in_t = false;
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"t" => {
                in_t = true;
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"t" => {
                in_t = false;
            }
            Ok(Event::Text(e)) if in_t => {
                if let Ok(t) = e.unescape() {
                    text.push_str(&t);
                    text.push(' ');
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.to_string()),
            _ => {}
        }
        buf.clear();
    }
    Ok(text)
}

pub fn extract_text_from_txt<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

pub fn extract_text_from_binary<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    // Maximum 2MB scan limit for binary safety & speed
    let mut chunk = vec![0u8; 2 * 1024 * 1024];
    let n = file.read(&mut chunk).map_err(|e| e.to_string())?;
    let data = &chunk[..n];

    let mut result = String::new();
    let mut temp = Vec::new();
    let mut i = 0;

    while i < data.len() {
        let b = data[i];
        
        // 1. Check UTF-8 Korean character (3 bytes)
        // Hangul syllables block in UTF-8 usually starts with 0xEA, 0xEB, 0xEC, 0xED
        if i + 2 < data.len() 
            && (b >= 0xEA && b <= 0xED) 
            && (data[i+1] >= 0x80 && data[i+1] <= 0xBF)
            && (data[i+2] >= 0x80 && data[i+2] <= 0xBF) 
        {
            temp.push(b);
            temp.push(data[i+1]);
            temp.push(data[i+2]);
            i += 3;
            continue;
        }

        // 2. Check Printable ASCII characters
        if (b >= 0x20 && b <= 0x7E) || b == b'\n' || b == b'\r' || b == b'\t' {
            temp.push(b);
            i += 1;
            continue;
        }

        // 3. Binary separator met: flush collected temp string if long enough
        if temp.len() >= 4 {
            if let Ok(s) = String::from_utf8(temp.clone()) {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    result.push_str(trimmed);
                    result.push(' ');
                }
            }
        }
        temp.clear();
        i += 1;
    }

    // Flush remainder
    if temp.len() >= 4 {
        if let Ok(s) = String::from_utf8(temp) {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                result.push_str(trimmed);
            }
        }
    }

    Ok(result)
}

pub fn try_extract_via_plugin(path: &Path, plugin_name: &str) -> Option<String> {
    let plugin_dir = Path::new("plugins");
    let plugin_path = plugin_dir.join(format!("{}.dll", plugin_name));
    if !plugin_path.exists() {
        return None;
    }

    unsafe {
        let path_wide: Vec<u16> = plugin_path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let h_lib = LoadLibraryW(path_wide.as_ptr());
        if h_lib.is_null() {
            return None;
        }

        let func_name = b"extract_text_plugin\0";
        let proc = GetProcAddress(h_lib, func_name.as_ptr() as *const i8);
        
        let mut result_text = None;
        if !proc.is_null() {
            type PluginFunc = unsafe extern "system" fn(*const u16, *mut u8, u32) -> i32;
            let func: PluginFunc = std::mem::transmute(proc);

            let path_to_parse_wide: Vec<u16> = path.as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let mut out_buffer = vec![0u8; 1024 * 1024]; // 1MB buffer
            let written = func(path_to_parse_wide.as_ptr(), out_buffer.as_mut_ptr(), out_buffer.len() as u32);
            if written > 0 {
                let text_len = written as usize;
                if let Ok(text) = String::from_utf8(out_buffer[..text_len].to_vec()) {
                    result_text = Some(text);
                }
            }
        }

        FreeLibrary(h_lib);
        result_text
    }
}

pub fn extract_text<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let path_ref = path.as_ref();
    let ext = path_ref.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
        
    let path_buf = path_ref.to_path_buf();

    // 1. Try plugin loading first for specific high-performance target formats (e.g. hwp, dwg)
    if ext == "hwp" || ext == "hwpx" {
        if let Some(text) = try_extract_via_plugin(&path_buf, "hwp_parser") {
            return Ok(text);
        }
    }
    if ext == "dwg" || ext == "dxf" {
        if let Some(text) = try_extract_via_plugin(&path_buf, "dwg_parser") {
            return Ok(text);
        }
    }
    
    let result = std::panic::catch_unwind(move || {
        match ext.as_str() {
            "pdf" => extract_text_from_pdf(&path_buf),
            "docx" => extract_text_from_docx(&path_buf),
            "xlsx" => extract_text_from_xlsx(&path_buf),
            "txt" | "csv" | "log" | "srt" | "vtt" | "md" | "json" | "xml" | "yaml" | "yml" | "ini" | "toml" => {
                extract_text_from_txt(&path_buf)
            }
            _ => extract_text_from_binary(&path_buf),
        }
    });

    match result {
        Ok(inner_res) => inner_res,
        Err(_) => Err("Parser panicked while processing this file.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_txt() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_hakar.txt");
        std::fs::write(&test_file, "하카르 DeepFileX 테스트").unwrap();
        
        let text = extract_text(&test_file).unwrap();
        assert!(text.contains("하카르"));
        
        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_extract_pdf_nonexistent() {
        let res = extract_text("nonexistent.pdf");
        assert!(res.is_err());
    }

    #[test]
    fn test_extract_pdf_real() {
        let pdf_path = "C:\\HAKAR\\등록증.pdf";
        if std::path::Path::new(pdf_path).exists() {
            let res = extract_text(pdf_path);
            assert!(res.is_ok(), "Real PDF extraction failed: {:?}", res.err());
        }
    }

    #[test]
    fn test_extract_binary_strings() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_hakar_bin.dat");
        
        let mut binary_data = vec![0x00, 0x01, 0x02, 0x03];
        binary_data.extend_from_slice(b"HakarHello");
        binary_data.extend_from_slice(&[0x00, 0x00, 0xff]);
        binary_data.extend_from_slice("테스트텍스트".as_bytes());
        binary_data.extend_from_slice(&[0x07, 0x08]);

        std::fs::write(&test_file, &binary_data).unwrap();
        
        let text = extract_text(&test_file).unwrap();
        assert!(text.contains("HakarHello"), "Extracted text was: '{}'", text);
        assert!(text.contains("테스트텍스트"), "Extracted text was: '{}'", text);
        
        let _ = std::fs::remove_file(test_file);
    }
}

