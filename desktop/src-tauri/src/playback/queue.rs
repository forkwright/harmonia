//! Desktop play queue: track IDs with metadata, shuffle, and repeat modes.

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RepeatMode {
    Off,
    One,
    All,
}

/// A single entry in the play queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QueueEntry {
    pub track_id: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
}

/// Desktop play queue with shuffle and repeat support.
///
/// Tracks are stored as `QueueEntry` values containing track IDs and display metadata.
/// Shuffle works by maintaining a separate shuffle order over the same entries.
pub(crate) struct DesktopQueue {
    entries: Vec<QueueEntry>,
    /// Indices into `entries` representing the playback order when shuffle is on.
    shuffle_order: Vec<usize>,
    current_position: usize,
    shuffle: bool,
    pub repeat: RepeatMode,
    /// Name of the source context ("Album: ...", "Tracks", etc.).
    pub source_label: String,
}

impl DesktopQueue {
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
            shuffle_order: Vec::new(),
            current_position: 0,
            shuffle: false,
            repeat: RepeatMode::Off,
            source_label: String::new(),
        }
    }

    /// Returns `true` when the queue holds no entries.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clears all entries and resets position.
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.shuffle_order.clear();
        self.current_position = 0;
    }

    /// Appends entries to the end of the queue.
    pub(crate) fn append(&mut self, entries: Vec<QueueEntry>) {
        let start = self.entries.len();
        self.entries.extend(entries);
        if self.shuffle {
            let new_indices: Vec<usize> = (start..self.entries.len()).collect();
            self.shuffle_order.extend(new_indices);
        }
    }

    /// Removes the entry at the given display index from the queue.
    pub(crate) fn remove(&mut self, display_index: usize) {
        let Some(phys) = self.display_to_physical(display_index) else {
            return;
        };

        if self.shuffle {
            self.shuffle_order.retain(|&i| i != phys);
            for idx in &mut self.shuffle_order {
                if *idx > phys {
                    *idx -= 1;
                }
            }
        }
        self.entries.remove(phys);

        let order_len = self.playback_order_len();
        if self.current_position > 0 && self.current_position >= order_len {
            self.current_position = order_len.saturating_sub(1);
        }
    }

    /// Moves an entry from one display index to another.
    pub(crate) fn move_entry(&mut self, from_display: usize, to_display: usize) {
        let order_len = self.playback_order_len();
        if from_display >= order_len || to_display >= order_len {
            return;
        }
        if self.shuffle {
            let entry = self.shuffle_order.remove(from_display);
            self.shuffle_order.insert(to_display, entry);
        } else {
            let entry = self.entries.remove(from_display);
            self.entries.insert(to_display, entry);
        }
        // Track current_position through the move.
        if from_display == self.current_position {
            self.current_position = to_display;
        } else if from_display < self.current_position && to_display >= self.current_position {
            self.current_position = self.current_position.saturating_sub(1);
        } else if from_display > self.current_position && to_display <= self.current_position {
            self.current_position += 1;
        }
    }

    /// Returns the current entry, or `None` if the queue is empty.
    pub(crate) fn current(&self) -> Option<&QueueEntry> {
        let phys = self.physical_index(self.current_position)?;
        self.entries.get(phys)
    }

    /// Advances to the next entry. Returns `Some(entry)` if successful.
    ///
    /// Handles `RepeatMode::One` (replays current) and `RepeatMode::All` (wraps).
    pub(crate) fn advance(&mut self) -> Option<&QueueEntry> {
        match self.repeat {
            RepeatMode::One => self.current(),
            RepeatMode::All => {
                let len = self.playback_order_len();
                if len == 0 {
                    return None;
                }
                self.current_position = (self.current_position + 1) % len;
                self.current()
            }
            RepeatMode::Off => {
                let next = self.current_position + 1;
                if next < self.playback_order_len() {
                    self.current_position = next;
                    self.current()
                } else {
                    None
                }
            }
        }
    }

    /// Moves to the previous entry. Restarts the current track when `position_ms > 3000`.
    pub(crate) fn back(&mut self, position_ms: u64) -> Option<&QueueEntry> {
        if position_ms > 3000 {
            return self.current();
        }
        if self.current_position > 0 {
            self.current_position -= 1;
        }
        self.current()
    }

    /// Sets shuffle mode. When enabling, generates a random play order starting from current.
    pub(crate) fn set_shuffle(&mut self, enabled: bool) {
        if enabled == self.shuffle {
            return;
        }
        self.shuffle = enabled;
        if enabled {
            self.rebuild_shuffle_order();
        } else {
            let current_phys = self.physical_index(self.current_position);
            self.shuffle_order.clear();
            self.current_position = current_phys.unwrap_or(0);
        }
    }

    pub(crate) fn shuffle_enabled(&self) -> bool {
        self.shuffle
    }

    /// Returns all entries in display order (shuffle order if active).
    pub(crate) fn display_entries(&self) -> Vec<&QueueEntry> {
        if self.shuffle {
            self.shuffle_order
                .iter()
                .filter_map(|&i| self.entries.get(i))
                .collect()
        } else {
            self.entries.iter().collect()
        }
    }

    /// Returns the current display index (position in the playback order).
    pub(crate) fn current_display_index(&self) -> usize {
        self.current_position
    }

    // ---------------------------------------------------------------------------
    // Private helpers
    // ---------------------------------------------------------------------------

    fn rebuild_shuffle_order(&mut self) {
        let len = self.entries.len();
        let current_phys = self.physical_index(self.current_position);
        let mut order: Vec<usize> = (0..len).collect();
        order.shuffle(&mut rand::rng());

        // Place current track first so it continues playing without interruption.
        if let Some(phys) = current_phys
            && let Some(pos) = order.iter().position(|&i| i == phys)
        {
            order.swap(0, pos);
        }
        self.shuffle_order = order;
        self.current_position = 0;
    }

    fn playback_order_len(&self) -> usize {
        if self.shuffle {
            self.shuffle_order.len()
        } else {
            self.entries.len()
        }
    }

    fn physical_index(&self, logical: usize) -> Option<usize> {
        if self.shuffle {
            self.shuffle_order.get(logical).copied()
        } else if logical < self.entries.len() {
            Some(logical)
        } else {
            None
        }
    }

    fn display_to_physical(&self, display: usize) -> Option<usize> {
        self.physical_index(display)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str) -> QueueEntry {
        QueueEntry {
            track_id: id.to_string(),
            title: id.to_string(),
            artist: None,
            album: None,
            duration_ms: Some(180_000),
        }
    }

    #[test]
    fn empty_queue_current_is_none() {
        let q = DesktopQueue::new();
        assert!(q.current().is_none());
    }

    #[test]
    fn empty_queue_is_empty() {
        let q = DesktopQueue::new();
        assert!(q.is_empty());
    }

    #[test]
    fn append_and_current_returns_first() {
        let mut q = DesktopQueue::new();
        q.append(vec![entry("a"), entry("b")]);
        assert_eq!(q.current().map(|e| e.track_id.as_str()), Some("a"));
        assert!(!q.is_empty());
    }

    #[test]
    fn advance_moves_to_next() {
        let mut q = DesktopQueue::new();
        q.append(vec![entry("a"), entry("b"), entry("c")]);
        q.advance();
        assert_eq!(q.current().map(|e| e.track_id.as_str()), Some("b"));
    }

    #[test]
    fn advance_at_end_returns_none_when_repeat_off() {
        let mut q = DesktopQueue::new();
        q.append(vec![entry("a")]);
        let result = q.advance();
        assert!(result.is_none());
    }

    #[test]
    fn repeat_all_wraps_to_start() {
        let mut q = DesktopQueue::new();
        q.repeat = RepeatMode::All;
        q.append(vec![entry("a"), entry("b")]);
        q.advance();
        let result = q.advance();
        assert_eq!(result.map(|e| e.track_id.as_str()), Some("a"));
    }

    #[test]
    fn repeat_one_stays_on_current() {
        let mut q = DesktopQueue::new();
        q.repeat = RepeatMode::One;
        q.append(vec![entry("a"), entry("b")]);
        let result = q.advance();
        assert_eq!(result.map(|e| e.track_id.as_str()), Some("a"));
    }

    #[test]
    fn back_under_3s_goes_to_previous() {
        let mut q = DesktopQueue::new();
        q.append(vec![entry("a"), entry("b")]);
        q.advance();
        let result = q.back(1000);
        assert_eq!(result.map(|e| e.track_id.as_str()), Some("a"));
    }

    #[test]
    fn back_over_3s_restarts_current() {
        let mut q = DesktopQueue::new();
        q.append(vec![entry("a"), entry("b")]);
        q.advance();
        let result = q.back(5000);
        assert_eq!(result.map(|e| e.track_id.as_str()), Some("b"));
    }

    #[test]
    fn remove_entry_shrinks_queue() {
        let mut q = DesktopQueue::new();
        q.append(vec![entry("a"), entry("b"), entry("c")]);
        q.remove(1);
        assert_eq!(q.display_entries().len(), 2);
        assert_eq!(q.display_entries()[1].track_id, "c");
    }

    #[test]
    fn shuffle_produces_same_entries() {
        let mut q = DesktopQueue::new();
        q.append(vec![entry("a"), entry("b"), entry("c")]);
        q.set_shuffle(true);
        let mut display: Vec<String> = q
            .display_entries()
            .iter()
            .map(|e| e.track_id.clone())
            .collect();
        display.sort();
        assert_eq!(display, vec!["a", "b", "c"]);
    }
}
