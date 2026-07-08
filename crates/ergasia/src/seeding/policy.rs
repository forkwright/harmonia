use std::time::Duration;

use horismos::ErgasiaConfig;
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

// WHY: derive-over-declare — the seed monitor rebuilds the policy FROM the
// live config Section on every poll tick, so a threshold reload applies to
// in-flight seeding without a restart (the LIVE classification in
// horismos::diff depends on this being the only construction path).
impl From<&ErgasiaConfig> for SeedingPolicy {
    fn from(config: &ErgasiaConfig) -> Self {
        Self {
            ratio_threshold: config.seed_ratio_threshold,
            time_threshold: Duration::from_secs(
                config.seed_time_threshold_hours.saturating_mul(3600),
            ),
        }
    }
}
