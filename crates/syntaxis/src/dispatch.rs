//! Slot allocator and mpsc channel dispatch to Ergasia.
//!
//! Tracks how many downloads are active per-tracker and globally, and sends
//! `QueueItem`s to Ergasia when a slot opens.

use std::collections::HashMap;

use crate::types::{DownloadProtocol, QueueItem};

/// Tracks active download slots globally and per tracker.
///
/// The slot allocator enforces:
/// - `max_concurrent_downloads`: total active downloads across all protocols
/// - `max_per_tracker`: per-indexer limit for torrent downloads
#[derive(Debug)]
pub(crate) struct SlotAllocator {
    max_concurrent: usize,
    max_per_tracker: usize,
    /// Total active download count.
    active_total: usize,
    /// Active download count keyed by tracker_id (torrent only).
    active_per_tracker: HashMap<i64, usize>,
}

impl SlotAllocator {
    pub(crate) fn new(max_concurrent: usize, max_per_tracker: usize) -> Self {
        Self {
            max_concurrent,
            max_per_tracker,
            active_total: 0,
            active_per_tracker: HashMap::new(),
        }
    }

    /// Returns `true` if `item` can be dispatched given current slot state.
    pub(crate) fn has_slot(&self, item: &QueueItem) -> bool {
        if self.active_total >= self.max_concurrent {
            return false;
        }
        // Per-tracker limit applies only to torrent downloads with a known tracker_id.
        if item.protocol == DownloadProtocol::Torrent
            && let Some(tracker_id) = item.tracker_id
        {
            let tracker_active = self
                .active_per_tracker
                .get(&tracker_id)
                .copied()
                .unwrap_or(0);
            if tracker_active >= self.max_per_tracker {
                return false;
            }
        }
        true
    }

    /// Registers `item` as active, consuming a slot.
    ///
    /// Callers must verify `has_slot` returns `true` before calling this.
    pub(crate) fn acquire(&mut self, item: &QueueItem) {
        self.active_total += 1;
        if item.protocol == DownloadProtocol::Torrent
            && let Some(tracker_id) = item.tracker_id
        {
            *self.active_per_tracker.entry(tracker_id).or_insert(0) += 1;
        }
    }

    /// Releases the slot held by a completed or failed download.
    pub(crate) fn release(&mut self, protocol: DownloadProtocol, tracker_id: Option<i64>) {
        if self.active_total > 0 {
            self.active_total -= 1;
        }
        let Some(id) = tracker_id else { return };
        if protocol != DownloadProtocol::Torrent {
            return;
        }
        // Decrement the per-tracker count, removing the entry when it reaches zero.
        let new_count = self
            .active_per_tracker
            .get(&id)
            .copied()
            .unwrap_or(0)
            .saturating_sub(1);
        if new_count == 0 {
            self.active_per_tracker.remove(&id);
        } else {
            self.active_per_tracker.insert(id, new_count);
        }
    }

    pub(crate) fn global_slot_available(&self) -> bool {
        self.active_total < self.max_concurrent
    }

    /// Returns a snapshot of per-tracker active counts for use in closures
    /// that cannot hold a reference to `self`.
    pub(crate) fn per_tracker_snapshot(&self) -> HashMap<i64, usize> {
        self.active_per_tracker.clone()
    }
}

#[cfg(test)]
impl SlotAllocator {
    pub(crate) fn active_total(&self) -> usize {
        self.active_total
    }

    pub(crate) fn active_for_tracker(&self, tracker_id: i64) -> usize {
        self.active_per_tracker
            .get(&tracker_id)
            .copied()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DownloadProtocol;
    use harmonia_common::ids::{ReleaseId, WantId};
    use uuid::Uuid;

    fn torrent_item(tracker_id: Option<i64>) -> QueueItem {
        QueueItem {
            id: Uuid::now_v7(),
            want_id: WantId::new(),
            release_id: ReleaseId::new(),
            download_url: "magnet:?xt=urn:btih:test".to_string(),
            protocol: DownloadProtocol::Torrent,
            priority: 2,
            tracker_id,
            info_hash: None,
        }
    }

    #[test]
    fn has_slot_when_empty() {
        let allocator = SlotAllocator::new(5, 3);
        assert!(allocator.has_slot(&torrent_item(Some(1))));
    }

    #[test]
    fn no_slot_when_global_limit_reached() {
        let mut allocator = SlotAllocator::new(2, 3);
        let item1 = torrent_item(Some(1));
        let item2 = torrent_item(Some(2));
        allocator.acquire(&item1);
        allocator.acquire(&item2);
        assert_eq!(allocator.active_total(), 2);
        assert!(!allocator.has_slot(&torrent_item(Some(3))));
    }

    #[test]
    fn no_slot_when_per_tracker_limit_reached() {
        let mut allocator = SlotAllocator::new(10, 2);
        let item1 = torrent_item(Some(1));
        let item2 = torrent_item(Some(1));
        allocator.acquire(&item1);
        allocator.acquire(&item2);
        assert_eq!(allocator.active_for_tracker(1), 2);
        assert!(!allocator.has_slot(&torrent_item(Some(1))));
        // Different tracker can still go
        assert!(allocator.has_slot(&torrent_item(Some(2))));
    }

    #[test]
    fn release_opens_slot() {
        let mut allocator = SlotAllocator::new(1, 3);
        let item = torrent_item(Some(1));
        allocator.acquire(&item);
        assert!(!allocator.global_slot_available());
        allocator.release(DownloadProtocol::Torrent, Some(1));
        assert!(allocator.global_slot_available());
        assert_eq!(allocator.active_for_tracker(1), 0);
    }

    #[test]
    fn release_removes_tracker_entry_when_zero() {
        let mut allocator = SlotAllocator::new(5, 3);
        let item = torrent_item(Some(99));
        allocator.acquire(&item);
        allocator.release(DownloadProtocol::Torrent, Some(99));
        // Tracker entry removed; next call returns 0
        assert_eq!(allocator.active_for_tracker(99), 0);
    }

    #[test]
    fn no_per_tracker_limit_for_no_tracker_id() {
        let mut allocator = SlotAllocator::new(5, 1);
        // torrent with no tracker_id — no per-tracker limit applies
        let item1 = torrent_item(None);
        let item2 = torrent_item(None);
        allocator.acquire(&item1);
        assert!(allocator.has_slot(&item2));
    }

    #[test]
    fn release_underflow_does_not_panic() {
        let mut allocator = SlotAllocator::new(5, 3);
        // Releasing when nothing is active should not panic
        allocator.release(DownloadProtocol::Torrent, Some(1));
        assert_eq!(allocator.active_total(), 0);
    }
}
