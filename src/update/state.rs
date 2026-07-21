#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateState {
    Idle,
    Checking,
    Downloading,
    Verifying,
    Installing,
    Complete,
    RolledBack,
    Failed,
    UpToDate,
}

impl UpdateState {
    pub fn as_str(&self) -> &'static str {
        match self {
            UpdateState::Idle => "You are using the latest version (v3.3.0)",
            UpdateState::Checking => "Checking for updates...",
            UpdateState::Downloading => "Downloading...",
            UpdateState::Verifying => "Verifying integrity...",
            UpdateState::Installing => "Installing...",
            UpdateState::Complete => "Update complete",
            UpdateState::RolledBack => "Rollback complete (restored)",
            UpdateState::Failed => "Update failed",
            UpdateState::UpToDate => "You are using the latest version (v3.3.0)",
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateContext {
    pub state: UpdateState,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub error_message: Option<String>,
    pub new_version: Option<String>,
    pub backup_id: Option<String>,
}

impl Default for UpdateContext {
    fn default() -> Self {
        Self {
            state: UpdateState::Idle,
            downloaded_bytes: 0,
            total_bytes: 0,
            error_message: None,
            new_version: None,
            backup_id: None,
        }
    }
}

impl UpdateContext {
    pub fn transition(&mut self, new_state: UpdateState) {
        self.state = new_state;
        if new_state == UpdateState::Idle || new_state == UpdateState::UpToDate {
            self.downloaded_bytes = 0;
            self.total_bytes = 0;
            self.error_message = None;
            self.new_version = None;
            self.backup_id = None;
        }
    }

    pub fn set_progress(&mut self, downloaded: u64, total: u64) {
        self.downloaded_bytes = downloaded;
        self.total_bytes = total;
    }

    pub fn set_error(&mut self, err: String) {
        self.state = UpdateState::Failed;
        self.error_message = Some(err);
    }
}
