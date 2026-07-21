use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Ui,
    Search,
    Indexing,
    Updater,
    Error,
    System,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::Ui => "ui",
            EventType::Search => "search",
            EventType::Indexing => "indexing",
            EventType::Updater => "updater",
            EventType::Error => "error",
            EventType::System => "system",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventResult {
    Success,
    Cancelled,
    Error { code: String, message: String },
    Timeout,
}

impl EventResult {
    pub fn as_str(&self) -> String {
        match self {
            EventResult::Success => "success".to_string(),
            EventResult::Cancelled => "cancelled".to_string(),
            EventResult::Error { code, .. } => format!("error:{}", code),
            EventResult::Timeout => "timeout".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackboxEvent {
    pub id: Option<i64>,
    pub timestamp_ms: i64,
    pub datetime: String,
    pub session_id: String,
    pub sequence: u64,
    pub event_type: EventType,
    pub component: String,
    pub action: String,
    pub context: Option<serde_json::Value>,
    pub result: EventResult,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: Option<u64>,
    pub duration_formatted: Option<String>,
    pub app_version: String,
    pub os_version: String,
    pub build_type: String,
    pub pii_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub session_id: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub app_version: String,
    pub os_info: String,
    pub total_events: u64,
    pub config_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackboxConfig {
    pub enabled: bool,
    pub max_events: usize,
    pub encrypt_db: bool,
    pub pii_masking: bool,
    pub flush_interval_ms: u64,
    pub max_db_size_mb: u64,
    pub auto_export_on_crash: bool,
}

impl Default for BlackboxConfig {
    fn default() -> Self {
        Self {
            enabled: true, // Enabled by default
            max_events: 10_000,
            encrypt_db: false,
            pii_masking: false,
            flush_interval_ms: 5_000,
            max_db_size_mb: 50,
            auto_export_on_crash: true,
        }
    }
}
