use sqlx::SqlitePool;

use crate::error::{DbError, QuerySnafu};
use snafu::ResultExt;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TvSeries {
    pub id: Vec<u8>,
    pub registry_id: Option<Vec<u8>>,
    pub title: String,
    pub tmdb_id: Option<i64>,
    pub tvdb_id: Option<i64>,
    pub imdb_id: Option<String>,
    pub status: String,
    pub overview: Option<String>,
    pub network: Option<String>,
    pub quality_profile_id: Option<i64>,
    pub added_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TvSeason {
    pub id: Vec<u8>,
    pub series_id: Vec<u8>,
    pub season_number: i64,
    pub title: Option<String>,
    pub episode_count: Option<i64>,
    pub air_date: Option<String>,
    pub overview: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TvEpisode {
    pub id: Vec<u8>,
    pub season_id: Vec<u8>,
    pub episode_number: i64,
    pub title: Option<String>,
    pub air_date: Option<String>,
    pub runtime_min: Option<i64>,
    pub overview: Option<String>,
    pub tmdb_episode_id: Option<i64>,
    pub file_path: Option<String>,
    pub file_format: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub resolution: Option<String>,
    pub codec: Option<String>,
    pub hdr_type: Option<String>,
    pub quality_score: Option<i64>,
    pub source_type: String,
    pub added_at: String,
}

// --- series ---

pub async fn insert_series(pool: &SqlitePool, series: &TvSeries) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO tv_series
         (id, registry_id, title, tmdb_id, tvdb_id, imdb_id, status,
          overview, network, quality_profile_id, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&series.id)
    .bind(&series.registry_id)
    .bind(&series.title)
    .bind(series.tmdb_id)
    .bind(series.tvdb_id)
    .bind(&series.imdb_id)
    .bind(&series.status)
    .bind(&series.overview)
    .bind(&series.network)
    .bind(series.quality_profile_id)
    .bind(&series.added_at)
    .execute(pool)
    .await
    .context(QuerySnafu { table: "tv_series" })?;
    Ok(())
}

pub async fn get_series(pool: &SqlitePool, id: &[u8]) -> Result<Option<TvSeries>, DbError> {
    sqlx::query_as::<_, TvSeries>(
        "SELECT id, registry_id, title, tmdb_id, tvdb_id, imdb_id, status,
                overview, network, quality_profile_id, added_at
         FROM tv_series WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu { table: "tv_series" })
}

pub async fn list_series(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<TvSeries>, DbError> {
    sqlx::query_as::<_, TvSeries>(
        "SELECT id, registry_id, title, tmdb_id, tvdb_id, imdb_id, status,
                overview, network, quality_profile_id, added_at
         FROM tv_series ORDER BY title LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu { table: "tv_series" })
}

pub async fn update_series(
    pool: &SqlitePool,
    id: &[u8],
    title: &str,
    status: &str,
    quality_profile_id: Option<i64>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE tv_series SET title = ?, status = ?, quality_profile_id = ? WHERE id = ?")
        .bind(title)
        .bind(status)
        .bind(quality_profile_id)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "tv_series" })?;
    Ok(())
}

pub async fn delete_series(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM tv_series WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "tv_series" })?;
    Ok(())
}

// --- seasons ---

pub async fn insert_season(pool: &SqlitePool, season: &TvSeason) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO tv_seasons
         (id, series_id, season_number, title, episode_count, air_date, overview)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&season.id)
    .bind(&season.series_id)
    .bind(season.season_number)
    .bind(&season.title)
    .bind(season.episode_count)
    .bind(&season.air_date)
    .bind(&season.overview)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "tv_seasons",
    })?;
    Ok(())
}

pub async fn get_season(pool: &SqlitePool, id: &[u8]) -> Result<Option<TvSeason>, DbError> {
    sqlx::query_as::<_, TvSeason>(
        "SELECT id, series_id, season_number, title, episode_count, air_date, overview
         FROM tv_seasons WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "tv_seasons",
    })
}

pub async fn list_seasons(pool: &SqlitePool, series_id: &[u8]) -> Result<Vec<TvSeason>, DbError> {
    sqlx::query_as::<_, TvSeason>(
        "SELECT id, series_id, season_number, title, episode_count, air_date, overview
         FROM tv_seasons WHERE series_id = ? ORDER BY season_number",
    )
    .bind(series_id)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "tv_seasons",
    })
}

pub async fn update_season(
    pool: &SqlitePool,
    id: &[u8],
    title: Option<&str>,
    episode_count: Option<i64>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE tv_seasons SET title = ?, episode_count = ? WHERE id = ?")
        .bind(title)
        .bind(episode_count)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "tv_seasons",
        })?;
    Ok(())
}

pub async fn delete_season(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM tv_seasons WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "tv_seasons",
        })?;
    Ok(())
}

// --- episodes ---

pub async fn insert_episode(pool: &SqlitePool, ep: &TvEpisode) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO tv_episodes
         (id, season_id, episode_number, title, air_date, runtime_min, overview,
          tmdb_episode_id, file_path, file_format, file_size_bytes, resolution,
          codec, hdr_type, quality_score, source_type, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&ep.id)
    .bind(&ep.season_id)
    .bind(ep.episode_number)
    .bind(&ep.title)
    .bind(&ep.air_date)
    .bind(ep.runtime_min)
    .bind(&ep.overview)
    .bind(ep.tmdb_episode_id)
    .bind(&ep.file_path)
    .bind(&ep.file_format)
    .bind(ep.file_size_bytes)
    .bind(&ep.resolution)
    .bind(&ep.codec)
    .bind(&ep.hdr_type)
    .bind(ep.quality_score)
    .bind(&ep.source_type)
    .bind(&ep.added_at)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "tv_episodes",
    })?;
    Ok(())
}

pub async fn get_episode(pool: &SqlitePool, id: &[u8]) -> Result<Option<TvEpisode>, DbError> {
    sqlx::query_as::<_, TvEpisode>(
        "SELECT id, season_id, episode_number, title, air_date, runtime_min, overview,
                tmdb_episode_id, file_path, file_format, file_size_bytes, resolution,
                codec, hdr_type, quality_score, source_type, added_at
         FROM tv_episodes WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "tv_episodes",
    })
}

pub async fn list_episodes(pool: &SqlitePool, season_id: &[u8]) -> Result<Vec<TvEpisode>, DbError> {
    sqlx::query_as::<_, TvEpisode>(
        "SELECT id, season_id, episode_number, title, air_date, runtime_min, overview,
                tmdb_episode_id, file_path, file_format, file_size_bytes, resolution,
                codec, hdr_type, quality_score, source_type, added_at
         FROM tv_episodes WHERE season_id = ? ORDER BY episode_number",
    )
    .bind(season_id)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "tv_episodes",
    })
}

pub async fn update_episode(
    pool: &SqlitePool,
    id: &[u8],
    title: Option<&str>,
    quality_score: Option<i64>,
    file_path: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE tv_episodes SET title = ?, quality_score = ?, file_path = ? WHERE id = ?")
        .bind(title)
        .bind(quality_score)
        .bind(file_path)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "tv_episodes",
        })?;
    Ok(())
}

pub async fn delete_episode(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM tv_episodes WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "tv_episodes",
        })?;
    Ok(())
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

    fn make_id() -> Vec<u8> {
        uuid::Uuid::now_v7().as_bytes().to_vec()
    }

    fn now() -> String {
        "2026-01-01T00:00:00Z".to_string()
    }

    #[tokio::test]
    async fn series_season_episode_round_trip() {
        let pool = setup().await;

        let series_id = make_id();
        let series = TvSeries {
            id: series_id.clone(),
            registry_id: None,
            title: "Breaking Bad".to_string(),
            tmdb_id: Some(1396),
            tvdb_id: Some(81189),
            imdb_id: Some("tt0903747".to_string()),
            status: "ended".to_string(),
            overview: None,
            network: Some("AMC".to_string()),
            quality_profile_id: None,
            added_at: now(),
        };
        insert_series(&pool, &series).await.unwrap();

        let season_id = make_id();
        let season = TvSeason {
            id: season_id.clone(),
            series_id: series_id.clone(),
            season_number: 1,
            title: Some("Season 1".to_string()),
            episode_count: Some(7),
            air_date: Some("2008-01-20".to_string()),
            overview: None,
        };
        insert_season(&pool, &season).await.unwrap();

        let ep_id = make_id();
        let ep = TvEpisode {
            id: ep_id.clone(),
            season_id: season_id.clone(),
            episode_number: 1,
            title: Some("Pilot".to_string()),
            air_date: Some("2008-01-20".to_string()),
            runtime_min: Some(58),
            overview: None,
            tmdb_episode_id: None,
            file_path: None,
            file_format: None,
            file_size_bytes: None,
            resolution: None,
            codec: None,
            hdr_type: None,
            quality_score: None,
            source_type: "local".to_string(),
            added_at: now(),
        };
        insert_episode(&pool, &ep).await.unwrap();

        let fetched_series = get_series(&pool, &series_id).await.unwrap().unwrap();
        assert_eq!(fetched_series.title, "Breaking Bad");
        assert_eq!(fetched_series.status, "ended");

        let seasons = list_seasons(&pool, &series_id).await.unwrap();
        assert_eq!(seasons.len(), 1);

        let episodes = list_episodes(&pool, &season_id).await.unwrap();
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].title, Some("Pilot".to_string()));
    }

    #[tokio::test]
    async fn list_empty_returns_empty() {
        let pool = setup().await;
        let results = list_series(&pool, 10, 0).await.unwrap();
        assert!(results.is_empty());
    }
}
