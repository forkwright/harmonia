use snafu::ResultExt;
use sqlx::SqlitePool;

use crate::error::{DbError, QuerySnafu};

// WHY: wire DTO — SQLx row from the news_feeds table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NewsFeed {
    pub id: Vec<u8>,
    pub title: String,
    pub url: String,
    pub site_url: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub icon_url: Option<String>,
    pub last_fetched_at: Option<String>,
    pub fetch_interval_minutes: i64,
    pub is_active: i64,
    pub added_at: String,
    pub updated_at: String,
}

// WHY: wire DTO — SQLx row from the news_articles table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NewsArticle {
    pub id: Vec<u8>,
    pub feed_id: Vec<u8>,
    pub guid: String,
    pub title: String,
    pub url: String,
    pub author: Option<String>,
    pub content_html: Option<String>,
    pub summary: Option<String>,
    pub published_at: Option<String>,
    pub is_read: i64,
    pub is_starred: i64,
    pub source_type: String,
    pub added_at: String,
}

pub async fn insert_feed(pool: &SqlitePool, feed: &NewsFeed) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO news_feeds
         (id, title, url, site_url, description, category, icon_url,
          last_fetched_at, fetch_interval_minutes, is_active, added_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&feed.id)
    .bind(&feed.title)
    .bind(&feed.url)
    .bind(&feed.site_url)
    .bind(&feed.description)
    .bind(&feed.category)
    .bind(&feed.icon_url)
    .bind(&feed.last_fetched_at)
    .bind(feed.fetch_interval_minutes)
    .bind(feed.is_active)
    .bind(&feed.added_at)
    .bind(&feed.updated_at)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "news_feeds",
    })?;
    Ok(())
}

pub async fn get_feed(pool: &SqlitePool, id: &[u8]) -> Result<Option<NewsFeed>, DbError> {
    sqlx::query_as::<_, NewsFeed>(
        "SELECT id, title, url, site_url, description, category, icon_url,
                last_fetched_at, fetch_interval_minutes, is_active, added_at, updated_at
         FROM news_feeds WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "news_feeds",
    })
}

pub async fn list_feeds(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<NewsFeed>, DbError> {
    sqlx::query_as::<_, NewsFeed>(
        "SELECT id, title, url, site_url, description, category, icon_url,
                last_fetched_at, fetch_interval_minutes, is_active, added_at, updated_at
         FROM news_feeds ORDER BY title LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "news_feeds",
    })
}

pub async fn update_feed(
    pool: &SqlitePool,
    id: &[u8],
    title: &str,
    is_active: i64,
    last_fetched_at: Option<&str>,
    updated_at: &str,
) -> Result<(), DbError> {
    let result = sqlx::query(
        "UPDATE news_feeds SET title = ?, is_active = ?, last_fetched_at = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(title)
    .bind(is_active)
    .bind(last_fetched_at)
    .bind(updated_at)
    .bind(id)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "news_feeds",
    })?;
    super::require_affected(result, "news_feeds", super::id_hex(id))
}

pub async fn delete_feed(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    let result = sqlx::query("DELETE FROM news_feeds WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "news_feeds",
        })?;
    super::require_affected(result, "news_feeds", super::id_hex(id))
}

pub async fn insert_article(pool: &SqlitePool, article: &NewsArticle) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO news_articles
         (id, feed_id, guid, title, url, author, content_html, summary,
          published_at, is_read, is_starred, source_type, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&article.id)
    .bind(&article.feed_id)
    .bind(&article.guid)
    .bind(&article.title)
    .bind(&article.url)
    .bind(&article.author)
    .bind(&article.content_html)
    .bind(&article.summary)
    .bind(&article.published_at)
    .bind(article.is_read)
    .bind(article.is_starred)
    .bind(&article.source_type)
    .bind(&article.added_at)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "news_articles",
    })?;
    Ok(())
}

pub async fn get_article(pool: &SqlitePool, id: &[u8]) -> Result<Option<NewsArticle>, DbError> {
    sqlx::query_as::<_, NewsArticle>(
        "SELECT id, feed_id, guid, title, url, author, content_html, summary,
                published_at, is_read, is_starred, source_type, added_at
         FROM news_articles WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "news_articles",
    })
}

pub async fn list_articles(
    pool: &SqlitePool,
    feed_id: &[u8],
    limit: i64,
    offset: i64,
) -> Result<Vec<NewsArticle>, DbError> {
    sqlx::query_as::<_, NewsArticle>(
        "SELECT id, feed_id, guid, title, url, author, content_html, summary,
                published_at, is_read, is_starred, source_type, added_at
         FROM news_articles WHERE feed_id = ?
         ORDER BY published_at DESC LIMIT ? OFFSET ?",
    )
    .bind(feed_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "news_articles",
    })
}

pub async fn update_article(
    pool: &SqlitePool,
    id: &[u8],
    is_read: i64,
    is_starred: i64,
) -> Result<(), DbError> {
    let result = sqlx::query("UPDATE news_articles SET is_read = ?, is_starred = ? WHERE id = ?")
        .bind(is_read)
        .bind(is_starred)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "news_articles",
        })?;
    super::require_affected(result, "news_articles", super::id_hex(id))
}

pub async fn delete_article(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    let result = sqlx::query("DELETE FROM news_articles WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "news_articles",
        })?;
    super::require_affected(result, "news_articles", super::id_hex(id))
}

pub async fn article_guid_exists(
    pool: &SqlitePool,
    feed_id: &[u8],
    guid: &str,
) -> Result<bool, DbError> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM news_articles WHERE feed_id = ? AND guid = ?")
            .bind(feed_id)
            .bind(guid)
            .fetch_one(pool)
            .await
            .context(QuerySnafu {
                table: "news_articles",
            })?;
    Ok(count > 0)
}

pub async fn count_articles_for_feed(pool: &SqlitePool, feed_id: &[u8]) -> Result<i64, DbError> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM news_articles WHERE feed_id = ?")
        .bind(feed_id)
        .fetch_one(pool)
        .await
        .context(QuerySnafu {
            table: "news_articles",
        })?;
    Ok(count)
}

/// Delete articles published before `cutoff_iso8601`. Returns deleted row count.
pub async fn delete_articles_older_than(
    pool: &SqlitePool,
    feed_id: &[u8],
    cutoff_iso8601: &str,
) -> Result<u64, DbError> {
    let result = sqlx::query("DELETE FROM news_articles WHERE feed_id = ? AND published_at < ?")
        .bind(feed_id)
        .bind(cutoff_iso8601)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "news_articles",
        })?;
    Ok(result.rows_affected())
}

/// Delete oldest articles so that at most `keep_count` remain for the feed.
/// Articles are ordered by published_at DESC; oldest are removed first.
/// Returns deleted row count.
pub async fn delete_articles_exceeding_count(
    pool: &SqlitePool,
    feed_id: &[u8],
    keep_count: i64,
) -> Result<u64, DbError> {
    let result = sqlx::query(
        "DELETE FROM news_articles
         WHERE feed_id = ?
           AND id NOT IN (
               SELECT id FROM news_articles
               WHERE feed_id = ?
               ORDER BY published_at DESC
               LIMIT ?
           )",
    )
    .bind(feed_id)
    .bind(feed_id)
    .bind(keep_count)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "news_articles",
    })?;
    Ok(result.rows_affected())
}

pub async fn feed_by_url(pool: &SqlitePool, url: &str) -> Result<Option<NewsFeed>, DbError> {
    sqlx::query_as::<_, NewsFeed>(
        "SELECT id, title, url, site_url, description, category, icon_url,
                last_fetched_at, fetch_interval_minutes, is_active, added_at, updated_at
         FROM news_feeds WHERE url = ?",
    )
    .bind(url)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "news_feeds",
    })
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
    async fn feed_round_trip() {
        let pool = setup().await;
        let id = make_id();
        let feed = NewsFeed {
            id: id.clone(),
            title: "Ars Technica".to_string(),
            url: "https://feeds.arstechnica.com/arstechnica/index".to_string(),
            site_url: Some("https://arstechnica.com".to_string()),
            description: None,
            category: Some("tech".to_string()),
            icon_url: None,
            last_fetched_at: None,
            fetch_interval_minutes: 30,
            is_active: 1,
            added_at: now(),
            updated_at: now(),
        };
        insert_feed(&pool, &feed).await.unwrap();
        let fetched = get_feed(&pool, &id).await.unwrap().unwrap();
        assert_eq!(fetched.title, "Ars Technica");
        assert_eq!(fetched.fetch_interval_minutes, 30);
    }

    #[tokio::test]
    async fn article_round_trip() {
        let pool = setup().await;
        let feed_id = make_id();
        let feed = NewsFeed {
            id: feed_id.clone(),
            title: "Test Feed".to_string(),
            url: "https://example.com/feed2.xml".to_string(),
            site_url: None,
            description: None,
            category: None,
            icon_url: None,
            last_fetched_at: None,
            fetch_interval_minutes: 60,
            is_active: 1,
            added_at: now(),
            updated_at: now(),
        };
        insert_feed(&pool, &feed).await.unwrap();

        let art_id = make_id();
        let article = NewsArticle {
            id: art_id.clone(),
            feed_id: feed_id.clone(),
            guid: "article-001".to_string(),
            title: "Test Article".to_string(),
            url: "https://example.com/article1".to_string(),
            author: Some("Author Name".to_string()),
            content_html: None,
            summary: Some("A short summary".to_string()),
            published_at: Some("2026-01-01T00:00:00Z".to_string()),
            is_read: 0,
            is_starred: 0,
            source_type: "rss".to_string(),
            added_at: now(),
        };
        insert_article(&pool, &article).await.unwrap();

        let articles = list_articles(&pool, &feed_id, 10, 0).await.unwrap();
        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].title, "Test Article");
    }

    #[tokio::test]
    async fn list_empty_returns_empty() {
        let pool = setup().await;
        let results = list_feeds(&pool, 10, 0).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn update_feed_nonexistent_returns_not_found() {
        let pool = setup().await;
        let err = update_feed(&pool, &make_id(), "Ghost", 1, None, &now())
            .await
            .unwrap_err();
        assert!(matches!(err, DbError::NotFound { .. }));
    }

    #[tokio::test]
    async fn delete_article_nonexistent_returns_not_found() {
        let pool = setup().await;
        let err = delete_article(&pool, &make_id()).await.unwrap_err();
        assert!(matches!(err, DbError::NotFound { .. }));
    }

    async fn seed_feed(pool: &SqlitePool, url: &str) -> Vec<u8> {
        let id = make_id();
        let feed = NewsFeed {
            id: id.clone(),
            title: "Seed Feed".to_string(),
            url: url.to_string(),
            site_url: None,
            description: None,
            category: None,
            icon_url: None,
            last_fetched_at: None,
            fetch_interval_minutes: 60,
            is_active: 1,
            added_at: now(),
            updated_at: now(),
        };
        insert_feed(pool, &feed).await.unwrap();
        id
    }

    async fn seed_article(pool: &SqlitePool, feed_id: &[u8], guid: &str, published_at: &str) {
        let article = NewsArticle {
            id: make_id(),
            feed_id: feed_id.to_vec(),
            guid: guid.to_string(),
            title: format!("Article {guid}"),
            url: format!("https://example.com/{guid}"),
            author: None,
            content_html: None,
            summary: None,
            published_at: Some(published_at.to_string()),
            is_read: 0,
            is_starred: 0,
            source_type: "rss".to_string(),
            added_at: now(),
        };
        insert_article(pool, &article).await.unwrap();
    }

    // -- article_guid_exists dedup guard --

    #[tokio::test]
    async fn article_guid_exists_true_after_insert() {
        let pool = setup().await;
        let feed_id = seed_feed(&pool, "https://example.com/a.xml").await;
        seed_article(&pool, &feed_id, "guid-001", "2026-01-01T00:00:00Z").await;

        assert!(
            article_guid_exists(&pool, &feed_id, "guid-001")
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn article_guid_exists_false_for_unseen_guid() {
        let pool = setup().await;
        let feed_id = seed_feed(&pool, "https://example.com/b.xml").await;
        seed_article(&pool, &feed_id, "guid-001", "2026-01-01T00:00:00Z").await;

        assert!(
            !article_guid_exists(&pool, &feed_id, "guid-999")
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn article_guid_exists_false_for_same_guid_different_feed() {
        let pool = setup().await;
        let feed_a = seed_feed(&pool, "https://example.com/c.xml").await;
        let feed_b = seed_feed(&pool, "https://example.com/d.xml").await;
        seed_article(&pool, &feed_a, "shared-guid", "2026-01-01T00:00:00Z").await;

        assert!(
            !article_guid_exists(&pool, &feed_b, "shared-guid")
                .await
                .unwrap(),
            "guid dedup must be scoped per feed"
        );
    }

    // -- delete_articles_exceeding_count retention --

    #[tokio::test]
    async fn delete_articles_exceeding_count_removes_oldest() {
        let pool = setup().await;
        let feed_id = seed_feed(&pool, "https://example.com/e.xml").await;
        for (guid, published) in [
            ("old-1", "2026-01-01T00:00:00Z"),
            ("old-2", "2026-01-02T00:00:00Z"),
            ("new-1", "2026-01-03T00:00:00Z"),
            ("new-2", "2026-01-04T00:00:00Z"),
        ] {
            seed_article(&pool, &feed_id, guid, published).await;
        }

        let deleted = delete_articles_exceeding_count(&pool, &feed_id, 2)
            .await
            .unwrap();
        assert_eq!(deleted, 2);

        for (guid, expected) in [
            ("old-1", false),
            ("old-2", false),
            ("new-1", true),
            ("new-2", true),
        ] {
            assert_eq!(
                article_guid_exists(&pool, &feed_id, guid).await.unwrap(),
                expected,
                "retention must keep the newest keep_count articles ({guid})"
            );
        }
    }

    #[tokio::test]
    async fn delete_articles_exceeding_count_noop_when_under_limit() {
        let pool = setup().await;
        let feed_id = seed_feed(&pool, "https://example.com/f.xml").await;
        seed_article(&pool, &feed_id, "only-1", "2026-01-01T00:00:00Z").await;
        seed_article(&pool, &feed_id, "only-2", "2026-01-02T00:00:00Z").await;

        let deleted = delete_articles_exceeding_count(&pool, &feed_id, 5)
            .await
            .unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(count_articles_for_feed(&pool, &feed_id).await.unwrap(), 2);
    }
}
