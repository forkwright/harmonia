use std::env;
use std::path::{Path, PathBuf};

/// Names an explicit secrets file path, taking priority over the
/// config-sibling default. Set by the NixOS module via systemd
/// `LoadCredential` (`%d/secrets.toml`), where the config file lives in the
/// read-only Nix store and has no writable sibling directory to resolve
/// `secrets.toml` against.
pub const SECRETS_PATH_ENV: &str = "HARMONIA_SECRETS_PATH";

/// Returns the secrets.toml path.
///
/// `HARMONIA_SECRETS_PATH` takes priority when set to a non-blank value;
/// otherwise falls back to a `secrets.toml` sibling of the given config file.
pub fn secrets_path(config_path: &Path) -> PathBuf {
    if let Ok(path) = env::var(SECRETS_PATH_ENV)
        && !path.trim().is_empty()
    {
        return PathBuf::from(path);
    }
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("secrets.toml")
}
