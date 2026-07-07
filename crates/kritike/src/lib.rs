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
use horismos::{KritikeConfig, Section};
use sqlx::SqlitePool;
use themelion::{EventSender, HarmoniaEvent, HaveId, LiveGate, MediaId, MediaType, QualityProfile};
use tracing::instrument;
pub use upgrade::UpgradeDecision;

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
    config: Section<KritikeConfig>,
    /// Live admission gate bounding concurrent `assess_quality` calls to
    /// `KritikeConfig::quality_check_concurrency`. Unlike a fixed
    /// `Semaphore`, the limit is re-read on every admission decision (via
    /// `config.get()`) — #529 step 8, `LiveGate::enter`'s first real
    /// consumer: under quality-check traffic guards drop continuously, so
    /// the live-limit re-read observes a config change on the next
    /// admission.
    quality_check_gate: Arc<LiveGate>,
}

impl DefaultCurationService {
    pub fn new(pool: SqlitePool, events: EventSender, config: Section<KritikeConfig>) -> Self {
        Self {
            pool,
            events,
            config,
            quality_check_gate: Arc::new(LiveGate::new()),
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
        // WHY: `.max(1)` clamps a 0-permit config as defense-in-depth — a
        // zero limit would block every quality check forever, the same hang
        // class `taxis.scan_concurrency` had. Callers are expected to reject
        // `0` upstream (see `horismos` validation); this is the last line of
        // defense.
        let _guard = self
            .quality_check_gate
            .enter(|| self.config.get().quality_check_concurrency.max(1))
            .await;
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
    use std::path::PathBuf;
    use std::time::Duration;

    use horismos::{Config, ConfigManager, ConfigOverrides};
    use themelion::create_event_bus;

    use super::*;

    // WHY: no schema needed — none of these tests reach a real query
    // (the concurrency-limit tests inspect the gate directly; the
    // exhausted-limit test times out before `assessment::assess` runs).
    async fn unmigrated_pool() -> SqlitePool {
        SqlitePool::connect("sqlite::memory:").await.unwrap()
    }

    fn metadata() -> QualityMetadata {
        QualityMetadata {
            format: "FLAC_24BIT".to_string(),
            custom_format_score: 0,
            profile_id: 1,
            codec: None,
            bit_depth: None,
            sample_rate: None,
            file_size: None,
            channels: None,
        }
    }

    fn fixed_config(quality_check_concurrency: usize) -> Section<KritikeConfig> {
        Section::fixed(KritikeConfig {
            quality_check_concurrency,
            ..Default::default()
        })
    }

    #[tokio::test]
    async fn quality_check_gate_admits_up_to_configured_concurrency() {
        let (events, _rx) = create_event_bus(8);
        let service = DefaultCurationService::new(unmigrated_pool().await, events, fixed_config(3));

        let a = service
            .quality_check_gate
            .try_enter(3)
            .expect("first entry admitted");
        let b = service
            .quality_check_gate
            .try_enter(3)
            .expect("second entry admitted");
        let c = service
            .quality_check_gate
            .try_enter(3)
            .expect("third entry admitted");
        assert!(
            service.quality_check_gate.try_enter(3).is_none(),
            "a fourth entry must be refused at concurrency 3"
        );
        drop((a, b, c));
    }

    #[tokio::test]
    async fn zero_quality_check_concurrency_clamps_to_one() {
        let (events, _rx) = create_event_bus(8);
        let service = DefaultCurationService::new(unmigrated_pool().await, events, fixed_config(0));

        // Hold the slot at the CLAMPED limit of 1 — a config of 0 must not
        // leave every quality check blocked forever.
        let _held = service
            .quality_check_gate
            .try_enter(1)
            .expect("clamped limit of 1 admits the first entry");

        let result = tokio::time::timeout(
            Duration::from_millis(100),
            service.assess_quality(MediaType::Music, &metadata()),
        )
        .await;

        assert!(
            result.is_err(),
            "a zero-concurrency config must clamp to 1, not silently admit more"
        );
    }

    #[tokio::test]
    async fn assess_quality_blocks_when_concurrency_limit_is_exhausted() {
        let (events, _rx) = create_event_bus(8);
        let service = DefaultCurationService::new(unmigrated_pool().await, events, fixed_config(1));

        // Hold the only slot externally — simulates a quality check already
        // in flight FROM another caller.
        let _held = service
            .quality_check_gate
            .try_enter(1)
            .expect("first entry admitted at limit 1");

        let result = tokio::time::timeout(
            Duration::from_millis(100),
            service.assess_quality(MediaType::Music, &metadata()),
        )
        .await;

        assert!(
            result.is_err(),
            "assess_quality must block on the exhausted concurrency gate, not bypass it"
        );
    }

    // #529 step 8: a `quality_check_concurrency` change made through a REAL
    // `ConfigManager::replace` must be reflected on the NEXT admission
    // decision — no service rebuild.
    #[tokio::test]
    async fn quality_check_concurrency_change_reflects_on_next_admission() {
        let mut config = Config::default();
        config.exousia.jwt_secret = "test-secret-that-is-long-enough-for-hs256".to_string();
        config.kritike.quality_check_concurrency = 1;
        let (manager, handle) = ConfigManager::new(
            config.clone(),
            PathBuf::from("unused.toml"),
            ConfigOverrides::default(),
        );
        let (events, _rx) = create_event_bus(8);
        let service = DefaultCurationService::new(
            unmigrated_pool().await,
            events,
            handle.section(|c| &c.kritike),
        );

        // Hold the sole slot at the boot-time limit of 1.
        let held = service
            .quality_check_gate
            .try_enter(1)
            .expect("first entry admitted at limit 1");

        let blocked = tokio::time::timeout(
            Duration::from_millis(100),
            service.assess_quality(MediaType::Music, &metadata()),
        )
        .await;
        assert!(
            blocked.is_err(),
            "must block while the sole slot is held at the boot-time limit of 1"
        );

        let mut raised = config.clone();
        raised.kritike.quality_check_concurrency = 2;
        manager
            .replace(raised)
            .expect("replace applies the raised concurrency");

        // `LiveGate::enter` re-reads the limit on its first `try_enter`
        // BEFORE ever awaiting a wake — so a FRESH admission decision at the
        // raised limit succeeds immediately, without needing the held guard
        // to drop first.
        let admitted = tokio::time::timeout(
            Duration::from_millis(200),
            service.assess_quality(MediaType::Music, &metadata()),
        )
        .await;
        assert!(
            admitted.is_ok(),
            "a raised live limit must admit a fresh call without waiting on a guard drop"
        );

        drop(held);
    }
}
