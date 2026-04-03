use harmonia_db::DbPools;
use harmonia_db::repo::news;
use snafu::ResultExt;

use crate::error::{DatabaseSnafu, KomideError};

/// Apply retention policy for a news feed after a refresh.
///
/// If `retention_days` > 0, articles published before the cutoff are deleted.
/// If `retention_articles` > 0, the oldest articles beyond the cap are deleted.
///
/// Both limits can be active simultaneously; day-based trimming runs first.
pub async fn apply_retention(
    db: &DbPools,
    feed_id: &[u8],
    retention_days: u64,
    retention_articles: u64,
) -> Result<u64, KomideError> {
    let mut deleted: u64 = 0;

    if retention_days > 0 {
        let cutoff = cutoff_iso8601(retention_days);
        deleted += news::delete_articles_older_than(&db.write, feed_id, &cutoff)
            .await
            .context(DatabaseSnafu)?;
    }

    if retention_articles > 0 {
        deleted +=
            news::delete_articles_exceeding_count(&db.write, feed_id, i64::try_from(retention_articles).unwrap_or_default())
                .await
                .context(DatabaseSnafu)?;
    }

    Ok(deleted)
}

/// Format an ISO 8601 UTC timestamp for the point `days` ago.
fn cutoff_iso8601(days: u64) -> String {
    let now = jiff::Timestamp::now();
    // Timestamp arithmetic only supports units ≤ hours (no calendar days).
    let hours = (days * 24) as i64;
    let cutoff = now - jiff::Span::new().hours(hours);
    cutoff.strftime("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use harmonia_db::{DbPools, migrate::MIGRATOR, repo::news};
    use sqlx::SqlitePool;

    async fn setup() -> DbPools {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        DbPools {
            read: pool.clone(),
            write: pool,
        }
    }

    fn make_id() -> Vec<u8> {
        uuid::Uuid::now_v7().as_bytes().to_vec()
    }

    fn now() -> String {
        "2026-01-15T12:00:00Z".to_string()
    }

    async fn insert_test_feed(db: &DbPools) -> Vec<u8> {
        let id = make_id();
        let feed = news::NewsFeed {
            id: id.clone(),
            title: "Test".to_string(),
            url: "https://example.com/feed.xml".to_string(),
            site_url: None,
            description: None,
            category: None,
            icon_url: None,
            last_fetched_at: None,
            fetch_interval_minutes: 15,
            is_active: 1,
            added_at: now(),
            updated_at: now(),
        };
        news::insert_feed(&db.write, &feed).await.unwrap();
        id
    }

    async fn insert_article(db: &DbPools, feed_id: &[u8], guid: &str, published_at: &str) {
        let article = news::NewsArticle {
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
        news::insert_article(&db.write, &article).await.unwrap();
    }

    #[tokio::test]
    async fn retention_days_removes_old_articles() {
        let db = setup().await;
        let feed_id = insert_test_feed(&db).await;

        // Insert one old article (2025) and one recent (2026)
        insert_article(&db, &feed_id, "old", "2025-01-01T00:00:00Z").await;
        insert_article(&db, &feed_id, "recent", "2026-01-14T00:00:00Z").await;

        // retention_days=7 with "now" = 2026-01-15 → cutoff = 2026-01-08
        // The old article (2025-01-01) should be deleted; recent (2026-01-14) kept.
        // We can't control "now" in the test, so test with a large window that still deletes old.
        let deleted = apply_retention(&db, &feed_id, 365, 0).await.unwrap();
        assert!(deleted > 0, "should have deleted old article");

        let remaining = news::list_articles(&db.read, &feed_id, 10, 0)
            .await
            .unwrap();
        assert!(
            remaining.iter().all(|a| a.guid != "old"),
            "old article should be gone"
        );
    }

    #[tokio::test]
    async fn retention_count_removes_oldest_articles() {
        let db = setup().await;
        let feed_id = insert_test_feed(&db).await;

        insert_article(&db, &feed_id, "a1", "2026-01-01T00:00:00Z").await;
        insert_article(&db, &feed_id, "a2", "2026-01-02T00:00:00Z").await;
        insert_article(&db, &feed_id, "a3", "2026-01-03T00:00:00Z").await;

        // Keep only 2 most recent
        let deleted = apply_retention(&db, &feed_id, 0, 2).await.unwrap();
        assert_eq!(deleted, 1, "should DELETE 1 article");

        let remaining = news::list_articles(&db.read, &feed_id, 10, 0)
            .await
            .unwrap();
        assert_eq!(remaining.len(), 2);
        // a1 (oldest) should be gone
        assert!(!remaining.iter().any(|a| a.guid == "a1"));
    }

    #[tokio::test]
    async fn retention_zero_limits_deletes_nothing() {
        let db = setup().await;
        let feed_id = insert_test_feed(&db).await;

        insert_article(&db, &feed_id, "x1", "2020-01-01T00:00:00Z").await;
        insert_article(&db, &feed_id, "x2", "2021-01-01T00:00:00Z").await;

        let deleted = apply_retention(&db, &feed_id, 0, 0).await.unwrap();
        assert_eq!(deleted, 0);

        let remaining = news::list_articles(&db.read, &feed_id, 10, 0)
            .await
            .unwrap();
        assert_eq!(remaining.len(), 2);
    }

    #[test]
    fn cutoff_iso8601_format() {
        let cutoff = cutoff_iso8601(30);
        // Should be a valid ISO 8601 timestamp with Z suffix
        assert!(cutoff.ends_with('Z'), "cutoff should end with Z: {cutoff}");
        assert!(cutoff.contains('T'), "cutoff should contain T: {cutoff}");
    }
}
