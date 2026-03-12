use sqlx::SqlitePool;

use crate::error::{DbError, QuerySnafu};
use snafu::ResultExt;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Comic {
    pub id: Vec<u8>,
    pub registry_id: Option<Vec<u8>>,
    pub series_name: String,
    pub volume: Option<i64>,
    pub issue_number: Option<f64>,
    pub title: Option<String>,
    pub publisher: Option<String>,
    pub release_date: Option<String>,
    pub page_count: Option<i64>,
    pub summary: Option<String>,
    pub language: Option<String>,
    pub comicinfo_writer: Option<String>,
    pub comicinfo_penciller: Option<String>,
    pub comicinfo_inker: Option<String>,
    pub comicinfo_colorist: Option<String>,
    pub file_path: Option<String>,
    pub file_format: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub quality_score: Option<i64>,
    pub quality_profile_id: Option<i64>,
    pub source_type: String,
    pub added_at: String,
}

pub async fn insert_comic(pool: &SqlitePool, comic: &Comic) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO comics
         (id, registry_id, series_name, volume, issue_number, title, publisher,
          release_date, page_count, summary, language, comicinfo_writer,
          comicinfo_penciller, comicinfo_inker, comicinfo_colorist,
          file_path, file_format, file_size_bytes, quality_score,
          quality_profile_id, source_type, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&comic.id)
    .bind(&comic.registry_id)
    .bind(&comic.series_name)
    .bind(comic.volume)
    .bind(comic.issue_number)
    .bind(&comic.title)
    .bind(&comic.publisher)
    .bind(&comic.release_date)
    .bind(comic.page_count)
    .bind(&comic.summary)
    .bind(&comic.language)
    .bind(&comic.comicinfo_writer)
    .bind(&comic.comicinfo_penciller)
    .bind(&comic.comicinfo_inker)
    .bind(&comic.comicinfo_colorist)
    .bind(&comic.file_path)
    .bind(&comic.file_format)
    .bind(comic.file_size_bytes)
    .bind(comic.quality_score)
    .bind(comic.quality_profile_id)
    .bind(&comic.source_type)
    .bind(&comic.added_at)
    .execute(pool)
    .await
    .context(QuerySnafu { table: "comics" })?;
    Ok(())
}

pub async fn get_comic(pool: &SqlitePool, id: &[u8]) -> Result<Option<Comic>, DbError> {
    sqlx::query_as::<_, Comic>(
        "SELECT id, registry_id, series_name, volume, issue_number, title, publisher,
                release_date, page_count, summary, language, comicinfo_writer,
                comicinfo_penciller, comicinfo_inker, comicinfo_colorist,
                file_path, file_format, file_size_bytes, quality_score,
                quality_profile_id, source_type, added_at
         FROM comics WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu { table: "comics" })
}

pub async fn list_comics(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<Comic>, DbError> {
    sqlx::query_as::<_, Comic>(
        "SELECT id, registry_id, series_name, volume, issue_number, title, publisher,
                release_date, page_count, summary, language, comicinfo_writer,
                comicinfo_penciller, comicinfo_inker, comicinfo_colorist,
                file_path, file_format, file_size_bytes, quality_score,
                quality_profile_id, source_type, added_at
         FROM comics ORDER BY series_name, volume, issue_number LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu { table: "comics" })
}

pub async fn update_comic(
    pool: &SqlitePool,
    id: &[u8],
    title: Option<&str>,
    quality_score: Option<i64>,
    file_path: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE comics SET title = ?, quality_score = ?, file_path = ? WHERE id = ?")
        .bind(title)
        .bind(quality_score)
        .bind(file_path)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "comics" })?;
    Ok(())
}

pub async fn delete_comic(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM comics WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "comics" })?;
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
    async fn comic_round_trip() {
        let pool = setup().await;
        let id = make_id();
        let comic = Comic {
            id: id.clone(),
            registry_id: None,
            series_name: "Saga".to_string(),
            volume: Some(1),
            issue_number: Some(1.0),
            title: Some("Chapter One".to_string()),
            publisher: Some("Image Comics".to_string()),
            release_date: Some("2012-03-14".to_string()),
            page_count: Some(44),
            summary: None,
            language: Some("en".to_string()),
            comicinfo_writer: Some("Brian K. Vaughan".to_string()),
            comicinfo_penciller: None,
            comicinfo_inker: None,
            comicinfo_colorist: None,
            file_path: None,
            file_format: None,
            file_size_bytes: None,
            quality_score: None,
            quality_profile_id: None,
            source_type: "local".to_string(),
            added_at: "2026-01-01T00:00:00Z".to_string(),
        };
        insert_comic(&pool, &comic).await.unwrap();
        let fetched = get_comic(&pool, &id).await.unwrap().unwrap();
        assert_eq!(fetched.series_name, "Saga");
        assert_eq!(fetched.issue_number, Some(1.0));
        assert_eq!(
            fetched.comicinfo_writer,
            Some("Brian K. Vaughan".to_string())
        );
    }

    #[tokio::test]
    async fn list_empty_returns_empty() {
        let pool = setup().await;
        let results = list_comics(&pool, 10, 0).await.unwrap();
        assert!(results.is_empty());
    }
}
