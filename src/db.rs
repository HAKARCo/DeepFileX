#![allow(clippy::type_complexity, clippy::collapsible_if, clippy::manual_flatten, clippy::too_many_arguments, clippy::bool_assert_comparison)]

use rusqlite::{params, Connection, Result};
use std::path::Path;



#[derive(Debug, Default, Clone)]
pub struct SearchFilter {
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub min_mtime: Option<u64>,
    pub extensions: Option<Vec<String>>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        
        // Optimize SQLite performance for high-speed writes
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;
             PRAGMA cache_size = -64000;
             PRAGMA mmap_size = 268435456;
             PRAGMA foreign_keys = ON;"
        )?;

        let db = Self { conn };
        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> Result<()> {
        // 1. Files table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS FILES (
                file_id INTEGER PRIMARY KEY,
                file_path TEXT NOT NULL UNIQUE,
                file_name TEXT NOT NULL DEFAULT '',
                file_size INTEGER DEFAULT 0,
                mtime INTEGER DEFAULT 0,
                is_dir INTEGER DEFAULT 0
             );",
            [],
        )?;

        // Ensure migration for older databases
        let _ = self.conn.execute(
            "ALTER TABLE FILES ADD COLUMN file_name TEXT NOT NULL DEFAULT '';",
            []
        );

        // 2. FTS5 virtual table for lightning-fast search
        self.conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS FILES_FTS USING fts5(
                file_id UNINDEXED,
                file_name,
                file_path
             );",
            [],
        )?;

        // 2.1 FTS5 virtual table for high-speed file content search
        self.conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS FILES_CONTENT_FTS USING fts5(
                file_id UNINDEXED,
                content
             );",
            [],
        )?;

        // 3. Document content cache table (Zstd compressed)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS FILE_CONTENTS (
                file_id INTEGER PRIMARY KEY,
                compressed_text BLOB,
                FOREIGN KEY(file_id) REFERENCES FILES(file_id) ON DELETE CASCADE
             );",
            [],
        )?;

        // 4. Scan history table to track loaded directory paths
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS SCAN_HISTORY (
                path TEXT PRIMARY KEY,
                scan_time INTEGER DEFAULT 0
             );",
            [],
        )?;

        // Create indexes
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_files_path ON FILES(file_path);",
            [],
        )?;

        Ok(())
    }

    pub fn begin_transaction(&self) -> Result<()> {
        self.conn.execute("BEGIN TRANSACTION", []).map(|_| ())
    }

    pub fn commit_transaction(&self) -> Result<()> {
        self.conn.execute("COMMIT", []).map(|_| ())
    }

    pub fn rollback_transaction(&self) -> Result<()> {
        self.conn.execute("ROLLBACK", []).map(|_| ())
    }


    pub fn insert_file(&self, path: &str, size: u64, mtime: u64, is_dir: bool) -> Result<i64> {
        // 1. Check if the file already exists to preserve file_id and avoid CASCADE deletes of content cache
        let mut stmt = self.conn.prepare("SELECT file_id, file_size, mtime FROM FILES WHERE file_path = ?")?;
        let mut rows = stmt.query(params![path])?;
        
        let file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let old_size: u64 = row.get(1)?;
            let old_mtime: u64 = row.get(2)?;
            
            // If size or modification time has changed, update metadata and drop cached content for re-extraction
            if old_size != size || old_mtime != mtime {
                self.conn.execute(
                    "UPDATE FILES SET file_size = ?, mtime = ?, file_name = ? WHERE file_id = ?",
                    params![size, mtime, file_name, id],
                )?;
                // Clear obsolete cache to trigger re-extraction in main loop
                let _ = self.conn.execute("DELETE FROM FILE_CONTENTS WHERE file_id = ?", params![id]);
                let _ = self.conn.execute("DELETE FROM FILES_CONTENT_FTS WHERE file_id = ?", params![id]);
            }
            return Ok(id);
        }

        // 2. Insert new entry if not exists
        self.conn.execute(
            "INSERT INTO FILES (file_path, file_name, file_size, mtime, is_dir) VALUES (?, ?, ?, ?, ?)",
            params![path, file_name, size, mtime, if is_dir { 1 } else { 0 }],
        )?;
        let id = self.conn.last_insert_rowid();

        // Update FTS5 virtual table
        self.conn.execute(
            "INSERT INTO FILES_FTS (file_id, file_name, file_path) VALUES (?, ?, ?)",
            params![id, file_name, path],
        )?;

        Ok(id)
    }

    pub fn insert_files_batch(&self, batch: &[(i64, String, bool, u64, u64)]) -> Result<()> {
        self.conn.execute("BEGIN TRANSACTION", [])?;
        
        let res = (|| -> Result<()> {
            let mut stmt_files = self.conn.prepare(
                "INSERT OR REPLACE INTO FILES (file_path, file_name, file_size, mtime, is_dir) VALUES (?, ?, ?, ?, ?)"
            )?;

            for item in batch {
                let path = &item.1;
                let size = item.3;
                let mtime = item.4;
                let is_dir = item.2;

                let file_name = Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                stmt_files.execute(params![path, file_name, size, mtime, if is_dir { 1 } else { 0 }])?;
            }
            Ok(())
        })();

        if res.is_ok() {
            self.conn.execute("COMMIT", [])?;
        } else {
            let _ = self.conn.execute("ROLLBACK", []);
        }
        res
    }

    pub fn save_content(&self, file_id: i64, compressed_data: &[u8], plain_text: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO FILE_CONTENTS (file_id, compressed_text) VALUES (?, ?)",
            params![file_id, compressed_data],
        )?;

        // Update Content FTS
        let _ = self.conn.execute(
            "DELETE FROM FILES_CONTENT_FTS WHERE file_id = ?",
            params![file_id],
        );
        self.conn.execute(
            "INSERT INTO FILES_CONTENT_FTS (file_id, content) VALUES (?, ?)",
            params![file_id, plain_text],
        )?;

        Ok(())
    }

    pub fn save_content_batch(&self, batch: &[(i64, Vec<u8>, String)]) -> Result<()> {
        self.conn.execute("BEGIN TRANSACTION", [])?;
        let res = (|| -> Result<()> {
            let mut stmt_fc = self.conn.prepare(
                "INSERT OR REPLACE INTO FILE_CONTENTS (file_id, compressed_text) VALUES (?, ?)"
            )?;
            let mut stmt_del_fts = self.conn.prepare(
                "DELETE FROM FILES_CONTENT_FTS WHERE file_id = ?"
            )?;
            let mut stmt_ins_fts = self.conn.prepare(
                "INSERT INTO FILES_CONTENT_FTS (file_id, content) VALUES (?, ?)"
            )?;

            for (file_id, compressed_data, plain_text) in batch {
                stmt_fc.execute(params![file_id, compressed_data])?;
                let _ = stmt_del_fts.execute(params![file_id]);
                stmt_ins_fts.execute(params![file_id, plain_text])?;
            }
            Ok(())
        })();

        if res.is_ok() {
            self.conn.execute("COMMIT", [])?;
        } else {
            let _ = self.conn.execute("ROLLBACK", []);
        }
        res
    }

    pub fn get_all_document_files(&self) -> Result<Vec<(i64, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT file_id, file_path FROM FILES 
             WHERE is_dir = 0 AND (
                file_path LIKE '%.pdf' OR 
                file_path LIKE '%.docx' OR 
                file_path LIKE '%.xlsx' OR 
                file_path LIKE '%.pptx' OR 
                file_path LIKE '%.hwp' OR 
                file_path LIKE '%.hwpx' OR 
                file_path LIKE '%.txt' OR 
                file_path LIKE '%.csv' OR 
                file_path LIKE '%.log' OR 
                file_path LIKE '%.md' OR 
                file_path LIKE '%.json' OR 
                file_path LIKE '%.xml' OR 
                file_path LIKE '%.yaml' OR 
                file_path LIKE '%.yml'
             )"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        let mut results = Vec::new();
        for r in rows {
            if let Ok(item) = r {
                results.push(item);
            }
        }
        Ok(results)
    }

    pub fn populate_fts_table(&self) -> Result<()> {
        self.conn.execute("DELETE FROM FILES_FTS", [])?;
        self.conn.execute(
            "INSERT INTO FILES_FTS (file_id, file_name, file_path) 
             SELECT file_id, file_name, file_path FROM FILES",
            []
        )?;
        Ok(())
    }

    pub fn has_content_index(&self, file_id: i64) -> Result<bool> {
        let mut stmt = self.conn.prepare(
            "SELECT 1 FROM FILE_CONTENTS WHERE file_id = ? LIMIT 1"
        )?;
        let mut rows = stmt.query(params![file_id])?;
        Ok(rows.next()?.is_some())
    }

    pub fn search_files_by_content(&self, keyword: &str, filter: &SearchFilter, limit: usize, external_dbs: &[String]) -> Result<Vec<(i64, String, bool, u64, u64)>> {
        if keyword.trim().is_empty() {
            return Ok(Vec::new());
        }

        let search_query = format!("\"{}\" *", keyword.trim());
        let mut results = Vec::new();

        if external_dbs.is_empty() {
            let mut query = "SELECT f.file_id, f.file_path, f.is_dir, f.file_size, f.mtime 
                             FROM FILES_CONTENT_FTS cfts
                             JOIN FILES f ON f.file_id = cfts.file_id
                             WHERE cfts.content MATCH ?".to_string();
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(search_query)];

            if let Some(min) = filter.min_size {
                query.push_str(" AND f.file_size >= ?");
                params.push(Box::new(min));
            }
            if let Some(max) = filter.max_size {
                query.push_str(" AND f.file_size <= ?");
                params.push(Box::new(max));
            }
            if let Some(min_t) = filter.min_mtime {
                query.push_str(" AND f.mtime >= ?");
                params.push(Box::new(min_t));
            }
            if let Some(ref exts) = filter.extensions {
                if !exts.is_empty() {
                    for ext in exts {
                        query.push_str(" AND f.file_path LIKE ?");
                        params.push(Box::new(format!("%.{}", ext.to_lowercase())));
                    }
                }
            }

            query.push_str(" LIMIT ?");
            params.push(Box::new(limit));

            let mut stmt = self.conn.prepare(&query)?;
            let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

            let rows = stmt.query_map(&params_ref[..], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get::<_, i32>(2)? == 1,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?;
            for r in rows {
                if let Ok(item) = r {
                    results.push(item);
                }
            }
        } else {
            for db_path in external_dbs {
                let p = std::path::Path::new(db_path);
                if let Ok(conn) = rusqlite::Connection::open(p) {
                    let mut query = "SELECT f.file_id, f.file_path, f.is_dir, f.file_size, f.mtime 
                                     FROM FILES_CONTENT_FTS cfts
                                     JOIN FILES f ON f.file_id = cfts.file_id
                                     WHERE cfts.content MATCH ?".to_string();
                    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(search_query.clone())];

                    if let Some(min) = filter.min_size {
                        query.push_str(" AND f.file_size >= ?");
                        params.push(Box::new(min));
                    }
                    if let Some(max) = filter.max_size {
                        query.push_str(" AND f.file_size <= ?");
                        params.push(Box::new(max));
                    }
                    if let Some(min_t) = filter.min_mtime {
                        query.push_str(" AND f.mtime >= ?");
                        params.push(Box::new(min_t));
                    }
                    if let Some(ref exts) = filter.extensions {
                        if !exts.is_empty() {
                            for ext in exts {
                                query.push_str(" AND f.file_path LIKE ?");
                                params.push(Box::new(format!("%.{}", ext.to_lowercase())));
                            }
                        }
                    }

                    query.push_str(" LIMIT ?");
                    params.push(Box::new(limit));

                    if let Ok(mut stmt) = conn.prepare(&query) {
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
                                    results.push(item);
                                }
                            }
                        }
                    }
                }
            }
            results.truncate(limit);
        }

        Ok(results)
    }

    pub fn search_files(&self, keyword: &str, filter: &SearchFilter, limit: usize) -> Result<Vec<(i64, String, bool, u64, u64)>> {
        let mut query = "SELECT f.file_id, f.file_path, f.is_dir, f.file_size, f.mtime FROM FILES f".to_string();
        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        let keyword_trimmed = keyword.trim();
        if !keyword_trimmed.is_empty() {
            query = "SELECT f.file_id, f.file_path, f.is_dir, f.file_size, f.mtime 
                     FROM FILES_FTS fts
                     JOIN FILES f ON f.file_id = fts.file_id".to_string();
            conditions.push("FILES_FTS MATCH ?".to_string());
            params.push(Box::new(format!("\"{}\" *", keyword_trimmed)));
        }

        if let Some(min) = filter.min_size {
            conditions.push("f.file_size >= ?".to_string());
            params.push(Box::new(min));
        }
        if let Some(max) = filter.max_size {
            conditions.push("f.file_size <= ?".to_string());
            params.push(Box::new(max));
        }
        if let Some(min_t) = filter.min_mtime {
            conditions.push("f.mtime >= ?".to_string());
            params.push(Box::new(min_t));
        }
        if let Some(ref exts) = filter.extensions {
            if !exts.is_empty() {
                let ext_conditions: Vec<String> = exts.iter()
                    .map(|ext| {
                        params.push(Box::new(format!("%.{}", ext.to_lowercase())));
                        "f.file_path LIKE ?".to_string()
                    })
                    .collect();
                conditions.push(format!("({})", ext_conditions.join(" OR ")));
            }
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(" LIMIT ?");
        params.push(Box::new(limit));

        let mut stmt = self.conn.prepare(&query)?;
        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(&params_ref[..], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get::<_, i32>(2)? == 1,
                row.get(3)?,
                row.get(4)?,
            ))
        })?;

        let mut results = Vec::new();
        for r in rows {
            if let Ok(item) = r {
                results.push(item);
            }
        }

        if results.is_empty() && !keyword_trimmed.is_empty() {
            let mut fallback_query = "SELECT file_id, file_path, is_dir, file_size, mtime FROM FILES WHERE 1=1".to_string();
            let mut fb_conditions = Vec::new();
            let mut fb_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            fb_conditions.push("file_path LIKE ?".to_string());
            fb_params.push(Box::new(format!("%{}%", keyword_trimmed)));

            if let Some(min) = filter.min_size {
                fb_conditions.push("file_size >= ?".to_string());
                fb_params.push(Box::new(min));
            }
            if let Some(max) = filter.max_size {
                fb_conditions.push("file_size <= ?".to_string());
                fb_params.push(Box::new(max));
            }
            if let Some(min_t) = filter.min_mtime {
                fb_conditions.push("mtime >= ?".to_string());
                fb_params.push(Box::new(min_t));
            }
            if let Some(ref exts) = filter.extensions {
                if !exts.is_empty() {
                    let ext_conditions: Vec<String> = exts.iter()
                        .map(|ext| {
                            fb_params.push(Box::new(format!("%.{}", ext.to_lowercase())));
                            "file_path LIKE ?".to_string()
                        })
                        .collect();
                    fb_conditions.push(format!("({})", ext_conditions.join(" OR ")));
                }
            }

            if !fb_conditions.is_empty() {
                fallback_query.push_str(" AND ");
                fallback_query.push_str(&fb_conditions.join(" AND "));
            }

            fallback_query.push_str(" LIMIT ?");
            fb_params.push(Box::new(limit));

            let mut stmt_fb = self.conn.prepare(&fallback_query)?;
            let fb_ref: Vec<&dyn rusqlite::ToSql> = fb_params.iter().map(|p| p.as_ref()).collect();

            let rows_fb = stmt_fb.query_map(&fb_ref[..], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get::<_, i32>(2)? == 1,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?;

            for r in rows_fb {
                if let Ok(item) = r {
                    results.push(item);
                }
            }
        }

        Ok(results)
    }

    pub fn get_file_content(&self, file_id: i64) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT compressed_text FROM FILE_CONTENTS WHERE file_id = ?"
        )?;
        let mut rows = stmt.query(params![file_id])?;
        if let Some(row) = rows.next()? {
            let compressed: Vec<u8> = row.get(0)?;
            if let Ok(decompressed) = crate::crypto::decompress_zstd(&compressed) {
                if let Ok(text) = String::from_utf8(decompressed) {
                    return Ok(Some(text));
                }
            }
        }
        Ok(None)
    }

    pub fn backup_to<P: AsRef<Path>>(&self, dest_path: P) -> Result<()> {
        let mut dest_conn = Connection::open(dest_path)?;
        let backup = rusqlite::backup::Backup::new(&self.conn, &mut dest_conn)?;
        backup.run_to_completion(5, std::time::Duration::from_millis(5), None)?;
        Ok(())
    }

    pub fn restore_from(&mut self, src_path: &Path) -> Result<()> {
        let src_conn = Connection::open(src_path)?;
        let backup = rusqlite::backup::Backup::new(&src_conn, &mut self.conn)?;
        backup.run_to_completion(5, std::time::Duration::from_millis(5), None)?;
        Ok(())
    }

    pub fn insert_scan_history(&self, path: &str) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.conn.execute(
            "INSERT OR REPLACE INTO SCAN_HISTORY (path, scan_time) VALUES (?, ?)",
            params![path, now],
        )?;
        Ok(())
    }

    pub fn get_scan_history(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT path FROM SCAN_HISTORY ORDER BY scan_time DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            let path: String = row.get(0)?;
            Ok(path)
        })?;
        let mut paths = Vec::new();
        for r in rows {
            if let Ok(p) = r {
                paths.push(p);
            }
        }
        Ok(paths)
    }

    pub fn delete_scan_history(&self, path: &str) -> Result<()> {
        let like_pattern = format!("{}%", path);
        // Execute manual cascades inside a transaction
        self.conn.execute("BEGIN TRANSACTION", [])?;

        let _ = self.conn.execute(
            "DELETE FROM FILE_CONTENTS WHERE file_id IN (SELECT file_id FROM FILES WHERE file_path LIKE ?)",
            params![like_pattern],
        );
        let _ = self.conn.execute(
            "DELETE FROM FILES_CONTENT_FTS WHERE file_id IN (SELECT file_id FROM FILES WHERE file_path LIKE ?)",
            params![like_pattern],
        );
        let _ = self.conn.execute(
            "DELETE FROM FILES_FTS WHERE file_id IN (SELECT file_id FROM FILES WHERE file_path LIKE ?)",
            params![like_pattern],
        );

        let _ = self.conn.execute(
            "DELETE FROM FILES WHERE file_path LIKE ?",
            params![like_pattern],
        );
        let _ = self.conn.execute(
            "DELETE FROM SCAN_HISTORY WHERE path = ?",
            params![path],
        );
        self.conn.execute("COMMIT", [])?;
        Ok(())
    }

    pub fn clear_all_data(&self) -> Result<()> {
        let _ = self.conn.execute("DELETE FROM FILES", []);
        let _ = self.conn.execute("DELETE FROM FILES_FTS", []);
        let _ = self.conn.execute("DELETE FROM FILES_CONTENT_FTS", []);
        let _ = self.conn.execute("DELETE FROM FILE_CONTENTS", []);
        let _ = self.conn.execute("DELETE FROM SCAN_HISTORY", []);
        let _ = self.conn.execute("VACUUM", []);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_crud() {
        let db = Database::open(":memory:").unwrap();
        
        let file_id = db.insert_file("C:\\Test\\test.txt", 1024, 12345678, false).unwrap();
        assert!(file_id > 0);
        
        let results = db.search_files("test", &SearchFilter::default(), 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, "C:\\Test\\test.txt");
        assert_eq!(results[0].3, 1024);
        assert_eq!(results[0].4, 12345678);
        assert_eq!(results[0].2, false);

        let plain_text = "하카르 인덱싱 본문 테스트 데이터";
        let compressed = crate::crypto::compress_zstd(plain_text.as_bytes(), 3).unwrap();
        db.save_content(file_id, &compressed, plain_text).unwrap();

        let loaded_content = db.get_file_content(file_id).unwrap().unwrap();
        assert_eq!(loaded_content, plain_text);

        let content_results = db.search_files_by_content("인덱싱", &SearchFilter::default(), 10, &[]).unwrap();
        assert_eq!(content_results.len(), 1);
        assert_eq!(content_results[0].1, "C:\\Test\\test.txt");
    }
}




