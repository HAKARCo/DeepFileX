use std::fmt;

#[derive(Debug)]
pub enum BlackboxError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    Json(serde_json::Error),
    Disabled,
    BufferFull,
    InvalidPath(String),
}

impl std::error::Error for BlackboxError {}

impl fmt::Display for BlackboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlackboxError::Io(e) => write!(f, "Blackbox I/O 에러: {}", e),
            BlackboxError::Sqlite(e) => write!(f, "Blackbox SQLite 에러: {}", e),
            BlackboxError::Json(e) => write!(f, "Blackbox JSON 직렬화 에러: {}", e),
            BlackboxError::Disabled => write!(f, "블랙박스 로거가 비활성화되어 있습니다."),
            BlackboxError::BufferFull => write!(f, "블랙박스 이벤트 버퍼가 가득 차있습니다."),
            BlackboxError::InvalidPath(msg) => write!(f, "유효하지 않은 경로: {}", msg),
        }
    }
}

impl From<std::io::Error> for BlackboxError {
    fn from(err: std::io::Error) -> Self {
        BlackboxError::Io(err)
    }
}

impl From<rusqlite::Error> for BlackboxError {
    fn from(err: rusqlite::Error) -> Self {
        BlackboxError::Sqlite(err)
    }
}

impl From<serde_json::Error> for BlackboxError {
    fn from(err: serde_json::Error) -> Self {
        BlackboxError::Json(err)
    }
}

pub type Result<T> = std::result::Result<T, BlackboxError>;
