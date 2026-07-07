use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::watch;

use crate::config::Config;
use crate::diff::{diff_config, held_back_merge};
use crate::validation::{ValidationWarning, validate_config};
use crate::{HorismosError, load_config};

/// CLI-level overrides pinned for the lifetime of the process. Re-applied to
/// the config after EVERY load so a reload cannot silently un-pin them.
#[derive(Debug, Clone, Default)]
pub struct ConfigOverrides {
    pub listen_addr: Option<String>,
    pub port: Option<u16>,
}

impl ConfigOverrides {
    fn apply(&self, config: &mut Config) {
        if let Some(listen_addr) = &self.listen_addr {
            config.paroche.listen_addr = listen_addr.clone();
        }
        if let Some(port) = self.port {
            config.paroche.port = port;
        }
    }
}

/// Result of a reload/replace. `applied` and `restart_pending` are dotted
/// leaf paths (e.g. `paroche.port`); restart-class leaves are held back from
/// the published config and listed in `restart_pending` until the process
/// restarts or the file reverts.
#[derive(Debug)]
pub struct ReloadOutcome {
    pub warnings: Vec<ValidationWarning>,
    pub applied: Vec<String>,
    pub restart_pending: Vec<String>,
}

impl ReloadOutcome {
    /// True when nothing changed relative to the effective config.
    pub fn is_unchanged(&self) -> bool {
        self.applied.is_empty() && self.restart_pending.is_empty()
    }

    /// True when at least one change is held back pending a restart.
    pub fn needs_restart(&self) -> bool {
        !self.restart_pending.is_empty()
    }
}

/// A shared handle to the live configuration. Subsystems hold a `ConfigHandle`
/// and call `.current()` for the current config, `.section()` for a typed live
/// sub-view, or `.subscribe()` to react to changes.
#[derive(Clone)]
pub struct ConfigHandle {
    rx: watch::Receiver<Arc<Config>>,
    pending_rx: watch::Receiver<Vec<String>>,
}

/// The owner side — held by archon to push config updates.
#[derive(Clone)]
pub struct ConfigManager {
    tx: Arc<watch::Sender<Arc<Config>>>,
    pending_tx: Arc<watch::Sender<Vec<String>>>,
    config_path: PathBuf,
    overrides: ConfigOverrides,
}

impl ConfigManager {
    pub fn new(
        initial: Config,
        config_path: PathBuf,
        overrides: ConfigOverrides,
    ) -> (Self, ConfigHandle) {
        let mut initial = initial;
        overrides.apply(&mut initial);
        let (tx, rx) = watch::channel(Arc::new(initial));
        let (pending_tx, pending_rx) = watch::channel(Vec::new());
        let manager = Self {
            tx: Arc::new(tx),
            pending_tx: Arc::new(pending_tx),
            config_path,
            overrides,
        };
        (manager, ConfigHandle { rx, pending_rx })
    }

    /// Re-read config from disk, re-apply overrides, and publish the merged
    /// effective config. Restart-class changed leaves are held back (the
    /// published config keeps their effective values) and reported in
    /// `restart_pending`.
    ///
    /// Errors are returned to the caller rather than crashing — the current
    /// config remains active on failure.
    ///
    /// WARNING: performs blocking file I/O (figment reads the TOML from disk).
    /// Callers on an async runtime must dispatch through
    /// `tokio::task::spawn_blocking` to avoid stalling a worker thread.
    pub fn reload(&self) -> Result<ReloadOutcome, HorismosError> {
        let (new_config, warnings) = load_config(Some(&self.config_path))?;
        self.publish(new_config, warnings)
    }

    /// Publish a programmatic config through the same override + held-back
    /// pipeline as `reload`. Validates the incoming config first.
    pub fn replace(&self, new: Config) -> Result<ReloadOutcome, HorismosError> {
        let warnings = validate_config(&new)?;
        self.publish(new, warnings)
    }

    fn publish(
        &self,
        mut new_config: Config,
        warnings: Vec<ValidationWarning>,
    ) -> Result<ReloadOutcome, HorismosError> {
        self.overrides.apply(&mut new_config);

        let current = self.tx.borrow().clone();
        let changes = diff_config(&current, &new_config);

        let mut applied = Vec::new();
        let mut restart_pending = Vec::new();
        for change in changes {
            if change.requires_restart {
                tracing::warn!(
                    field = %change.path,
                    "config: change held back — restart required to take effect",
                );
                restart_pending.push(change.path);
            } else {
                tracing::info!(field = %change.path, "config: field updated");
                applied.push(change.path);
            }
        }

        if applied.is_empty() {
            tracing::info!("config: no live changes to apply");
        } else {
            // INVARIANT: `current()` always reflects what is actually in
            // effect — restart-class changed leaves are reverted to their
            // effective values before the new config is published.
            let effective = held_back_merge(&current, &new_config)?;
            self.tx.send_replace(Arc::new(effective));
        }

        // NOTE: restart-pending is derived (file-vs-effective) on every
        // publish, so reverting the change on disk clears it.
        self.pending_tx.send_if_modified(|pending| {
            if *pending == restart_pending {
                false
            } else {
                pending.clone_from(&restart_pending);
                true
            }
        });

        Ok(ReloadOutcome {
            warnings,
            applied,
            restart_pending,
        })
    }
}

impl ConfigHandle {
    // NOTE: no `borrow()` accessor — a public `watch::Ref` held across an
    // .await point blocks the reload writer; `current()` returns an owned
    // Arc snapshot with no lifetime hazard at the cost of one Arc clone.

    /// Get a cloned Arc of the current effective config.
    pub fn current(&self) -> Arc<Config> {
        self.rx.borrow().clone()
    }

    /// Subscribe to config changes. The returned receiver marks itself changed
    /// whenever a new effective config is broadcast.
    pub fn subscribe(&self) -> watch::Receiver<Arc<Config>> {
        self.rx.clone()
    }

    /// Typed live sub-view of one config section.
    ///
    /// Consumption idiom: read once per operation (`let cfg = section.get();`)
    /// and use that snapshot for the whole operation — consistency within an
    /// op, liveness across ops.
    pub fn section<T: Clone>(&self, project: fn(&Config) -> &T) -> Section<T> {
        Section {
            inner: SectionInner::Live {
                rx: self.rx.clone(),
                project,
            },
        }
    }

    /// Change-detection watcher over one config section.
    ///
    /// Unlike `section()`, this is consumed by a long-lived supervisor task
    /// (`.changed().await` in a loop) rather than read per-operation — the
    /// primitive every REBUILD/LIVE-B consumer builds on.
    pub fn watch_section<T: Clone + PartialEq>(
        &self,
        project: fn(&Config) -> &T,
    ) -> SectionWatcher<T> {
        let last = project(&self.rx.borrow()).clone();
        SectionWatcher {
            rx: self.rx.clone(),
            project,
            last,
        }
    }

    /// Dotted leaf paths changed on disk but held back pending a restart.
    pub fn restart_pending(&self) -> Vec<String> {
        self.pending_rx.borrow().clone()
    }

    /// A handle serving a fixed config (tests / static wiring). The senders
    /// are dropped; `current()` and `section()` keep serving the last value.
    pub fn fixed(config: Config) -> ConfigHandle {
        let (_, rx) = watch::channel(Arc::new(config));
        let (_, pending_rx) = watch::channel(Vec::new());
        ConfigHandle { rx, pending_rx }
    }
}

/// A typed view of one config section.
///
/// Consumption idiom: `get()` returns an OWNED clone (sections are small).
/// Read once per operation and use that snapshot throughout — consistency
/// within an op, liveness across ops. No guard is ever held across an .await.
#[derive(Clone)]
pub struct Section<T> {
    inner: SectionInner<T>,
}

#[derive(Clone)]
enum SectionInner<T> {
    Live {
        rx: watch::Receiver<Arc<Config>>,
        project: fn(&Config) -> &T,
    },
    Fixed(Arc<T>),
}

impl<T: Clone> Section<T> {
    /// Owned snapshot of the current section value.
    pub fn get(&self) -> T {
        match &self.inner {
            SectionInner::Live { rx, project } => project(&rx.borrow()).clone(),
            SectionInner::Fixed(value) => (**value).clone(),
        }
    }

    /// A section pinned to a static value (tests / non-live wiring).
    pub fn fixed(value: T) -> Self {
        Self {
            inner: SectionInner::Fixed(Arc::new(value)),
        }
    }
}

/// Change-detection watcher over one config section, built by
/// `ConfigHandle::watch_section`.
///
/// Consumption idiom: `.changed().await` in a supervisor loop — it yields
/// only when the projected section actually differs from the last-seen
/// value (a publish that touches a different section is not a wakeup), and
/// returns `None` once the `ConfigManager` (and every clone) is dropped —
/// the supervisor's exit signal.
pub struct SectionWatcher<T> {
    rx: watch::Receiver<Arc<Config>>,
    project: fn(&Config) -> &T,
    last: T,
}

impl<T: Clone + PartialEq> SectionWatcher<T> {
    /// Waits for the next publish that changes this section's projected
    /// value, returning the new value. Returns `None` when the sender side
    /// is dropped and no further changes can ever arrive.
    pub async fn changed(&mut self) -> Option<T> {
        loop {
            if self.rx.changed().await.is_err() {
                return None;
            }
            let next = (self.project)(&self.rx.borrow_and_update()).clone();
            if next != self.last {
                self.last = next.clone();
                return Some(next);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use figment::Jail;

    use super::*;
    use crate::load_config;

    const VALID_JWT: &str = "a-very-long-secret-key-that-is-at-least-32-bytes-long";

    fn toml_with_port(port: u16) -> String {
        format!("[exousia]\njwt_secret = \"{VALID_JWT}\"\n\n[paroche]\nport = {port}\n")
    }

    fn toml_with_port_and_db(port: u16, db_path: &str) -> String {
        format!(
            "[exousia]\njwt_secret = \"{VALID_JWT}\"\n\n[paroche]\nport = {port}\n\n[database]\ndb_path = \"{db_path}\"\n"
        )
    }

    fn valid_config() -> Config {
        let mut config = Config::default();
        config.exousia.jwt_secret = VALID_JWT.into();
        config
    }

    #[allow(
        clippy::result_large_err,
        reason = "figment::Jail::expect_with requires figment::Result; this lint is version-dependent"
    )]
    fn with_jail(run: impl FnOnce(&mut Jail)) {
        Jail::expect_with(|jail| {
            run(jail);
            Ok(())
        });
    }

    // ── ConfigManager::new ────────────────────────────────────────────────────

    #[test]
    fn new_creates_manager_and_handle_with_initial_config() {
        let mut config = valid_config();
        config.paroche.port = 9191;

        let (_, handle) = ConfigManager::new(
            config,
            PathBuf::from("harmonia.toml"),
            ConfigOverrides::default(),
        );
        assert_eq!(handle.current().paroche.port, 9191);
        assert!(handle.restart_pending().is_empty());
    }

    #[test]
    fn new_applies_overrides_to_initial_config() {
        let config = valid_config();
        let overrides = ConfigOverrides {
            listen_addr: Some("127.0.0.1".to_string()),
            port: Some(9999),
        };

        let (_, handle) = ConfigManager::new(config, PathBuf::from("harmonia.toml"), overrides);
        assert_eq!(handle.current().paroche.port, 9999);
        assert_eq!(handle.current().paroche.listen_addr, "127.0.0.1");
    }

    // ── ConfigHandle accessors ────────────────────────────────────────────────

    #[test]
    fn current_returns_cloned_arc() {
        let config = valid_config();

        let (_, handle) = ConfigManager::new(
            config,
            PathBuf::from("harmonia.toml"),
            ConfigOverrides::default(),
        );
        let a = handle.current();
        let b = handle.current();
        assert_eq!(a.paroche.port, b.paroche.port);
    }

    #[test]
    fn fixed_handle_serves_config_via_current_and_section() {
        let mut config = valid_config();
        config.paroche.port = 9393;

        let handle = ConfigHandle::fixed(config);
        assert_eq!(handle.current().paroche.port, 9393);

        let section = handle.section(|c| &c.paroche);
        assert_eq!(section.get().port, 9393);
        assert!(handle.restart_pending().is_empty());
    }

    // ── Section ───────────────────────────────────────────────────────────────

    #[test]
    fn section_get_returns_current_value_and_tracks_replace() {
        let config = valid_config();
        let (manager, handle) = ConfigManager::new(
            config,
            PathBuf::from("harmonia.toml"),
            ConfigOverrides::default(),
        );
        let section = handle.section(|c| &c.paroche);
        assert_eq!(section.get().port, 8096);

        let mut changed = valid_config();
        changed.paroche.port = 9090;
        manager.replace(changed).unwrap();

        assert_eq!(section.get().port, 9090);
    }

    #[test]
    fn section_fixed_serves_pinned_value() {
        let mut config = valid_config();
        config.paroche.port = 4242;

        let section = Section::fixed(config.paroche.clone());
        assert_eq!(section.get().port, 4242);
        assert_eq!(section.get().listen_addr, config.paroche.listen_addr);
    }

    // ── ConfigManager::reload ─────────────────────────────────────────────────

    #[test]
    fn reload_with_unchanged_file_returns_empty_outcome() {
        with_jail(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port(8096))
                .unwrap();
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, _) = ConfigManager::new(
                config,
                PathBuf::from("harmonia.toml"),
                ConfigOverrides::default(),
            );

            let outcome = manager.reload().unwrap();
            assert!(outcome.warnings.is_empty());
            assert!(outcome.applied.is_empty());
            assert!(outcome.restart_pending.is_empty());
            assert!(outcome.is_unchanged());
            assert!(!outcome.needs_restart());
        });
    }

    #[test]
    fn reload_with_changed_paroche_updates_config() {
        with_jail(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port(8096))
                .unwrap();
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, handle) = ConfigManager::new(
                config,
                PathBuf::from("harmonia.toml"),
                ConfigOverrides::default(),
            );

            jail.create_file("harmonia.toml", &toml_with_port(9090))
                .unwrap();
            let outcome = manager.reload().unwrap();

            assert_eq!(handle.current().paroche.port, 9090);
            assert_eq!(outcome.applied, vec!["paroche.port"]);
            assert!(outcome.restart_pending.is_empty());
        });
    }

    #[test]
    fn reload_holds_back_database_change_and_applies_live_change() {
        with_jail(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port_and_db(8096, "/tmp/a.db"))
                .unwrap();
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, handle) = ConfigManager::new(
                config,
                PathBuf::from("harmonia.toml"),
                ConfigOverrides::default(),
            );

            jail.create_file("harmonia.toml", &toml_with_port_and_db(9090, "/tmp/b.db"))
                .unwrap();
            let outcome = manager.reload().unwrap();

            let current = handle.current();
            assert_eq!(current.paroche.port, 9090);
            assert_eq!(current.database.db_path, PathBuf::from("/tmp/a.db"));
            assert_eq!(outcome.applied, vec!["paroche.port"]);
            assert_eq!(outcome.restart_pending, vec!["database.db_path"]);
            assert!(outcome.needs_restart());
            assert!(!outcome.is_unchanged());
            assert_eq!(handle.restart_pending(), vec!["database.db_path"]);
        });
    }

    #[test]
    fn restart_only_change_does_not_wake_subscribers() {
        with_jail(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port_and_db(8096, "/tmp/a.db"))
                .unwrap();
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, handle) = ConfigManager::new(
                config,
                PathBuf::from("harmonia.toml"),
                ConfigOverrides::default(),
            );
            let rx = handle.subscribe();

            jail.create_file("harmonia.toml", &toml_with_port_and_db(8096, "/tmp/b.db"))
                .unwrap();
            let outcome = manager.reload().unwrap();

            assert!(outcome.applied.is_empty());
            assert_eq!(outcome.restart_pending, vec!["database.db_path"]);
            assert!(!rx.has_changed().unwrap());
            assert_eq!(
                handle.current().database.db_path,
                PathBuf::from("/tmp/a.db")
            );
        });
    }

    #[test]
    fn restart_pending_clears_when_file_reverted() {
        with_jail(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port_and_db(8096, "/tmp/a.db"))
                .unwrap();
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, handle) = ConfigManager::new(
                config,
                PathBuf::from("harmonia.toml"),
                ConfigOverrides::default(),
            );

            jail.create_file("harmonia.toml", &toml_with_port_and_db(8096, "/tmp/b.db"))
                .unwrap();
            manager.reload().unwrap();
            assert_eq!(handle.restart_pending(), vec!["database.db_path"]);

            jail.create_file("harmonia.toml", &toml_with_port_and_db(8096, "/tmp/a.db"))
                .unwrap();
            let outcome = manager.reload().unwrap();
            assert!(outcome.restart_pending.is_empty());
            assert!(handle.restart_pending().is_empty());
        });
    }

    #[test]
    fn restart_pending_persists_across_reload_with_unchanged_file() {
        with_jail(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port_and_db(8096, "/tmp/a.db"))
                .unwrap();
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, handle) = ConfigManager::new(
                config,
                PathBuf::from("harmonia.toml"),
                ConfigOverrides::default(),
            );

            jail.create_file("harmonia.toml", &toml_with_port_and_db(8096, "/tmp/b.db"))
                .unwrap();
            manager.reload().unwrap();

            let outcome = manager.reload().unwrap();
            assert_eq!(outcome.restart_pending, vec!["database.db_path"]);
            assert_eq!(handle.restart_pending(), vec!["database.db_path"]);
            assert_eq!(
                handle.current().database.db_path,
                PathBuf::from("/tmp/a.db")
            );
        });
    }

    #[test]
    fn overrides_are_reapplied_after_reload() {
        with_jail(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port(8096))
                .unwrap();
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let overrides = ConfigOverrides {
                listen_addr: None,
                port: Some(9999),
            };
            let (manager, handle) =
                ConfigManager::new(config, PathBuf::from("harmonia.toml"), overrides);
            assert_eq!(handle.current().paroche.port, 9999);

            jail.create_file("harmonia.toml", &toml_with_port(9090))
                .unwrap();
            let outcome = manager.reload().unwrap();

            assert_eq!(handle.current().paroche.port, 9999);
            assert!(outcome.applied.is_empty());
        });
    }

    #[test]
    fn reload_with_invalid_config_returns_error_and_keeps_current() {
        with_jail(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port(8096))
                .unwrap();
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, handle) = ConfigManager::new(
                config,
                PathBuf::from("harmonia.toml"),
                ConfigOverrides::default(),
            );

            // Remove jwt_secret — validation will reject it
            jail.create_file("harmonia.toml", "[paroche]\nport = 9090\n")
                .unwrap();
            let result = manager.reload();

            assert!(result.is_err());
            assert_eq!(handle.current().paroche.port, 8096);
        });
    }

    #[test]
    fn reload_broadcasts_to_all_subscribers() {
        with_jail(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port(8096))
                .unwrap();
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, handle) = ConfigManager::new(
                config,
                PathBuf::from("harmonia.toml"),
                ConfigOverrides::default(),
            );

            let mut rx1 = handle.subscribe();
            let mut rx2 = handle.subscribe();

            jail.create_file("harmonia.toml", &toml_with_port(9090))
                .unwrap();
            manager.reload().unwrap();

            assert!(rx1.has_changed().unwrap());
            assert!(rx2.has_changed().unwrap());
            assert_eq!(rx1.borrow_and_update().paroche.port, 9090);
            assert_eq!(rx2.borrow_and_update().paroche.port, 9090);
        });
    }

    // ── ConfigManager::replace ────────────────────────────────────────────────

    #[test]
    fn replace_applies_held_back_logic() {
        let mut config = valid_config();
        config.database.db_path = PathBuf::from("/tmp/a.db");
        let (manager, handle) = ConfigManager::new(
            config,
            PathBuf::from("unused.toml"),
            ConfigOverrides::default(),
        );

        let mut changed = valid_config();
        changed.database.db_path = PathBuf::from("/tmp/b.db");
        changed.paroche.port = 9090;
        let outcome = manager.replace(changed).unwrap();

        let current = handle.current();
        assert_eq!(current.database.db_path, PathBuf::from("/tmp/a.db"));
        assert_eq!(current.paroche.port, 9090);
        assert_eq!(outcome.applied, vec!["paroche.port"]);
        assert_eq!(outcome.restart_pending, vec!["database.db_path"]);
    }

    // #529 step 7: seed thresholds are frozen into librqbit's `SeedingPolicy`
    // at session build with no reconfigure API — a reload must hold them back
    // exactly like the database prefix, not silently "apply" a value with
    // zero live effect.
    #[test]
    fn replace_holds_back_seed_threshold_changes() {
        let config = valid_config();
        let (manager, handle) = ConfigManager::new(
            config,
            PathBuf::from("unused.toml"),
            ConfigOverrides::default(),
        );

        let mut changed = valid_config();
        changed.ergasia.seed_ratio_threshold = 3.0;
        changed.ergasia.seed_time_threshold_hours = 999;
        changed.paroche.port = 9090;
        let outcome = manager.replace(changed).unwrap();

        let current = handle.current();
        assert_eq!(current.ergasia.seed_ratio_threshold, 1.0);
        assert_eq!(current.ergasia.seed_time_threshold_hours, 72);
        assert_eq!(current.paroche.port, 9090);
        assert_eq!(
            outcome.restart_pending,
            vec![
                "ergasia.seed_ratio_threshold",
                "ergasia.seed_time_threshold_hours"
            ]
        );
        assert_eq!(outcome.applied, vec!["paroche.port"]);
        assert!(outcome.needs_restart());
    }

    #[test]
    fn replace_reapplies_overrides() {
        let config = valid_config();
        let overrides = ConfigOverrides {
            listen_addr: None,
            port: Some(9999),
        };
        let (manager, handle) = ConfigManager::new(config, PathBuf::from("unused.toml"), overrides);

        let mut changed = valid_config();
        changed.paroche.port = 9090;
        manager.replace(changed).unwrap();

        assert_eq!(handle.current().paroche.port, 9999);
    }

    #[test]
    fn replace_rejects_invalid_config() {
        let config = valid_config();
        let (manager, handle) = ConfigManager::new(
            config,
            PathBuf::from("unused.toml"),
            ConfigOverrides::default(),
        );

        let mut invalid = valid_config();
        invalid.exousia.jwt_secret = "tooshort".into();
        assert!(manager.replace(invalid).is_err());
        assert_eq!(handle.current().exousia.jwt_secret, VALID_JWT);
    }

    // ── ConfigHandle::subscribe ───────────────────────────────────────────────

    #[test]
    fn subscribe_yields_on_change() {
        with_jail(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port(8096))
                .unwrap();
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, handle) = ConfigManager::new(
                config,
                PathBuf::from("harmonia.toml"),
                ConfigOverrides::default(),
            );
            let mut rx = handle.subscribe();

            jail.create_file("harmonia.toml", &toml_with_port(9090))
                .unwrap();
            manager.reload().unwrap();

            assert!(rx.has_changed().unwrap());
            assert_eq!(rx.borrow_and_update().paroche.port, 9090);
        });
    }

    #[test]
    fn subscribe_does_not_yield_when_config_unchanged() {
        with_jail(|jail| {
            jail.create_file("harmonia.toml", &toml_with_port(8096))
                .unwrap();
            let (config, _) = load_config(Some(Path::new("harmonia.toml"))).unwrap();
            let (manager, handle) = ConfigManager::new(
                config,
                PathBuf::from("harmonia.toml"),
                ConfigOverrides::default(),
            );
            let rx = handle.subscribe();

            // File unchanged — reload should be a no-op
            manager.reload().unwrap();

            assert!(!rx.has_changed().unwrap());
        });
    }

    // ── ConfigHandle::watch_section ───────────────────────────────────────────

    #[tokio::test]
    async fn watch_section_yields_only_on_its_section_change() {
        let config = valid_config();
        let (manager, handle) = ConfigManager::new(
            config,
            PathBuf::from("unused.toml"),
            ConfigOverrides::default(),
        );
        let mut watcher = handle.watch_section(|c| &c.paroche);

        // A publish that changes a DIFFERENT section must not surface as a
        // distinct wakeup with a stale/unrelated value.
        let mut changed = valid_config();
        changed.exousia.access_token_ttl_secs = 1234;
        manager.replace(changed.clone()).unwrap();

        // A publish that changes the watched section wakes it with the new
        // projected value.
        changed.paroche.port = 9191;
        manager.replace(changed).unwrap();

        let seen = tokio::time::timeout(std::time::Duration::from_secs(1), watcher.changed())
            .await
            .expect("watcher did not wake for a section change")
            .expect("watcher returned None while the manager is alive");
        assert_eq!(seen.port, 9191);
    }

    #[tokio::test]
    async fn watch_section_returns_none_when_manager_dropped() {
        let config = valid_config();
        let (manager, handle) = ConfigManager::new(
            config,
            PathBuf::from("unused.toml"),
            ConfigOverrides::default(),
        );
        let mut watcher = handle.watch_section(|c| &c.paroche);
        drop(manager);

        let seen = tokio::time::timeout(std::time::Duration::from_secs(1), watcher.changed())
            .await
            .expect("watcher did not resolve after the manager was dropped");
        assert!(seen.is_none());
    }
}
