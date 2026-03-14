//! Application configuration: persisted as JSON in the platform config directory.
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub(crate) fn config_dir() -> PathBuf {
    let base = if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("Library")
            .join("Application Support")
    } else if cfg!(target_os = "windows") {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
    } else {
        // Linux/other: XDG_CONFIG_HOME or ~/.config
        std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(".config")
            })
    };
    base.join("harmonia")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct NotificationConfig {
    pub enabled: bool,
    pub track_change: bool,
    pub downloads: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            track_change: true,
            downloads: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct WindowConfig {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 1280,
            height: 800,
            maximized: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct AppConfig {
    pub server_url: String,
    pub minimize_to_tray: bool,
    pub auto_start: bool,
    pub start_minimized: bool,
    pub notifications: NotificationConfig,
    pub window: WindowConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:7700".to_string(),
            minimize_to_tray: true,
            auto_start: false,
            start_minimized: false,
            notifications: NotificationConfig::default(),
            window: WindowConfig::default(),
        }
    }
}

const CONFIG_FILE: &str = "config.json";

pub(crate) fn load_config() -> AppConfig {
    let path = config_dir().join(CONFIG_FILE);
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub(crate) fn save_config(config: &AppConfig) -> Result<(), std::io::Error> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    let json = serde_json::to_string_pretty(config).unwrap_or_else(|_| "{}".to_string());
    fs::write(dir.join(CONFIG_FILE), json)
}

/// Reads only the server URL from the persisted config.
pub(crate) fn load_server_url() -> String {
    load_config().server_url
}

/// Updates the server URL in the persisted config.
pub(crate) fn save_server_url(url: &str) -> Result<(), std::io::Error> {
    let mut config = load_config();
    config.server_url = url.to_string();
    save_config(&config)
}
