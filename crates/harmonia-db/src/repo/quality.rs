use sqlx::SqlitePool;

use crate::error::{DbError, QuerySnafu};
use snafu::ResultExt;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QualityProfile {
    pub id: i64,
    pub name: String,
    pub media_type: String,
    pub min_quality_score: i64,
    pub upgrade_until_score: i64,
    pub min_custom_format_score: i64,
    pub upgrade_until_format_score: i64,
    pub upgrades_allowed: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QualityRankRow {
    pub rank: i64,
    pub format: String,
    pub score: i64,
}

pub async fn insert_profile(pool: &SqlitePool, profile: &QualityProfile) -> Result<i64, DbError> {
    let row = sqlx::query(
        "INSERT INTO quality_profiles
         (name, media_type, min_quality_score, upgrade_until_score,
          min_custom_format_score, upgrade_until_format_score, upgrades_allowed)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&profile.name)
    .bind(&profile.media_type)
    .bind(profile.min_quality_score)
    .bind(profile.upgrade_until_score)
    .bind(profile.min_custom_format_score)
    .bind(profile.upgrade_until_format_score)
    .bind(profile.upgrades_allowed)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "quality_profiles",
    })?;
    Ok(row.last_insert_rowid())
}

pub async fn get_profile(pool: &SqlitePool, id: i64) -> Result<Option<QualityProfile>, DbError> {
    sqlx::query_as::<_, QualityProfile>(
        "SELECT id, name, media_type, min_quality_score, upgrade_until_score,
                min_custom_format_score, upgrade_until_format_score, upgrades_allowed
         FROM quality_profiles WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "quality_profiles",
    })
}

pub async fn list_profiles(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<QualityProfile>, DbError> {
    sqlx::query_as::<_, QualityProfile>(
        "SELECT id, name, media_type, min_quality_score, upgrade_until_score,
                min_custom_format_score, upgrade_until_format_score, upgrades_allowed
         FROM quality_profiles ORDER BY media_type, name LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "quality_profiles",
    })
}

pub async fn list_profiles_for_type(
    pool: &SqlitePool,
    media_type: &str,
) -> Result<Vec<QualityProfile>, DbError> {
    sqlx::query_as::<_, QualityProfile>(
        "SELECT id, name, media_type, min_quality_score, upgrade_until_score,
                min_custom_format_score, upgrade_until_format_score, upgrades_allowed
         FROM quality_profiles WHERE media_type = ? ORDER BY name",
    )
    .bind(media_type)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "quality_profiles",
    })
}

pub async fn update_profile(
    pool: &SqlitePool,
    id: i64,
    min_quality_score: i64,
    upgrade_until_score: i64,
    upgrades_allowed: i64,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE quality_profiles
         SET min_quality_score = ?, upgrade_until_score = ?, upgrades_allowed = ?
         WHERE id = ?",
    )
    .bind(min_quality_score)
    .bind(upgrade_until_score)
    .bind(upgrades_allowed)
    .bind(id)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "quality_profiles",
    })?;
    Ok(())
}

pub async fn delete_profile(pool: &SqlitePool, id: i64) -> Result<(), DbError> {
    sqlx::query("DELETE FROM quality_profiles WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "quality_profiles",
        })?;
    Ok(())
}

/// Look up the score for a given format in the appropriate rank table for the media type.
pub async fn score_for_format(
    pool: &SqlitePool,
    media_type: &str,
    format: &str,
) -> Result<Option<i64>, DbError> {
    let table = rank_table_for(media_type);
    let sql = format!("SELECT score FROM {table} WHERE format = ?");
    let row: Option<(i64,)> = sqlx::query_as(&sql)
        .bind(format)
        .fetch_optional(pool)
        .await
        .context(QuerySnafu { table })?;
    Ok(row.map(|(s,)| s))
}

/// List all ranks for a given media type's rank table.
pub async fn list_ranks(
    pool: &SqlitePool,
    media_type: &str,
) -> Result<Vec<QualityRankRow>, DbError> {
    let table = rank_table_for(media_type);
    let sql = format!("SELECT rank, format, score FROM {table} ORDER BY rank");
    sqlx::query_as::<_, QualityRankRow>(&sql)
        .fetch_all(pool)
        .await
        .context(QuerySnafu { table })
}

fn rank_table_for(media_type: &str) -> &'static str {
    match media_type {
        "music" => "music_quality_ranks",
        "audiobook" => "audiobook_quality_ranks",
        "book" => "book_quality_ranks",
        "comic" => "comic_quality_ranks",
        "podcast" => "podcast_quality_ranks",
        "movie" | "tv" => "video_quality_ranks",
        _ => "music_quality_ranks",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate::MIGRATOR;

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn seed_profiles_exist() {
        let pool = setup().await;
        let music_profiles = list_profiles_for_type(&pool, "music").await.unwrap();
        assert!(!music_profiles.is_empty());
        assert!(music_profiles.iter().any(|p| p.name == "Lossless"));
    }

    #[tokio::test]
    async fn score_lookup_music() {
        let pool = setup().await;
        let score = score_for_format(&pool, "music", "FLAC_24BIT")
            .await
            .unwrap();
        assert_eq!(score, Some(100));

        let score = score_for_format(&pool, "music", "MP3_320_CBR")
            .await
            .unwrap();
        assert_eq!(score, Some(70));
    }

    #[tokio::test]
    async fn score_lookup_video() {
        let pool = setup().await;
        let score = score_for_format(&pool, "movie", "WEBDL_1080P")
            .await
            .unwrap();
        assert_eq!(score, Some(70));

        let score = score_for_format(&pool, "tv", "WEBDL_1080P").await.unwrap();
        assert_eq!(score, Some(70));
    }

    #[tokio::test]
    async fn unknown_format_returns_none() {
        let pool = setup().await;
        let score = score_for_format(&pool, "music", "UNKNOWN_FORMAT")
            .await
            .unwrap();
        assert_eq!(score, None);
    }

    #[tokio::test]
    async fn quality_profile_round_trip() {
        let pool = setup().await;
        let profile = QualityProfile {
            id: 0,
            name: "Custom Lossless".to_string(),
            media_type: "music".to_string(),
            min_quality_score: 85,
            upgrade_until_score: 100,
            min_custom_format_score: 0,
            upgrade_until_format_score: 0,
            upgrades_allowed: 1,
        };
        let id = insert_profile(&pool, &profile).await.unwrap();
        let fetched = get_profile(&pool, id).await.unwrap().unwrap();
        assert_eq!(fetched.name, "Custom Lossless");
        assert_eq!(fetched.min_quality_score, 85);
    }

    #[tokio::test]
    async fn list_ranks_music() {
        let pool = setup().await;
        let ranks = list_ranks(&pool, "music").await.unwrap();
        assert_eq!(ranks.len(), 7);
        assert_eq!(ranks[0].format, "FLAC_24BIT");
        assert_eq!(ranks[0].score, 100);
    }
}
