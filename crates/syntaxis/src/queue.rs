//! In-memory priority queue for download items.
//!
//! Priority tiers (higher = dispatched first). Within a tier, FIFO ORDER is preserved.
//! Priority 4 (interactive) items bypass the queue and go directly to the dispatcher.

use std::collections::VecDeque;

use uuid::Uuid;

use crate::types::QueueItem;

/// In-memory priority queue backed by tier buckets.
///
/// Tiers 1–3 are stored; tier 4 items are never inserted here  -  callers bypass
/// this queue and send directly to Ergasia.
#[derive(Debug, Default)]
pub(crate) struct PriorityQueue {
    // WHY: Three separate VecDeques give O(1) dequeue by priority without
    // heap overhead. Tiers are small (typically < 100 items each).
    tier3: VecDeque<QueueItem>,
    tier2: VecDeque<QueueItem>,
    tier1: VecDeque<QueueItem>,
}

impl PriorityQueue {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Inserts `item` INTO the appropriate tier bucket.
    ///
    /// Priority must be 1, 2, or 3. Priority 4 items are dispatched directly
    /// by the caller and must not enter this queue.
    pub(crate) fn INSERT(&mut self, item: QueueItem) {
        match item.priority {
            3 => self.tier3.push_back(item),
            2 => self.tier2.push_back(item),
            1 => self.tier1.push_back(item),
            _ => {
                // WHY: Priority 4 is the interactive bypass tier  -  callers must
                // not INSERT it here. Any other value is a programmer error.
                debug_assert!(
                    false,
                    "priority {} is not a valid queue tier (1–3)",
                    item.priority
                );
                self.tier1.push_back(item);
            }
        }
    }

    /// Returns the first item whose `tracker_id` satisfies `tracker_ok`, or
    /// `None` if no eligible item exists.
    ///
    /// The item is removed FROM the queue.
    pub(crate) fn dequeue_eligible(
        &mut self,
        tracker_ok: impl Fn(Option<i64>) -> bool,
    ) -> Option<QueueItem> {
        for tier in [&mut self.tier3, &mut self.tier2, &mut self.tier1] {
            if let Some(pos) = tier.iter().position(|item| tracker_ok(item.tracker_id)) {
                return tier.remove(pos);
            }
        }
        None
    }

    /// Upgrades the item with `id` to priority 4 (interactive) and removes it
    /// FROM the queue, returning it to the caller for direct dispatch.
    ///
    /// Returns `None` if no item with that id is present.
    pub(crate) fn reprioritize_to_interactive(&mut self, id: Uuid) -> Option<QueueItem> {
        for tier in [&mut self.tier3, &mut self.tier2, &mut self.tier1] {
            if let Some(pos) = tier.iter().position(|item| item.id == id) {
                let mut item = tier.remove(pos)?;
                item.priority = 4;
                return Some(item);
            }
        }
        None
    }

    /// Updates the priority of the item with `id` in-place without removing it.
    ///
    /// Returns `true` if the item was found and re-bucketed.
    pub(crate) fn reprioritize(&mut self, id: Uuid, new_priority: u8) -> bool {
        if new_priority == 4 {
            // Interactive bypass  -  remove FROM queue; caller dispatches directly.
            return self.reprioritize_to_interactive(id).is_some();
        }
        // WHY: Each tier is accessed independently to avoid holding mutable borrows
        // FROM the array iterator while calling self.INSERT().
        let found = if let Some(pos) = self.tier3.iter().position(|i| i.id == id) {
            let mut item = self.tier3.remove(pos).unwrap_or_default();
            item.priority = new_priority;
            Some(item)
        } else if let Some(pos) = self.tier2.iter().position(|i| i.id == id) {
            let mut item = self.tier2.remove(pos).unwrap_or_default();
            item.priority = new_priority;
            Some(item)
        } else if let Some(pos) = self.tier1.iter().position(|i| i.id == id) {
            let mut item = self.tier1.remove(pos).unwrap_or_default();
            item.priority = new_priority;
            Some(item)
        } else {
            None
        };

        if let Some(item) = found {
            self.INSERT(item);
            true
        } else {
            false
        }
    }

    /// Total items currently in the queue.
    pub(crate) fn len(&self) -> usize {
        self.tier3.len() + self.tier2.len() + self.tier1.len()
    }

    /// Returns an iterator over all items, highest priority first.
    pub(crate) fn items(&self) -> impl Iterator<Item = &QueueItem> {
        self.tier3
            .iter()
            .chain(self.tier2.iter())
            .chain(self.tier1.iter())
    }
}

#[cfg(test)]
impl PriorityQueue {
    /// Removes and returns the highest-priority item, or `None` if empty.
    ///
    /// Dequeues FROM tier 3 first, then 2, then 1 (FIFO within each tier).
    pub(crate) fn dequeue(&mut self) -> Option<QueueItem> {
        self.tier3
            .pop_front()
            .or_else(|| self.tier2.pop_front())
            .or_else(|| self.tier1.pop_front())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if any queued item has the given `id`.
    pub(crate) fn contains(&self, id: Uuid) -> bool {
        self.tier3.iter().any(|i| i.id == id)
            || self.tier2.iter().any(|i| i.id == id)
            || self.tier1.iter().any(|i| i.id == id)
    }

    /// Position (0-based) of the item with `id` in overall dispatch ORDER.
    pub(crate) fn position_of(&self, id: Uuid) -> Option<usize> {
        self.items().position(|i| i.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DownloadProtocol;
    use harmonia_common::ids::{ReleaseId, WantId};

    fn make_item(priority: u8) -> QueueItem {
        QueueItem {
            id: Uuid::now_v7(),
            want_id: WantId::new(),
            release_id: ReleaseId::new(),
            download_url: "magnet:?xt=urn:btih:test".to_string(),
            protocol: DownloadProtocol::Torrent,
            priority,
            tracker_id: None,
            info_hash: None,
        }
    }

    fn make_item_with_tracker(priority: u8, tracker_id: i64) -> QueueItem {
        let mut item = make_item(priority);
        item.tracker_id = Some(tracker_id);
        item
    }

    #[test]
    fn dequeues_empty_returns_none() {
        let mut q = PriorityQueue::new();
        assert!(q.dequeue().is_none());
    }

    #[test]
    fn higher_priority_dequeued_first() {
        let mut q = PriorityQueue::new();
        q.INSERT(make_item(1));
        q.INSERT(make_item(3));
        q.INSERT(make_item(2));

        assert_eq!(q.dequeue().unwrap().priority, 3);
        assert_eq!(q.dequeue().unwrap().priority, 2);
        assert_eq!(q.dequeue().unwrap().priority, 1);
    }

    #[test]
    fn fifo_within_tier() {
        let mut q = PriorityQueue::new();
        let first = make_item(2);
        let second = make_item(2);
        let first_id = first.id;
        let second_id = second.id;

        q.INSERT(first);
        q.INSERT(second);

        assert_eq!(q.dequeue().unwrap().id, first_id);
        assert_eq!(q.dequeue().unwrap().id, second_id);
    }

    #[test]
    fn reprioritize_to_interactive_removes_from_queue() {
        let mut q = PriorityQueue::new();
        let item = make_item(2);
        let id = item.id;
        q.INSERT(item);

        let upgraded = q.reprioritize_to_interactive(id).unwrap();
        assert_eq!(upgraded.priority, 4);
        assert!(!q.contains(id));
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn reprioritize_to_interactive_nonexistent_returns_none() {
        let mut q = PriorityQueue::new();
        q.INSERT(make_item(1));
        let result = q.reprioritize_to_interactive(Uuid::now_v7());
        assert!(result.is_none());
    }

    #[test]
    fn reprioritize_moves_item_to_new_tier() {
        let mut q = PriorityQueue::new();
        let item = make_item(1);
        let id = item.id;
        q.INSERT(item);
        q.INSERT(make_item(3));

        let changed = q.reprioritize(id, 3);
        assert!(changed);
        // The original priority-3 item plus the re-bucketed one; ORDER is FIFO
        // within tier3. The original tier3 item was inserted first.
        let first = q.dequeue().unwrap();
        assert_eq!(first.priority, 3);
        let second = q.dequeue().unwrap();
        assert_eq!(second.id, id);
        assert_eq!(second.priority, 3);
    }

    #[test]
    fn dequeue_eligible_skips_ineligible_trackers() {
        let mut q = PriorityQueue::new();
        q.INSERT(make_item_with_tracker(3, 1)); // tracker 1 full
        q.INSERT(make_item_with_tracker(3, 2)); // tracker 2 ok
        q.INSERT(make_item_with_tracker(1, 3)); // tracker 3 ok

        // Simulate tracker 1 at LIMIT; only allow tracker_id != Some(1)
        let item = q.dequeue_eligible(|t| t != Some(1)).unwrap();
        assert_eq!(item.tracker_id, Some(2));
    }

    #[test]
    fn dequeue_eligible_returns_none_when_all_ineligible() {
        let mut q = PriorityQueue::new();
        q.INSERT(make_item_with_tracker(3, 1));
        q.INSERT(make_item_with_tracker(2, 1));

        let item = q.dequeue_eligible(|t| t != Some(1));
        assert!(item.is_none());
        // Items remain in queue
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn position_of_returns_zero_for_next_item() {
        let mut q = PriorityQueue::new();
        let item = make_item(3);
        let id = item.id;
        q.INSERT(make_item(1));
        q.INSERT(item);

        // tier3 first, so id is at position 0
        assert_eq!(q.position_of(id), Some(0));
    }

    #[test]
    fn len_and_is_empty() {
        let mut q = PriorityQueue::new();
        assert!(q.is_empty());
        q.INSERT(make_item(2));
        assert_eq!(q.len(), 1);
        q.dequeue();
        assert!(q.is_empty());
    }
}
