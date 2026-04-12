use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use sqlx::SqlitePool;
use tracing::instrument;

use crate::error::{DatabaseSnafu, KritikeError, ProfileNotFoundSnafu};
use harmonia_common::HaveId;
use harmonia_db::repo::{quality, want};

/// Decision about whether to upgrade an existing have.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum UpgradeDecision {
    /// Candidate is not better than current have, or upgrades are disabled.
    Skip,
    /// Candidate is better and current have is below the upgrade ceiling.
    Upgrade,
    /// Current have is already at or above the upgrade ceiling.
    AtCeiling,
}

#[instrument(skip(pool), fields(have_id = %have_id, candidate_score = candidate_score))]
pub async fn check_upgrade_eligibility(
    pool: &SqlitePool,
    have_id: HaveId,
    candidate_score: i32,
) -> Result<UpgradeDecision, KritikeError> {
    let have_bytes = have_id.as_bytes().to_vec();
    let have = want::get_have(pool, &have_bytes)
        .await
        .context(DatabaseSnafu)?;

    let Some(have) = have else {
        tracing::warn!(%have_id, "have not found for upgrade eligibility check");
        return Ok(UpgradeDecision::Skip);
    };

    let want_row = want::get_want(pool, &have.want_id)
        .await
        .context(DatabaseSnafu)?;

    let Some(want_row) = want_row else {
        tracing::warn!(%have_id, "want not found for upgrade eligibility check");
        return Ok(UpgradeDecision::Skip);
    };

    let profile = quality::get_profile(pool, want_row.quality_profile_id)
        .await
        .context(DatabaseSnafu)?
        .ok_or_else(|| {
            ProfileNotFoundSnafu {
                id: want_row.quality_profile_id,
            }
            .build()
        })?;

    let current_score = i32::try_from(have.quality_score).unwrap_or_default();
    let upgrade_until = i32::try_from(profile.upgrade_until_score).unwrap_or_default();
    let upgrades_allowed = profile.upgrades_allowed != 0;

    if !upgrades_allowed {
        return Ok(UpgradeDecision::Skip);
    }

    if current_score >= upgrade_until {
        return Ok(UpgradeDecision::AtCeiling);
    }

    if candidate_score <= current_score {
        return Ok(UpgradeDecision::Skip);
    }

    Ok(UpgradeDecision::Upgrade)
}

#[cfg(test)]
mod tests {
    use super::*;
    use harmonia_db::migrate::MIGRATOR;
    use harmonia_db::repo::want::{Have, Want, insert_have, insert_want};
    use sqlx::SqlitePool;

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    fn make_bytes() -> Vec<u8> {
        uuid::Uuid::now_v7().as_bytes().to_vec()
    }

    async fn profile_id_for(pool: &SqlitePool, media_type: &str, name: &str) -> i64 {
        use sqlx::Row;
        sqlx::query("SELECT id FROM quality_profiles WHERE media_type = ? AND name = ? LIMIT 1")
            .bind(media_type)
            .bind(name)
            .fetch_one(pool)
            .await
            .unwrap()
            .try_get::<i64, _>("id")
            .unwrap()
    }

    async fn insert_test_have(pool: &SqlitePool, profile_name: &str, quality_score: i64) -> HaveId {
        let profile_id = profile_id_for(pool, "music", profile_name).await;
        let want_id = make_bytes();
        let want_row = Want {
            id: want_id.clone(),
            media_type: "music_album".to_string(),
            title: "Test Album".to_string(),
            registry_id: None,
            quality_profile_id: profile_id,
            status: "fulfilled".to_string(),
            source: None,
            source_ref: None,
            added_at: "2026-01-01T00:00:00Z".to_string(),
            fulfilled_at: None,
        };
        insert_want(pool, &want_row).await.unwrap();

        let have_uuid = uuid::Uuid::now_v7();
        let have_id_bytes = have_uuid.as_bytes().to_vec();
        let have = Have {
            id: have_id_bytes,
            want_id,
            release_id: None,
            media_type: "music".to_string(),
            media_type_id: make_bytes(),
            quality_score,
            file_path: "/music/test/".to_string(),
            file_size_bytes: 100_000_000,
            status: "complete".to_string(),
            imported_at: "2026-01-01T00:00:00Z".to_string(),
            upgraded_from_id: None,
        };
        insert_have(pool, &have).await.unwrap();

        HaveId::from_uuid(have_uuid)
    }

    #[tokio::test]
    async fn upgrade_when_candidate_better_and_below_ceiling() {
        let pool = setup().await;
        // Standard profile: min=70, ceiling=90, upgrades_allowed=1
        let have_id = insert_test_have(&pool, "Standard", 70).await;
        let decision = check_upgrade_eligibility(&pool, have_id, 90).await.unwrap();
        assert_eq!(decision, UpgradeDecision::Upgrade);
    }

    #[tokio::test]
    async fn at_ceiling_when_have_meets_upgrade_until() {
        let pool = setup().await;
        // Standard profile: ceiling=90
        let have_id = insert_test_have(&pool, "Standard", 90).await;
        let decision = check_upgrade_eligibility(&pool, have_id, 100)
            .await
            .unwrap();
        assert_eq!(decision, UpgradeDecision::AtCeiling);
    }

    #[tokio::test]
    async fn skip_when_upgrades_disabled() {
        let pool = setup().await;
        // Insert a profile with upgrades_allowed = 0
        let profile_id: i64 = sqlx::query_scalar(
            "INSERT INTO quality_profiles
             (name, media_type, min_quality_score, upgrade_until_score,
              min_custom_format_score, upgrade_until_format_score, upgrades_allowed)
             VALUES ('NoUpgrades', 'music', 1, 100, 0, 0, 0)
             RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let want_id = make_bytes();
        let want_row = Want {
            id: want_id.clone(),
            media_type: "music_album".to_string(),
            title: "No Upgrade Album".to_string(),
            registry_id: None,
            quality_profile_id: profile_id,
            status: "fulfilled".to_string(),
            source: None,
            source_ref: None,
            added_at: "2026-01-01T00:00:00Z".to_string(),
            fulfilled_at: None,
        };
        insert_want(&pool, &want_row).await.unwrap();

        let have_uuid = uuid::Uuid::now_v7();
        let have = Have {
            id: have_uuid.as_bytes().to_vec(),
            want_id,
            release_id: None,
            media_type: "music".to_string(),
            media_type_id: make_bytes(),
            quality_score: 70,
            file_path: "/music/noupgrade/".to_string(),
            file_size_bytes: 100_000_000,
            status: "complete".to_string(),
            imported_at: "2026-01-01T00:00:00Z".to_string(),
            upgraded_from_id: None,
        };
        insert_have(&pool, &have).await.unwrap();

        let have_id = HaveId::from_uuid(have_uuid);
        let decision = check_upgrade_eligibility(&pool, have_id, 100)
            .await
            .unwrap();
        assert_eq!(decision, UpgradeDecision::Skip);
    }

    #[tokio::test]
    async fn skip_when_candidate_not_better() {
        let pool = setup().await;
        // Standard profile: min=70, ceiling=90
        let have_id = insert_test_have(&pool, "Standard", 85).await;
        // Candidate same score as have
        let decision = check_upgrade_eligibility(&pool, have_id, 85).await.unwrap();
        assert_eq!(decision, UpgradeDecision::Skip);
    }
}
