use super::*;
use themelion::aggelia::create_event_bus;
use apotheke::{DbPools, migrate::MIGRATOR};
use sqlx::SqlitePool;

async fn setup() -> (KomideService, themelion::aggelia::EventReceiver) {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    let db = DbPools {
        read: pool.clone(),
        write: pool,
    };
    let (tx, rx) = create_event_bus(64);
    let client = reqwest::Client::new();
    let config = KomideConfig::default();
    let svc = KomideService::new(db, tx, client, config);
    (svc, rx)
}

#[tokio::test]
async fn validate_url_rejects_empty() {
    assert!(validate_url("").is_err());
}

#[tokio::test]
async fn validate_url_rejects_non_http() {
    assert!(validate_url("ftp://example.com/feed.xml").is_err());
}

#[tokio::test]
async fn validate_url_accepts_https() {
    assert!(validate_url("https://example.com/feed.xml").is_ok());
}

#[tokio::test]
async fn validate_url_accepts_http() {
    assert!(validate_url("http://example.com/feed.xml").is_ok());
}

#[tokio::test]
async fn list_feeds_empty_returns_empty() {
    let (svc, _rx) = setup().await;
    let podcasts = svc.list_feeds(MediaType::Podcast).await.unwrap();
    assert!(podcasts.is_empty());
    let news = svc.list_feeds(MediaType::News).await.unwrap();
    assert!(news.is_empty());
}

#[tokio::test]
async fn unsubscribe_nonexistent_returns_error() {
    let (svc, _rx) = setup().await;
    let result = svc.unsubscribe(FeedId::new()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn mark_consumed_nonexistent_is_ok() {
    let (svc, _rx) = setup().await;
    // Should silently succeed for unknown IDs
    let result = svc.mark_consumed(MediaId::new()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn insert_episodes_deduplicates_by_guid() {
    let (svc, _rx) = setup().await;
    let sub_id = make_subscription(&svc).await;

    let entries = vec![
        make_podcast_entry("ep-001", "Episode 1"),
        make_podcast_entry("ep-001", "Episode 1 duplicate"),
    ];

    let now = now_iso8601();
    let count = svc
        .insert_new_podcast_episodes(&sub_id, &entries, &now)
        .await
        .unwrap();
    assert_eq!(count, 1, "duplicate GUID should not be inserted");

    let episodes = podcast::list_episodes(&svc.db.read, &sub_id, 10, 0)
        .await
        .unwrap();
    assert_eq!(episodes.len(), 1);
}

#[tokio::test]
async fn insert_articles_deduplicates_by_guid() {
    let (svc, _rx) = setup().await;
    let feed_id = make_news_feed(&svc).await;

    let entries = vec![
        make_news_entry("art-001", "Article 1"),
        make_news_entry("art-001", "Article 1 duplicate"),
    ];

    let now = now_iso8601();
    let count = svc
        .insert_new_articles(&feed_id, &entries, &now)
        .await
        .unwrap();
    assert_eq!(count, 1, "duplicate GUID should not be inserted");
}

#[tokio::test]
async fn episode_available_event_emitted_on_new_episode() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    let db = DbPools {
        read: pool.clone(),
        write: pool,
    };
    let (tx, mut rx) = create_event_bus(64);
    let svc = KomideService::new(db, tx, reqwest::Client::new(), KomideConfig::default());

    let sub_id = make_subscription(&svc).await;
    let entries = vec![make_podcast_entry("ep-new", "New Episode")];
    let now = now_iso8601();
    svc.insert_new_podcast_episodes(&sub_id, &entries, &now)
        .await
        .unwrap();

    let event = rx.try_recv().unwrap();
    assert!(matches!(
        event,
        themelion::aggelia::HarmoniaEvent::EpisodeAvailable { .. }
    ));
}

// ── Test helpers ─────────────────────────────────────────────────────────

async fn make_subscription(svc: &KomideService) -> Vec<u8> {
    let feed_id = FeedId::new();
    let id_bytes = feed_id.as_bytes().to_vec();
    let sub = podcast::PodcastSubscription {
        id: id_bytes.clone(),
        feed_url: "https://example.com/podcast.xml".to_string(),
        title: Some("Test Podcast".to_string()),
        description: None,
        author: None,
        image_url: None,
        language: None,
        last_checked_at: None,
        auto_download: 1,
        quality_profile_id: None,
        added_at: now_iso8601(),
    };
    podcast::insert_subscription(&svc.db.write, &sub)
        .await
        .unwrap();
    id_bytes
}

async fn make_news_feed(svc: &KomideService) -> Vec<u8> {
    let feed_id = FeedId::new();
    let id_bytes = feed_id.as_bytes().to_vec();
    let feed = news::NewsFeed {
        id: id_bytes.clone(),
        title: "Test News".to_string(),
        url: "https://example.com/news.xml".to_string(),
        site_url: None,
        description: None,
        category: None,
        icon_url: None,
        last_fetched_at: None,
        fetch_interval_minutes: 15,
        is_active: 1,
        added_at: now_iso8601(),
        updated_at: now_iso8601(),
    };
    news::insert_feed(&svc.db.write, &feed).await.unwrap();
    id_bytes
}

fn make_podcast_entry(guid: &str, title: &str) -> crate::parser::NormalizedEntry {
    crate::parser::NormalizedEntry {
        guid: guid.to_string(),
        title: title.to_string(),
        published: Some("2026-01-01T00:00:00Z".to_string()),
        summary: None,
        content: None,
        enclosures: vec![crate::parser::Enclosure {
            url: format!("https://example.com/{guid}.mp3"),
            content_type: Some("audio/mpeg".to_string()),
            length: None,
        }],
        link: None,
    }
}

fn make_news_entry(guid: &str, title: &str) -> crate::parser::NormalizedEntry {
    crate::parser::NormalizedEntry {
        guid: guid.to_string(),
        title: title.to_string(),
        published: Some("2026-01-01T00:00:00Z".to_string()),
        summary: Some("Summary".to_string()),
        content: None,
        enclosures: vec![],
        link: Some(format!("https://example.com/{guid}")),
    }
}
