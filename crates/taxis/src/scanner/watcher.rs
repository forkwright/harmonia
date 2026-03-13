use std::path::Path;
use std::time::Duration;

use notify::{Config, Event, EventKind, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher};
use snafu::ResultExt;
use tokio::sync::mpsc;
use tracing::instrument;

use crate::error::{TaxisError, WatcherInitSnafu};
use crate::event::{WatchEvent, WatchEventKind};

const NETWORK_FS_TYPES: &[&str] = &["nfs", "nfs4", "cifs", "smbfs", "smb", "fuse.sshfs"];

/// Runtime watcher mode after auto-detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveWatcherMode {
    Inotify,
    Poll,
}

pub enum AnyWatcher {
    Recommended(RecommendedWatcher),
    Poll(PollWatcher),
}

/// Detect whether to use inotify or poll for a library path.
pub fn detect_watcher_mode(watcher_mode: &horismos::WatcherMode, path: &Path) -> ActiveWatcherMode {
    detect_watcher_mode_at(watcher_mode, path, Path::new("/proc/mounts"))
}

pub fn detect_watcher_mode_at(
    watcher_mode: &horismos::WatcherMode,
    path: &Path,
    mounts_path: &Path,
) -> ActiveWatcherMode {
    match watcher_mode {
        horismos::WatcherMode::Inotify => ActiveWatcherMode::Inotify,
        horismos::WatcherMode::Poll => ActiveWatcherMode::Poll,
        horismos::WatcherMode::Auto => {
            if is_network_mount_at(path, mounts_path) {
                tracing::info!(path = %path.display(), "NFS mount detected — using PollWatcher");
                ActiveWatcherMode::Poll
            } else {
                ActiveWatcherMode::Inotify
            }
        }
    }
}

pub fn is_network_mount(path: &Path) -> bool {
    is_network_mount_at(path, Path::new("/proc/mounts"))
}

pub fn is_network_mount_at(path: &Path, mounts_path: &Path) -> bool {
    let content = match std::fs::read_to_string(mounts_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "NFS detection inconclusive — cannot read mounts");
            return false;
        }
    };
    let path_str = path.to_string_lossy();
    let mut best_mount: Option<(usize, bool)> = None;

    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let _device = parts.next();
        let mount_point = match parts.next() {
            Some(m) => m,
            None => continue,
        };
        let fs_type = match parts.next() {
            Some(f) => f,
            None => continue,
        };

        if path_str.starts_with(mount_point) {
            let is_net = NETWORK_FS_TYPES.contains(&fs_type);
            let len = mount_point.len();
            match best_mount {
                None => {
                    best_mount = Some((len, is_net));
                }
                Some((prev_len, _)) if len > prev_len => {
                    best_mount = Some((len, is_net));
                }
                _ => {}
            }
        }
    }

    best_mount.is_some_and(|(_, is_net)| is_net)
}

#[instrument(skip(tx))]
pub fn create_watcher(
    library_name: &str,
    lib: &horismos::LibraryConfig,
    tx: mpsc::Sender<notify::Result<Event>>,
) -> Result<AnyWatcher, TaxisError> {
    let mode = detect_watcher_mode(&lib.watcher_mode, &lib.path);
    let config =
        Config::default().with_poll_interval(Duration::from_secs(lib.poll_interval_seconds));

    match mode {
        ActiveWatcherMode::Inotify => {
            let tx2 = tx.clone();
            let mut w = RecommendedWatcher::new(
                move |result| {
                    let _ = tx2.blocking_send(result);
                },
                config,
            )
            .context(WatcherInitSnafu {
                library: library_name.to_string(),
            })?;
            w.watch(&lib.path, RecursiveMode::Recursive)
                .context(WatcherInitSnafu {
                    library: library_name.to_string(),
                })?;
            Ok(AnyWatcher::Recommended(w))
        }
        ActiveWatcherMode::Poll => {
            let tx2 = tx.clone();
            let mut w = PollWatcher::new(
                move |result| {
                    let _ = tx2.blocking_send(result);
                },
                config,
            )
            .context(WatcherInitSnafu {
                library: library_name.to_string(),
            })?;
            w.watch(&lib.path, RecursiveMode::Recursive)
                .context(WatcherInitSnafu {
                    library: library_name.to_string(),
                })?;
            Ok(AnyWatcher::Poll(w))
        }
    }
}

pub fn normalize_event(event: Event, library_name: String) -> Vec<WatchEvent> {
    let kind = match event.kind {
        EventKind::Create(_) => WatchEventKind::Created,
        EventKind::Modify(_) => WatchEventKind::Modified,
        EventKind::Remove(_) => WatchEventKind::Removed,
        EventKind::Access(_) => return vec![],
        EventKind::Other => return vec![],
        EventKind::Any => WatchEventKind::Modified,
    };
    event
        .paths
        .into_iter()
        .filter(|p| p.is_file() || matches!(kind, WatchEventKind::Removed))
        .map(|path| WatchEvent {
            path,
            kind: kind.clone(),
            library_name: library_name.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;

    fn write_mounts(dir: &TempDir, content: &str) -> PathBuf {
        let p = dir.path().join("mounts");
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn nfs_detection_returns_poll_for_nfs_mount() {
        let dir = TempDir::new().unwrap();
        let mounts = write_mounts(&dir, "server:/export /mnt/nfs nfs rw,defaults 0 0\n");
        let path = PathBuf::from("/mnt/nfs/library");
        assert!(is_network_mount_at(&path, &mounts));
    }

    #[test]
    fn nfs_detection_returns_false_for_local_mount() {
        let dir = TempDir::new().unwrap();
        let mounts = write_mounts(&dir, "/dev/sda1 /mnt/local ext4 rw,defaults 0 0\n");
        let path = PathBuf::from("/mnt/local/library");
        assert!(!is_network_mount_at(&path, &mounts));
    }

    #[test]
    fn nfs_detection_picks_deepest_mount() {
        let dir = TempDir::new().unwrap();
        let mounts = write_mounts(
            &dir,
            "/dev/sda1 / ext4 rw 0 0\n\
             server:/export /mnt/nfs nfs rw 0 0\n\
             /dev/sda2 /mnt/nfs/local ext4 rw 0 0\n",
        );
        // /mnt/nfs/local is the deepest match and it's ext4 (not NFS)
        let path = PathBuf::from("/mnt/nfs/local/library");
        assert!(!is_network_mount_at(&path, &mounts));
    }

    #[test]
    fn nfs_detection_cifs_is_network() {
        let dir = TempDir::new().unwrap();
        let mounts = write_mounts(&dir, "//server/share /mnt/smb cifs rw 0 0\n");
        let path = PathBuf::from("/mnt/smb/files");
        assert!(is_network_mount_at(&path, &mounts));
    }

    #[test]
    fn nfs_detection_missing_file_returns_false() {
        let path = PathBuf::from("/mnt/library");
        let mounts_path = PathBuf::from("/nonexistent/proc/mounts");
        assert!(!is_network_mount_at(&path, &mounts_path));
    }

    #[test]
    fn detect_watcher_mode_explicit_poll_ignores_fs() {
        let dir = TempDir::new().unwrap();
        let mode = detect_watcher_mode_at(
            &horismos::WatcherMode::Poll,
            dir.path(),
            Path::new("/nonexistent"),
        );
        assert_eq!(mode, ActiveWatcherMode::Poll);
    }

    #[test]
    fn detect_watcher_mode_explicit_inotify_ignores_fs() {
        let dir = TempDir::new().unwrap();
        let mode = detect_watcher_mode_at(
            &horismos::WatcherMode::Inotify,
            dir.path(),
            Path::new("/nonexistent"),
        );
        assert_eq!(mode, ActiveWatcherMode::Inotify);
    }

    #[test]
    fn detect_watcher_mode_auto_nfs_selects_poll() {
        let dir = TempDir::new().unwrap();
        let mounts_path = dir.path().join("mounts");
        std::fs::write(&mounts_path, "server:/export /mnt/nfs nfs rw 0 0\n").unwrap();
        let lib_path = PathBuf::from("/mnt/nfs/music");
        let mode = detect_watcher_mode_at(&horismos::WatcherMode::Auto, &lib_path, &mounts_path);
        assert_eq!(mode, ActiveWatcherMode::Poll);
    }

    #[test]
    fn detect_watcher_mode_auto_local_selects_inotify() {
        let dir = TempDir::new().unwrap();
        let mounts_path = dir.path().join("mounts");
        std::fs::write(&mounts_path, "/dev/sda1 /mnt/local ext4 rw 0 0\n").unwrap();
        let lib_path = PathBuf::from("/mnt/local/music");
        let mode = detect_watcher_mode_at(&horismos::WatcherMode::Auto, &lib_path, &mounts_path);
        assert_eq!(mode, ActiveWatcherMode::Inotify);
    }
}
