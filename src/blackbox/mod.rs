pub mod buffer;
pub mod error;
pub mod exporter;
pub mod facade;
pub mod macros;
pub mod sanitizer;
pub mod session;
pub mod storage;
pub mod types;

pub use facade::{get_global_logger, BlackboxFacade};
pub use types::{BlackboxConfig, BlackboxEvent, EventResult, EventType};

#[cfg(test)]
mod tests {
    use super::*;
    use exporter::ExportFormat;
    use sanitizer::{mask_pii, PiiType};

    #[test]
    fn test_pii_masking() {
        let path = "C:\\Users\\kim\\Documents\\secret.hwp";
        let (raw_path, _) = mask_pii(path, PiiType::Path);
        assert_eq!(raw_path, path);

        let query = "hakar contract";
        let (raw_query, _) = mask_pii(query, PiiType::Query);
        assert_eq!(raw_query, query);
    }

    #[test]
    fn test_blackbox_facade_and_storage() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("test_blackbox.db");
        let _ = std::fs::remove_file(&db_path);

        let _storage = storage::StorageAdapter::new(&db_path).unwrap();
        let logger = get_global_logger();
        logger.set_enabled(true);

        logger.log_event(
            EventType::Search,
            "search_bar",
            "query_submit",
            Some(serde_json::json!({ "query": "hakar" })),
            EventResult::Success,
            Some(12),
        );

        logger.flush();

        let json_export = temp_dir.join("blackbox_export.json");
        let res = logger.export_logs(&json_export, ExportFormat::Json);
        assert!(res.is_ok());

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(&json_export);
    }
}
