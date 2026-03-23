use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// Sequential play queue with forward/backward navigation.
///
/// Phase 1: no shuffle, no repeat. Both are added in Phase 2.
pub struct PlayQueue {
    tracks: VecDeque<PathBuf>,
    current_index: usize,
    history: Vec<PathBuf>,
}

impl PlayQueue {
    /// Creates an empty queue.
    pub fn new() -> Self {
        Self {
            tracks: VecDeque::new(),
            current_index: 0,
            history: Vec::new(),
        }
    }

    /// Creates a queue pre-loaded with `tracks`.
    pub fn from_tracks(tracks: impl IntoIterator<Item = PathBuf>) -> Self {
        Self {
            tracks: tracks.into_iter().collect(),
            current_index: 0,
            history: Vec::new(),
        }
    }

    /// Appends a track to the end of the queue.
    pub fn push(&mut self, path: PathBuf) {
        self.tracks.push_back(path);
    }

    /// Returns the currently active track path, or `None` when the queue is empty.
    pub fn current(&self) -> Option<&Path> {
        self.tracks.get(self.current_index).map(PathBuf::as_path)
    }

    /// Advances to the next track and returns its path.
    ///
    /// Returns `None` when there is no next track (end of queue).
    #[expect(
        clippy::should_implement_trait,
        reason = "PlayQueue::next() advances the queue and returns the track; naming matches domain language, not Iterator"
    )]
    pub fn next(&mut self) -> Option<PathBuf> {
        if let Some(current) = self.current().map(PathBuf::from) {
            self.history.push(current);
        }
        let next_index = self.current_index + 1;
        if next_index < self.tracks.len() {
            self.current_index = next_index;
            self.current().map(PathBuf::from)
        } else {
            None
        }
    }

    /// Moves to the previous track and returns its path.
    ///
    /// If `current_elapsed_secs` > 3.0, restarts the current track instead of going
    /// back one track. Returns `None` only when the queue is empty.
    pub fn previous(&mut self, current_elapsed_secs: f64) -> Option<PathBuf> {
        if current_elapsed_secs > 3.0 {
            // Restart current track.
            return self.current().map(PathBuf::from);
        }
        if self.current_index > 0 {
            self.current_index -= 1;
        }
        self.current().map(PathBuf::from)
    }

    /// Returns the number of tracks in the queue.
    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// Returns the current track index (0-based).
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Returns the playback history (tracks played before the current).
    pub fn history(&self) -> &[PathBuf] {
        &self.history
    }
}

impl Default for PlayQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn empty_queue_current_is_none() {
        let q = PlayQueue::new();
        assert!(q.current().is_none());
    }

    #[test]
    fn empty_queue_next_is_none() {
        let mut q = PlayQueue::new();
        assert!(q.next().is_none());
    }

    #[test]
    fn single_track_current_returns_it() {
        let q = PlayQueue::from_tracks(paths(&["a.flac"]));
        assert_eq!(q.current(), Some(Path::new("a.flac")));
    }

    #[test]
    fn sequential_three_files() {
        let mut q = PlayQueue::from_tracks(paths(&["a.flac", "b.flac", "c.flac"]));

        assert_eq!(q.current(), Some(Path::new("a.flac")));

        let n = q.next();
        assert_eq!(n.as_deref(), Some(Path::new("b.flac")));
        assert_eq!(q.current(), Some(Path::new("b.flac")));

        let n = q.next();
        assert_eq!(n.as_deref(), Some(Path::new("c.flac")));
        assert_eq!(q.current(), Some(Path::new("c.flac")));

        let n = q.next();
        assert!(n.is_none(), "should return None at end of queue");
    }

    #[test]
    fn previous_at_start_stays_at_first_track() {
        let mut q = PlayQueue::from_tracks(paths(&["a.flac", "b.flac"]));
        let prev = q.previous(0.0);
        assert_eq!(prev.as_deref(), Some(Path::new("a.flac")));
        assert_eq!(q.current_index(), 0);
    }

    #[test]
    fn previous_after_next_goes_back() {
        let mut q = PlayQueue::from_tracks(paths(&["a.flac", "b.flac", "c.flac"]));
        q.next();
        q.next();
        // At c.flac, elapsed < 3s → go to b.flac
        let prev = q.previous(1.0);
        assert_eq!(prev.as_deref(), Some(Path::new("b.flac")));
    }

    #[test]
    fn previous_with_elapsed_over_3s_restarts_current() {
        let mut q = PlayQueue::from_tracks(paths(&["a.flac", "b.flac"]));
        q.next(); // at b.flac
        // elapsed > 3s → restart b.flac
        let prev = q.previous(5.0);
        assert_eq!(prev.as_deref(), Some(Path::new("b.flac")));
        assert_eq!(q.current_index(), 1, "index should not change");
    }

    #[test]
    fn history_tracks_played_tracks() {
        let mut q = PlayQueue::from_tracks(paths(&["a.flac", "b.flac", "c.flac"]));
        q.next();
        q.next();
        assert_eq!(q.history().len(), 2);
        assert_eq!(q.history()[0], PathBuf::from("a.flac"));
        assert_eq!(q.history()[1], PathBuf::from("b.flac"));
    }

    #[test]
    fn push_appends_to_queue() {
        let mut q = PlayQueue::new();
        q.push(PathBuf::from("a.flac"));
        q.push(PathBuf::from("b.flac"));
        assert_eq!(q.len(), 2);
        assert_eq!(q.current(), Some(Path::new("a.flac")));
    }

    #[test]
    fn len_and_is_empty() {
        let mut q = PlayQueue::new();
        assert!(q.is_empty());
        q.push(PathBuf::from("x.flac"));
        assert_eq!(q.len(), 1);
        assert!(!q.is_empty());
    }
}
