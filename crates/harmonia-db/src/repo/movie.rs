use sqlx::SqlitePool;

use crate::error::{DbError, QuerySnafu};
use snafu::ResultExt;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Movie {
    pub id: Vec<u8>,
    pub registry_id: Option<Vec<u8>>,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i64>,
    pub tmdb_id: Option<i64>,
    pub imdb_id: Option<String>,
    pub runtime_min: Option<i64>,
    pub overview: Option<String>,
    pub certification: Option<String>,
    pub file_path: Option<String>,
    pub file_format: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub resolution: Option<String>,
    pub codec: Option<String>,
    pub hdr_type: Option<String>,
    pub quality_score: Option<i64>,
    pub quality_profile_id: Option<i64>,
    pub source_type: String,
    pub added_at: String,
}

pub async fn insert_movie(pool: &SqlitePool, movie: &Movie) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO movies
         (id, registry_id, title, original_title, year, tmdb_id, imdb_id,
          runtime_min, overview, certification, file_path, file_format,
          file_size_bytes, resolution, codec, hdr_type, quality_score,
          quality_profile_id, source_type, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&movie.id)
    .bind(&movie.registry_id)
    .bind(&movie.title)
    .bind(&movie.original_title)
    .bind(movie.year)
    .bind(movie.tmdb_id)
    .bind(&movie.imdb_id)
    .bind(movie.runtime_min)
    .bind(&movie.overview)
    .bind(&movie.certification)
    .bind(&movie.file_path)
    .bind(&movie.file_format)
    .bind(movie.file_size_bytes)
    .bind(&movie.resolution)
    .bind(&movie.codec)
    .bind(&movie.hdr_type)
    .bind(movie.quality_score)
    .bind(movie.quality_profile_id)
    .bind(&movie.source_type)
    .bind(&movie.added_at)
    .execute(pool)
    .await
    .context(QuerySnafu { table: "movies" })?;
    Ok(())
}

pub async fn get_movie(pool: &SqlitePool, id: &[u8]) -> Result<Option<Movie>, DbError> {
    sqlx::query_as::<_, Movie>(
        "SELECT id, registry_id, title, original_title, year, tmdb_id, imdb_id,
                runtime_min, overview, certification, file_path, file_format,
                file_size_bytes, resolution, codec, hdr_type, quality_score,
                quality_profile_id, source_type, added_at
         FROM movies WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu { table: "movies" })
}

pub async fn list_movies(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<Movie>, DbError> {
    sqlx::query_as::<_, Movie>(
        "SELECT id, registry_id, title, original_title, year, tmdb_id, imdb_id,
                runtime_min, overview, certification, file_path, file_format,
                file_size_bytes, resolution, codec, hdr_type, quality_score,
                quality_profile_id, source_type, added_at
         FROM movies ORDER BY title LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu { table: "movies" })
}

pub async fn update_movie(
    pool: &SqlitePool,
    id: &[u8],
    title: &str,
    quality_score: Option<i64>,
    file_path: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE movies SET title = ?, quality_score = ?, file_path = ? WHERE id = ?")
        .bind(title)
        .bind(quality_score)
        .bind(file_path)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "movies" })?;
    Ok(())
}

pub async fn delete_movie(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM movies WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "movies" })?;
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

    #[tokio::test]
    async fn movie_round_trip() {
        let pool = setup().await;
        let id = make_id();
        let movie = Movie {
            id: id.clone(),
            registry_id: None,
            title: "Dune: Part Two".to_string(),
            original_title: None,
            year: Some(2024),
            tmdb_id: Some(693134),
            imdb_id: Some("tt15239678".to_string()),
            runtime_min: Some(167),
            overview: Some("Follow the mythic journey of Paul Atreides.".to_string()),
            certification: Some("PG-13".to_string()),
            file_path: None,
            file_format: None,
            file_size_bytes: None,
            resolution: None,
            codec: None,
            hdr_type: None,
            quality_score: None,
            quality_profile_id: None,
            source_type: "local".to_string(),
            added_at: "2026-01-01T00:00:00Z".to_string(),
        };
        insert_movie(&pool, &movie).await.unwrap();
        let fetched = get_movie(&pool, &id).await.unwrap().unwrap();
        assert_eq!(fetched.title, "Dune: Part Two");
        assert_eq!(fetched.tmdb_id, Some(693134));
        assert_eq!(fetched.runtime_min, Some(167));
    }

    #[tokio::test]
    async fn list_empty_returns_empty() {
        let pool = setup().await;
        let results = list_movies(&pool, 10, 0).await.unwrap();
        assert!(results.is_empty());
    }
}
