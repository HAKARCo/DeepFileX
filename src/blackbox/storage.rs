use std::path::{Path, PathBuf};
use rusqlite::{params, Connection};
use super::error::Result;
use super::types::{BlackboxEvent, SessionMeta};

pub struct StorageAdapter {
    db_path: PathBuf,
}

impl StorageAdapter {
    pub fn get_default_db_path() -> PathBuf {
        let home_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
        PathBuf::from(home_dir)
            .join("Documents")
            .join("DeepFileX")
            .join("Logs")
            .join("blackbox.db")
    }

    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db_path = path.as_ref().to_path_buf();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS blackbox_events (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_ms    INTEGER NOT NULL,
                session_id      TEXT NOT NULL,
                sequence        INTEGER NOT NULL,
                event_type      TEXT NOT NULL,
                component       TEXT NOT NULL,
                action          TEXT NOT NULL,
                context_json    TEXT,
                result          TEXT NOT NULL,
                error_code      TEXT,
                error_message   TEXT,
                duration_ms     INTEGER,
                app_version     TEXT NOT NULL,
                os_version      TEXT,
                build_type      TEXT,
                pii_hash        TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_bb_session_time ON blackbox_events(session_id, timestamp_ms);
            CREATE INDEX IF NOT EXISTS idx_bb_type_time ON blackbox_events(event_type, timestamp_ms);

            CREATE TABLE IF NOT EXISTS blackbox_sessions (
                session_id      TEXT PRIMARY KEY,
                started_at      INTEGER NOT NULL,
                ended_at        INTEGER,
                app_version     TEXT NOT NULL,
                os_info         TEXT,
                total_events    INTEGER DEFAULT 0,
                config_hash     TEXT
            );

            CREATE TABLE IF NOT EXISTS blackbox_config (
                key             TEXT PRIMARY KEY,
                value           TEXT NOT NULL,
                updated_at      INTEGER NOT NULL
            );
            ",
        )?;

        Ok(Self { db_path })
    }

    pub fn get_default_json_path() -> PathBuf {
        let home_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
        PathBuf::from(home_dir)
            .join("Documents")
            .join("DeepFileX")
            .join("Logs")
            .join("blackbox_log.json")
    }

    pub fn insert_session(&self, meta: &SessionMeta) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT OR REPLACE INTO blackbox_sessions 
            (session_id, started_at, ended_at, app_version, os_info, total_events, config_hash)
            VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                meta.session_id,
                meta.started_at,
                meta.ended_at,
                meta.app_version,
                meta.os_info,
                meta.total_events,
                meta.config_hash,
            ],
        )?;
        Ok(())
    }

    pub fn write_batch(&self, events: &[BlackboxEvent]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        // 1. Write to SQLite database
        let mut conn = Connection::open(&self.db_path)?;
        let tx = conn.transaction()?;

        {
            let mut stmt = tx.prepare(
                "INSERT INTO blackbox_events (
                    timestamp_ms, session_id, sequence, event_type, component, action,
                    context_json, result, error_code, error_message, duration_ms,
                    app_version, os_version, build_type, pii_hash
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )?;

            for ev in events {
                let context_str = ev.context.as_ref().map(|v| v.to_string());
                stmt.execute(params![
                    ev.timestamp_ms,
                    ev.session_id,
                    ev.sequence,
                    ev.event_type.as_str(),
                    ev.component,
                    ev.action,
                    context_str,
                    ev.result.as_str(),
                    ev.error_code,
                    ev.error_message,
                    ev.duration_ms,
                    ev.app_version,
                    ev.os_version,
                    ev.build_type,
                    ev.pii_hash,
                ])?;
            }
        }
        tx.commit()?;

        // 2. Real-time auto-save to JSON file (blackbox_log.json)
        let json_path = Self::get_default_json_path();
        if let Some(parent) = json_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut all_events: Vec<BlackboxEvent> = if json_path.exists() {
            std::fs::read_to_string(&json_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        all_events.extend_from_slice(events);
        if let Ok(json_str) = serde_json::to_string_pretty(&all_events) {
            let _ = std::fs::write(&json_path, json_str);
        }

        Ok(())
    }

    pub fn query_all_events(&self) -> Result<Vec<BlackboxEvent>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT timestamp_ms, session_id, sequence, event_type, component, action,
                    context_json, result, error_code, error_message, duration_ms,
                    app_version, os_version, build_type, pii_hash
             FROM blackbox_events ORDER BY id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let type_str: String = row.get(3)?;
            let event_type = match type_str.as_str() {
                "ui" => super::types::EventType::Ui,
                "search" => super::types::EventType::Search,
                "indexing" => super::types::EventType::Indexing,
                "updater" => super::types::EventType::Updater,
                "error" => super::types::EventType::Error,
                _ => super::types::EventType::System,
            };

            let context_str: Option<String> = row.get(6)?;
            let context = context_str.and_then(|s| serde_json::from_str(&s).ok());

            let result_str: String = row.get(7)?;
            let result = if result_str == "success" {
                super::types::EventResult::Success
            } else if result_str == "cancelled" {
                super::types::EventResult::Cancelled
            } else {
                super::types::EventResult::Error {
                    code: row.get(8).unwrap_or_default(),
                    message: row.get(9).unwrap_or_default(),
                }
            };

            let duration_ms: Option<u64> = row.get(10)?;
            let duration_formatted = match duration_ms {
                Some(ms) if ms >= 1000 => Some(format!("{:.3}s", ms as f64 / 1000.0)),
                Some(ms) => Some(format!("{}ms", ms)),
                None => None,
            };

            let timestamp_ms: i64 = row.get(0)?;
            let datetime = chrono::DateTime::from_timestamp_millis(timestamp_ms)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "1970-01-01 00:00:00".to_string());

            Ok(BlackboxEvent {
                id: None,
                timestamp_ms,
                datetime,
                session_id: row.get(1)?,
                sequence: row.get(2)?,
                event_type,
                component: row.get(4)?,
                action: row.get(5)?,
                context,
                result,
                error_code: row.get(8)?,
                error_message: row.get(9)?,
                duration_ms,
                duration_formatted,
                app_version: row.get(11)?,
                os_version: row.get(12).unwrap_or_default(),
                build_type: row.get(13).unwrap_or_default(),
                pii_hash: row.get(14)?,
            })
        })?;

        let mut result_events = Vec::new();
        for r in rows {
            if let Ok(ev) = r {
                result_events.push(ev);
            }
        }

        Ok(result_events)
    }

    pub fn clear_all(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute("DELETE FROM blackbox_events", [])?;
        conn.execute("DELETE FROM blackbox_sessions", [])?;
        conn.execute("VACUUM", [])?;

        let json_path = Self::get_default_json_path();
        let _ = std::fs::remove_file(json_path);
        Ok(())
    }
}
