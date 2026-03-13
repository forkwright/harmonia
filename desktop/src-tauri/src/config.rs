use std::fs;
use std::path::PathBuf;

const DEFAULT_SERVER_URL: &str = "http://localhost:7700";
const CONFIG_FILE: &str = "server_url.txt";

pub fn config_dir() -> PathBuf {
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
        // Linux/other: use XDG_CONFIG_HOME or ~/.config
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

pub fn load_server_url() -> String {
    let path = config_dir().join(CONFIG_FILE);
    fs::read_to_string(path)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| DEFAULT_SERVER_URL.to_string())
}

pub fn save_server_url(url: &str) -> Result<(), std::io::Error> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(CONFIG_FILE), url)
}
