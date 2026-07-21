use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use super::types::SessionMeta;

pub struct SessionManager {
    session_id: String,
    started_at: i64,
    sequence: AtomicU64,
    app_version: String,
    os_info: String,
}

impl SessionManager {
    pub fn new(app_version: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let uuid = format!("{:016x}{:016x}", now, rand_simple());

        Self {
            session_id: uuid,
            started_at: now,
            sequence: AtomicU64::new(1),
            app_version: app_version.to_string(),
            os_info: std::env::consts::OS.to_string(),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn next_sequence(&self) -> u64 {
        self.sequence.fetch_add(1, Ordering::SeqCst)
    }

    pub fn app_version(&self) -> &str {
        &self.app_version
    }

    pub fn os_info(&self) -> &str {
        &self.os_info
    }

    pub fn to_meta(&self, total_events: u64, config_hash: &str) -> SessionMeta {
        SessionMeta {
            session_id: self.session_id.clone(),
            started_at: self.started_at,
            ended_at: None,
            app_version: self.app_version.clone(),
            os_info: self.os_info.clone(),
            total_events,
            config_hash: config_hash.to_string(),
        }
    }
}

fn rand_simple() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    std::time::Instant::now().hash(&mut hasher);
    hasher.finish()
}
