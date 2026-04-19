/// Periodic clock sync probe scheduler with adaptive interval.
use std::time::Duration;

use super::ClockEstimator;
use crate::config::ClockConfig;

#[derive(Debug)]
pub struct SyncScheduler {
    interval: Duration,
    last_stable_offset: i64,
    config: ClockConfig,
}

impl SyncScheduler {
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(ClockConfig::default())
    }

    #[must_use]
    pub fn with_config(config: ClockConfig) -> Self {
        Self {
            interval: config.initial_interval(),
            last_stable_offset: 0,
            config,
        }
    }

    /// Update the scheduler based on the current estimator state.
    /// Returns the interval to use before the next probe.
    #[must_use]
    pub fn update(&mut self, estimator: &ClockEstimator) -> Duration {
        let initial = self.config.initial_interval();
        let stable = self.config.stable_interval();
        if estimator.is_stable() {
            let drift = (estimator.offset_us() - self.last_stable_offset).unsigned_abs() as i64;
            if drift > self.config.drift_threshold_us {
                self.interval = initial;
            } else {
                self.interval = self.interval.min(stable).max(initial);
                // WHY: Gradually increase toward stable interval when OFFSET is steady.
                let step = initial;
                if self.interval < stable {
                    self.interval = (self.interval + step).min(stable);
                }
            }
            self.last_stable_offset = estimator.offset_us();
        } else {
            self.interval = initial;
        }
        self.interval
    }

    /// Current probe interval.
    #[must_use]
    pub fn interval(&self) -> Duration {
        self.interval
    }
}

impl Default for SyncScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_initial_interval() {
        let sched = SyncScheduler::new();
        assert_eq!(sched.interval(), ClockConfig::default().initial_interval());
    }

    #[test]
    fn backs_off_when_stable() {
        let mut sched = SyncScheduler::new();
        let mut est = ClockEstimator::new();
        for i in 0..10 {
            let base = i * 100_000;
            est.record_exchange(base, base + 500, base + 600, base + 1100);
        }
        let mut prev = sched.update(&est);
        for _ in 0..10 {
            let next = sched.update(&est);
            assert!(next >= prev, "interval should not decrease when stable");
            prev = next;
        }
        assert_eq!(prev, ClockConfig::default().stable_interval());
    }

    #[test]
    fn re_accelerates_on_drift() {
        let mut sched = SyncScheduler::new();
        let mut est = ClockEstimator::new();
        for i in 0..10 {
            let base = i * 100_000;
            est.record_exchange(base, base + 500, base + 600, base + 1100);
        }
        for _ in 0..10 {
            let _ = sched.update(&est);
        }
        assert_eq!(sched.interval(), ClockConfig::default().stable_interval());

        // Inject drift: OFFSET jumps by a large amount
        for i in 10..20 {
            let base = i * 100_000;
            est.record_exchange(base, base + 5500, base + 5600, base + 1100);
        }
        let interval = sched.update(&est);
        assert_eq!(
            interval,
            ClockConfig::default().initial_interval(),
            "should re-accelerate on drift"
        );
    }

    #[test]
    fn custom_config_changes_starting_interval() {
        // WHY: non-default config observably changes the scheduler's starting interval.
        let cfg = ClockConfig {
            initial_interval_secs: 11,
            stable_interval_secs: 77,
            ..ClockConfig::default()
        };
        let sched = SyncScheduler::with_config(cfg);
        assert_eq!(sched.interval(), Duration::from_secs(11));
    }
}
