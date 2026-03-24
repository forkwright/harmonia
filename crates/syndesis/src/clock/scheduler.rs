/// Periodic clock sync probe scheduler with adaptive interval.
use std::time::Duration;

use super::ClockEstimator;

const INITIAL_INTERVAL: Duration = Duration::from_secs(5);
const STABLE_INTERVAL: Duration = Duration::from_secs(30);
const DRIFT_THRESHOLD_US: i64 = 500;

#[derive(Debug)]
pub struct SyncScheduler {
    interval: Duration,
    last_stable_offset: i64,
}

impl SyncScheduler {
    #[must_use]
    pub fn new() -> Self {
        Self {
            interval: INITIAL_INTERVAL,
            last_stable_offset: 0,
        }
    }

    /// Update the scheduler based on the current estimator state.
    /// Returns the interval to use before the next probe.
    #[must_use]
    pub fn update(&mut self, estimator: &ClockEstimator) -> Duration {
        if estimator.is_stable() {
            let drift = (estimator.offset_us() - self.last_stable_offset).unsigned_abs() as i64;
            if drift > DRIFT_THRESHOLD_US {
                self.interval = INITIAL_INTERVAL;
            } else {
                self.interval = self.interval.min(STABLE_INTERVAL).max(INITIAL_INTERVAL);
                // WHY: Gradually increase toward stable interval when offset is steady.
                let step = Duration::from_secs(5);
                if self.interval < STABLE_INTERVAL {
                    self.interval = (self.interval + step).min(STABLE_INTERVAL);
                }
            }
            self.last_stable_offset = estimator.offset_us();
        } else {
            self.interval = INITIAL_INTERVAL;
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
        assert_eq!(sched.interval(), INITIAL_INTERVAL);
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
        assert_eq!(prev, STABLE_INTERVAL);
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
        assert_eq!(sched.interval(), STABLE_INTERVAL);

        // Inject drift: offset jumps by a large amount
        for i in 10..20 {
            let base = i * 100_000;
            est.record_exchange(base, base + 5500, base + 5600, base + 1100);
        }
        let interval = sched.update(&est);
        assert_eq!(interval, INITIAL_INTERVAL, "should re-accelerate on drift");
    }
}
