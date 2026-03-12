use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::watch;

use crate::HorismosError;
use crate::config::Config;
use crate::diff::diff_config;
use crate::load_config;
use crate::validation::ValidationWarning;

/// A shared handle to the live configuration. Subsystems hold a `ConfigHandle`
/// and call `.borrow()` for the current config or `.subscribe()` to react to changes.
#[derive(Clone)]
pub struct ConfigHandle {
    rx: watch::Receiver<Arc<Config>>,
}

/// The owner side — held by harmonia-host to push config updates.
#[derive(Clone)]
pub struct ConfigManager {
    tx: Arc<watch::Sender<Arc<Config>>>,
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new(initial: Config, config_path: PathBuf) -> (Self, ConfigHandle) {
        let (tx, rx) = watch::channel(Arc::new(initial));
        let manager = Self {
            tx: Arc::new(tx),
            config_path,
        };
        (manager, ConfigHandle { rx })
    }

    /// Re-read config from disk, validate, and broadcast if changed.
    ///
    /// Returns validation warnings. Errors are returned to the caller rather than
    /// crashing — the current config remains active on failure.
    pub fn reload(&self) -> Result<Vec<ValidationWarning>, HorismosError> {
        let (new_config, warnings) = load_config(Some(&self.config_path))?;

        let current = self.tx.borrow().clone();
        let changes = diff_config(&current, &new_config);

        if changes.is_empty() {
            tracing::info!("SIGHUP: config unchanged");
            return Ok(warnings);
        }

        for change in &changes {
            if change.requires_restart {
                tracing::warn!(
                    field = %change.field,
                    "SIGHUP: field changed but requires restart to take effect",
                );
            } else {
                tracing::info!(field = %change.field, "SIGHUP: field updated");
            }
        }

        self.tx.send_replace(Arc::new(new_config));
        Ok(warnings)
    }
}

impl ConfigHandle {
    /// Get the current config snapshot.
    pub fn borrow(&self) -> watch::Ref<'_, Arc<Config>> {
        self.rx.borrow()
    }

    /// Get a cloned Arc of the current config.
    pub fn current(&self) -> Arc<Config> {
        self.rx.borrow().clone()
    }

    /// Subscribe to config changes. The returned receiver marks itself changed
    /// whenever a new config is broadcast.
    pub fn subscribe(&self) -> watch::Receiver<Arc<Config>> {
        self.rx.clone()
    }
}

#[cfg(test)]
#[expect(clippy::result_large_err, reason = "test closures via figment::Jail")]
mod tests {
    use std::path::Path;

    use figment::Jail;

    use super::*;
    use crate::load_config;

    const VALID_JWT: &str = "a-very-long-secret-key-that-is-at-least-32-bytes-long";

    fn toml_with_port(port: u16) -> String {
        format!("[exousia]\njwt_secret = \"{VALID_JWT}\"\n\n[paroche]\nport = {port}\n")
    }

    // ── ConfigManager::new ────────────────────────────────────────────────────

    #[test]
    fn new_creates_manager_and_handle_with_initial_config() {
        let mut config = Config::default();
        config.exousia.jwt_secret = VALID_JWT.into();
        config.paroche.port = 9191;

        let (_, handle) = ConfigManager::new(config, PathBuf::from("harmonia.toml"));
        assert_eq!(handle.current().paroche.port, 9191);
    }

    // ── ConfigHandle accessors ────────────────────────────────────────────────

    #[test]
    fn borrow_returns_current_config() {
        let mut config = Config::default();
        config.exousia.jwt_secret = VALID_JWT.into();
        config.paroche.port = 8096;

        let (_, handle) = ConfigManager::new(config, PathBuf::from("harmonia.toml"));
        assert_eq!(handle.borrow().paroche.port, 8096);
    }

    #[test]
    fn current_returns_cloned_arc() {
        let mut config = Config::default();
        config.exousia.jwt_secret = VALID_JWT.into();

        let (_, handle) = ConfigManager::new(config, PathBuf::from("harmonia.toml"));
        let a = handle.current();
        let b = handle.current();
        assert_eq!(a.paroche.port, b.paroche.port);
    }

    // ── ConfigManager::reload ─────────────────────────────────────────────────

    #[test]
    fn reload_with_unchanged_file_returns_no_warnings() {
        Jail::expect_with(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port(8096))?;
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, _) = ConfigManager::new(config, PathBuf::from("harmonia.toml"));

            let warnings = manager.reload().unwrap();
            assert!(warnings.is_empty());
            Ok(())
        });
    }

    #[test]
    fn reload_with_changed_paroche_updates_config() {
        Jail::expect_with(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port(8096))?;
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, handle) = ConfigManager::new(config, PathBuf::from("harmonia.toml"));

            jail.create_file("harmonia.toml", &toml_with_port(9090))?;
            manager.reload().unwrap();

            assert_eq!(handle.current().paroche.port, 9090);
            Ok(())
        });
    }

    #[test]
    fn reload_with_changed_database_path_updates_config() {
        Jail::expect_with(|jail| {
            jail.create_file(
                "harmonia.toml",
                &format!(
                    "[exousia]\njwt_secret = \"{VALID_JWT}\"\n\n[database]\ndb_path = \"/tmp/harmonia.db\"\n"
                ),
            )?;
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, handle) = ConfigManager::new(config, PathBuf::from("harmonia.toml"));

            jail.create_file(
                "harmonia.toml",
                &format!(
                    "[exousia]\njwt_secret = \"{VALID_JWT}\"\n\n[database]\ndb_path = \"/tmp/harmonia2.db\"\n"
                ),
            )?;
            manager.reload().unwrap();

            assert_eq!(
                handle.current().database.db_path,
                PathBuf::from("/tmp/harmonia2.db")
            );
            Ok(())
        });
    }

    #[test]
    fn reload_with_invalid_config_returns_error_and_keeps_current() {
        Jail::expect_with(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port(8096))?;
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, handle) = ConfigManager::new(config, PathBuf::from("harmonia.toml"));

            // Remove jwt_secret — validation will reject it
            jail.create_file("harmonia.toml", "[paroche]\nport = 9090\n")?;
            let result = manager.reload();

            assert!(result.is_err());
            assert_eq!(handle.current().paroche.port, 8096);
            Ok(())
        });
    }

    #[test]
    fn reload_broadcasts_to_all_subscribers() {
        Jail::expect_with(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port(8096))?;
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, handle) = ConfigManager::new(config, PathBuf::from("harmonia.toml"));

            let mut rx1 = handle.subscribe();
            let mut rx2 = handle.subscribe();

            jail.create_file("harmonia.toml", &toml_with_port(9090))?;
            manager.reload().unwrap();

            assert!(rx1.has_changed().unwrap());
            assert!(rx2.has_changed().unwrap());
            assert_eq!(rx1.borrow_and_update().paroche.port, 9090);
            assert_eq!(rx2.borrow_and_update().paroche.port, 9090);
            Ok(())
        });
    }

    // ── ConfigHandle::subscribe ───────────────────────────────────────────────

    #[test]
    fn subscribe_yields_on_change() {
        Jail::expect_with(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port(8096))?;
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, handle) = ConfigManager::new(config, PathBuf::from("harmonia.toml"));
            let mut rx = handle.subscribe();

            jail.create_file("harmonia.toml", &toml_with_port(9090))?;
            manager.reload().unwrap();

            assert!(rx.has_changed().unwrap());
            assert_eq!(rx.borrow_and_update().paroche.port, 9090);
            Ok(())
        });
    }

    #[test]
    fn subscribe_does_not_yield_when_config_unchanged() {
        Jail::expect_with(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port(8096))?;
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, handle) = ConfigManager::new(config, PathBuf::from("harmonia.toml"));
            let rx = handle.subscribe();

            // File unchanged — reload should be a no-op
            manager.reload().unwrap();

            assert!(!rx.has_changed().unwrap());
            Ok(())
        });
    }
}
