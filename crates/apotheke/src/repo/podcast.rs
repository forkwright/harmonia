use snafu::ResultExt;
use sqlx::SqlitePool;

use crate::error::{DbError, QuerySnafu};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PodcastSubscription {
    pub id: Vec<u8>,
    pub feed_url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub image_url: Option<String>,
    pub language: Option<String>,
    pub last_checked_at: Option<String>,
    pub auto_download: i64,
    pub quality_profile_id: Option<i64>,
    pub added_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PodcastEpisode {
    pub id: Vec<u8>,
    pub subscription_id: Vec<u8>,
    pub guid: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub episode_number: Option<i64>,
    pub season_number: Option<i64>,
    pub publication_date: Option<String>,
    pub duration_ms: Option<i64>,
    pub enclosure_url: Option<String>,
    pub file_path: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub file_format: Option<String>,
    pub quality_score: Option<i64>,
    pub source_type: String,
    pub listened: i64,
    pub added_at: String,
}

pub async fn insert_subscription(
    pool: &SqlitePool,
    sub: &PodcastSubscription,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO podcast_subscriptions
         (id, feed_url, title, description, author, image_url, language,
          last_checked_at, auto_download, quality_profile_id, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&sub.id)
    .bind(&sub.feed_url)
    .bind(&sub.title)
    .bind(&sub.description)
    .bind(&sub.author)
    .bind(&sub.image_url)
    .bind(&sub.language)
    .bind(&sub.last_checked_at)
    .bind(sub.auto_download)
    .bind(sub.quality_profile_id)
    .bind(&sub.added_at)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "podcast_subscriptions",
    })?;
    Ok(())
}

pub async fn get_subscription(
    pool: &SqlitePool,
    id: &[u8],
) -> Result<Option<PodcastSubscription>, DbError> {
    sqlx::query_as::<_, PodcastSubscription>(
        "SELECT id, feed_url, title, description, author, image_url, language,
                last_checked_at, auto_download, quality_profile_id, added_at
         FROM podcast_subscriptions WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "podcast_subscriptions",
    })
}

pub async fn list_subscriptions(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<PodcastSubscription>, DbError> {
    sqlx::query_as::<_, PodcastSubscription>(
        "SELECT id, feed_url, title, description, author, image_url, language,
                last_checked_at, auto_download, quality_profile_id, added_at
         FROM podcast_subscriptions ORDER BY title LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "podcast_subscriptions",
    })
}

pub async fn update_subscription(
    pool: &SqlitePool,
    id: &[u8],
    title: Option<&str>,
    auto_download: i64,
    last_checked_at: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE podcast_subscriptions SET title = ?, auto_download = ?, last_checked_at = ?
         WHERE id = ?",
    )
    .bind(title)
    .bind(auto_download)
    .bind(last_checked_at)
    .bind(id)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "podcast_subscriptions",
    })?;
    Ok(())
}

pub async fn delete_subscription(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM podcast_subscriptions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "podcast_subscriptions",
        })?;
    Ok(())
}

pub async fn insert_episode(pool: &SqlitePool, ep: &PodcastEpisode) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO podcast_episodes
         (id, subscription_id, guid, title, description, episode_number, season_number,
          publication_date, duration_ms, enclosure_url, file_path, file_size_bytes,
          file_format, quality_score, source_type, listened, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&ep.id)
    .bind(&ep.subscription_id)
    .bind(&ep.guid)
    .bind(&ep.title)
    .bind(&ep.description)
    .bind(ep.episode_number)
    .bind(ep.season_number)
    .bind(&ep.publication_date)
    .bind(ep.duration_ms)
    .bind(&ep.enclosure_url)
    .bind(&ep.file_path)
    .bind(ep.file_size_bytes)
    .bind(&ep.file_format)
    .bind(ep.quality_score)
    .bind(&ep.source_type)
    .bind(ep.listened)
    .bind(&ep.added_at)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "podcast_episodes",
    })?;
    Ok(())
}

pub async fn get_episode(pool: &SqlitePool, id: &[u8]) -> Result<Option<PodcastEpisode>, DbError> {
    sqlx::query_as::<_, PodcastEpisode>(
        "SELECT id, subscription_id, guid, title, description, episode_number, season_number,
                publication_date, duration_ms, enclosure_url, file_path, file_size_bytes,
                file_format, quality_score, source_type, listened, added_at
         FROM podcast_episodes WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "podcast_episodes",
    })
}

pub async fn list_episodes(
    pool: &SqlitePool,
    subscription_id: &[u8],
    limit: i64,
    offset: i64,
) -> Result<Vec<PodcastEpisode>, DbError> {
    sqlx::query_as::<_, PodcastEpisode>(
        "SELECT id, subscription_id, guid, title, description, episode_number, season_number,
                publication_date, duration_ms, enclosure_url, file_path, file_size_bytes,
                file_format, quality_score, source_type, listened, added_at
         FROM podcast_episodes WHERE subscription_id = ?
         ORDER BY publication_date DESC LIMIT ? OFFSET ?",
    )
    .bind(subscription_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "podcast_episodes",
    })
}

pub async fn update_episode(
    pool: &SqlitePool,
    id: &[u8],
    listened: i64,
    file_path: Option<&str>,
    quality_score: Option<i64>,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE podcast_episodes SET listened = ?, file_path = ?, quality_score = ?
         WHERE id = ?",
    )
    .bind(listened)
    .bind(file_path)
    .bind(quality_score)
    .bind(id)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "podcast_episodes",
    })?;
    Ok(())
}

pub async fn delete_episode(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM podcast_episodes WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "podcast_episodes",
        })?;
    Ok(())
}

pub async fn episode_guid_exists(
    pool: &SqlitePool,
    subscription_id: &[u8],
    guid: &str,
) -> Result<bool, DbError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM podcast_episodes WHERE subscription_id = ? AND guid = ?",
    )
    .bind(subscription_id)
    .bind(guid)
    .fetch_one(pool)
    .await
    .context(QuerySnafu {
        table: "podcast_episodes",
    })?;
    Ok(count > 0)
}

pub async fn count_episodes_for_subscription(
    pool: &SqlitePool,
    subscription_id: &[u8],
) -> Result<i64, DbError> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM podcast_episodes WHERE subscription_id = ?")
            .bind(subscription_id)
            .fetch_one(pool)
            .await
            .context(QuerySnafu {
                table: "podcast_episodes",
            })?;
    Ok(count)
}

pub async fn subscription_by_url(
    pool: &SqlitePool,
    url: &str,
) -> Result<Option<PodcastSubscription>, DbError> {
    sqlx::query_as::<_, PodcastSubscription>(
        "SELECT id, feed_url, title, description, author, image_url, language,
                last_checked_at, auto_download, quality_profile_id, added_at
         FROM podcast_subscriptions WHERE feed_url = ?",
    )
    .bind(url)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "podcast_subscriptions",
    })
}

pub async fn list_recent_episodes(
    pool: &SqlitePool,
    subscription_id: &[u8],
    limit: i64,
) -> Result<Vec<PodcastEpisode>, DbError> {
    sqlx::query_as::<_, PodcastEpisode>(
        "SELECT id, subscription_id, guid, title, description, episode_number, season_number,
                publication_date, duration_ms, enclosure_url, file_path, file_size_bytes,
                file_format, quality_score, source_type, listened, added_at
         FROM podcast_episodes WHERE subscription_id = ?
         ORDER BY publication_date DESC LIMIT ?",
    )
    .bind(subscription_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "podcast_episodes",
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
    async fn subscription_round_trip() {
        let pool = setup().await;
        let id = make_id();
        let sub = PodcastSubscription {
            id: id.clone(),
            feed_url: "https://example.com/feed.xml".to_string(),
            title: Some("Test Podcast".to_string()),
            description: None,
            author: None,
            image_url: None,
            language: Some("en".to_string()),
            last_checked_at: None,
            auto_download: 1,
            quality_profile_id: None,
            added_at: now(),
        };
        insert_subscription(&pool, &sub).await.unwrap();
        let fetched = get_subscription(&pool, &id).await.unwrap().unwrap();
        assert_eq!(fetched.feed_url, "https://example.com/feed.xml");
        assert_eq!(fetched.auto_download, 1);
    }

    #[tokio::test]
    async fn episode_round_trip() {
        let pool = setup().await;
        let sub_id = make_id();
        let sub = PodcastSubscription {
            id: sub_id.clone(),
            feed_url: "https://example.com/feed2.xml".to_string(),
            title: None,
            description: None,
            author: None,
            image_url: None,
            language: None,
            last_checked_at: None,
            auto_download: 1,
            quality_profile_id: None,
            added_at: now(),
        };
        insert_subscription(&pool, &sub).await.unwrap();

        let ep_id = make_id();
        let ep = PodcastEpisode {
            id: ep_id.clone(),
            subscription_id: sub_id.clone(),
            guid: "ep-001".to_string(),
            title: Some("Episode 1".to_string()),
            description: None,
            episode_number: Some(1),
            season_number: Some(1),
            publication_date: Some("2026-01-01T00:00:00Z".to_string()),
            duration_ms: Some(3600000),
            enclosure_url: None,
            file_path: None,
            file_size_bytes: None,
            file_format: None,
            quality_score: None,
            source_type: "rss".to_string(),
            listened: 0,
            added_at: now(),
        };
        insert_episode(&pool, &ep).await.unwrap();

        let episodes = list_episodes(&pool, &sub_id, 10, 0).await.unwrap();
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].guid, "ep-001");
    }

    #[tokio::test]
    async fn list_empty_returns_empty() {
        let pool = setup().await;
        let results = list_subscriptions(&pool, 10, 0).await.unwrap();
        assert!(results.is_empty());
    }
}
