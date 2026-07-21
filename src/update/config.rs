use std::path::PathBuf;
use crate::update::manifest::Channel;

#[derive(Debug, Clone)]
pub struct UpdateConfig {
    pub enabled: bool,
    pub auto_check: bool,
    pub auto_download: bool,
    pub channel: Channel,
    pub check_interval_hours: u32,
    pub last_check: Option<String>, // ISO8601 형식
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_check: true,
            auto_download: false,
            channel: Channel::Stable,
            check_interval_hours: 24,
            last_check: None,
        }
    }
}

#[cfg(feature = "auto-update")]
impl<'de> serde::Deserialize<'de> for UpdateConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct TempConfig {
            enabled: Option<bool>,
            auto_check: Option<bool>,
            auto_download: Option<bool>,
            channel: Option<Channel>,
            check_interval_hours: Option<u32>,
            last_check: Option<String>,
        }
        let temp = TempConfig::deserialize(deserializer)?;
        Ok(UpdateConfig {
            enabled: temp.enabled.unwrap_or(false),
            auto_check: temp.auto_check.unwrap_or(false),
            auto_download: temp.auto_download.unwrap_or(false),
            channel: temp.channel.unwrap_or(Channel::Stable),
            check_interval_hours: temp.check_interval_hours.unwrap_or(24),
            last_check: temp.last_check,
        })
    }
}

#[cfg(feature = "auto-update")]
impl serde::Serialize for UpdateConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct TempConfig {
            enabled: bool,
            auto_check: bool,
            auto_download: bool,
            channel: Channel,
            check_interval_hours: u32,
            last_check: Option<String>,
        }
        let temp = TempConfig {
            enabled: self.enabled,
            auto_check: self.auto_check,
            auto_download: self.auto_download,
            channel: self.channel,
            check_interval_hours: self.check_interval_hours,
            last_check: self.last_check.clone(),
        };
        temp.serialize(serializer)
    }
}

impl UpdateConfig {
    pub fn get_config_path() -> PathBuf {
        let home_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
        std::path::PathBuf::from(home_dir)
            .join("Documents")
            .join("DeepFileX")
            .join("update_config.json")
    }

    pub fn load() -> Self {
        #[cfg(feature = "auto-update")]
        {
            let path = Self::get_config_path();
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(config) = serde_json::from_str::<UpdateConfig>(&content) {
                        return config;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        #[cfg(feature = "auto-update")]
        {
            let path = Self::get_config_path();
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(content) = serde_json::to_string_pretty(self) {
                let _ = fs::write(path, content);
            }
        }
    }
}
