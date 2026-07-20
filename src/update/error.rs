use std::{fmt, io};

#[derive(Debug)]
pub enum UpdateError {
    Io(io::Error),
    Http(String),
    Json(String),
    HashMismatch { expected: String, actual: String },
    ManifestInvalid(String),
    InstallerFailed(i32),
    BackupFailed(String),
    RollbackFailed(String),
    VerificationFailed(String),
    InvalidStateTransition(String),
    AlreadyRunning,
    Cancelled,
}

impl std::error::Error for UpdateError {}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpdateError::Io(e) => write!(f, "I/O error: {}", e),
            UpdateError::Http(e) => write!(f, "HTTP error: {}", e),
            UpdateError::Json(e) => write!(f, "JSON parsing error: {}", e),
            UpdateError::HashMismatch { expected, actual } => {
                write!(f, "Hash mismatch: expected {}, got {}", expected, actual)
            }
            UpdateError::ManifestInvalid(e) => write!(f, "Invalid manifest: {}", e),
            UpdateError::InstallerFailed(code) => write!(f, "Installer execution failed (code: {})", code),
            UpdateError::BackupFailed(e) => write!(f, "Backup failed: {}", e),
            UpdateError::RollbackFailed(e) => write!(f, "Rollback failed: {}", e),
            UpdateError::VerificationFailed(e) => write!(f, "Verification failed: {}", e),
            UpdateError::InvalidStateTransition(e) => write!(f, "Invalid state transition: {}", e),
            UpdateError::AlreadyRunning => write!(f, "Update is already running."),
            UpdateError::Cancelled => write!(f, "Update cancelled by user."),
        }
    }
}

impl From<io::Error> for UpdateError {
    fn from(err: io::Error) -> Self {
        UpdateError::Io(err)
    }
}

#[cfg(feature = "auto-update")]
impl From<reqwest::Error> for UpdateError {
    fn from(err: reqwest::Error) -> Self {
        UpdateError::Http(err.to_string())
    }
}

#[cfg(feature = "auto-update")]
impl From<serde_json::Error> for UpdateError {
    fn from(err: serde_json::Error) -> Self {
        UpdateError::Json(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, UpdateError>;
