use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use sqlx::SqlitePool;
use tracing::instrument;

use crate::error::{DatabaseSnafu, KritikeError};
use crate::profile::load_profile;
use harmonia_common::MediaType;
use harmonia_db::repo::quality;

/// Raw quality metadata for an item being assessed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetadata {
    /// Format identifier matching the rank table (e.g. "FLAC_24BIT", "MP3_128").
    pub format: String,
    /// Additional custom format score to add to the base rank score.
    pub custom_format_score: i32,
    /// Quality profile ID to evaluate against.
    pub profile_id: i64,
    pub codec: Option<String>,
    pub bit_depth: Option<u32>,
    pub sample_rate: Option<u32>,
    pub file_size: Option<u64>,
    pub channels: Option<u32>,
}

/// Result of a quality assessment against a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityAssessment {
    /// Total quality score (rank score + custom format score).
    pub score: i32,
    /// Format string used for the score lookup.
    pub format: String,
    /// True when score >= profile.min_quality_score.
    pub meets_minimum: bool,
    /// True when score >= profile.upgrade_until_score.
    pub meets_ceiling: bool,
}

#[instrument(skip(pool, metadata), fields(media_type = %media_type))]
pub async fn assess(
    pool: &SqlitePool,
    media_type: MediaType,
    metadata: &QualityMetadata,
) -> Result<QualityAssessment, KritikeError> {
    let profile = load_profile(pool, metadata.profile_id).await?;
    let media_type_str = media_type.to_string();

    let rank_score = quality::score_for_format(pool, &media_type_str, &metadata.format)
        .await
        .context(DatabaseSnafu)?
        .unwrap_or(0) as i32;

    let total_score = rank_score + metadata.custom_format_score;

    Ok(QualityAssessment {
        score: total_score,
        format: metadata.format.clone(),
        meets_minimum: total_score >= profile.min_quality_score,
        meets_ceiling: total_score >= profile.upgrade_until_score,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use harmonia_db::migrate::MIGRATOR;
    use sqlx::SqlitePool;

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    async fn music_any_profile_id(pool: &SqlitePool) -> i64 {
        use sqlx::Row;
        sqlx::query("SELECT id FROM quality_profiles WHERE media_type = 'music' AND name = 'Any'")
            .fetch_one(pool)
            .await
            .unwrap()
            .try_get::<i64, _>("id")
            .unwrap()
    }

    async fn music_standard_profile_id(pool: &SqlitePool) -> i64 {
        use sqlx::Row;
        sqlx::query(
            "SELECT id FROM quality_profiles WHERE media_type = 'music' AND name = 'Standard'",
        )
        .fetch_one(pool)
        .await
        .unwrap()
        .try_get::<i64, _>("id")
        .unwrap()
    }

    async fn movie_any_profile_id(pool: &SqlitePool) -> i64 {
        use sqlx::Row;
        sqlx::query("SELECT id FROM quality_profiles WHERE media_type = 'movie' AND name = 'Any'")
            .fetch_one(pool)
            .await
            .unwrap()
            .try_get::<i64, _>("id")
            .unwrap()
    }

    #[tokio::test]
    async fn flac_24bit_scores_100() {
        let pool = setup().await;
        let profile_id = music_any_profile_id(&pool).await;

        let metadata = QualityMetadata {
            format: "FLAC_24BIT".to_string(),
            custom_format_score: 0,
            profile_id,
            codec: None,
            bit_depth: Some(24),
            sample_rate: Some(96000),
            file_size: None,
            channels: None,
        };

        let assessment = assess(&pool, MediaType::Music, &metadata).await.unwrap();
        assert_eq!(assessment.score, 100);
        assert_eq!(assessment.format, "FLAC_24BIT");
        assert!(assessment.meets_minimum);
        assert!(assessment.meets_ceiling);
    }

    #[tokio::test]
    async fn mp3_128_scores_30() {
        let pool = setup().await;
        let profile_id = music_any_profile_id(&pool).await;

        let metadata = QualityMetadata {
            format: "MP3_128".to_string(),
            custom_format_score: 0,
            profile_id,
            codec: None,
            bit_depth: None,
            sample_rate: None,
            file_size: None,
            channels: None,
        };

        let assessment = assess(&pool, MediaType::Music, &metadata).await.unwrap();
        assert_eq!(assessment.score, 30);
    }

    #[tokio::test]
    async fn score_30_fails_standard_profile_min_70() {
        let pool = setup().await;
        let profile_id = music_standard_profile_id(&pool).await;

        let metadata = QualityMetadata {
            format: "MP3_128".to_string(),
            custom_format_score: 0,
            profile_id,
            codec: None,
            bit_depth: None,
            sample_rate: None,
            file_size: None,
            channels: None,
        };

        let assessment = assess(&pool, MediaType::Music, &metadata).await.unwrap();
        assert_eq!(assessment.score, 30);
        assert!(!assessment.meets_minimum);
    }

    #[tokio::test]
    async fn cross_type_isolation_music_vs_movie() {
        let pool = setup().await;
        let music_profile_id = music_any_profile_id(&pool).await;
        let movie_profile_id = movie_any_profile_id(&pool).await;

        let music_meta = QualityMetadata {
            format: "FLAC_24BIT".to_string(),
            custom_format_score: 0,
            profile_id: music_profile_id,
            codec: None,
            bit_depth: None,
            sample_rate: None,
            file_size: None,
            channels: None,
        };

        let movie_meta = QualityMetadata {
            format: "WEBDL_1080P".to_string(),
            custom_format_score: 0,
            profile_id: movie_profile_id,
            codec: None,
            bit_depth: None,
            sample_rate: None,
            file_size: None,
            channels: None,
        };

        let music_assessment = assess(&pool, MediaType::Music, &music_meta).await.unwrap();
        let movie_assessment = assess(&pool, MediaType::Movie, &movie_meta).await.unwrap();

        // Both happen to be 100 and 70 from their own rank tables — these are never mixed
        assert_eq!(music_assessment.score, 100);
        assert_eq!(movie_assessment.score, 70);

        // A music format key doesn't exist in the movie rank table — unknown format scores 0
        let cross_meta = QualityMetadata {
            format: "FLAC_24BIT".to_string(),
            custom_format_score: 0,
            profile_id: movie_profile_id,
            codec: None,
            bit_depth: None,
            sample_rate: None,
            file_size: None,
            channels: None,
        };
        let cross_assessment = assess(&pool, MediaType::Movie, &cross_meta).await.unwrap();
        assert_eq!(cross_assessment.score, 0);
    }
}
