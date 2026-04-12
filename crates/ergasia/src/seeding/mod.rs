mod policy;

pub use policy::{SeedingPolicy, TrackerSeedPolicy};

use std::collections::HashMap;
use std::time::{Duration, Instant};

impl SeedingPolicy {
    pub fn resolve_for_trackers(
        &self,
        tracker_urls: &[String],
        overrides: &HashMap<String, TrackerSeedPolicy>,
    ) -> SeedingPolicy {
        let mut most_restrictive = self.clone();
        let mut matched = false;

        for url in tracker_urls {
            for (pattern, policy) in overrides {
                if url.contains(pattern.as_str()) {
                    if !matched {
                        most_restrictive.ratio_threshold = policy.ratio_threshold;
                        most_restrictive.time_threshold =
                            Duration::from_secs(policy.time_threshold_hours * 3600);
                        matched = true;
                    } else {
                        if policy.ratio_threshold > most_restrictive.ratio_threshold {
                            most_restrictive.ratio_threshold = policy.ratio_threshold;
                        }
                        let candidate = Duration::from_secs(policy.time_threshold_hours * 3600);
                        if candidate > most_restrictive.time_threshold {
                            most_restrictive.time_threshold = candidate;
                        }
                    }
                }
            }
        }

        most_restrictive
    }

    pub fn is_satisfied(
        &self,
        uploaded_bytes: u64,
        downloaded_bytes: u64,
        seeding_since: Instant,
    ) -> bool {
        let ratio = if downloaded_bytes == 0 {
            0.0
        } else {
            uploaded_bytes as f64 / downloaded_bytes as f64
        };
        let elapsed = seeding_since.elapsed();

        ratio >= self.ratio_threshold || elapsed >= self.time_threshold
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
        let seeding_since = Instant::now();
        assert!(policy.is_satisfied(1000, 1000, seeding_since));
        assert!(policy.is_satisfied(2000, 1000, seeding_since));
    }

    #[test]
    fn ratio_threshold_not_satisfied() {
        let policy = SeedingPolicy {
            ratio_threshold: 1.0,
            time_threshold: Duration::from_secs(999_999),
        };
        let seeding_since = Instant::now();
        assert!(!policy.is_satisfied(500, 1000, seeding_since));
    }

    #[test]
    fn time_threshold_satisfied() {
        let policy = SeedingPolicy {
            ratio_threshold: 999.0,
            time_threshold: Duration::from_millis(0),
        };
        let seeding_since = Instant::now() - Duration::from_millis(1);
        assert!(policy.is_satisfied(0, 1000, seeding_since));
    }

    #[test]
    fn zero_downloaded_bytes_does_not_panic() {
        let policy = SeedingPolicy {
            ratio_threshold: 1.0,
            time_threshold: Duration::from_secs(72 * 3600),
        };
        let seeding_since = Instant::now();
        assert!(!policy.is_satisfied(0, 0, seeding_since));
    }

    #[test]
    fn tracker_override_selects_most_restrictive() {
        let base = SeedingPolicy {
            ratio_threshold: 1.0,
            time_threshold: Duration::from_secs(72 * 3600),
        };

        let mut overrides = HashMap::new();
        overrides.insert(
            "tracker.alpha.cc".to_string(),
            TrackerSeedPolicy {
                ratio_threshold: 2.0,
                time_threshold_hours: 168,
            },
        );
        overrides.insert(
            "tracker.beta.org".to_string(),
            TrackerSeedPolicy {
                ratio_threshold: 1.5,
                time_threshold_hours: 240,
            },
        );

        let trackers = vec![
            "https://tracker.alpha.cc/announce".to_string(),
            "https://tracker.beta.org/announce".to_string(),
        ];

        let resolved = base.resolve_for_trackers(&trackers, &overrides);
        assert!(
            (resolved.ratio_threshold - 2.0).abs() < f64::EPSILON,
            "expected ratio 2.0, got {}",
            resolved.ratio_threshold
        );
        assert_eq!(
            resolved.time_threshold,
            Duration::from_secs(240 * 3600),
            "expected time 240h"
        );
    }

    #[test]
    fn no_tracker_override_uses_base_policy() {
        let base = SeedingPolicy {
            ratio_threshold: 1.0,
            time_threshold: Duration::from_secs(72 * 3600),
        };

        let overrides = HashMap::new();
        let trackers = vec!["https://public.tracker.example/announce".to_string()];

        let resolved = base.resolve_for_trackers(&trackers, &overrides);
        assert!((resolved.ratio_threshold - 1.0).abs() < f64::EPSILON);
        assert_eq!(resolved.time_threshold, Duration::from_secs(72 * 3600));
    }
}
