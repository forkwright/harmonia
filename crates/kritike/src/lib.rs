pub mod assessment;
pub mod error;
pub mod format_score;
pub mod health;
pub mod profile;
pub mod upgrade;

use std::sync::Arc;

pub use assessment::{QualityAssessment, QualityMetadata};
pub use error::KritikeError;
pub use format_score::QualityScore;
pub use health::{HealthReport, TypeHealthReport};
use sqlx::SqlitePool;
use themelion::{EventSender, HarmoniaEvent, HaveId, MediaId, MediaType, QualityProfile};
use tokio::sync::Semaphore;
use tracing::instrument;
pub use upgrade::UpgradeDecision;

use crate::error::ConcurrencyLimiterClosedSnafu;

#[expect(
    async_fn_in_trait,
    reason = "used with static dispatch; Send bounds on concrete impls are sufficient"
)]
pub trait CurationService: Send + Sync {
    /// Assess quality score for an imported item.
    async fn assess_quality(
        &self,
        media_type: MediaType,
        item_metadata: &QualityMetadata,
    ) -> Result<QualityAssessment, KritikeError>;

    /// Check if an existing have should be upgraded.
    async fn check_upgrade_eligibility(
        &self,
        have_id: HaveId,
        candidate_score: i32,
    ) -> Result<UpgradeDecision, KritikeError>;

    /// Generate library health report.
    async fn health_report(&self) -> Result<HealthReport, KritikeError>;
}

pub struct DefaultCurationService {
    pool: SqlitePool,
    events: EventSender,
    /// Bounds concurrent `assess_quality` calls to
    /// `KritikeConfig::quality_check_concurrency`.
    quality_check_semaphore: Arc<Semaphore>,
}

impl DefaultCurationService {
    /// `quality_check_concurrency` is clamped to at least 1 as
    /// defense-in-depth — a 0-permit semaphore would block every quality
    /// check forever, the same hang class `taxis.scan_concurrency` had.
    /// Callers are expected to reject `0` upstream (see `horismos`
    /// validation).
    pub fn new(pool: SqlitePool, events: EventSender, quality_check_concurrency: usize) -> Self {
        Self {
            pool,
            events,
            quality_check_semaphore: Arc::new(Semaphore::new(quality_check_concurrency.max(1))),
        }
    }
}

impl CurationService for DefaultCurationService {
    #[instrument(skip(self, item_metadata), fields(media_type = %media_type))]
    async fn assess_quality(
        &self,
        media_type: MediaType,
        item_metadata: &QualityMetadata,
    ) -> Result<QualityAssessment, KritikeError> {
        let _permit = self
            .quality_check_semaphore
            .acquire()
            .await
            .map_err(|_| ConcurrencyLimiterClosedSnafu.build())?;
        assessment::assess(&self.pool, media_type, item_metadata).await
    }

    #[instrument(skip(self), fields(have_id = %have_id, candidate_score = candidate_score))]
    async fn check_upgrade_eligibility(
        &self,
        have_id: HaveId,
        candidate_score: i32,
    ) -> Result<UpgradeDecision, KritikeError> {
        let decision =
            upgrade::check_upgrade_eligibility(&self.pool, have_id, candidate_score).await?;

        if decision == UpgradeDecision::Upgrade
            && let Err(e) = self.events.send(HarmoniaEvent::QualityUpgradeTriggered {
                media_id: MediaId::new(),
                current_quality: QualityProfile::new(0),
            })
        {
            tracing::warn!(error = %e, "operation failed");
        }

        Ok(decision)
    }

    #[instrument(skip(self))]
    async fn health_report(&self) -> Result<HealthReport, KritikeError> {
        health::generate(&self.pool).await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use themelion::create_event_bus;

    use super::*;

    // WHY: no schema needed — none of these tests reach a real query
    // (the concurrency-limit tests inspect the semaphore directly; the
    // exhausted-permit test times out before `assessment::assess` runs).
    async fn unmigrated_pool() -> SqlitePool {
        SqlitePool::connect("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn quality_check_semaphore_matches_configured_concurrency() {
        let (events, _rx) = create_event_bus(8);
        let service = DefaultCurationService::new(unmigrated_pool().await, events, 3);
        assert_eq!(service.quality_check_semaphore.available_permits(), 3);
    }

    #[tokio::test]
    async fn zero_quality_check_concurrency_clamps_to_one() {
        let (events, _rx) = create_event_bus(8);
        let service = DefaultCurationService::new(unmigrated_pool().await, events, 0);
        assert_eq!(service.quality_check_semaphore.available_permits(), 1);
    }

    #[tokio::test]
    async fn assess_quality_blocks_when_concurrency_limit_is_exhausted() {
        let (events, _rx) = create_event_bus(8);
        let service = DefaultCurationService::new(unmigrated_pool().await, events, 1);

        // Hold the only permit externally — simulates a quality check already
        // in flight FROM another caller.
        let _held = service
            .quality_check_semaphore
            .clone()
            .try_acquire_owned()
            .unwrap();

        let metadata = QualityMetadata {
            format: "FLAC_24BIT".to_string(),
            custom_format_score: 0,
            profile_id: 1,
            codec: None,
            bit_depth: None,
            sample_rate: None,
            file_size: None,
            channels: None,
        };

        let result = tokio::time::timeout(
            Duration::from_millis(100),
            service.assess_quality(MediaType::Music, &metadata),
        )
        .await;

        assert!(
            result.is_err(),
            "assess_quality must block on the exhausted concurrency semaphore, not bypass it"
        );
    }
}
