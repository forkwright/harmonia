pub mod filter;
pub mod walk;
pub mod watcher;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use horismos::TaxisConfig;
use themelion::{EventSender, HarmoniaEvent, MediaType};
use tokio::sync::{Semaphore, mpsc, watch};
use tokio::task::JoinHandle;
use tracing::{Instrument, instrument};

use crate::error::TaxisError;
use crate::event::Debouncer;
use crate::import::identify::resolve_media_type;
use crate::scanner::walk::walk_library;
use crate::scanner::watcher::{AnyWatcher, create_watcher, normalize_event};

const DEFAULT_DEBOUNCE_MS: u64 = 500;
const DEFAULT_SCAN_CONCURRENCY: usize = 4;

pub struct ScannerManager {
    watcher_handles: Vec<JoinHandle<()>>,
    scan_handles: Vec<JoinHandle<()>>,
    shutdown_tx: watch::Sender<bool>,
    scan_triggers: HashMap<String, mpsc::Sender<()>>,
}

impl ScannerManager {
    #[instrument(skip(config, event_tx))]
    pub async fn start(config: &TaxisConfig, event_tx: EventSender) -> Result<Self, TaxisError> {
        let (shutdown_tx, _) = watch::channel(false);
        let semaphore = Arc::new(Semaphore::new(DEFAULT_SCAN_CONCURRENCY));
        let mut watcher_handles = Vec::new();
        let mut scan_handles = Vec::new();
        let mut scan_triggers = HashMap::new();

        for (name, lib) in &config.libraries {
            let (event_raw_tx, event_raw_rx) = mpsc::channel(256);

            let lib_name = name.clone();
            let lib_config = lib.clone();
            let event_tx_clone = event_tx.clone();
            let debounce_ms = DEFAULT_DEBOUNCE_MS;

            match create_watcher(&lib_name, &lib_config, event_raw_tx) {
                Ok(watcher) => {
                    let handle = tokio::spawn(
                        run_watcher_task(
                            lib_name.clone(),
                            watcher,
                            event_raw_rx,
                            debounce_ms,
                            event_tx_clone.clone(),
                            shutdown_tx.subscribe(),
                        )
                        .instrument(tracing::info_span!("watcher", library = %lib_name)),
                    );
                    watcher_handles.push(handle);
                }
                Err(e) => {
                    tracing::error!(
                        library = %lib_name,
                        error = %e,
                        "watcher init failed, skipping library"
                    );
                }
            }

            let (scan_tx, scan_rx) = mpsc::channel(4);
            scan_triggers.insert(name.clone(), scan_tx);

            let lib_name2 = name.clone();
            let lib_path = lib.path.clone();
            let media_type = resolve_media_type(&lib.media_type);
            let scan_interval = Duration::from_secs(lib.scan_interval_hours * 3600);
            let event_tx_scan = event_tx.clone();
            let sem_clone = Arc::clone(&semaphore);

            let scan_handle = tokio::spawn(
                run_scan_task(
                    ScanTaskConfig {
                        library_name: lib_name2.clone(),
                        root: lib_path,
                        media_type,
                        interval: scan_interval,
                    },
                    scan_rx,
                    event_tx_scan,
                    sem_clone,
                    shutdown_tx.subscribe(),
                )
                .instrument(tracing::info_span!("scanner", library = %lib_name2)),
            );
            scan_handles.push(scan_handle);
        }

        Ok(Self {
            watcher_handles,
            scan_handles,
            shutdown_tx,
            scan_triggers,
        })
    }

    /// Trigger an immediate full scan of the named library.
    pub async fn trigger_scan(&self, library: &str) -> Result<(), TaxisError> {
        if let Some(tx) = self.scan_triggers.get(library) {
            let _ = tx.send(()).await;
        }
        Ok(())
    }

    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(true);
        for h in self.watcher_handles {
            let _ = h.await;
        }
        for h in self.scan_handles {
            let _ = h.await;
        }
    }
}

async fn run_watcher_task(
    library_name: String,
    _watcher: AnyWatcher,
    mut rx: mpsc::Receiver<notify::Result<notify::Event>>,
    debounce_ms: u64,
    _event_tx: EventSender,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let mut debouncer = Debouncer::new(debounce_ms);

    loop {
        let deadline = debouncer.next_deadline();
        let sleep_duration = deadline
            .map(|d| d.saturating_duration_since(std::time::Instant::now()))
            .unwrap_or(Duration::from_secs(3600));

        tokio::select! {
            biased;

            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() { break; }
            }

            result = rx.recv() => {
                match result {
                    Some(Ok(event)) => {
                        for watch_event in normalize_event(event, library_name.clone()) {
                            debouncer.push(watch_event);
                        }
                    }
                    Some(Err(e)) => {
                        tracing::warn!(library = %library_name, error = %e, "watcher error");
                    }
                    None => break,
                }
            }

            _ = tokio::time::sleep(sleep_duration) => {
                // flush ready events
            }
        }

        for event in debouncer.drain_ready() {
            tracing::debug!(
                library = %library_name,
                path = %event.path.display(),
                kind = ?event.kind,
                "watcher event ready"
            );
        }
    }
    tracing::info!(library = %library_name, "watcher task stopped");
}

struct ScanTaskConfig {
    library_name: String,
    root: PathBuf,
    media_type: MediaType,
    interval: Duration,
}

async fn run_scan_task(
    cfg: ScanTaskConfig,
    mut trigger_rx: mpsc::Receiver<()>,
    event_tx: EventSender,
    semaphore: Arc<Semaphore>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let mut interval_timer = tokio::time::interval(cfg.interval);
    interval_timer.tick().await; // skip the immediate first tick

    loop {
        tokio::select! {
            biased;

            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() { break; }
            }

            _ = trigger_rx.recv() => {
                run_full_scan(&cfg.library_name, &cfg.root, cfg.media_type, &event_tx, &semaphore).await;
            }

            _ = interval_timer.tick() => {
                run_full_scan(&cfg.library_name, &cfg.root, cfg.media_type, &event_tx, &semaphore).await;
            }
        }
    }
    tracing::info!(library = %cfg.library_name, "scan task stopped");
}

async fn run_full_scan(
    library_name: &str,
    root: &Path,
    media_type: MediaType,
    event_tx: &EventSender,
    semaphore: &Arc<Semaphore>,
) {
    tracing::info!(library = %library_name, "starting full scan");
    match walk_library(root, media_type, semaphore).await {
        Ok((results, stats)) => {
            tracing::info!(
                library = %library_name,
                found = results.len(),
                scanned = stats.scanned,
                skipped_ignored = stats.skipped_ignored,
                skipped_unsupported = stats.skipped_unsupported,
                "scan complete"
            );
            let items_added = results.len();
            if let Err(e) = event_tx.send(HarmoniaEvent::LibraryScanCompleted {
                items_scanned: stats.scanned,
                items_added,
                items_removed: 0,
            }) {
                tracing::warn!(error = %e, "operation failed");
            }
        }
        Err(e) => {
            tracing::error!(library = %library_name, error = %e, "scan failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tempfile::TempDir;
    use themelion::create_event_bus;

    use super::*;

    #[tokio::test]
    async fn scanner_detects_new_file_in_watched_directory() {
        let dir = TempDir::new().unwrap();
        let mut config = TaxisConfig::default();
        let lib = horismos::LibraryConfig {
            path: dir.path().to_path_buf(),
            watcher_mode: horismos::WatcherMode::Poll,
            poll_interval_seconds: 1,
            scan_interval_hours: 9999, // don't auto-scan
            ..Default::default()
        };
        config.libraries.insert("test".to_string(), lib);

        let (tx, mut rx) = create_event_bus(64);
        let manager = ScannerManager::start(&config, tx).await.unwrap();

        std::fs::write(dir.path().join("track.flac"), b"FLAC").unwrap();
        manager.trigger_scan("test").await.unwrap();

        tokio::time::sleep(Duration::from_millis(200)).await;

        manager.shutdown().await;

        let mut got_scan_completed = false;
        while let Ok(event) = rx.try_recv() {
            if let HarmoniaEvent::LibraryScanCompleted { items_added, .. } = event
                && items_added >= 1
            {
                got_scan_completed = true;
            }
        }
        assert!(
            got_scan_completed,
            "expected LibraryScanCompleted event with items_added >= 1"
        );
    }
}
