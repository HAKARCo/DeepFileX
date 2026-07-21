use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;
use quick_xml::Reader;
use quick_xml::events::Event;
use std::os::windows::ffi::OsStrExt;
use winapi::um::libloaderapi::{LoadLibraryW, GetProcAddress, FreeLibrary};
use std::sync::atomic::{AtomicBool, Ordering};

pub static ENABLE_HWP_PLUGIN: AtomicBool = AtomicBool::new(true);
pub static ENABLE_DWG_PLUGIN: AtomicBool = AtomicBool::new(true);

pub fn extract_text_from_pdf<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let doc = lopdf::Document::load(path).map_err(|e| e.to_string())?;
    let mut full_text = String::new();
    let pages = doc.get_pages();
    
    let mut page_numbers: Vec<u32> = pages.keys().cloned().collect();
    page_numbers.sort_unstable();
    
    for page_num in page_numbers {
        if let Ok(page_text) = doc.extract_text(&[page_num]) {
            full_text.push_str(&page_text);
            full_text.push('\x0c'); // Page marker
        }
    }
    Ok(full_text)
}

pub fn extract_text_from_docx<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    let doc_file = archive.by_name("word/document.xml").map_err(|e| e.to_string())?;
    
    let mut reader = Reader::from_reader(std::io::BufReader::with_capacity(4096, doc_file));
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
            Ok(Event::End(ref e)) if e.name().as_ref() == b"w:p" => {
                text.push('\n'); // Preserve paragraph boundaries!
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
    
    let doc_file = match archive.by_name("xl/sharedStrings.xml") {
        Ok(f) => f,
        Err(_) => return Ok(String::new()),
    };
    
    let mut reader = Reader::from_reader(std::io::BufReader::with_capacity(4096, doc_file));
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

pub fn extract_text_from_pptx<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut text = String::new();

    let file_names: Vec<String> = archive.file_names().map(|s| s.to_string()).collect();
    for name in file_names {
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            if let Ok(slide_file) = archive.by_name(&name) {
                let mut reader = Reader::from_reader(std::io::BufReader::with_capacity(4096, slide_file));
                let mut buf = Vec::new();
                let mut in_t = false;
                loop {
                    match reader.read_event_into(&mut buf) {
                        Ok(Event::Start(ref e)) if e.name().as_ref() == b"a:t" => {
                            in_t = true;
                        }
                        Ok(Event::End(ref e)) if e.name().as_ref() == b"a:t" => {
                            in_t = false;
                        }
                        Ok(Event::Text(e)) if in_t => {
                            if let Ok(t) = e.unescape() {
                                text.push_str(&t);
                                text.push(' ');
                            }
                        }
                        Ok(Event::End(ref e)) if e.name().as_ref() == b"p:sp" => {
                            text.push('\n');
                        }
                        Ok(Event::Eof) => break,
                        Err(_) => break,
                        _ => {}
                    }
                    buf.clear();
                }
            }
        }
    }
    Ok(text)
}

pub fn extract_text_from_txt<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    if let Ok(s) = String::from_utf8(buf.clone()) {
        Ok(s)
    } else {
        let (cow, _, _) = encoding_rs::EUC_KR.decode(&buf);
        Ok(cow.into_owned())
    }
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

pub fn extract_text_from_hwpx<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let file = File::open(&path).map_err(|e| format!("Failed to open HWPX file: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Failed to read HWPX zip archive: {}", e))?;

    let mut text = String::new();

    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let name = file.name().to_string();

        if (name.contains("section") || name.contains("Section")) && name.ends_with(".xml") {
            let mut xml_content = String::new();
            if file.read_to_string(&mut xml_content).is_ok() {
                let mut reader = Reader::from_str(&xml_content);
                reader.trim_text(true);
                let mut buf = Vec::new();
                let mut in_t = false;

                loop {
                    match reader.read_event_into(&mut buf) {
                        Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                            let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                            if tag.ends_with(":t") || tag == "t" || tag.contains("hp:t") {
                                in_t = true;
                            }
                        }
                        Ok(Event::End(ref e)) => {
                            let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                            if tag.ends_with(":t") || tag == "t" || tag.contains("hp:t") {
                                in_t = false;
                            }
                        }
                        Ok(Event::Text(e)) => {
                            if in_t {
                                if let Ok(t) = e.unescape() {
                                    text.push_str(&t);
                                    text.push(' ');
                                }
                            }
                        }
                        Ok(Event::Eof) => break,
                        Err(_) => break,
                        _ => {}
                    }
                    buf.clear();
                }
            }
        }
    }

    if text.trim().is_empty() {
        extract_text_from_binary(&path)
    } else {
        Ok(text)
    }
}

pub fn extract_text_from_hwp<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let file = File::open(&path).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    // Safety Limit: Max 4MB read per HWP file to eliminate OOM & RAM spikes
    let _ = file.take(4 * 1024 * 1024).read_to_end(&mut buf);

    if buf.is_empty() {
        return Err("HWP file is empty".to_string());
    }

    let mut result_text = String::new();

    // 1. Decompress zlib streams safely (HWP 5.0 BodyText streams use zlib header 0x78)
    let mut decompressed_blocks = Vec::new();
    let mut i = 0;
    while i + 2 < buf.len() {
        if buf[i] == 0x78 && (buf[i+1] == 0x9c || buf[i+1] == 0x01 || buf[i+1] == 0xda) {
            use flate2::read::ZlibDecoder;
            let mut decoder = ZlibDecoder::new(&buf[i..]);
            let mut out = Vec::new();
            if decoder.read_to_end(&mut out).is_ok() && out.len() > 16 {
                decompressed_blocks.extend_from_slice(&out);
                if decompressed_blocks.len() >= 2 * 1024 * 1024 {
                    break; // Hard cap total decompressed buffer to 2MB
                }
            }
            i += 64; // Jump forward to avoid redundant overlapping decompressions
        } else {
            i += 1;
        }
    }

    let mut buffers_to_scan = Vec::new();
    if !decompressed_blocks.is_empty() {
        buffers_to_scan.push(decompressed_blocks);
    }
    buffers_to_scan.push(buf.clone());

    for data in &buffers_to_scan {
        // Pass A: UTF-16LE scan
        let mut idx = 0;
        let mut temp_utf16 = Vec::new();
        while idx + 1 < data.len() {
            let code = u16::from_le_bytes([data[idx], data[idx+1]]);
            if (code >= 0xAC00 && code <= 0xD7A3) || (code >= 0x20 && code <= 0x7E) || code == 0x0A || code == 0x0D {
                if let Some(ch) = char::from_u32(code as u32) {
                    temp_utf16.push(ch);
                }
                idx += 2;
            } else {
                if temp_utf16.len() >= 2 {
                    let s: String = temp_utf16.iter().collect();
                    let trimmed = s.trim();
                    if !trimmed.is_empty() {
                        result_text.push_str(trimmed);
                        result_text.push(' ');
                        if result_text.len() >= 200_000 {
                            break;
                        }
                    }
                }
                temp_utf16.clear();
                idx += 2; // Fixed 2-byte alignment for UTF-16LE
            }
        }
        if temp_utf16.len() >= 2 {
            let s: String = temp_utf16.iter().collect();
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                result_text.push_str(trimmed);
                result_text.push(' ');
            }
        }

        if result_text.len() >= 200_000 {
            break;
        }

        // Pass B: EUC-KR / UTF-8 fallback scan if UTF-16 was short
        if result_text.len() < 10 {
            let (cow, _, _) = encoding_rs::EUC_KR.decode(data);
            let euc_kr_str = cow.into_owned();
            for chunk in euc_kr_str.split(|c: char| !c.is_alphanumeric() && !c.is_whitespace()) {
                let trimmed = chunk.trim();
                if trimmed.chars().count() >= 2 {
                    result_text.push_str(trimmed);
                    result_text.push(' ');
                    if result_text.len() >= 200_000 {
                        break;
                    }
                }
            }
        }
    }

    if result_text.trim().is_empty() {
        extract_text_from_binary(&path)
    } else {
        Ok(result_text)
    }
}

pub fn extract_text<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let path_ref = path.as_ref();
    let ext = path_ref.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
        
    let path_buf = path_ref.to_path_buf();

    // 1. Try plugin loading first for specific high-performance target formats (e.g. dwg)
    if (ext == "dwg" || ext == "dxf") && ENABLE_DWG_PLUGIN.load(Ordering::Relaxed) {
        if let Some(text) = try_extract_via_plugin(&path_buf, "dwg_parser") {
            return Ok(text);
        }
    }
    
    let result = std::panic::catch_unwind(move || {
        match ext.as_str() {
            "pdf" => extract_text_from_pdf(&path_buf),
            "docx" => extract_text_from_docx(&path_buf),
            "xlsx" => extract_text_from_xlsx(&path_buf),
            "pptx" => extract_text_from_pptx(&path_buf),
            "hwpx" => extract_text_from_hwpx(&path_buf),
            "hwp" => extract_text_from_hwp(&path_buf),
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

    #[test]
    fn test_extract_hwp_mock() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_doc.hwp");
        
        // Construct UTF-16LE Korean bytes for "하카르한글테스트"
        let korean_str = "하카르한글테스트";
        let utf16_bytes: Vec<u8> = korean_str.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        std::fs::write(&test_file, &utf16_bytes).unwrap();
        
        let text = extract_text(&test_file).unwrap();
        assert!(text.contains("하카르"), "Extracted HWP text was: '{}'", text);
        
        let _ = std::fs::remove_file(test_file);
    }
}

