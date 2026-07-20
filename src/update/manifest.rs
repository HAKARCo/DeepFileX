use std::collections::HashMap;
use crate::update::error::{UpdateError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    Stable,
    Beta,
    Nightly,
}

impl Channel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
            Channel::Nightly => "nightly",
        }
    }

    pub fn from_str(s: &str) -> std::result::Result<Self, String> {
        match s.to_lowercase().as_str() {
            "stable" => Ok(Channel::Stable),
            "beta" => Ok(Channel::Beta),
            "nightly" => Ok(Channel::Nightly),
            _ => Err(format!("알 수 없는 채널: {}", s)),
        }
    }
}

#[cfg(feature = "auto-update")]
impl<'de> serde::Deserialize<'de> for Channel {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Channel::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "auto-update")]
impl serde::Serialize for Channel {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre: Option<String>,
}

impl Version {
    pub fn parse(s: &str) -> std::result::Result<Self, String> {
        let clean = s.trim().trim_start_matches('v');
        let parts: Vec<&str> = clean.split('-').collect();
        let main_part = parts[0];
        let pre_part = parts.get(1).map(|&s| s.to_string());

        let nums: Vec<&str> = main_part.split('.').collect();
        if nums.len() != 3 {
            return Err(format!("유효하지 않은 버전 형식: {}", s));
        }

        let major = nums[0].parse::<u32>().map_err(|e| e.to_string())?;
        let minor = nums[1].parse::<u32>().map_err(|e| e.to_string())?;
        let patch = nums[2].parse::<u32>().map_err(|e| e.to_string())?;

        Ok(Version {
            major,
            minor,
            patch,
            pre: pre_part,
        })
    }

    pub fn to_string(&self) -> String {
        let mut s = format!("{}.{}.{}", self.major, self.minor, self.patch);
        if let Some(ref pre) = self.pre {
            s.push('-');
            s.push_str(pre);
        }
        s
    }
}

#[cfg(feature = "auto-update")]
impl<'de> serde::Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Version::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "auto-update")]
impl serde::Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(feature = "auto-update")]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PlatformArtifact {
    pub url: String,
    pub size: u64,
    pub sha256: String,
    pub installer_type: String, // "nsis" 등
    pub requires_admin: bool,
    pub auto_exit_app: bool,
}

#[cfg(feature = "auto-update")]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ChannelInfo {
    pub current_version: Version,
    pub minimum_supported_version: Version,
    pub release_date: String,
    pub release_notes_url: String,
    pub platforms: HashMap<String, PlatformArtifact>,
}

#[cfg(feature = "auto-update")]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Manifest {
    pub version: u32,
    pub generated_at: String,
    pub channels: HashMap<Channel, ChannelInfo>,
}

impl Manifest {
    #[cfg(feature = "auto-update")]
    pub fn from_json(json_str: &str) -> Result<Self> {
        serde_json::from_str(json_str).map_err(|e| UpdateError::ManifestInvalid(e.to_string()))
    }
}
