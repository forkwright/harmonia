/// NTP-style round-trip clock offset estimator.
use std::collections::VecDeque;

const WINDOW_SIZE: usize = 20;
const OUTLIER_FACTOR: u64 = 2;

#[derive(Debug)]
pub struct ClockEstimator {
    samples: VecDeque<Sample>,
    current_offset: i64,
    is_stable: bool,
}

#[derive(Debug, Clone, Copy)]
struct Sample {
    offset: i64,
    rtt: u64,
}

impl ClockEstimator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(WINDOW_SIZE),
            current_offset: 0,
            is_stable: false,
        }
    }

    /// Process a complete clock sync exchange and update the offset estimate.
    ///
    /// Timestamps are in microseconds:
    /// - `originate`: server sent the probe
    /// - `receive`: renderer received the probe
    /// - `transmit`: renderer sent the reply
    /// - `destination`: server received the reply
    pub fn record_exchange(
        &mut self,
        originate: u64,
        receive: u64,
        transmit: u64,
        destination: u64,
    ) {
        let rtt = destination.saturating_sub(originate);
        // WHY: NTP offset formula. Positive offset means renderer clock is ahead.
        let offset =
            ((receive as i128 - originate as i128) + (transmit as i128 - destination as i128)) / 2;
        let offset = offset as i64;

        let sample = Sample { offset, rtt };

        if self.samples.len() >= WINDOW_SIZE {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);

        self.recompute();
    }

    fn recompute(&mut self) {
        if self.samples.is_empty() {
            return;
        }

        let median_rtt = self.median_rtt();
        let threshold = median_rtt.saturating_mul(OUTLIER_FACTOR);

        let mut filtered_offsets: Vec<i64> = self
            .samples
            .iter()
            .filter(|s| s.rtt <= threshold)
            .map(|s| s.offset)
            .collect();

        if filtered_offsets.is_empty() {
            filtered_offsets = self.samples.iter().map(|s| s.offset).collect();
        }

        filtered_offsets.sort_unstable();
        self.current_offset = median_of_sorted(&filtered_offsets);

        self.is_stable = self.samples.len() >= 5 && self.offset_spread(&filtered_offsets) < 1000;
    }

    fn median_rtt(&self) -> u64 {
        let mut rtts: Vec<u64> = self.samples.iter().map(|s| s.rtt).collect();
        rtts.sort_unstable();
        median_of_sorted_u64(&rtts)
    }

    fn offset_spread(&self, offsets: &[i64]) -> u64 {
        if offsets.len() < 2 {
            return 0;
        }
        let min = offsets[0];
        let max = offsets[offsets.len() - 1];
        (max - min).unsigned_abs()
    }

    /// Current estimated clock offset in microseconds.
    /// Positive means the remote clock is ahead of local.
    #[must_use]
    pub fn offset_us(&self) -> i64 {
        self.current_offset
    }

    /// Whether the estimator has converged to a stable offset.
    #[must_use]
    pub fn is_stable(&self) -> bool {
        self.is_stable
    }

    /// Number of samples in the sliding window.
    #[must_use]
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
}

impl Default for ClockEstimator {
    fn default() -> Self {
        Self::new()
    }
}

fn median_of_sorted(values: &[i64]) -> i64 {
    let len = values.len();
    if len == 0 {
        return 0;
    }
    if len % 2 == 1 {
        values[len / 2]
    } else {
        (values[len / 2 - 1] + values[len / 2]) / 2
    }
}

fn median_of_sorted_u64(values: &[u64]) -> u64 {
    let len = values.len();
    if len == 0 {
        return 0;
    }
    if len % 2 == 1 {
        values[len / 2]
    } else {
        (values[len / 2 - 1] + values[len / 2]) / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converges_to_zero_on_symmetric_loopback() {
        let mut est = ClockEstimator::new();
        for i in 0..20 {
            let base = i * 100_000;
            let originate = base;
            let receive = base + 500;
            let transmit = base + 600;
            let destination = base + 1100;
            est.record_exchange(originate, receive, transmit, destination);
        }
        assert!(
            est.offset_us().unsigned_abs() < 1000,
            "offset should be near zero on loopback, got {}us",
            est.offset_us()
        );
        assert!(est.is_stable());
    }

    #[test]
    fn detects_fixed_offset() {
        let mut est = ClockEstimator::new();
        let clock_offset: i64 = 5000;
        for i in 0..20 {
            let base = i * 100_000u64;
            let originate = base;
            // WHY: renderer clock is ahead by clock_offset, so receive = originate + one_way_delay + offset
            let receive = (base as i64 + 500 + clock_offset) as u64;
            let transmit = (base as i64 + 600 + clock_offset) as u64;
            let destination = base + 1100;
            est.record_exchange(originate, receive, transmit, destination);
        }
        let measured = est.offset_us();
        assert!(
            (measured - clock_offset).unsigned_abs() < 100,
            "expected offset ~{clock_offset}, got {measured}"
        );
    }

    #[test]
    fn rejects_outliers() {
        let mut est = ClockEstimator::new();
        for i in 0..15 {
            let base = i * 100_000;
            est.record_exchange(base, base + 500, base + 600, base + 1100);
        }
        // Inject an outlier with huge RTT
        est.record_exchange(2_000_000, 2_500_000, 2_500_100, 3_000_000);

        for i in 16..20 {
            let base = i * 100_000;
            est.record_exchange(base, base + 500, base + 600, base + 1100);
        }
        assert!(
            est.offset_us().unsigned_abs() < 1000,
            "outlier should be rejected, got {}us",
            est.offset_us()
        );
    }

    #[test]
    fn not_stable_with_few_samples() {
        let mut est = ClockEstimator::new();
        est.record_exchange(0, 500, 600, 1100);
        assert!(!est.is_stable());
    }

    #[test]
    fn sliding_window_evicts_old_samples() {
        let mut est = ClockEstimator::new();
        for i in 0..30 {
            let base = i * 100_000;
            est.record_exchange(base, base + 500, base + 600, base + 1100);
        }
        assert_eq!(est.sample_count(), WINDOW_SIZE);
    }
}
