pub mod assessment;
pub mod error;
pub mod health;
pub mod profile;
pub mod upgrade;

pub use assessment::{QualityAssessment, QualityMetadata};
pub use error::KritikeError;
pub use health::{HealthReport, TypeHealthReport};
pub use upgrade::UpgradeDecision;

use harmonia_common::{EventSender, HarmoniaEvent, HaveId, MediaId, MediaType, QualityProfile};
use sqlx::SqlitePool;
use tracing::instrument;

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

    /// Register an imported item for quality tracking.
    async fn register_import(
        &self,
        media_id: MediaId,
        media_type: MediaType,
        quality_score: i32,
    ) -> Result<(), KritikeError>;

    /// Generate library health report.
    async fn health_report(&self) -> Result<HealthReport, KritikeError>;
}

pub struct DefaultCurationService {
    pool: SqlitePool,
    events: EventSender,
}

impl DefaultCurationService {
    pub fn new(pool: SqlitePool, events: EventSender) -> Self {
        Self { pool, events }
    }
}

impl CurationService for DefaultCurationService {
    #[instrument(skip(self, item_metadata), fields(media_type = %media_type))]
    async fn assess_quality(
        &self,
        media_type: MediaType,
        item_metadata: &QualityMetadata,
    ) -> Result<QualityAssessment, KritikeError> {
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

        if decision == UpgradeDecision::Upgrade {
            self.events
                .send(HarmoniaEvent::QualityUpgradeTriggered {
                    media_id: MediaId::new(),
                    current_quality: QualityProfile::new(0),
                })
               if let Err(e) =   { tracing::warn!(error = %e, "operation failed"); }
        }

        Ok(decision)
    }

    #[instrument(skip(self), fields(media_id = %media_id, media_type = %media_type, quality_score = quality_score))]
    async fn register_import(
        &self,
        media_id: MediaId,
        media_type: MediaType,
        quality_score: i32,
    ) -> Result<(), KritikeError> {
        tracing::info!(%media_id, %media_type, quality_score, "import registered for quality tracking");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn health_report(&self) -> Result<HealthReport, KritikeError> {
        health::generate(&self.pool).await
    }
}
