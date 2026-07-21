use std::sync::{Arc, Mutex, OnceLock};
use super::buffer::EventBuffer;
use super::error::Result;
use super::exporter::{ExportFormat, Exporter};
use super::session::SessionManager;
use super::storage::StorageAdapter;
use super::types::{BlackboxConfig, BlackboxEvent, EventResult, EventType};

pub struct BlackboxFacade {
    config: Mutex<BlackboxConfig>,
    session: SessionManager,
    buffer: EventBuffer,
    storage: Option<StorageAdapter>,
}

impl BlackboxFacade {
    pub fn new(app_version: &str) -> Self {
        let config = BlackboxConfig::default();
        let session = SessionManager::new(app_version);
        let buffer = EventBuffer::new(config.max_events);
        let storage = StorageAdapter::new(StorageAdapter::get_default_db_path()).ok();

        if let Some(ref st) = storage {
            let meta = session.to_meta(0, "default");
            let _ = st.insert_session(&meta);
        }

        Self {
            config: Mutex::new(config),
            session,
            buffer,
            storage,
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        let mut cfg = self.config.lock().unwrap_or_else(|e| e.into_inner());
        cfg.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        let cfg = self.config.lock().unwrap_or_else(|e| e.into_inner());
        cfg.enabled
    }

    pub fn get_config(&self) -> BlackboxConfig {
        let cfg = self.config.lock().unwrap_or_else(|e| e.into_inner());
        cfg.clone()
    }

    pub fn update_config(&self, new_config: BlackboxConfig) {
        let mut cfg = self.config.lock().unwrap_or_else(|e| e.into_inner());
        *cfg = new_config;
    }

    pub fn log_event(
        &self,
        event_type: EventType,
        component: &str,
        action: &str,
        context: Option<serde_json::Value>,
        result: EventResult,
        duration_ms: Option<u64>,
    ) {
        if !self.is_enabled() {
            return;
        }

        let now = chrono::Local::now();
        let timestamp_ms = now.timestamp_millis();
        let datetime = now.format("%Y-%m-%d %H:%M:%S.%3f").to_string();

        let sequence = self.session.next_sequence();

        let (error_code, error_message) = match &result {
            EventResult::Error { code, message } => (Some(code.clone()), Some(message.clone())),
            _ => (None, None),
        };

        let duration_formatted = match duration_ms {
            Some(ms) if ms >= 1000 => Some(format!("{:.3}s", ms as f64 / 1000.0)),
            Some(ms) => Some(format!("{}ms", ms)),
            None => None,
        };

        let event = BlackboxEvent {
            id: None,
            timestamp_ms,
            datetime,
            session_id: self.session.session_id().to_string(),
            sequence,
            event_type,
            component: component.to_string(),
            action: action.to_string(),
            context,
            result,
            error_code,
            error_message,
            duration_ms,
            duration_formatted,
            app_version: self.session.app_version().to_string(),
            os_version: self.session.os_info().to_string(),
            build_type: if cfg!(debug_assertions) { "debug" } else { "release" }.to_string(),
            pii_hash: None,
        };

        let should_flush = self.buffer.push(event);
        if should_flush {
            self.flush();
        }
    }

    pub fn flush(&self) {
        let events = self.buffer.drain();
        if events.is_empty() {
            return;
        }

        if let Some(ref storage) = self.storage {
            let _ = storage.write_batch(&events);
        }
    }

    pub fn export_logs<P: AsRef<std::path::Path>>(&self, dest_path: P, format: ExportFormat) -> Result<()> {
        self.flush();
        if let Some(ref storage) = self.storage {
            let cfg = self.get_config();
            Exporter::export_to_file(storage, dest_path, format, cfg.pii_masking)?;
        }
        Ok(())
    }

    pub fn clear_logs(&self) -> Result<()> {
        let _ = self.buffer.drain();
        if let Some(ref storage) = self.storage {
            storage.clear_all()?;
        }
        Ok(())
    }
}

pub fn get_global_logger() -> Arc<BlackboxFacade> {
    static LOGGER: OnceLock<Arc<BlackboxFacade>> = OnceLock::new();
    LOGGER.get_or_init(|| Arc::new(BlackboxFacade::new("3.3.0"))).clone()
}
