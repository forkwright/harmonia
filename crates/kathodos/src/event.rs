use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum WatchEventKind {
    Created,
    Modified,
    Removed,
    Renamed { from: PathBuf },
}

#[derive(Debug, Clone)]
pub struct WatchEvent {
    pub path: PathBuf,
    pub kind: WatchEventKind,
    pub library_name: String,
}

/// Collapses rapid file events for the same path into a single event.
pub struct Debouncer {
    window: Duration,
    pending: HashMap<PathBuf, (WatchEvent, Instant)>,
}

impl Debouncer {
    pub(crate) fn new(window_ms: u64) -> Self {
        Self {
            window: Duration::from_millis(window_ms),
            pending: HashMap::new(),
        }
    }

    pub(crate) fn push(&mut self, event: WatchEvent) {
        let deadline = Instant::now() + self.window;
        self.pending.insert(event.path.clone(), (event, deadline));
    }

    pub(crate) fn drain_ready(&mut self) -> Vec<WatchEvent> {
        let now = Instant::now();
        let ready: Vec<PathBuf> = self
            .pending
            .iter()
            .filter(|(_, (_, deadline))| *deadline <= now)
            .map(|(path, _)| path.clone())
            .collect();
        ready
            .into_iter()
            .filter_map(|p| self.pending.remove(&p).map(|(ev, _)| ev))
            .collect()
    }

    /// The earliest deadline across all pending events, if any.
    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        self.pending.values().map(|(_, d)| *d).min()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debounce_collapses_same_path() {
        let mut d = Debouncer::new(100_000); // very long window
        let ev1 = WatchEvent {
            path: PathBuf::from("/lib/track.flac"),
            kind: WatchEventKind::Created,
            library_name: "music".into(),
        };
        let ev2 = WatchEvent {
            path: PathBuf::from("/lib/track.flac"),
            kind: WatchEventKind::Modified,
            library_name: "music".into(),
        };
        d.push(ev1);
        d.push(ev2); // same path, replaces previous
        // window hasn't expired yet — nothing ready
        assert!(d.drain_ready().is_empty());
        assert!(!d.is_empty());
    }

    #[test]
    fn debounce_drains_after_window() {
        let mut d = Debouncer::new(0); // zero window = immediately ready
        let ev = WatchEvent {
            path: PathBuf::from("/lib/track.flac"),
            kind: WatchEventKind::Created,
            library_name: "music".into(),
        };
        d.push(ev);
        // sleep briefly to ensure deadline has passed
        std::thread::sleep(Duration::from_millis(5));
        let ready = d.drain_ready();
        assert_eq!(ready.len(), 1);
        assert!(d.is_empty());
    }

    #[test]
    fn debounce_different_paths_independent() {
        let mut d = Debouncer::new(0);
        for i in 0..3 {
            let ev = WatchEvent {
                path: PathBuf::from(format!("/lib/track{i}.flac")),
                kind: WatchEventKind::Created,
                library_name: "music".into(),
            };
            d.push(ev);
        }
        std::thread::sleep(Duration::from_millis(5));
        let ready = d.drain_ready();
        assert_eq!(ready.len(), 3);
    }

    #[test]
    fn debounce_next_deadline_some_when_pending() {
        let mut d = Debouncer::new(500);
        let ev = WatchEvent {
            path: PathBuf::from("/lib/track.flac"),
            kind: WatchEventKind::Created,
            library_name: "music".into(),
        };
        d.push(ev);
        assert!(d.next_deadline().is_some());
    }

    #[test]
    fn debounce_next_deadline_none_when_empty() {
        let d = Debouncer::new(500);
        assert!(d.next_deadline().is_none());
    }
}
