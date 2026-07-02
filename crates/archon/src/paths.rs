// Shared filesystem path resolution for the archon host binary.

use std::path::PathBuf;

/// Harmonia's base configuration directory.
///
/// Resolves `$XDG_CONFIG_HOME/harmonia`, falling back to
/// `$HOME/.config/harmonia`, then `/tmp/harmonia` when neither variable is
/// set. Never returns a path containing a literal `~` — shells expand tilde,
/// `PathBuf` does not.
pub(crate) fn dirs_config_path() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".config"))
                .unwrap_or_else(|_| PathBuf::from("/tmp"))
        })
        .join("harmonia")
}

/// Default directory for renderer TLS certificates and pairing credentials.
pub(crate) fn default_renderer_cert_dir() -> PathBuf {
    dirs_config_path().join("renderer")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirs_config_path_never_contains_tilde() {
        let path = dirs_config_path();
        assert!(
            path.components()
                .all(|c| c.as_os_str().to_str() != Some("~")),
            "config path must never carry an unexpanded tilde: {}",
            path.display()
        );
        assert!(path.ends_with("harmonia"), "got: {}", path.display());
    }

    #[test]
    fn dirs_config_path_is_absolute() {
        // WHY: every fallback branch (XDG, HOME/.config, /tmp) yields an
        // absolute root, so the joined path must be absolute regardless of CWD.
        assert!(dirs_config_path().is_absolute());
    }

    #[test]
    fn default_renderer_cert_dir_is_under_config_root() {
        let cert_dir = default_renderer_cert_dir();
        assert!(cert_dir.starts_with(dirs_config_path()));
        assert!(cert_dir.ends_with("harmonia/renderer"));
        assert!(
            cert_dir
                .components()
                .all(|c| c.as_os_str().to_str() != Some("~"))
        );
    }
}
