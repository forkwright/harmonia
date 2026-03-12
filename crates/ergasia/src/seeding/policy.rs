use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedingPolicy {
    pub ratio_threshold: f64,
    pub time_threshold: Duration,
}

impl Default for SeedingPolicy {
    fn default() -> Self {
        Self {
            ratio_threshold: 1.0,
            time_threshold: Duration::from_secs(72 * 3600),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerSeedPolicy {
    pub ratio_threshold: f64,
    pub time_threshold_hours: u64,
}
