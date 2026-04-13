use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use sqlx::{Row, SqlitePool};
use tracing::instrument;

use crate::error::{DatabaseSnafu, KritikeError};
use themelion::MediaType;
use apotheke::error::QuerySnafu as DbQuerySnafu;
use apotheke::repo::quality;

/// Health metrics for a single media type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeHealthReport {
    pub total: u64,
    /// format string → count of haves at that format's score
    pub quality_distribution: HashMap<String, u64>,
    /// items below profile minimum quality score
    pub below_minimum: u64,
    /// items at or above upgrade ceiling
    pub at_ceiling: u64,
    /// items eligible for upgrade (above min, below ceiling, upgrades allowed)
    pub upgrade_eligible: u64,
}

/// Library-wide health report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub total_items: u64,
    pub per_type: HashMap<MediaType, TypeHealthReport>,
}

#[instrument(skip(pool))]
pub async fn generate(pool: &SqlitePool) -> Result<HealthReport, KritikeError> {
    let total_items: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM haves")
        .fetch_one(pool)
        .await
        .context(DbQuerySnafu { table: "haves" })
        .context(DatabaseSnafu)?;

    // Per-type metrics via JOIN through wants → quality_profiles
    let metrics_rows = sqlx::query(
        "SELECT
             p.media_type,
             COUNT(*) as total,
             SUM(CASE WHEN h.quality_score < p.min_quality_score THEN 1 ELSE 0 END) as below_minimum,
             SUM(CASE WHEN h.quality_score >= p.upgrade_until_score THEN 1 ELSE 0 END) as at_ceiling,
             SUM(CASE WHEN p.upgrades_allowed = 1
                       AND h.quality_score >= p.min_quality_score
                       AND h.quality_score < p.upgrade_until_score THEN 1 ELSE 0 END) as upgrade_eligible
         FROM haves h
         JOIN wants w ON h.want_id = w.id
         JOIN quality_profiles p ON w.quality_profile_id = p.id
         GROUP BY p.media_type",
    )
    .fetch_all(pool)
    .await
    .context(DbQuerySnafu { table: "haves" })
    .context(DatabaseSnafu)?;

    // Score-grouped distribution per type
    let dist_rows = sqlx::query(
        "SELECT p.media_type, h.quality_score, COUNT(*) as cnt
         FROM haves h
         JOIN wants w ON h.want_id = w.id
         JOIN quality_profiles p ON w.quality_profile_id = p.id
         GROUP BY p.media_type, h.quality_score",
    )
    .fetch_all(pool)
    .await
    .context(DbQuerySnafu { table: "haves" })
    .context(DatabaseSnafu)?;

    // Build score → format lookup per media type
    let mut rank_maps: HashMap<String, HashMap<i64, String>> = HashMap::new();

    let mut per_type: HashMap<MediaType, TypeHealthReport> = HashMap::new();

    for row in &metrics_rows {
        let media_type_str: String = row.try_get("media_type").unwrap_or_default();
        let total: i64 = row.try_get("total").unwrap_or(0);
        let below_minimum: i64 = row.try_get("below_minimum").unwrap_or(0);
        let at_ceiling: i64 = row.try_get("at_ceiling").unwrap_or(0);
        let upgrade_eligible: i64 = row.try_get("upgrade_eligible").unwrap_or(0);

        if !rank_maps.contains_key(&media_type_str) {
            let ranks = quality::list_ranks(pool, &media_type_str)
                .await
                .context(DatabaseSnafu)?;
            let map: HashMap<i64, String> =
                ranks.into_iter().map(|r| (r.score, r.format)).collect();
            rank_maps.insert(media_type_str.clone(), map);
        }

        let media_type = parse_media_type(&media_type_str);
        per_type.insert(
            media_type,
            TypeHealthReport {
                total: u64::try_from(total).unwrap_or_default(),
                quality_distribution: HashMap::new(),
                below_minimum: u64::try_from(below_minimum).unwrap_or_default(),
                at_ceiling: u64::try_from(at_ceiling).unwrap_or_default(),
                upgrade_eligible: u64::try_from(upgrade_eligible).unwrap_or_default(),
            },
        );
    }

    // Fill in quality_distribution
    for row in &dist_rows {
        let media_type_str: String = row.try_get("media_type").unwrap_or_default();
        let score: i64 = row.try_get("quality_score").unwrap_or(0);
        let cnt: i64 = row.try_get("cnt").unwrap_or(0);

        let media_type = parse_media_type(&media_type_str);
        if let Some(type_report) = per_type.get_mut(&media_type) {
            let format = rank_maps
                .get(&media_type_str)
                .and_then(|m| m.get(&score))
                .cloned()
                .unwrap_or_else(|| format!("score:{score}"));
            *type_report.quality_distribution.entry(format).or_insert(0) +=
                u64::try_from(cnt).unwrap_or_default();
        }
    }

    Ok(HealthReport {
        total_items: u64::try_from(total_items).unwrap_or_default(),
        per_type,
    })
}

fn parse_media_type(s: &str) -> MediaType {
    match s {
        "music" => MediaType::Music,
        "audiobook" => MediaType::Audiobook,
        "book" => MediaType::Book,
        "comic" => MediaType::Comic,
        "podcast" => MediaType::Podcast,
        "news" => MediaType::News,
        "movie" => MediaType::Movie,
        "tv" => MediaType::Tv,
        _ => MediaType::Music,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apotheke::migrate::MIGRATOR;
    use apotheke::repo::want::{Have, Want, insert_have, insert_want};
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

    async fn insert_test_have_with_score(
        pool: &SqlitePool,
        media_type: &str,
        profile_id: i64,
        quality_score: i64,
    ) {
        let want_id = make_bytes();
        let want_row = Want {
            id: want_id.clone(),
            media_type: media_type.to_string(),
            title: format!("Test {media_type} {quality_score}"),
            registry_id: None,
            quality_profile_id: profile_id,
            status: "fulfilled".to_string(),
            source: None,
            source_ref: None,
            added_at: "2026-01-01T00:00:00Z".to_string(),
            fulfilled_at: None,
        };
        insert_want(pool, &want_row).await.unwrap();

        let have = Have {
            id: make_bytes(),
            want_id,
            release_id: None,
            media_type: media_type.to_string(),
            media_type_id: make_bytes(),
            quality_score,
            file_path: format!("/{media_type}/track_{quality_score}/"),
            file_size_bytes: 100_000_000,
            status: "complete".to_string(),
            imported_at: "2026-01-01T00:00:00Z".to_string(),
            upgraded_from_id: None,
        };
        insert_have(pool, &have).await.unwrap();
    }

    #[tokio::test]
    async fn health_report_empty_library() {
        let pool = setup().await;
        let report = generate(&pool).await.unwrap();
        assert_eq!(report.total_items, 0);
        assert!(report.per_type.is_empty());
    }

    #[tokio::test]
    async fn health_report_correct_distribution_counts() {
        let pool = setup().await;
        // Standard music profile: min=70, ceiling=90, upgrades_allowed=1
        let standard_id = profile_id_for(&pool, "music", "Standard").await;
        // Lossless music profile: min=85, ceiling=100, upgrades_allowed=1
        let lossless_id = profile_id_for(&pool, "music", "Lossless").await;

        // Insert items (wants.media_type must be 'music_album' per schema CHECK):
        // FLAC_16BIT (90)  -  meets Standard ceiling, meets Lossless min
        insert_test_have_with_score(&pool, "music_album", standard_id, 90).await;
        // MP3_320_CBR (70)  -  meets Standard min, below Standard ceiling
        insert_test_have_with_score(&pool, "music_album", standard_id, 70).await;
        // MP3_128 (30)  -  below Standard min
        insert_test_have_with_score(&pool, "music_album", standard_id, 30).await;
        // FLAC_24BIT (100)  -  meets Lossless ceiling
        insert_test_have_with_score(&pool, "music_album", lossless_id, 100).await;

        let report = generate(&pool).await.unwrap();
        assert_eq!(report.total_items, 4);

        let music = report.per_type.get(&MediaType::Music).unwrap();
        assert_eq!(music.total, 4);

        // 1 item below Standard min (score 30 < 70) + Lossless item scores 100 >= 85 so not below
        // Standard: score 30 < 70 → 1 below_minimum
        // Lossless: score 100 >= 85 → 0 below_minimum
        // Total below_minimum = 1
        assert_eq!(music.below_minimum, 1);

        // Standard ceiling=90: score 90 >= 90 → 1 at_ceiling
        // Lossless ceiling=100: score 100 >= 100 → 1 at_ceiling
        assert_eq!(music.at_ceiling, 2);

        // Standard upgrade_eligible: upgrades_allowed=1, min<=score<ceiling
        //   score 70: 70 >= 70 AND 70 < 90 → eligible
        //   score 90: 90 >= 90 AND 90 < 90 → NOT eligible (at ceiling)
        //   score 30: 30 < 70 (below min) → NOT eligible
        // Lossless upgrade_eligible:
        //   score 100: 100 >= 100 AND 100 < 100 → NOT eligible (at ceiling)
        assert_eq!(music.upgrade_eligible, 1);

        // Distribution should include format labels
        assert!(music.quality_distribution.contains_key("FLAC_24BIT"));
        assert!(music.quality_distribution.contains_key("FLAC_16BIT"));
    }
}
