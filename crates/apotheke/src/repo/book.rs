use sqlx::SqlitePool;

use crate::error::{DbError, QuerySnafu};
use snafu::ResultExt;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Book {
    pub id: Vec<u8>,
    pub registry_id: Option<Vec<u8>>,
    pub title: String,
    pub subtitle: Option<String>,
    pub isbn: Option<String>,
    pub isbn13: Option<String>,
    pub openlibrary_id: Option<String>,
    pub goodreads_id: Option<String>,
    pub publisher: Option<String>,
    pub publish_date: Option<String>,
    pub language: Option<String>,
    pub page_count: Option<i64>,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub file_format: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub quality_score: Option<i64>,
    pub quality_profile_id: Option<i64>,
    pub source_type: String,
    pub added_at: String,
}

pub async fn insert_book(pool: &SqlitePool, book: &Book) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO books
         (id, registry_id, title, subtitle, isbn, isbn13, openlibrary_id, goodreads_id,
          publisher, publish_date, language, page_count, description, file_path,
          file_format, file_size_bytes, quality_score, quality_profile_id,
          source_type, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&book.id)
    .bind(&book.registry_id)
    .bind(&book.title)
    .bind(&book.subtitle)
    .bind(&book.isbn)
    .bind(&book.isbn13)
    .bind(&book.openlibrary_id)
    .bind(&book.goodreads_id)
    .bind(&book.publisher)
    .bind(&book.publish_date)
    .bind(&book.language)
    .bind(book.page_count)
    .bind(&book.description)
    .bind(&book.file_path)
    .bind(&book.file_format)
    .bind(book.file_size_bytes)
    .bind(book.quality_score)
    .bind(book.quality_profile_id)
    .bind(&book.source_type)
    .bind(&book.added_at)
    .execute(pool)
    .await
    .context(QuerySnafu { table: "books" })?;
    Ok(())
}

pub async fn get_book(pool: &SqlitePool, id: &[u8]) -> Result<Option<Book>, DbError> {
    sqlx::query_as::<_, Book>(
        "SELECT id, registry_id, title, subtitle, isbn, isbn13, openlibrary_id, goodreads_id,
                publisher, publish_date, language, page_count, description, file_path,
                file_format, file_size_bytes, quality_score, quality_profile_id,
                source_type, added_at
         FROM books WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu { table: "books" })
}

pub async fn list_books(pool: &SqlitePool, limit: i64, offset: i64) -> Result<Vec<Book>, DbError> {
    sqlx::query_as::<_, Book>(
        "SELECT id, registry_id, title, subtitle, isbn, isbn13, openlibrary_id, goodreads_id,
                publisher, publish_date, language, page_count, description, file_path,
                file_format, file_size_bytes, quality_score, quality_profile_id,
                source_type, added_at
         FROM books ORDER BY title LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu { table: "books" })
}

pub async fn update_book(
    pool: &SqlitePool,
    id: &[u8],
    title: &str,
    quality_score: Option<i64>,
    file_path: Option<&str>,
    file_format: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE books SET title = ?, quality_score = ?, file_path = ?, file_format = ?
         WHERE id = ?",
    )
    .bind(title)
    .bind(quality_score)
    .bind(file_path)
    .bind(file_format)
    .bind(id)
    .execute(pool)
    .await
    .context(QuerySnafu { table: "books" })?;
    Ok(())
}

pub async fn delete_book(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM books WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "books" })?;
    Ok(())
}

pub async fn search_books(
    pool: &SqlitePool,
    query: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<Book>, DbError> {
    let pattern = format!("%{query}%");
    sqlx::query_as::<_, Book>(
        "SELECT id, registry_id, title, subtitle, isbn, isbn13, openlibrary_id, goodreads_id,
                publisher, publish_date, language, page_count, description, file_path,
                file_format, file_size_bytes, quality_score, quality_profile_id,
                source_type, added_at
         FROM books WHERE title LIKE ? OR publisher LIKE ?
         ORDER BY title LIMIT ? OFFSET ?",
    )
    .bind(&pattern)
    .bind(&pattern)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu { table: "books" })
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
    async fn book_round_trip() {
        let pool = setup().await;
        let id = make_id();
        let book = Book {
            id: id.clone(),
            registry_id: None,
            title: "Dune".to_string(),
            subtitle: None,
            isbn: None,
            isbn13: Some("9780441013593".to_string()),
            openlibrary_id: None,
            goodreads_id: None,
            publisher: Some("Ace Books".to_string()),
            publish_date: Some("1990-09-01".to_string()),
            language: Some("en".to_string()),
            page_count: Some(896),
            description: None,
            file_path: None,
            file_format: None,
            file_size_bytes: None,
            quality_score: None,
            quality_profile_id: None,
            source_type: "local".to_string(),
            added_at: "2026-01-01T00:00:00Z".to_string(),
        };
        insert_book(&pool, &book).await.unwrap();
        let fetched = get_book(&pool, &id).await.unwrap().unwrap();
        assert_eq!(fetched.title, "Dune");
        assert_eq!(fetched.isbn13, Some("9780441013593".to_string()));
        assert_eq!(fetched.page_count, Some(896));
    }

    #[tokio::test]
    async fn list_empty_returns_empty() {
        let pool = setup().await;
        let results = list_books(&pool, 10, 0).await.unwrap();
        assert!(results.is_empty());
    }
}
