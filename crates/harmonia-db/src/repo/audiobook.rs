use sqlx::SqlitePool;

use crate::error::{DbError, QuerySnafu};
use snafu::ResultExt;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Audiobook {
    pub id: Vec<u8>,
    pub registry_id: Option<Vec<u8>>,
    pub title: String,
    pub subtitle: Option<String>,
    pub publisher: Option<String>,
    pub isbn: Option<String>,
    pub asin: Option<String>,
    pub duration_ms: Option<i64>,
    pub release_date: Option<String>,
    pub language: Option<String>,
    pub series_name: Option<String>,
    pub series_position: Option<f64>,
    pub file_path: Option<String>,
    pub file_format: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub quality_score: Option<i64>,
    pub quality_profile_id: Option<i64>,
    pub source_type: String,
    pub added_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AudiobookChapter {
    pub id: Vec<u8>,
    pub audiobook_id: Vec<u8>,
    pub position: i64,
    pub title: Option<String>,
    pub start_ms: i64,
    pub end_ms: i64,
}

pub async fn insert_audiobook(pool: &SqlitePool, book: &Audiobook) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO audiobooks
         (id, registry_id, title, subtitle, publisher, isbn, asin, duration_ms,
          release_date, language, series_name, series_position, file_path,
          file_format, file_size_bytes, quality_score, quality_profile_id,
          source_type, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&book.id)
    .bind(&book.registry_id)
    .bind(&book.title)
    .bind(&book.subtitle)
    .bind(&book.publisher)
    .bind(&book.isbn)
    .bind(&book.asin)
    .bind(book.duration_ms)
    .bind(&book.release_date)
    .bind(&book.language)
    .bind(&book.series_name)
    .bind(book.series_position)
    .bind(&book.file_path)
    .bind(&book.file_format)
    .bind(book.file_size_bytes)
    .bind(book.quality_score)
    .bind(book.quality_profile_id)
    .bind(&book.source_type)
    .bind(&book.added_at)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "audiobooks",
    })?;
    Ok(())
}

pub async fn get_audiobook(pool: &SqlitePool, id: &[u8]) -> Result<Option<Audiobook>, DbError> {
    sqlx::query_as::<_, Audiobook>(
        "SELECT id, registry_id, title, subtitle, publisher, isbn, asin, duration_ms,
                release_date, language, series_name, series_position, file_path,
                file_format, file_size_bytes, quality_score, quality_profile_id,
                source_type, added_at
         FROM audiobooks WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "audiobooks",
    })
}

pub async fn list_audiobooks(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<Audiobook>, DbError> {
    sqlx::query_as::<_, Audiobook>(
        "SELECT id, registry_id, title, subtitle, publisher, isbn, asin, duration_ms,
                release_date, language, series_name, series_position, file_path,
                file_format, file_size_bytes, quality_score, quality_profile_id,
                source_type, added_at
         FROM audiobooks ORDER BY title LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "audiobooks",
    })
}

pub async fn update_audiobook(
    pool: &SqlitePool,
    id: &[u8],
    title: &str,
    quality_score: Option<i64>,
    file_path: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE audiobooks SET title = ?, quality_score = ?, file_path = ? WHERE id = ?")
        .bind(title)
        .bind(quality_score)
        .bind(file_path)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "audiobooks",
        })?;
    Ok(())
}

pub async fn delete_audiobook(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM audiobooks WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "audiobooks",
        })?;
    Ok(())
}

pub async fn insert_chapter(pool: &SqlitePool, chapter: &AudiobookChapter) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO audiobook_chapters (id, audiobook_id, position, title, start_ms, end_ms)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&chapter.id)
    .bind(&chapter.audiobook_id)
    .bind(chapter.position)
    .bind(&chapter.title)
    .bind(chapter.start_ms)
    .bind(chapter.end_ms)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "audiobook_chapters",
    })?;
    Ok(())
}

pub async fn get_chapter(
    pool: &SqlitePool,
    id: &[u8],
) -> Result<Option<AudiobookChapter>, DbError> {
    sqlx::query_as::<_, AudiobookChapter>(
        "SELECT id, audiobook_id, position, title, start_ms, end_ms
         FROM audiobook_chapters WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "audiobook_chapters",
    })
}

pub async fn list_chapters(
    pool: &SqlitePool,
    audiobook_id: &[u8],
) -> Result<Vec<AudiobookChapter>, DbError> {
    sqlx::query_as::<_, AudiobookChapter>(
        "SELECT id, audiobook_id, position, title, start_ms, end_ms
         FROM audiobook_chapters WHERE audiobook_id = ? ORDER BY position",
    )
    .bind(audiobook_id)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "audiobook_chapters",
    })
}

pub async fn update_chapter(
    pool: &SqlitePool,
    id: &[u8],
    title: Option<&str>,
    start_ms: i64,
    end_ms: i64,
) -> Result<(), DbError> {
    sqlx::query("UPDATE audiobook_chapters SET title = ?, start_ms = ?, end_ms = ? WHERE id = ?")
        .bind(title)
        .bind(start_ms)
        .bind(end_ms)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "audiobook_chapters",
        })?;
    Ok(())
}

pub async fn delete_chapter(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM audiobook_chapters WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "audiobook_chapters",
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
    async fn audiobook_round_trip() {
        let pool = setup().await;
        let id = make_id();
        let book = Audiobook {
            id: id.clone(),
            registry_id: None,
            title: "Dune".to_string(),
            subtitle: None,
            publisher: Some("Macmillan Audio".to_string()),
            isbn: None,
            asin: Some("B002V1CBBG".to_string()),
            duration_ms: Some(21600000),
            release_date: None,
            language: Some("en".to_string()),
            series_name: Some("Dune Chronicles".to_string()),
            series_position: Some(1.0),
            file_path: None,
            file_format: None,
            file_size_bytes: None,
            quality_score: None,
            quality_profile_id: None,
            source_type: "local".to_string(),
            added_at: now(),
        };
        insert_audiobook(&pool, &book).await.unwrap();
        let fetched = get_audiobook(&pool, &id).await.unwrap().unwrap();
        assert_eq!(fetched.title, "Dune");
        assert_eq!(fetched.asin, Some("B002V1CBBG".to_string()));
        assert_eq!(fetched.series_position, Some(1.0));
    }

    #[tokio::test]
    async fn chapter_round_trip() {
        let pool = setup().await;
        let book_id = make_id();
        let book = Audiobook {
            id: book_id.clone(),
            registry_id: None,
            title: "Test Book".to_string(),
            subtitle: None,
            publisher: None,
            isbn: None,
            asin: None,
            duration_ms: None,
            release_date: None,
            language: None,
            series_name: None,
            series_position: None,
            file_path: None,
            file_format: None,
            file_size_bytes: None,
            quality_score: None,
            quality_profile_id: None,
            source_type: "local".to_string(),
            added_at: now(),
        };
        insert_audiobook(&pool, &book).await.unwrap();

        let ch_id = make_id();
        let chapter = AudiobookChapter {
            id: ch_id.clone(),
            audiobook_id: book_id.clone(),
            position: 1,
            title: Some("Chapter 1".to_string()),
            start_ms: 0,
            end_ms: 120000,
        };
        insert_chapter(&pool, &chapter).await.unwrap();

        let chapters = list_chapters(&pool, &book_id).await.unwrap();
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].title, Some("Chapter 1".to_string()));
    }

    #[tokio::test]
    async fn list_empty_returns_empty() {
        let pool = setup().await;
        let results = list_audiobooks(&pool, 10, 0).await.unwrap();
        assert!(results.is_empty());
    }
}
