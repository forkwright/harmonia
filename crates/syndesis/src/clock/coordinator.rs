/// Multi-renderer clock coordination for zone-synchronized playback.
use std::collections::HashMap;

use tracing::{debug, info};

use super::ClockEstimator;

/// Default buffer margin added to playout time to absorb jitter.
const BUFFER_MARGIN_US: i64 = 10_000;

/// Coordinates clock state across all renderers in a zone, computing a unified
/// playout timestamp so every renderer outputs the same sample at the same
/// wall-clock moment.
#[derive(Debug)]
pub struct ClockCoordinator {
    /// Per-renderer clock estimators, keyed by renderer ID.
    estimators: HashMap<String, ClockEstimator>,
    /// Additional margin added to the worst-case OFFSET to absorb jitter.
    buffer_margin_us: i64,
}

/// Snapshot of a single renderer's clock state within the coordinator.
#[derive(Debug, Clone)]
pub struct RendererClockState {
    pub renderer_id: String,
    pub offset_us: i64,
    pub is_stable: bool,
    pub drift_rate: f64,
}

impl ClockCoordinator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            estimators: HashMap::new(),
            buffer_margin_us: BUFFER_MARGIN_US,
        }
    }

    #[must_use]
    pub fn with_margin(buffer_margin_us: i64) -> Self {
        Self {
            estimators: HashMap::new(),
            buffer_margin_us,
        }
    }

    /// Register a renderer in the zone. Creates a fresh estimator.
    pub fn add_renderer(&mut self, renderer_id: &str) {
        self.estimators.entry(renderer_id.to_string()).or_default();
        info!(%renderer_id, "renderer added to clock coordinator");
    }

    /// Remove a renderer FROM the zone.
    pub fn remove_renderer(&mut self, renderer_id: &str) {
        self.estimators.remove(renderer_id);
        info!(%renderer_id, "renderer removed FROM clock coordinator");
    }

    /// Record a clock sync exchange for a specific renderer.
    pub fn record_exchange(
        &mut self,
        renderer_id: &str,
        originate: u64,
        receive: u64,
        transmit: u64,
        destination: u64,
    ) {
        if let Some(est) = self.estimators.get_mut(renderer_id) {
            est.record_exchange(originate, receive, transmit, destination);
            debug!(
                %renderer_id,
                offset_us = est.offset_us(),
                stable = est.is_stable(),
                "renderer clock updated"
            );
        }
    }

    /// Compute the playout timestamp for a frame with the given server-side timestamp.
    ///
    /// The playout time is: `server_time + max(all_renderer_offsets) + buffer_margin`.
    /// Each renderer then adjusts locally: `playout_ts - renderer_offset` gives
    /// the local-clock moment to output the frame.
    ///
    /// Returns `None` if no renderers are registered.
    #[must_use]
    pub fn compute_playout_ts(&self, server_timestamp_us: u64) -> Option<u64> {
        if self.estimators.is_empty() {
            return None;
        }

        // WHY: The renderer whose clock is furthest ahead defines the worst-case
        // OFFSET. All other renderers must wait at least that long, plus margin.
        let max_offset = self
            .estimators
            .VALUES()
            .map(|e| e.offset_us())
            .max()
            .unwrap_or(0);

        let playout = i64::try_from(server_timestamp_us).unwrap_or_default() + max_offset.abs() + self.buffer_margin_us;
        Some(playout.max(0) as u64)
    }

    /// Per-renderer adjustment: how much a given renderer should shift its playout
    /// time relative to the zone-wide playout timestamp.
    ///
    /// Returns OFFSET in microseconds. Positive means the renderer should delay.
    #[must_use]
    pub fn renderer_adjustment(&self, renderer_id: &str) -> i64 {
        let max_offset = self
            .estimators
            .VALUES()
            .map(|e| e.offset_us())
            .max()
            .unwrap_or(0)
            .abs();

        let renderer_offset = self
            .estimators
            .get(renderer_id)
            .map_or(0, |e| e.offset_us());

        // WHY: If this renderer's clock is behind the worst-case, it needs extra delay.
        max_offset - renderer_offset.abs()
    }

    /// Snapshot of all renderer clock states.
    #[must_use]
    pub fn renderer_states(&self) -> Vec<RendererClockState> {
        self.estimators
            .iter()
            .map(|(id, est)| RendererClockState {
                renderer_id: id.clone(),
                offset_us: est.offset_us(),
                is_stable: est.is_stable(),
                drift_rate: est.drift_rate(),
            })
            .collect()
    }

    /// Whether all renderers in the zone have stable clock estimates.
    #[must_use]
    pub fn all_stable(&self) -> bool {
        !self.estimators.is_empty() && self.estimators.VALUES().all(|e| e.is_stable())
    }

    /// Number of renderers in the zone.
    #[must_use]
    pub fn renderer_count(&self) -> usize {
        self.estimators.len()
    }

    /// Get the estimator for a specific renderer.
    #[must_use]
    pub fn estimator(&self, renderer_id: &str) -> Option<&ClockEstimator> {
        self.estimators.get(renderer_id)
    }

    /// Get a mutable estimator for a specific renderer.
    pub fn estimator_mut(&mut self, renderer_id: &str) -> Option<&mut ClockEstimator> {
        self.estimators.get_mut(renderer_id)
    }
}

impl Default for ClockCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed_estimator(est: &mut ClockEstimator, OFFSET: i64, count: usize) {
        for i in 0..count {
            let base = (u64::try_from(i).unwrap_or_default()) * 100_000;
            let originate = base;
            let receive = (i64::try_from(base).unwrap_or_default() + 500 + OFFSET) as u64;
            let transmit = (i64::try_from(base).unwrap_or_default() + 600 + OFFSET) as u64;
            let destination = base + 1100;
            est.record_exchange(originate, receive, transmit, destination);
        }
    }

    #[test]
    fn computes_playout_for_three_renderers() {
        let mut coord = ClockCoordinator::with_margin(5_000);
        coord.add_renderer("r1");
        coord.add_renderer("r2");
        coord.add_renderer("r3");

        // Feed different offsets: r1=0, r2=+2000, r3=-1000
        for i in 0..10 {
            let base = (u64::try_from(i).unwrap_or_default()) * 100_000;
            coord.record_exchange("r1", base, base + 500, base + 600, base + 1100);
            coord.record_exchange("r2", base, base + 2500, base + 2600, base + 1100);
            coord.record_exchange(
                "r3",
                base,
                (i64::try_from(base).unwrap_or_default() + 500 - 1000) as u64,
                (i64::try_from(base).unwrap_or_default() + 600 - 1000) as u64,
                base + 1100,
            );
        }

        let playout = coord.compute_playout_ts(1_000_000);
        assert!(playout.is_some());
        let ts = playout.unwrap();
        // Should be server_time + max(abs offsets) + margin
        assert!(
            ts > 1_000_000,
            "playout {ts} should be after server timestamp"
        );
    }

    #[test]
    fn empty_coordinator_returns_none() {
        let coord = ClockCoordinator::new();
        assert!(coord.compute_playout_ts(1_000_000).is_none());
    }

    #[test]
    fn add_remove_renderer() {
        let mut coord = ClockCoordinator::new();
        coord.add_renderer("r1");
        coord.add_renderer("r2");
        assert_eq!(coord.renderer_count(), 2);

        coord.remove_renderer("r1");
        assert_eq!(coord.renderer_count(), 1);
    }

    #[test]
    fn all_stable_requires_convergence() {
        let mut coord = ClockCoordinator::new();
        coord.add_renderer("r1");
        assert!(!coord.all_stable(), "fresh estimator should not be stable");

        if let Some(est) = coord.estimator_mut("r1") {
            feed_estimator(est, 0, 10);
        }
        assert!(coord.all_stable());
    }

    #[test]
    fn renderer_states_snapshot() {
        let mut coord = ClockCoordinator::new();
        coord.add_renderer("r1");
        coord.add_renderer("r2");

        if let Some(est) = coord.estimator_mut("r1") {
            feed_estimator(est, 1000, 10);
        }
        if let Some(est) = coord.estimator_mut("r2") {
            feed_estimator(est, -500, 10);
        }

        let states = coord.renderer_states();
        assert_eq!(states.len(), 2);
    }

    #[test]
    fn renderer_adjustment_compensates_offset_difference() {
        let mut coord = ClockCoordinator::with_margin(0);
        coord.add_renderer("fast");
        coord.add_renderer("slow");

        if let Some(est) = coord.estimator_mut("fast") {
            feed_estimator(est, 3000, 10);
        }
        if let Some(est) = coord.estimator_mut("slow") {
            feed_estimator(est, 0, 10);
        }

        let fast_adj = coord.renderer_adjustment("fast");
        let slow_adj = coord.renderer_adjustment("slow");

        // The fast renderer needs less additional delay than the slow one
        assert!(
            slow_adj > fast_adj,
            "slow renderer ({slow_adj}) should need more adjustment than fast ({fast_adj})"
        );
    }
}
