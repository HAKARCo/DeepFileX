#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(clippy::type_complexity, clippy::collapsible_if)]

use eframe::egui;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Instant;
use std::io::Read;
use deepfilex::db::{Database, SearchFilter};

mod win_shell;


struct DeepFileXApp {
    search_query: String,
    status_message: String,
    results: Arc<Mutex<Vec<(i64, String, bool, u64, u64)>>>,
    is_indexing: Arc<Mutex<bool>>,
    indexed_count: Arc<Mutex<usize>>,
    db: Arc<Mutex<Database>>,
    
    // Target Path & Previews
    scan_target_path: String,
    selected_file_id: Option<i64>,
    selected_file_path: Option<String>,
    selected_file_content: Option<String>,
    selected_file_size: Option<u64>,
    selected_file_mtime: Option<u64>,
    selected_file_is_dir: Option<bool>,

    
    // Everything-style high speed memory index & Active Search Session
    memory_index: Arc<Mutex<Vec<(i64, String, String, bool, u64, u64)>>>,
    search_file_name: bool,
    search_content: bool,
    active_live_query: Arc<Mutex<String>>,
    live_search_results: Arc<Mutex<Vec<(i64, String, bool, u64, u64)>>>,

    sort_ascending: bool,

    // Index Manager Load History
    loaded_indices: Vec<String>,
    was_indexing: bool,
    is_saving_index: Arc<Mutex<bool>>,
    was_saving_index: bool,

    // Debounce fields for Live Search
    last_input_time: Instant,
    pending_live_search: bool,

    // On-demand Content Indexing state fields
    is_content_indexing: Arc<Mutex<bool>>,
    content_indexed_count: Arc<Mutex<usize>>,
    content_total_count: Arc<Mutex<usize>>,
    content_indexing_path: String,
    show_backup_prompt: Arc<Mutex<Option<String>>>,
    selected_indices: std::collections::HashSet<String>,
    show_delete_confirm: Option<Vec<String>>,
    is_content_indexing_cancelled: Arc<AtomicBool>,

    // Advanced Filters fields
    filter_min_size: String,
    filter_max_size: String,
    filter_date_range: usize,
    filter_extensions: String,

    // Sorting fields
    sort_column: usize, // 0 = Name, 1 = Type, 2 = Size, 3 = Modified Date

    // Auto-update fields
    update_config: deepfilex::update::UpdateConfig,
    update_context: Arc<Mutex<deepfilex::update::UpdateContext>>,
    show_about: bool,
    show_update_settings: bool,
    show_plugins_settings: bool,
}

fn parse_size_limit(s: &str) -> Option<u64> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }
    let mut num_str = s.as_str();
    let mut multiplier = 1u64;
    if s.ends_with("kb") || s.ends_with("k") {
        multiplier = 1024;
        num_str = &s[..s.len() - if s.ends_with("kb") { 2 } else { 1 }];
    } else if s.ends_with("mb") || s.ends_with("m") {
        multiplier = 1024 * 1024;
        num_str = &s[..s.len() - if s.ends_with("mb") { 2 } else { 1 }];
    } else if s.ends_with("gb") || s.ends_with("g") {
        multiplier = 1024 * 1024 * 1024;
        num_str = &s[..s.len() - if s.ends_with("gb") { 2 } else { 1 }];
    }
    num_str.trim().parse::<f64>().ok().map(|n| (n * multiplier as f64) as u64)
}

#[allow(dead_code, unused_variables)]
impl DeepFileXApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let db = Database::open("turbo_search.db").expect("Failed to open SQLite database");
        setup_custom_fonts(&_cc.egui_ctx);

        let db_arc = Arc::new(Mutex::new(db));
        let memory_index = Arc::new(Mutex::new(Vec::new()));
        let live_search_results = Arc::new(Mutex::new(Vec::new()));
        
        let mut app = Self {
            search_query: String::new(),
            status_message: "Ready to search.".to_string(),
            results: Arc::new(Mutex::new(Vec::new())),
            is_indexing: Arc::new(Mutex::new(false)),
            indexed_count: Arc::new(Mutex::new(0)),
            db: db_arc,
            scan_target_path: "C:\\".to_string(),
            selected_file_id: None,
            selected_file_path: None,
            selected_file_content: None,
            selected_file_size: None,
            selected_file_mtime: None,
            selected_file_is_dir: None,
            memory_index,
            search_file_name: true,
            search_content: false,
            active_live_query: Arc::new(Mutex::new(String::new())),
            live_search_results,
            sort_ascending: true,
            loaded_indices: Vec::new(),
            was_indexing: false,
            is_saving_index: Arc::new(Mutex::new(false)),
            was_saving_index: false,
            last_input_time: Instant::now(),
            pending_live_search: false,
            is_content_indexing: Arc::new(Mutex::new(false)),
            content_indexed_count: Arc::new(Mutex::new(0)),
            content_total_count: Arc::new(Mutex::new(0)),
            content_indexing_path: String::new(),
            show_backup_prompt: Arc::new(Mutex::new(None)),
            selected_indices: std::collections::HashSet::new(),
            show_delete_confirm: None,
            is_content_indexing_cancelled: Arc::new(AtomicBool::new(false)),
            filter_min_size: String::new(),
            filter_max_size: String::new(),
            filter_date_range: 0,
            filter_extensions: String::new(),
            sort_column: 0,
            update_config: deepfilex::update::UpdateConfig::load(),
            update_context: Arc::new(Mutex::new(deepfilex::update::UpdateContext::default())),
            show_about: false,
            show_update_settings: false,
            show_plugins_settings: false,
        };

        app.refresh_loaded_indices();
        
        if app.update_config.enabled && app.update_config.auto_check {
            deepfilex::update::trigger_update_check(app.update_context.clone(), app.update_config.clone());
        }
        
        app.start_indexing(&_cc.egui_ctx);
        app
    }

    fn refresh_loaded_indices(&mut self) {
        let home_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
        let default_dir = std::path::PathBuf::from(home_dir)
            .join("Documents")
            .join("DeepFileX")
            .join("Indices");

        // Ensure the directory exists
        let _ = std::fs::create_dir_all(&default_dir);

        let mut list = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&default_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("db") {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        list.push(filename.to_string());
                    }
                }
            }
        }
        // Sort indices descending (newest YYMMDD_HHMM first)
        list.sort_by(|a, b| b.cmp(a));
        self.loaded_indices = list;
    }

    fn start_indexing(&self, ctx: &egui::Context) {
        let is_indexing = self.is_indexing.clone();
        let indexed_count = self.indexed_count.clone();
        let db = self.db.clone();
        let target_path = self.scan_target_path.clone();
        let memory_index = self.memory_index.clone();
        let ctx_clone = ctx.clone();

        *is_indexing.lock().unwrap_or_else(|e| e.into_inner()) = true;

        thread::spawn(move || {
            let start = Instant::now();
            let mut count = 0;

            let is_drive_root = target_path.len() <= 3 && target_path.contains(':');
            let mut mft_success = false;
            let mut scanned_items = Vec::new();

            if is_drive_root {
                let drive_letter = target_path.chars().next().unwrap_or('C');
                let volume_path = format!("\\\\.\\{}:", drive_letter);
                let drive_prefix = format!("{}:\\", drive_letter);

                println!("Attempting NTFS MFT Scan on {}", volume_path);
                match deepfilex::scan_volume(&volume_path) {
                    Ok(entries) => {
                        mft_success = true;
                        println!("MFT scan successful. Memory collecting {} entries...", entries.len());
                        scanned_items = Vec::with_capacity(entries.len());
                        for (_file_id, rel_path, is_dir) in entries {
                            let full_path = format!("{}{}", drive_prefix, rel_path);
                            
                            // Path::new 디바이스 매핑을 우회하고 백슬래시(\) 기준으로 고속 파일명 슬라이싱
                            let file_name = match rel_path.rfind('\\') {
                                Some(idx) => &rel_path[idx + 1..],
                                None => &rel_path[..],
                            };
                            let file_name_lower = file_name.to_lowercase();
                            
                            scanned_items.push((-1, full_path, file_name_lower, is_dir, 0, 0));
                            count += 1;
                            if count % 50000 == 0 { // Repaint 빈도수 완화로 GUI 락 경합 완화
                                *indexed_count.lock().unwrap_or_else(|e| e.into_inner()) = count;
                                ctx_clone.request_repaint();
                            }
                        }
                    }
                    Err(e) => {
                        println!("MFT scan failed: {}. Falling back to walkdir.", e);
                    }
                }
            }

            if !mft_success {
                println!("Running walkdir scanner on {}", target_path);
                let walker = walkdir::WalkDir::new(&target_path)
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
                    let file_name_lower = entry.file_name().to_string_lossy().to_lowercase();
                    
                    // MFT 실패 walkdir 폴백 시 5초 내 완료를 위해 개별 파일 디스크 metadata() 시스템 콜 호출 생략 (I/O 병목 완전 제거)
                    scanned_items.push((-1, path_str, file_name_lower, is_dir, 0, 0));
                    count += 1;
                    if count % 10000 == 0 {
                        *indexed_count.lock().unwrap_or_else(|e| e.into_inner()) = count;
                        ctx_clone.request_repaint();
                    }
                }
            }

            // Update RAM cache instantly (Everything-style immediate search activation)
            {
                let mut mem_lock = memory_index.lock().unwrap_or_else(|e| e.into_inner());
                *mem_lock = scanned_items;
            }
            ctx_clone.request_repaint();

            *is_indexing.lock().unwrap_or_else(|e| e.into_inner()) = false;
            *indexed_count.lock().unwrap_or_else(|e| e.into_inner()) = count;
            ctx_clone.request_repaint(); // Final notify UI thread of indexing complete
            println!("Indexing and DB sync completed. Total {} files in {:?}", count, start.elapsed());
        });
    }


    fn trigger_live_search(&self, ctx: &egui::Context) {
        let query = self.search_query.clone();
        if query.trim().is_empty() {
            return;
        }
        let active_query = self.active_live_query.clone();
        let results = self.live_search_results.clone();
        let target_path = self.scan_target_path.clone();
        let query_str = query.clone();
        let ctx_clone = ctx.clone();

        *active_query.lock().unwrap_or_else(|e| e.into_inner()) = query_str.clone();

        thread::spawn(move || {
            let walker = walkdir::WalkDir::new(&target_path)
                .into_iter()
                .filter_entry(|e| {
                    let name = e.file_name().to_string_lossy();
                    !name.starts_with('.') && name != "System Volume Information" && name != "$RECYCLE.BIN" && name != "Windows"
                });

            let mut local_matches = Vec::new();
            let query_lower = query_str.to_lowercase();

            for entry in walker.filter_map(|e| e.ok()) {
                if *active_query.lock().unwrap_or_else(|e| e.into_inner()) != query_str {
                    return; // Query changed, abort thread
                }

                let path_str = entry.path().to_string_lossy().to_string();
                let file_name_lower = entry.file_name().to_string_lossy().to_lowercase();
                
                if file_name_lower.contains(&query_lower) {
                    let is_dir = entry.file_type().is_dir();
                    let metadata = entry.metadata().ok();
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                    let mtime = metadata.as_ref().and_then(|m| m.modified().ok()).and_then(|t| {
                        t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok().map(|d| d.as_secs())
                    }).unwrap_or(0);

                    local_matches.push((-1, path_str, is_dir, size, mtime));

                    if local_matches.len() >= 100 {
                        break;
                    }
                }
            }

            if *active_query.lock().unwrap_or_else(|e| e.into_inner()) == query_str {
                let mut res_lock = results.lock().unwrap_or_else(|e| e.into_inner());
                res_lock.clear(); // Clear previous results ONLY when new results are ready
                for item in local_matches {
                    if !res_lock.iter().any(|r| r.1 == item.1) {
                        res_lock.push(item);
                    }
                }
                ctx_clone.request_repaint();
            }
        });
    }

    fn search(&mut self, ctx: &egui::Context) {
        let query = self.search_query.clone();

        if !self.search_file_name && !self.search_content {
            self.results.lock().unwrap_or_else(|e| e.into_inner()).clear();
            self.live_search_results.lock().unwrap_or_else(|e| e.into_inner()).clear();
            self.status_message = "Please select at least one search target (File or Content).".to_string();
            *self.active_live_query.lock().unwrap_or_else(|e| e.into_inner()) = String::new();
            self.pending_live_search = false;
            return;
        }

        // Build SearchFilter
        let min_mtime = match self.filter_date_range {
            1 => Some(chrono::Utc::now().timestamp() as u64 - 86400),
            2 => Some(chrono::Utc::now().timestamp() as u64 - 7 * 86400),
            3 => Some(chrono::Utc::now().timestamp() as u64 - 30 * 86400),
            _ => None,
        };

        let extensions = if self.filter_extensions.trim().is_empty() {
            None
        } else {
            Some(self.filter_extensions.split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>())
        };

        let filter = SearchFilter {
            min_size: parse_size_limit(&self.filter_min_size),
            max_size: parse_size_limit(&self.filter_max_size),
            min_mtime,
            extensions,
        };

        // Resolve full paths of selected external database files
        let home_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
        let default_dir = std::path::PathBuf::from(home_dir)
            .join("Documents")
            .join("DeepFileX")
            .join("Indices");

        let mut external_dbs = Vec::new();
        for filename in &self.selected_indices {
            let db_path = default_dir.join(filename).to_string_lossy().to_string();
            external_dbs.push(db_path);
        }

        let has_external_selection = !external_dbs.is_empty();
        let mut final_results = Vec::new();
        let mem = self.memory_index.lock().unwrap_or_else(|e| e.into_inner());

        if query.trim().is_empty() {
            if self.search_file_name {
                if has_external_selection {
                    // Query external DBs for default listing
                    for db_path in &external_dbs {
                        let p = std::path::Path::new(db_path);
                        if let Ok(conn) = rusqlite::Connection::open(p) {
                            let mut query_str = "SELECT file_id, file_path, is_dir, file_size, mtime FROM FILES WHERE 1=1".to_string();
                            let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
                            if let Some(min) = filter.min_size {
                                query_str.push_str(" AND file_size >= ?");
                                params.push(Box::new(min));
                            }
                            if let Some(max) = filter.max_size {
                                query_str.push_str(" AND file_size <= ?");
                                params.push(Box::new(max));
                            }
                            if let Some(min_t) = filter.min_mtime {
                                query_str.push_str(" AND mtime >= ?");
                                params.push(Box::new(min_t));
                            }
                            if let Some(ref exts) = filter.extensions {
                                if !exts.is_empty() {
                                    for ext in exts {
                                        query_str.push_str(" AND file_path LIKE ?");
                                        params.push(Box::new(format!("%.{}", ext.to_lowercase())));
                                    }
                                }
                            }
                            query_str.push_str(" LIMIT 200");
                            if let Ok(mut stmt) = conn.prepare(&query_str) {
                                let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
                                if let Ok(rows) = stmt.query_map(&params_ref[..], |row| {
                                    Ok((
                                        row.get(0)?,
                                        row.get(1)?,
                                        row.get::<_, i32>(2)? == 1,
                                        row.get(3)?,
                                        row.get(4)?,
                                    ))
                                }) {
                                    for r in rows {
                                        if let Ok(item) = r {
                                            final_results.push(item);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    final_results.truncate(200);
                } else {
                    for item in mem.iter() {
                        if let Some(min) = filter.min_size {
                            if item.4 < min { continue; }
                        }
                        if let Some(max) = filter.max_size {
                            if item.4 > max { continue; }
                        }
                        if let Some(min_t) = filter.min_mtime {
                            if item.5 < min_t { continue; }
                        }
                        if let Some(ref exts) = filter.extensions {
                            if !exts.is_empty() {
                                let ext_matched = exts.iter().any(|ext| item.1.to_lowercase().ends_with(&format!(".{}", ext)));
                                if !ext_matched { continue; }
                            }
                        }
                        final_results.push((item.0, item.1.clone(), item.3, item.4, item.5));
                        if final_results.len() >= 200 {
                            break;
                        }
                    }
                }
            }
            *self.results.lock().unwrap_or_else(|e| e.into_inner()) = final_results;
            self.status_message = "Ready to search.".to_string();
            return;
        }

        // 1. Search File Names (Sync & Instantly on Memory OR Query from selected DBs)
        if self.search_file_name {
            if has_external_selection {
                let query_lower = query.to_lowercase();
                let like_query = format!("%{}%", query_lower);
                for db_path in &external_dbs {
                    let p = std::path::Path::new(db_path);
                    if let Ok(conn) = rusqlite::Connection::open(p) {
                        let mut query_str = "SELECT file_id, file_path, is_dir, file_size, mtime FROM FILES WHERE REPLACE(LOWER(file_path), '/', '\\') LIKE ?".to_string();
                        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(like_query.clone())];
                        if let Some(min) = filter.min_size {
                            query_str.push_str(" AND file_size >= ?");
                            params.push(Box::new(min));
                        }
                        if let Some(max) = filter.max_size {
                            query_str.push_str(" AND file_size <= ?");
                            params.push(Box::new(max));
                        }
                        if let Some(min_t) = filter.min_mtime {
                            query_str.push_str(" AND mtime >= ?");
                            params.push(Box::new(min_t));
                        }
                        if let Some(ref exts) = filter.extensions {
                            if !exts.is_empty() {
                                for ext in exts {
                                    query_str.push_str(" AND file_path LIKE ?");
                                    params.push(Box::new(format!("%.{}", ext.to_lowercase())));
                                }
                            }
                        }
                        query_str.push_str(" LIMIT 200");
                        if let Ok(mut stmt) = conn.prepare(&query_str) {
                            let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
                            if let Ok(rows) = stmt.query_map(&params_ref[..], |row| {
                                Ok((
                                    row.get(0)?,
                                    row.get(1)?,
                                    row.get::<_, i32>(2)? == 1,
                                    row.get(3)?,
                                    row.get(4)?,
                                ))
                            }) {
                                for r in rows {
                                    if let Ok(item) = r {
                                        final_results.push(item);
                                    }
                                }
                            }
                        }
                    }
                }
                final_results.truncate(200);
            } else {
                let query_lower = query.to_lowercase();
                for item in mem.iter() {
                    if item.2.contains(&query_lower) {
                        if let Some(min) = filter.min_size {
                            if item.4 < min { continue; }
                        }
                        if let Some(max) = filter.max_size {
                            if item.4 > max { continue; }
                        }
                        if let Some(min_t) = filter.min_mtime {
                            if item.5 < min_t { continue; }
                        }
                        if let Some(ref exts) = filter.extensions {
                            if !exts.is_empty() {
                                let ext_matched = exts.iter().any(|ext| item.1.to_lowercase().ends_with(&format!(".{}", ext)));
                                if !ext_matched { continue; }
                            }
                        }
                        final_results.push((item.0, item.1.clone(), item.3, item.4, item.5));
                        if final_results.len() >= 200 {
                            break;
                        }
                    }
                }
            }
            *self.results.lock().unwrap_or_else(|e| e.into_inner()) = final_results;
            self.status_message = format!("Index Search: Found {} results.", self.results.lock().unwrap().len());
        }

        // 2. Search File Contents (Async via background thread to prevent UI IME freezing!)
        if self.search_content {
            let db_arc = self.db.clone();
            let results_arc = self.results.clone();
            let active_query = self.active_live_query.clone();
            let query_str = query.clone();
            let ctx_clone = ctx.clone();
            let filter_clone = filter.clone();

            *active_query.lock().unwrap_or_else(|e| e.into_inner()) = query_str.clone();

            thread::spawn(move || {
                if let Ok(db) = db_arc.lock() {
                    if let Ok(content_res) = db.search_files_by_content(&query_str, &filter_clone, 200, &external_dbs) {
                        if *active_query.lock().unwrap_or_else(|e| e.into_inner()) == query_str {
                            let mut res_lock = results_arc.lock().unwrap_or_else(|e| e.into_inner());
                            *res_lock = content_res;
                            ctx_clone.request_repaint();
                        }
                    }
                }
            });
            self.status_message = "Searching database contents asynchronously...".to_string();
        }
    }






    fn start_content_indexing(&self, ctx: &egui::Context) {
        let target_folder = self.content_indexing_path.trim().to_string();
        if target_folder.is_empty() {
            return;
        }
        let is_indexing = self.is_content_indexing.clone();
        let indexed_count = self.content_indexed_count.clone();
        let total_count = self.content_total_count.clone();
        let show_backup_prompt = self.show_backup_prompt.clone();
        let is_indexing_cancelled = self.is_content_indexing_cancelled.clone();
        let ctx_clone = ctx.clone();

        is_indexing_cancelled.store(false, Ordering::SeqCst);
        *is_indexing.lock().unwrap_or_else(|e| e.into_inner()) = true;

        thread::spawn(move || {
            if let Ok(bg_db) = Database::open("turbo_search.db") {
                let _ = bg_db.clear_all_data();
                println!("Content indexing started for folder: {}", target_folder);
                
                // 1. Gather target files directly using walkdir
                let mut target_files = Vec::new();
                let walker = walkdir::WalkDir::new(&target_folder)
                    .into_iter()
                    .filter_entry(|e| {
                        let path = e.path();
                        if path.is_dir() {
                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                if name.starts_with('.') || name == "System Volume Information" || name == "$RECYCLE.BIN" {
                                    return false;
                                }
                            }
                        }
                        true
                    });

                for entry in walker.filter_map(Result::ok) {
                    if is_indexing_cancelled.load(Ordering::SeqCst) {
                        break;
                    }
                    let is_dir = entry.file_type().is_dir();
                    if !is_dir {
                        let file_path = entry.path().to_string_lossy().into_owned();
                        let path_lower = file_path.to_lowercase();
                        let ext = std::path::Path::new(&path_lower)
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("");
                        
                        if matches!(ext, "pdf" | "docx" | "xlsx" | "txt" | "csv" | "log" | "srt" | "vtt" | "md" | "json" | "xml" | "yaml" | "yml" | "ini" | "toml") {
                            let mut size = 0;
                            let mut mtime = 0;
                            if let Ok(meta) = entry.metadata() {
                                size = meta.len();
                                if let Ok(modified) = meta.modified() {
                                    if let Ok(dur) = modified.duration_since(std::time::SystemTime::UNIX_EPOCH) {
                                        mtime = dur.as_secs();
                                    }
                                }
                            }
                            target_files.push((file_path, size, mtime, is_dir));
                        }
                    }
                }

                if !is_indexing_cancelled.load(Ordering::SeqCst) {
                    *total_count.lock().unwrap_or_else(|e| e.into_inner()) = target_files.len();
                    *indexed_count.lock().unwrap_or_else(|e| e.into_inner()) = 0;
                    ctx_clone.request_repaint();

                    let mut count = 0;
                    let mut was_cancelled = false;
                    let mut tx_active = false;
                    
                    for (file_path, file_size, mtime, is_dir) in target_files {
                        if is_indexing_cancelled.load(Ordering::SeqCst) {
                            was_cancelled = true;
                            break;
                        }
                        
                        if count % 100 == 0 {
                            if tx_active {
                                let _ = bg_db.commit_transaction();
                                tx_active = false;
                            }
                            if bg_db.begin_transaction().is_ok() {
                                tx_active = true;
                            }
                        }
                        
                        // 2. Insert into FILES table dynamically to get file_id
                        if let Ok(file_id) = bg_db.insert_file(&file_path, file_size, mtime, is_dir) {
                            if let Ok(exists) = bg_db.has_content_index(file_id) {
                                if exists {
                                    count += 1;
                                    if count % 10 == 0 {
                                        *indexed_count.lock().unwrap_or_else(|e| e.into_inner()) = count;
                                        ctx_clone.request_repaint();
                                    }
                                    continue;
                                }
                            }

                            if let Ok(text) = deepfilex::parser::extract_text(&file_path) {
                                if let Ok(compressed) = deepfilex::compress_data(text.as_bytes(), Some(3)) {
                                    let _ = bg_db.save_content(file_id, &compressed, &text);
                                }
                            }
                        }

                        count += 1;
                        if count % 5 == 0 || count == *total_count.lock().unwrap_or_else(|e| e.into_inner()) {
                            *indexed_count.lock().unwrap_or_else(|e| e.into_inner()) = count;
                            ctx_clone.request_repaint();
                        }
                    }
                    
                    if tx_active {
                        let _ = bg_db.commit_transaction();
                    }
                    
                    if !was_cancelled {
                        let _ = bg_db.insert_scan_history(&target_folder);
                    }
                }
            }

            let cancelled = is_indexing_cancelled.load(Ordering::SeqCst);
            *is_indexing.lock().unwrap_or_else(|e| e.into_inner()) = false;
            if !cancelled {
                *show_backup_prompt.lock().unwrap_or_else(|e| e.into_inner()) = Some(target_folder);
            }
            ctx_clone.request_repaint();
        });
    }

}


fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let font_path = "C:\\Windows\\Fonts\\malgun.ttf";
    if let Ok(mut file) = std::fs::File::open(font_path) {
        let mut buffer = Vec::new();
        if file.read_to_end(&mut buffer).is_ok() {
            fonts.font_data.insert(
                "malgun".to_owned(),
                egui::FontData::from_owned(buffer),
            );
            fonts.families.get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "malgun".to_owned());
            fonts.families.get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .insert(0, "malgun".to_owned());
        }
    }
    ctx.set_fonts(fonts);
}

fn format_size(bytes: u64, is_dir: bool) -> String {
    if is_dir {
        return "".to_string();
    }
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn format_date(timestamp: u64) -> String {
    if timestamp == 0 {
        return "-".to_string();
    }
    use chrono::TimeZone;
    if let Some(dt) = chrono::Local.timestamp_opt(timestamp as i64, 0).single() {
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        "-".to_string()
    }
}

fn force_enable_ime() {
    unsafe {
        let hwnd = winapi::um::winuser::GetActiveWindow();
        if !hwnd.is_null() {
            let module_name: Vec<u16> = "imm32.dll\0".encode_utf16().collect();
            let h_module = winapi::um::libloaderapi::LoadLibraryW(module_name.as_ptr());
            if !h_module.is_null() {
                let associate_name = std::ffi::CString::new("ImmAssociateContext").unwrap();
                let get_context_name = std::ffi::CString::new("ImmGetContext").unwrap();
                let create_context_name = std::ffi::CString::new("ImmCreateContext").unwrap();

                let associate_ptr = winapi::um::libloaderapi::GetProcAddress(h_module, associate_name.as_ptr());
                let get_context_ptr = winapi::um::libloaderapi::GetProcAddress(h_module, get_context_name.as_ptr());
                let create_context_ptr = winapi::um::libloaderapi::GetProcAddress(h_module, create_context_name.as_ptr());

                if !associate_ptr.is_null() && !get_context_ptr.is_null() && !create_context_ptr.is_null() {
                    type ImmAssociateContextFn = unsafe extern "system" fn(winapi::shared::windef::HWND, winapi::shared::windef::HWND) -> winapi::shared::windef::HWND;
                    type ImmGetContextFn = unsafe extern "system" fn(winapi::shared::windef::HWND) -> winapi::shared::windef::HWND;
                    type ImmCreateContextFn = unsafe extern "system" fn() -> winapi::shared::windef::HWND;

                    let associate_context: ImmAssociateContextFn = std::mem::transmute(associate_ptr);
                    let get_context: ImmGetContextFn = std::mem::transmute(get_context_ptr);
                    let create_context: ImmCreateContextFn = std::mem::transmute(create_context_ptr);

                    let current_imc = get_context(hwnd);
                    if current_imc.is_null() {
                        let new_imc = create_context();
                        associate_context(hwnd, new_imc);
                    }
                }
            }
        }
    }
}

impl eframe::App for DeepFileXApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.pending_live_search && self.last_input_time.elapsed().as_millis() >= 250 {
            self.pending_live_search = false;
            self.search(ctx);
        }
        if self.pending_live_search {
            ctx.request_repaint();
        }

        let update_repaint_needed = {
            let lock = self.update_context.lock().unwrap();
            lock.state == deepfilex::update::UpdateState::Checking ||
            lock.state == deepfilex::update::UpdateState::Downloading ||
            lock.state == deepfilex::update::UpdateState::Verifying ||
            lock.state == deepfilex::update::UpdateState::Installing
        };
        if update_repaint_needed {
            ctx.request_repaint();
        }

        #[cfg(target_os = "windows")]
        {
            force_enable_ime();
        }

        // Top Menu Bar
        egui::TopBottomPanel::top("menu_bar_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("📄 File", |ui| {
                    if ui.button("🔄 Refresh Index").clicked() {
                        self.start_indexing(ctx);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("❌ Exit").clicked() {
                        _frame.close();
                        ui.close_menu();
                    }
                });

                ui.menu_button("🔍 Search Mode", |ui| {
                    let mut changed = false;
                    if ui.radio_value(&mut self.search_file_name, true, "Filename Search").clicked() {
                        self.search_content = false;
                        changed = true;
                    }
                    if ui.radio_value(&mut self.search_content, true, "Content Search").clicked() {
                        self.search_file_name = false;
                        changed = true;
                    }
                    if changed {
                        self.last_input_time = std::time::Instant::now();
                        self.pending_live_search = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("🌐 Updates", |ui| {
                    if ui.button("⚙️ Settings...").clicked() {
                        self.show_update_settings = true;
                        ui.close_menu();
                    }
                    if ui.button("🔄 Check for Updates").clicked() {
                        deepfilex::update::trigger_update_check(
                            self.update_context.clone(),
                            self.update_config.clone(),
                        );
                        ui.close_menu();
                    }
                });

                ui.menu_button("🔌 Plugins", |ui| {
                    if ui.button("⚙️ Manage Plugins...").clicked() {
                        self.show_plugins_settings = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("❓ Help", |ui| {
                    if ui.button("ℹ️ About").clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });
            });
        });

        // Plugins Settings Window
        if self.show_plugins_settings {
            let mut show = self.show_plugins_settings;
            let mut close_clicked = false;
            egui::Window::new("🔌 Plugins Manager")
                .open(&mut show)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("Enable or disable dynamic text extraction plugins:").weak());
                    ui.add_space(5.0);

                    // HWP Plugin
                    let mut hwp_active = deepfilex::parser::ENABLE_HWP_PLUGIN.load(std::sync::atomic::Ordering::Relaxed);
                    if ui.checkbox(&mut hwp_active, "Enable HWP Parser").changed() {
                        deepfilex::parser::ENABLE_HWP_PLUGIN.store(hwp_active, std::sync::atomic::Ordering::Relaxed);
                    }
                    let hwp_status = if std::path::Path::new("plugins/hwp_parser.dll").exists() { "Detected" } else { "Not Found" };
                    ui.label(egui::RichText::new(format!("  - Status: {} (plugins/hwp_parser.dll)", hwp_status)).weak().small());

                    ui.add_space(10.0);

                    // DWG Plugin
                    let mut dwg_active = deepfilex::parser::ENABLE_DWG_PLUGIN.load(std::sync::atomic::Ordering::Relaxed);
                    if ui.checkbox(&mut dwg_active, "Enable DWG Parser").changed() {
                        deepfilex::parser::ENABLE_DWG_PLUGIN.store(dwg_active, std::sync::atomic::Ordering::Relaxed);
                    }
                    let dwg_status = if std::path::Path::new("plugins/dwg_parser.dll").exists() { "Detected" } else { "Not Found" };
                    ui.label(egui::RichText::new(format!("  - Status: {} (plugins/dwg_parser.dll)", dwg_status)).weak().small());

                    ui.separator();
                    ui.vertical_centered(|ui| {
                        if ui.button("Close").clicked() {
                            close_clicked = true;
                        }
                    });
                });
            self.show_plugins_settings = show && !close_clicked;
        }

        // About Dialog Window
        if self.show_about {
            let mut show = self.show_about;
            let mut close_clicked = false;
            egui::Window::new("ℹ️ About DeepFileX")
                .open(&mut show)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("DeepFileX v3.2.0");
                        ui.label("Ultra-fast NTFS file and content search engine based on Rust & egui");
                        ui.add_space(10.0);
                        ui.label("Developer: HAKAR");
                        ui.label("© 2026 HAKAR. All rights reserved.");
                        ui.add_space(15.0);
                        if ui.button("Close").clicked() {
                            close_clicked = true;
                        }
                    });
                });
            self.show_about = show && !close_clicked;
        }

        // Auto Update Settings Window
        if self.show_update_settings {
            let mut show = self.show_update_settings;
            egui::Window::new("🔄 Auto Update Settings")
                .open(&mut show)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    let mut config = self.update_config.clone();
                    let mut config_changed = false;

                    if ui.checkbox(&mut config.enabled, "Enable Auto Update").changed() {
                        config_changed = true;
                    }

                    ui.add_enabled_ui(config.enabled, |ui| {
                        if ui.checkbox(&mut config.auto_check, "Auto check periodically").changed() {
                            config_changed = true;
                        }
                        ui.add_enabled_ui(config.auto_check, |ui| {
                            if ui.checkbox(&mut config.auto_download, "Auto download updates").changed() {
                                config_changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Channel:");
                            let mut selected_channel = config.channel;
                            let channel_res = egui::ComboBox::from_label("")
                                .selected_text(match selected_channel {
                                    deepfilex::update::Channel::Stable => "Stable (Production)",
                                    deepfilex::update::Channel::Beta => "Beta (Release Candidate)",
                                    deepfilex::update::Channel::Nightly => "Nightly (Development)",
                                })
                                .show_ui(ui, |ui| {
                                    let mut ch = false;
                                    ch |= ui.selectable_value(&mut selected_channel, deepfilex::update::Channel::Stable, "Stable").changed();
                                    ch |= ui.selectable_value(&mut selected_channel, deepfilex::update::Channel::Beta, "Beta").changed();
                                    ch |= ui.selectable_value(&mut selected_channel, deepfilex::update::Channel::Nightly, "Nightly").changed();
                                    ch
                                });
                            if channel_res.inner.unwrap_or(false) {
                                config.channel = selected_channel;
                                config_changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Check Interval (hours):");
                            let mut val = config.check_interval_hours;
                            let slider = ui.add(egui::Slider::new(&mut val, 1..=168));
                            if slider.changed() {
                                config.check_interval_hours = val;
                                config_changed = true;
                            }
                        });
                    });

                    if config_changed {
                        config.save();
                        self.update_config = config;
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        let checking = {
                            let lock = self.update_context.lock().unwrap();
                            lock.state == deepfilex::update::UpdateState::Checking ||
                            lock.state == deepfilex::update::UpdateState::Downloading ||
                            lock.state == deepfilex::update::UpdateState::Verifying ||
                            lock.state == deepfilex::update::UpdateState::Installing
                        };
                        
                        if ui.add_enabled(!checking, egui::Button::new("Check Now")).clicked() {
                            deepfilex::update::trigger_update_check(
                                self.update_context.clone(),
                                self.update_config.clone(),
                            );
                        }
                        
                        let (state_text, progress, err_msg) = {
                            let lock = self.update_context.lock().unwrap();
                            let progress = if lock.total_bytes > 0 {
                                Some(lock.downloaded_bytes as f32 / lock.total_bytes as f32)
                            } else {
                                None
                            };
                            (lock.state.as_str().to_string(), progress, lock.error_message.clone())
                        };
                        
                        ui.label(format!("Status: {}", state_text));
                        if let Some(pct) = progress {
                            ui.add(egui::ProgressBar::new(pct).text(format!("{:.1}%", pct * 100.0)));
                        }
                        if let Some(err) = err_msg {
                            ui.colored_label(egui::Color32::from_rgb(220, 80, 80), format!("Error: {}", err));
                        }
                    });
                });
            self.show_update_settings = show;
        }

        // Detect background indexing thread completion to refresh loaded paths
        let currently_indexing = *self.is_indexing.lock().unwrap_or_else(|e| e.into_inner());
        if self.was_indexing && !currently_indexing {
            self.refresh_loaded_indices();
        }
        self.was_indexing = currently_indexing;

        // Detect background database saving completion
        let currently_saving = *self.is_saving_index.lock().unwrap_or_else(|e| e.into_inner());
        if self.was_saving_index && !currently_saving {
            self.refresh_loaded_indices();
        }
        self.was_saving_index = currently_saving;
        
        // Left SidePanel for Content Index Selection (Only shown when search_content is true)
        if self.search_content {
            egui::SidePanel::left("index_sidebar")
                .resizable(true)
                .default_width(220.0)
                .width_range(180.0..=300.0)
                .show(ctx, |ui| {
                    ui.add_space(10.0);
                    ui.heading("📂 Index Manager");
                    ui.add_space(5.0);

                    // 1. Load External Index File Button
                    if ui.button("➕ Load Index File").clicked() {
                        let home_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
                        let default_dir = std::path::PathBuf::from(home_dir)
                            .join("Documents")
                            .join("DeepFileX")
                            .join("Indices");

                        if let Some(path) = rfd::FileDialog::new()
                            .set_directory(&default_dir)
                            .add_filter("SQLite Database", &["db"])
                            .pick_file() 
                        {
                            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                                let filename_str = filename.to_string();
                                
                                // Copy file to standard Indice folder if external
                                let dest_path = default_dir.join(&filename_str);
                                if path != dest_path && !dest_path.exists() {
                                    let _ = std::fs::copy(&path, &dest_path);
                                }

                                if !self.loaded_indices.contains(&filename_str) {
                                    self.loaded_indices.push(filename_str.clone());
                                }
                                self.selected_indices.insert(filename_str);
                                self.search(ctx);
                                self.status_message = format!("Loaded index: {}", filename);
                            }
                        }
                    }
                    ui.separator();

                    ui.label("Select active index scope:");
                    ui.add_space(3.0);

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // Option A: Default Main Index (Clears selection)
                        let is_default_selected = self.selected_indices.is_empty();
                        let default_text = if is_default_selected {
                            "🔘 [Full MFT Index (Default)]"
                        } else {
                            "⚪ [Full MFT Index (Default)]"
                        };
                        
                        if ui.selectable_label(is_default_selected, default_text).clicked() {
                            self.selected_indices.clear();
                            self.search(ctx);
                            self.status_message = "Switched to default full index.".to_string();
                        }
                        
                        ui.add_space(5.0);

                        // Option B: Individual exported index files (Multiple Checkboxes)
                        let loaded = self.loaded_indices.clone();
                        for filename in loaded {
                            let mut is_checked = self.selected_indices.contains(&filename);
                            let clean_name = filename.strip_suffix(".db").unwrap_or(&filename);
                            
                            if ui.checkbox(&mut is_checked, clean_name).changed() {
                                if is_checked {
                                    self.selected_indices.insert(filename.clone());
                                } else {
                                    self.selected_indices.remove(&filename);
                                }
                                self.search(ctx);
                            }
                        }

                        ui.add_space(10.0);
                        let has_selection = !self.selected_indices.is_empty();
                        let delete_btn = egui::Button::new("🗑 Delete");
                        ui.add_enabled_ui(has_selection, |ui| {
                            if ui.add(delete_btn).clicked() {
                                let targets: Vec<String> = self.selected_indices.iter().cloned().collect();
                                self.show_delete_confirm = Some(targets);
                            }
                        });
                    });
                });
        }       // Bottom Status Bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(ref path) = self.selected_file_path {
                    let size = self.selected_file_size.unwrap_or(0);
                    let mtime = self.selected_file_mtime.unwrap_or(0);
                    let is_dir = self.selected_file_is_dir.unwrap_or(false);
                    
                    let size_str = format_size(size, is_dir);
                    let date_str = format_date(mtime);
                    
                    ui.label(egui::RichText::new(format!("Location: {}  |  Size: {}  |  Modified: {}", path, size_str, date_str)).strong());
                } else if self.selected_file_id.is_some() {
                    let path = self.selected_file_path.clone().unwrap_or_default();
                    let size = self.selected_file_size.unwrap_or(0);
                    let mtime = self.selected_file_mtime.unwrap_or(0);
                    let is_dir = self.selected_file_is_dir.unwrap_or(false);
                    
                    let size_str = format_size(size, is_dir);
                    let date_str = format_date(mtime);
                    
                    ui.label(egui::RichText::new(format!("Location: {}  |  Size: {}  |  Modified: {}", path, size_str, date_str)).strong());
                } else {
                    ui.label("No file selected.");
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if currently_indexing {
                        ui.label(format!("Scanning... ({} files indexed)", *self.indexed_count.lock().unwrap_or_else(|e| e.into_inner())));
                    } else {
                        ui.label("Ready");
                    }
                });
            });
        });



        // Right SidePanel for Content Preview
        if self.search_content && self.selected_file_path.is_some() {
            let path = self.selected_file_path.as_ref().unwrap();
            egui::SidePanel::right("preview_sidebar")
                .resizable(true)
                .default_width(320.0)
                .width_range(200.0..=600.0)
                .show(ctx, |ui| {
                    ui.add_space(10.0);
                    ui.heading("📄 File Preview");
                    ui.separator();
                    
                    let filename = std::path::Path::new(path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(path.as_str());
                    ui.label(egui::RichText::new(filename).strong());
                    
                    if let Some(ref content) = self.selected_file_content {
                        let query = self.search_query.trim();
                        if !query.is_empty() {
                            let content_lower = content.to_lowercase();
                            let query_lower = query.to_lowercase();
                            let occurrences = content_lower.matches(&query_lower).count();
                            ui.colored_label(egui::Color32::from_rgb(255, 140, 0), format!("🔍 Found {} time(s)", occurrences));
                        }
                    }
                    ui.add_space(5.0);

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if let Some(ref content) = self.selected_file_content {
                            let query = self.search_query.trim();
                            if !query.is_empty() {
                                let content_lower = content.to_lowercase();
                                let query_lower = query.to_lowercase();
                                
                                if let Some(idx) = content_lower.find(&query_lower) {
                                    let start = idx.saturating_sub(100);
                                    let end = std::cmp::min(content.len(), idx + query.len() + 100);
                                    
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing.x = 0.0;
                                        
                                        if start > 0 {
                                            ui.label("... ");
                                        }
                                        ui.label(&content[start..idx]);
                                        
                                        ui.label(egui::RichText::new(&content[idx..idx + query.len()])
                                            .strong()
                                            .background_color(egui::Color32::from_rgb(255, 140, 0))
                                            .color(egui::Color32::WHITE));
                                        
                                        ui.label(&content[idx + query.len()..end]);
                                        if end < content.len() {
                                            ui.label(" ...");
                                        }
                                    });
                                } else {
                                    ui.label(content);
                                }
                            } else {
                                ui.label(content);
                            }
                        } else {
                            ui.label(egui::RichText::new("(No preview available or file has no content index)").italics());
                        }
                    });
                });
        }

        // Central Panel: Results
        egui::CentralPanel::default().show(ctx, |ui| {
            let admin_status = if is_user_admin() { " [Admin]" } else { " [Non-Admin]" };
            ui.heading(format!("DeepFileX v3.1.0 - Rust Native{}", admin_status));

            // Beautiful glassmorphic backup prompt banner (Borrow Checker bypass via short-lived lock scope)
            let prompt_folder = {
                let lock = self.show_backup_prompt.lock().unwrap_or_else(|e| e.into_inner());
                lock.clone()
            };
            if let Some(ref folder_path) = prompt_folder {
                let folder_path_clone = folder_path.clone();
                let folder_name = std::path::Path::new(&folder_path_clone)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("scan");
                
                // Clean root or empty names (e.g. C:\ -> drive_root)
                let folder_name_clean = if folder_name.is_empty() || folder_name.contains(':') {
                    "drive_root".to_string()
                } else {
                    folder_name.to_string()
                };

                ui.group(|ui| {
                    ui.style_mut().visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(30, 48, 36); // Curated dark forest green
                    ui.style_mut().visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(50, 80, 60);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!(" [Content Index Complete] Folder: [{}]. Export index database?", folder_name_clean))
                            .color(egui::Color32::from_rgb(140, 220, 160))
                            .strong());
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(egui::RichText::new("Dismiss").color(egui::Color32::LIGHT_GRAY)).clicked() {
                                *self.show_backup_prompt.lock().unwrap_or_else(|e| e.into_inner()) = None;
                            }
                            
                            if ui.button(egui::RichText::new("💾 Save Index").color(egui::Color32::WHITE).strong()).clicked() {
                                let date_str = chrono::Local::now().format("%y%m%d_%H%M").to_string();
                                let default_filename = format!("{}_{}.db", date_str, folder_name_clean);

                                // Resolve OS Standard Documents directory
                                let home_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
                                let default_dir = std::path::PathBuf::from(home_dir)
                                    .join("Documents")
                                    .join("DeepFileX")
                                    .join("Indices");
                                
                                // Auto-create directory structure if not exists
                                let _ = std::fs::create_dir_all(&default_dir);

                                if let Some(dest_path) = rfd::FileDialog::new()
                                    .set_directory(&default_dir)
                                    .set_file_name(&default_filename)
                                    .add_filter("SQLite Database", &["db"])
                                    .save_file() 
                                {
                                    let db_arc = self.db.clone();
                                    let ctx_clone = ctx.clone();
                                    let folder_path_clone_t = folder_path_clone.clone();
                                    let dest_path_t = dest_path.clone();
                                    let is_saving_t = self.is_saving_index.clone();

                                    *is_saving_t.lock().unwrap() = true;
                                    self.status_message = "Saving index database in background...".to_string();

                                    std::thread::spawn(move || {
                                        let mut success = false;
                                        if let Ok(db) = db_arc.lock() {
                                            if db.backup_to(&dest_path_t).is_ok() {
                                                if let Ok(conn) = rusqlite::Connection::open(&dest_path_t) {
                                                    let scan_history_path = folder_path_clone_t.replace("/", "\\").to_lowercase();
                                                    let mut lower_folder = scan_history_path.clone();
                                                    if !lower_folder.ends_with('\\') {
                                                        lower_folder.push('\\');
                                                    }
                                                    let like_pattern = format!("{}%", lower_folder);

                                                    // A. Delete records not matching target folder path prefix
                                                    let _ = conn.execute(
                                                        "DELETE FROM FILES WHERE REPLACE(LOWER(file_path), '/', '\\') NOT LIKE ?",
                                                        rusqlite::params![like_pattern]
                                                    );
                                                    let _ = conn.execute(
                                                        "DELETE FROM SCAN_HISTORY WHERE REPLACE(LOWER(path), '/', '\\') NOT LIKE ?",
                                                        rusqlite::params![scan_history_path]
                                                    );

                                                    // B. Clean up FTS & FILE_CONTENTS orphans
                                                    let _ = conn.execute(
                                                        "DELETE FROM FILES_FTS WHERE file_id NOT IN (SELECT file_id FROM FILES)",
                                                        []
                                                    );
                                                    let _ = conn.execute(
                                                        "DELETE FROM FILES_CONTENT_FTS WHERE file_id NOT IN (SELECT file_id FROM FILES)",
                                                        []
                                                    );
                                                    let _ = conn.execute(
                                                        "DELETE FROM FILE_CONTENTS WHERE file_id NOT IN (SELECT file_id FROM FILES)",
                                                        []
                                                    );

                                                    // C. Compact
                                                    let _ = conn.execute("PRAGMA journal_mode = DELETE", []);
                                                    let _ = conn.execute("VACUUM", []);
                                                    success = true;
                                                }
                                            }
                                        }

                                        *is_saving_t.lock().unwrap() = false;
                                        ctx_clone.request_repaint();
                                    });
                                }
                                *self.show_backup_prompt.lock().unwrap_or_else(|e| e.into_inner()) = None;
                            }
                        });
                    });
                });
                ui.add_space(5.0);
            }

            ui.horizontal(|ui| {
                ui.label("Search:");
                let response = ui.text_edit_singleline(&mut self.search_query);
                if response.changed() {
                    response.request_focus();
                }
                
                let chk_file = ui.checkbox(&mut self.search_file_name, "Filename");
                let chk_content = ui.checkbox(&mut self.search_content, "Content");

                let mut changed = response.changed();

                if chk_file.changed() {
                    if self.search_file_name {
                        self.search_content = false;
                    } else if !self.search_content {
                        self.search_file_name = true; // Force at least one selected
                    }
                    changed = true;
                }
                if chk_content.changed() {
                    if self.search_content {
                        self.search_file_name = false;
                    } else if !self.search_file_name {
                        self.search_content = true; // Force at least one selected
                    }
                    changed = true;
                }

                if changed {
                    self.last_input_time = std::time::Instant::now();
                    self.pending_live_search = true;
                }
            });

            // Advanced Filters Panel
            ui.collapsing("🔍 Advanced Filters", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Min Size:");
                    let min_sz_res = ui.add(egui::TextEdit::singleline(&mut self.filter_min_size).hint_text("e.g. 1mb").desired_width(80.0));
                    if min_sz_res.changed() {
                        self.pending_live_search = true;
                        self.last_input_time = std::time::Instant::now();
                    }

                    ui.label("Max Size:");
                    let max_sz_res = ui.add(egui::TextEdit::singleline(&mut self.filter_max_size).hint_text("e.g. 10mb").desired_width(80.0));
                    if max_sz_res.changed() {
                        self.pending_live_search = true;
                        self.last_input_time = std::time::Instant::now();
                    }

                    ui.label("Exts:");
                    let ext_res = ui.add(egui::TextEdit::singleline(&mut self.filter_extensions).hint_text("e.g. pdf, docx").desired_width(120.0));
                    if ext_res.changed() {
                        self.pending_live_search = true;
                        self.last_input_time = std::time::Instant::now();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Date Modified:");
                    let date_res = egui::ComboBox::from_label("")
                        .selected_text(match self.filter_date_range {
                            1 => "Last 24 hours",
                            2 => "Last 7 days",
                            3 => "Last 30 days",
                            _ => "All time",
                        })
                        .show_ui(ui, |ui| {
                            let mut changed = false;
                            changed |= ui.selectable_value(&mut self.filter_date_range, 0, "All time").changed();
                            changed |= ui.selectable_value(&mut self.filter_date_range, 1, "Last 24 hours").changed();
                            changed |= ui.selectable_value(&mut self.filter_date_range, 2, "Last 7 days").changed();
                            changed |= ui.selectable_value(&mut self.filter_date_range, 3, "Last 30 days").changed();
                            changed
                        });
                    
                    if date_res.inner.unwrap_or(false) {
                        self.pending_live_search = true;
                        self.last_input_time = std::time::Instant::now();
                    }
                });
            });

            // Content Indexing Path & Button row under search bar
            if self.search_content {
                ui.horizontal(|ui| {
                    ui.label("Content Index Folder:");
                    ui.add(egui::TextEdit::singleline(&mut self.content_indexing_path).interactive(false));
                    if ui.button("📁 Folder").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.content_indexing_path = path.to_string_lossy().to_string();
                        }
                    }

                    let is_c_indexing = *self.is_content_indexing.lock().unwrap_or_else(|e| e.into_inner());
                    let c_indexed = *self.content_indexed_count.lock().unwrap_or_else(|e| e.into_inner());
                    let c_total = *self.content_total_count.lock().unwrap_or_else(|e| e.into_inner());

                    if is_c_indexing {
                        ui.add(egui::Spinner::new());
                        ui.label(format!("Indexing: {} / {}", c_indexed, c_total));
                        if ui.button("🛑 Stop").clicked() {
                            self.is_content_indexing_cancelled.store(true, Ordering::SeqCst);
                        }
                    } else {
                        let path_empty = self.content_indexing_path.trim().is_empty();
                        ui.add_enabled_ui(!path_empty, |ui| {
                            if ui.button("⚡ Scan Folder").clicked() {
                                self.start_content_indexing(ctx);
                            }
                        });
                    }
                });
            }


            ui.label(&self.status_message);
            let primary_items = self.results.lock().unwrap_or_else(|e| e.into_inner());
            let mut items = primary_items.clone();
            drop(primary_items);

            // Sort items dynamically by selected column and direction
            items.sort_by(|a, b| {
                match self.sort_column {
                    0 => {
                        let name_a = std::path::Path::new(&a.1).file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                        let name_b = std::path::Path::new(&b.1).file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                        if self.sort_ascending {
                            name_a.cmp(&name_b)
                        } else {
                            name_b.cmp(&name_a)
                        }
                    }
                    1 => {
                        let get_type = |x: &(i64, String, bool, u64, u64)| {
                            if x.2 { "Folder".to_string() } else {
                                std::path::Path::new(&x.1).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
                            }
                        };
                        if self.sort_ascending {
                            get_type(a).cmp(&get_type(b))
                        } else {
                            get_type(b).cmp(&get_type(a))
                        }
                    }
                    2 => {
                        if self.sort_ascending {
                            a.3.cmp(&b.3)
                        } else {
                            b.3.cmp(&a.3)
                        }
                    }
                    3 => {
                        if self.sort_ascending {
                            a.4.cmp(&b.4)
                        } else {
                            b.4.cmp(&a.4)
                        }
                    }
                    _ => std::cmp::Ordering::Equal
                }
            });

            let mut selected_to_process = None;

            use egui_extras::{TableBuilder, Column};

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::remainder().at_least(20.0)) // Name (can be shrunk down to 20px)
                .column(Column::initial(80.0).at_least(20.0))  // Type
                .column(Column::initial(80.0).at_least(20.0))  // Size
                .column(Column::initial(150.0).at_least(20.0)) // Modified Date
                .header(24.0, |mut header| {
                    // Column 1: Name Sort Button
                    header.col(|ui| {
                        let arrow = if self.sort_column == 0 { if self.sort_ascending { " ▲" } else { " ▼" } } else { "" };
                        let header_text = format!("Name{}", arrow);
                        if ui.selectable_label(self.sort_column == 0, egui::RichText::new(header_text).strong()).clicked() {
                            if self.sort_column == 0 {
                                self.sort_ascending = !self.sort_ascending;
                            } else {
                                self.sort_column = 0;
                                self.sort_ascending = true;
                            }
                        }
                    });
                    // Column 2: Type Sort Button
                    header.col(|ui| {
                        let arrow = if self.sort_column == 1 { if self.sort_ascending { " ▲" } else { " ▼" } } else { "" };
                        let header_text = format!("Type{}", arrow);
                        if ui.selectable_label(self.sort_column == 1, egui::RichText::new(header_text).strong()).clicked() {
                            if self.sort_column == 1 {
                                self.sort_ascending = !self.sort_ascending;
                            } else {
                                self.sort_column = 1;
                                self.sort_ascending = true;
                            }
                        }
                    });
                    // Column 3: Size Sort Button
                    header.col(|ui| {
                        let arrow = if self.sort_column == 2 { if self.sort_ascending { " ▲" } else { " ▼" } } else { "" };
                        let header_text = format!("Size{}", arrow);
                        if ui.selectable_label(self.sort_column == 2, egui::RichText::new(header_text).strong()).clicked() {
                            if self.sort_column == 2 {
                                self.sort_ascending = !self.sort_ascending;
                            } else {
                                self.sort_column = 2;
                                self.sort_ascending = true;
                            }
                        }
                    });
                    // Column 4: Modified Date Sort Button
                    header.col(|ui| {
                        let arrow = if self.sort_column == 3 { if self.sort_ascending { " ▲" } else { " ▼" } } else { "" };
                        let header_text = format!("Modified Date{}", arrow);
                        if ui.selectable_label(self.sort_column == 3, egui::RichText::new(header_text).strong()).clicked() {
                            if self.sort_column == 3 {
                                self.sort_ascending = !self.sort_ascending;
                            } else {
                                self.sort_column = 3;
                                self.sort_ascending = true;
                            }
                        }
                    });
                })
                .body(|body| {
                    body.rows(22.0, items.len(), |row_index, mut row| {
                        let (id, path, is_dir, size, mtime) = &items[row_index];
                        let is_selected = if *id == -1 {
                            self.selected_file_path.as_ref() == Some(path)
                        } else {
                            self.selected_file_id == Some(*id)
                        };

                        let file_name = std::path::Path::new(path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(path.as_str());

                        // Column 1: Name
                        row.col(|ui| {
                            ui.horizontal(|ui| {
                                let icon = if *is_dir { "📁" } else { "📄" };
                                ui.label(icon);
                                
                                let label_res = ui.selectable_label(is_selected, file_name);
                                if label_res.clicked() {
                                    selected_to_process = Some((*id, path.clone(), *is_dir, *size, *mtime));
                                }

                                // 우클릭 컨텍스트 메뉴 바인딩
                                label_res.context_menu(|ui| {
                                    if ui.button("📄 Open").clicked() {
                                        win_shell::open_file(path);
                                        ui.close_menu();
                                    }
                                    if ui.button("📁 Open Folder Location").clicked() {
                                        win_shell::open_file_location(path);
                                        ui.close_menu();
                                    }
                                    if ui.button("🔗 Copy Path").clicked() {
                                        ui.output_mut(|o| o.copied_text = path.clone());
                                        ui.close_menu();
                                    }
                                    if ui.button("ℹ️ Properties").clicked() {
                                        win_shell::show_file_properties(path);
                                        ui.close_menu();
                                    }
                                });
                            });
                        });

                        // Column 2: Type
                        row.col(|ui| {
                            if *is_dir {
                                ui.label("Folder");
                            } else {
                                let ext = std::path::Path::new(path)
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .unwrap_or("")
                                    .to_uppercase();
                                ui.label(if ext.is_empty() { "File".to_string() } else { format!("{} File", ext) });
                            }
                        });

                        // Column 3: Size
                        row.col(|ui| {
                            ui.label(format_size(*size, *is_dir));
                        });

                        // Column 4: Modified Date
                        row.col(|ui| {
                            ui.label(format_date(*mtime));
                        });
                    });
                });

            if let Some((id, path, is_dir, size, mtime)) = selected_to_process {
                self.selected_file_size = Some(size);
                self.selected_file_mtime = Some(mtime);
                self.selected_file_is_dir = Some(is_dir);

                if id == -1 {
                    self.selected_file_id = None;
                    self.selected_file_path = Some(path.clone());
                    if !is_dir {
                        let ext = std::path::Path::new(&path).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                        if ["pdf", "docx", "xlsx", "txt", "csv", "log"].contains(&ext.as_str()) {
                            self.selected_file_content = deepfilex::parser::extract_text(&path).ok();
                        } else {
                            self.selected_file_content = None;
                        }
                    } else {
                        self.selected_file_content = None;
                    }
                } else {
                    self.selected_file_path = Some(path.clone());
                    self.selected_file_id = Some(id);
                    if let Ok(db_lock) = self.db.try_lock() {
                        self.selected_file_content = db_lock.get_file_content(id).ok().flatten();
                    } else {
                        self.selected_file_content = Some("Database is currently busy scanning files. Please click again in a moment.".to_string());
                    }
                }
            }


            // 3. Delete Index Confirmation Dialog
            if let Some(ref target_files) = self.show_delete_confirm {
                let mut close_modal = false;
                let mut refresh_search = false;
                
                egui::Window::new("⚠️ Delete Index Confirmation")
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .resizable(false)
                    .collapsible(false)
                    .show(ctx, |ui| {
                        ui.label(format!("You have selected {} index file(s) to remove.", target_files.len()));
                        ui.add_space(5.0);
                        
                        ui.horizontal(|ui| {
                            // Option A: Remove from list only
                            if ui.button("Remove from List Only").clicked() {
                                for f in target_files {
                                    self.loaded_indices.retain(|x| x != f);
                                    self.selected_indices.remove(f);
                                }
                                self.status_message = "Index removed from list.".to_string();
                                close_modal = true;
                                refresh_search = true;
                            }
                            
                            // Option B: Delete from Disk Permanently
                            if ui.button("🗑 Permanently Delete from Disk").clicked() {
                                let home_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
                                let default_dir = std::path::PathBuf::from(home_dir)
                                    .join("Documents")
                                    .join("DeepFileX")
                                    .join("Indices");
                                    
                                for f in target_files {
                                    let file_path = default_dir.join(f);
                                    let _ = std::fs::remove_file(file_path);
                                    self.loaded_indices.retain(|x| x != f);
                                    self.selected_indices.remove(f);
                                }
                                self.status_message = "Index permanently deleted from disk.".to_string();
                                close_modal = true;
                                refresh_search = true;
                            }
                            
                            // Cancel
                            if ui.button("Cancel").clicked() {
                                close_modal = true;
                            }
                        });
                    });

                if close_modal {
                    self.show_delete_confirm = None;
                }
                if refresh_search {
                    self.search(ctx);
                }
            }

        });
    }
}

fn load_icon() -> Option<eframe::IconData> {
    let icon_bytes = include_bytes!("dfx_logo.png");
    if let Ok(image) = image::load_from_memory(icon_bytes) {
        let image = image.to_rgba8();
        let (width, height) = image.dimensions();
        Some(eframe::IconData {
            rgba: image.into_raw(),
            width,
            height,
        })
    } else {
        None
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        drag_and_drop_support: true,
        icon_data: load_icon(),
        ..Default::default()
    };
    eframe::run_native(
        "DeepFileX",
        options,
        Box::new(|cc| Box::new(DeepFileXApp::new(cc))),
    )
}

fn is_user_admin() -> bool {
    unsafe {
        windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0
    }
}









