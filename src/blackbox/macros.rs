#[macro_export]
macro_rules! bb_log {
    (ui, $comp:expr, $act:expr) => {
        $crate::blackbox::facade::get_global_logger().log_event(
            $crate::blackbox::types::EventType::Ui,
            $comp,
            $act,
            None,
            $crate::blackbox::types::EventResult::Success,
            None,
        );
    };
    (ui, $comp:expr, $act:expr, $ctx:expr) => {
        $crate::blackbox::facade::get_global_logger().log_event(
            $crate::blackbox::types::EventType::Ui,
            $comp,
            $act,
            Some(serde_json::json!($ctx)),
            $crate::blackbox::types::EventResult::Success,
            None,
        );
    };
    (search, $act:expr, $ctx:expr) => {
        $crate::blackbox::facade::get_global_logger().log_event(
            $crate::blackbox::types::EventType::Search,
            "search_bar",
            $act,
            Some(serde_json::json!($ctx)),
            $crate::blackbox::types::EventResult::Success,
            None,
        );
    };
    (indexing, $act:expr, $ctx:expr) => {
        $crate::blackbox::facade::get_global_logger().log_event(
            $crate::blackbox::types::EventType::Indexing,
            "indexer",
            $act,
            Some(serde_json::json!($ctx)),
            $crate::blackbox::types::EventResult::Success,
            None,
        );
    };
    (error, $comp:expr, $code:expr, $msg:expr) => {
        $crate::blackbox::facade::get_global_logger().log_event(
            $crate::blackbox::types::EventType::Error,
            $comp,
            "error",
            None,
            $crate::blackbox::types::EventResult::Error {
                code: $code.to_string(),
                message: $msg.to_string(),
            },
            None,
        );
    };
}
