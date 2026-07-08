mod policy;

use std::time::Duration;

pub use policy::SeedingPolicy;

impl SeedingPolicy {
    /// Reports whether seeding may stop: upload ratio met OR seed time
    /// elapsed.
    ///
    /// `seeding_elapsed` is a duration (not an `Instant`) because the seed
    /// clock continues from a persisted wall-clock start across restarts —
    /// callers compute `now - seed_started_at`.
    pub fn is_satisfied(
        &self,
        uploaded_bytes: u64,
        downloaded_bytes: u64,
        seeding_elapsed: Duration,
    ) -> bool {
        // WHY: zero-denominator guard — a torrent with no recorded size can
        // only satisfy by time, never by a fabricated infinite ratio.
        let ratio = if downloaded_bytes == 0 {
            0.0
        } else {
            uploaded_bytes as f64 / downloaded_bytes as f64
        };

        ratio >= self.ratio_threshold || seeding_elapsed >= self.time_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_threshold_satisfied() {
        let policy = SeedingPolicy {
            ratio_threshold: 1.0,
            time_threshold: Duration::from_secs(72 * 3600),
        };
        assert!(policy.is_satisfied(1000, 1000, Duration::ZERO));
        assert!(policy.is_satisfied(2000, 1000, Duration::ZERO));
    }

    #[test]
    fn neither_threshold_satisfied() {
        let policy = SeedingPolicy {
            ratio_threshold: 1.0,
            time_threshold: Duration::from_secs(999_999),
        };
        assert!(!policy.is_satisfied(500, 1000, Duration::from_secs(10)));
    }

    #[test]
    fn time_threshold_satisfied() {
        let policy = SeedingPolicy {
            ratio_threshold: 999.0,
            time_threshold: Duration::from_secs(3600),
        };
        assert!(policy.is_satisfied(0, 1000, Duration::from_secs(3600)));
        assert!(policy.is_satisfied(0, 1000, Duration::from_secs(7200)));
    }

    #[test]
    fn zero_time_threshold_satisfies_immediately() {
        let policy = SeedingPolicy {
            ratio_threshold: 999.0,
            time_threshold: Duration::ZERO,
        };
        assert!(policy.is_satisfied(0, 1000, Duration::ZERO));
    }

    #[test]
    fn zero_downloaded_bytes_does_not_panic() {
        let policy = SeedingPolicy {
            ratio_threshold: 1.0,
            time_threshold: Duration::from_secs(72 * 3600),
        };
        assert!(!policy.is_satisfied(0, 0, Duration::ZERO));
        assert!(!policy.is_satisfied(5000, 0, Duration::ZERO));
    }

    #[test]
    fn policy_derives_from_config() {
        let config = horismos::ErgasiaConfig {
            seed_ratio_threshold: 2.5,
            seed_time_threshold_hours: 10,
            ..horismos::ErgasiaConfig::default()
        };
        let policy = SeedingPolicy::from(&config);
        assert!((policy.ratio_threshold - 2.5).abs() < f64::EPSILON);
        assert_eq!(policy.time_threshold, Duration::from_secs(10 * 3600));
    }
}
