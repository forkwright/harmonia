use std::path::{Path, PathBuf};

/// Returns the secrets.toml path as a sibling of the given config file.
pub fn secrets_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("secrets.toml")
}
