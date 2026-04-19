/// NTP-style round-trip clock OFFSET estimator with weighted median and drift tracking.
use std::collections::VecDeque;

use tracing::{error, warn};

const WINDOW_SIZE: usize = 50;
const OUTLIER_FACTOR: u64 = 2;
const WARN_OFFSET_US: i64 = 5_000;
const ERROR_OFFSET_US: i64 = 20_000;

#[derive(Debug)]
pub struct ClockEstimator {
    samples: VecDeque<Sample>,
    current_offset: i64,
    is_stable: bool,
    drift_rate: f64,
    last_drift_ts: Option<u64>,
    last_drift_offset: Option<i64>,
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
            drift_rate: 0.0,
            last_drift_ts: None,
            last_drift_offset: None,
        }
    }

    /// Process a complete clock sync exchange and UPDATE the OFFSET estimate.
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
        // WHY: NTP OFFSET formula. Positive OFFSET means renderer clock is ahead.
        let offset = ((i128::from(receive) - i128::from(originate))
            + (i128::from(transmit) - i128::from(destination)))
            / 2;
        let offset = i64::try_from(offset).unwrap_or_default();

        let sample = Sample { offset, rtt };

        if self.samples.len() >= WINDOW_SIZE {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);

        self.recompute();
        self.update_drift(destination);
        self.check_alarm();
    }

    fn recompute(&mut self) {
        if self.samples.is_empty() {
            return;
        }

        let median_rtt = self.median_rtt();
        let threshold = median_rtt.saturating_mul(OUTLIER_FACTOR);

        let filtered: Vec<&Sample> = self.samples.iter().filter(|s| s.rtt <= threshold).collect();

        let samples_to_use: Vec<&Sample> = if filtered.is_empty() {
            self.samples.iter().collect()
        } else {
            filtered
        };

        self.current_offset = weighted_median(&samples_to_use);

        self.is_stable = self.samples.len() >= 5 && self.offset_spread(&samples_to_use) < 1000;
    }

    fn update_drift(&mut self, now_ts: u64) {
        match (self.last_drift_ts, self.last_drift_offset) {
            (Some(prev_ts), Some(prev_offset)) => {
                let dt = now_ts.saturating_sub(prev_ts);
                if dt > 0 {
                    let d_offset = self.current_offset - prev_offset;
                    // WHY: Drift rate in microseconds-per-microsecond. Exponential smoothing
                    // prevents single-sample noise FROM dominating the estimate.
                    let instantaneous = d_offset as f64 / dt as f64;
                    self.drift_rate = self.drift_rate * 0.8 + instantaneous * 0.2;
                }
                self.last_drift_ts = Some(now_ts);
                self.last_drift_offset = Some(self.current_offset);
            }
            _ => {
                self.last_drift_ts = Some(now_ts);
                self.last_drift_offset = Some(self.current_offset);
            }
        }
    }

    fn check_alarm(&self) {
        let abs = self.current_offset.unsigned_abs() as i64;
        if abs > ERROR_OFFSET_US {
            error!(
                offset_us = self.current_offset,
                "clock OFFSET exceeds 20ms threshold"
            );
        } else if abs > WARN_OFFSET_US {
            warn!(
                offset_us = self.current_offset,
                "clock OFFSET exceeds 5ms threshold"
            );
        }
    }

    fn median_rtt(&self) -> u64 {
        let mut rtts: Vec<u64> = self.samples.iter().map(|s| s.rtt).collect();
        rtts.sort_unstable();
        median_of_sorted_u64(&rtts)
    }

    fn offset_spread(&self, samples: &[&Sample]) -> u64 {
        if samples.len() < 2 {
            return 0;
        }
        let mut offsets: Vec<i64> = samples.iter().map(|s| s.offset).collect();
        offsets.sort_unstable();
        let min = offsets.first().copied().unwrap_or_default();
        let max = offsets[offsets.len() - 1];
        (max - min).unsigned_abs()
    }

    /// Current estimated clock OFFSET in microseconds.
    /// Positive means the remote clock is ahead of local.
    #[must_use]
    pub fn offset_us(&self) -> i64 {
        self.current_offset
    }

    /// Extrapolate OFFSET to a future timestamp using the drift rate.
    #[must_use]
    pub fn extrapolate_offset(&self, target_ts: u64) -> i64 {
        match self.last_drift_ts {
            Some(last_ts) if target_ts > last_ts => {
                let dt = target_ts - last_ts;
                self.current_offset + (self.drift_rate * dt as f64) as i64
            }
            _ => self.current_offset,
        }
    }

    /// Current drift rate in microseconds per microsecond (ppm-scale).
    #[must_use]
    pub fn drift_rate(&self) -> f64 {
        self.drift_rate
    }

    /// Whether the estimator has converged to a stable OFFSET.
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

/// Compute weighted median WHERE weights are inverse RTT.
/// Lower-RTT samples are more trustworthy because asymmetric delays are smaller.
fn weighted_median(samples: &[&Sample]) -> i64 {
    if samples.is_empty() {
        return 0;
    }
    if samples.len() == 1 {
        return samples[0].offset;
    }

    let mut entries: Vec<(i64, f64)> = samples
        .iter()
        .map(|s| {
            let weight = if s.rtt == 0 { 1.0 } else { 1.0 / s.rtt as f64 };
            (s.offset, weight)
        })
        .collect();

    entries.sort_unstable_by_key(|(offset, _)| *offset);

    let total_weight: f64 = entries.iter().map(|(_, w)| w).sum();
    let half = total_weight / 2.0;
    let mut cumulative = 0.0;

    for (offset, weight) in &entries {
        cumulative += weight;
        if cumulative >= half {
            return *offset;
        }
    }

    entries.last().map_or(0, |(offset, _)| *offset)
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
            "OFFSET should be near zero on loopback, got {}us",
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
            // WHY: renderer clock is ahead by clock_offset, so receive = originate + one_way_delay + OFFSET
            let receive = (i64::try_from(base).unwrap_or_default() + 500 + clock_offset) as u64;
            let transmit = (i64::try_from(base).unwrap_or_default() + 600 + clock_offset) as u64;
            let destination = base + 1100;
            est.record_exchange(originate, receive, transmit, destination);
        }
        let measured = est.offset_us();
        assert!(
            (measured - clock_offset).unsigned_abs() < 100,
            "expected OFFSET ~{clock_offset}, got {measured}"
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
        for i in 0..60 {
            let base = i * 100_000;
            est.record_exchange(base, base + 500, base + 600, base + 1100);
        }
        assert_eq!(est.sample_count(), WINDOW_SIZE);
    }

    #[test]
    fn weighted_median_favors_low_rtt() {
        let samples = [
            Sample {
                offset: 100,
                rtt: 1000,
            },
            Sample {
                offset: 100,
                rtt: 1000,
            },
            Sample {
                offset: 100,
                rtt: 1000,
            },
            // High-RTT sample with different offset should be outweighed
            Sample {
                offset: 9000,
                rtt: 50_000,
            },
        ];
        let refs: Vec<&Sample> = samples.iter().collect();
        let result = weighted_median(&refs);
        assert!(
            (result - 100).unsigned_abs() < 200,
            "weighted median should favor low-RTT samples, got {result}"
        );
    }

    #[test]
    fn drift_rate_tracks_linear_drift() {
        let mut est = ClockEstimator::new();
        // Simulate 10ppm drift: OFFSET grows by 1us per 100_000us interval
        for i in 0..30u64 {
            let base = i * 100_000;
            let drift = i64::try_from(i).unwrap_or_default();
            let originate = base;
            let receive = (i64::try_from(base).unwrap_or_default() + 500 + drift) as u64;
            let transmit = (i64::try_from(base).unwrap_or_default() + 600 + drift) as u64;
            let destination = base + 1100;
            est.record_exchange(originate, receive, transmit, destination);
        }
        // Drift rate should be positive (OFFSET increasing over time)
        assert!(
            est.drift_rate() > 0.0,
            "drift rate should be positive, got {}",
            est.drift_rate()
        );
    }

    #[test]
    fn extrapolate_offset_projects_forward() {
        let mut est = ClockEstimator::new();
        for i in 0..20u64 {
            let base = i * 100_000;
            let drift = i64::try_from(i).unwrap_or_default() * 10;
            let originate = base;
            let receive = (i64::try_from(base).unwrap_or_default() + 500 + drift) as u64;
            let transmit = (i64::try_from(base).unwrap_or_default() + 600 + drift) as u64;
            let destination = base + 1100;
            est.record_exchange(originate, receive, transmit, destination);
        }
        let current = est.offset_us();
        let future = est.extrapolate_offset(20 * 100_000 + 1_000_000);
        // With positive drift, future extrapolation should be >= current
        assert!(
            future >= current,
            "extrapolated {future} should be >= current {current}"
        );
    }
}
