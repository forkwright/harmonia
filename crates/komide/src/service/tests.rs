use apotheke::DbPools;
use apotheke::migrate::MIGRATOR;
use sqlx::SqlitePool;
use themelion::aggelia::{HarmoniaEvent, create_event_bus};

use super::*;
use crate::test_support::{http_response, spawn_scripted_http};

const RSS_TWO_EPISODES: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test Podcast</title>
    <description>A test podcast feed</description>
    <item>
      <title>Episode 1</title>
      <guid>ep-001</guid>
      <pubDate>Mon, 01 Jan 2024 00:00:00 +0000</pubDate>
      <enclosure url="https://example.com/ep1.mp3" type="audio/mpeg" length="1234"/>
    </item>
    <item>
      <title>Episode 2</title>
      <guid>ep-002</guid>
      <pubDate>Tue, 02 Jan 2024 00:00:00 +0000</pubDate>
      <enclosure url="https://example.com/ep2.mp3" type="audio/mpeg" length="5678"/>
    </item>
  </channel>
</rss>"#;

/// Atom fixture with two fresh articles. Published timestamps are "now" so
/// the default 30-day retention pass keeps them.
fn atom_two_articles() -> Vec<u8> {
    let now = now_iso8601();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test News</title>
  <entry>
    <id>article-001</id>
    <title>Breaking News</title>
    <published>{now}</published>
    <summary>Something happened</summary>
    <link href="https://news.example.com/breaking"/>
  </entry>
  <entry>
    <id>article-002</id>
    <title>Follow Up</title>
    <published>{now}</published>
    <summary>More details</summary>
    <link href="https://news.example.com/followup"/>
  </entry>
</feed>"#
    )
    .into_bytes()
}

async fn setup() -> (FeedSchedulerService, themelion::aggelia::EventReceiver) {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    let db = DbPools {
        read: pool.clone(),
        write: pool,
    };
    let (tx, rx) = create_event_bus(64);
    let client = reqwest::Client::new();
    let config = KomideConfig::default();
    let svc = FeedSchedulerService::new(db, tx, client, config);
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
async fn validate_url_accepts_loopback_with_port() {
    assert!(validate_url("http://127.0.0.1:7878").is_ok());
}

#[tokio::test]
async fn validate_url_accepts_lan_host() {
    assert!(validate_url("http://kanon.lan").is_ok());
}

#[tokio::test]
async fn validate_url_rejects_javascript_scheme() {
    assert!(validate_url("javascript:alert(1)").is_err());
}

#[tokio::test]
async fn validate_url_rejects_unparseable() {
    assert!(validate_url("not a url").is_err());
}

#[tokio::test]
async fn validate_url_rejects_scheme_relative() {
    // old prefix-match would accept this as invalid; url::Url rejects it outright
    assert!(validate_url("//no-scheme.com").is_err());
}

#[tokio::test]
async fn validate_url_rejects_http_without_host() {
    // old prefix-match silently accepted "http://"; url::Url parses but has no host
    assert!(validate_url("http://").is_err());
}

#[tokio::test]
async fn validate_url_rejects_https_colon_only() {
    // old prefix-match rejected; new parse rejects too (no host)
    assert!(validate_url("https:").is_err());
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
    let sub_id = make_subscription(&svc, "https://example.com/podcast.xml").await;

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
    let feed_id = make_news_feed(&svc, "https://example.com/news.xml").await;

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
    let svc = FeedSchedulerService::new(db, tx, reqwest::Client::new(), KomideConfig::default());

    let sub_id = make_subscription(&svc, "https://example.com/podcast.xml").await;
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

// ── refresh_feed integration (scripted in-process HTTP server) ───────────

#[tokio::test]
async fn refresh_podcast_feed_inserts_items_and_emits_event() {
    let (svc, mut rx) = setup().await;
    let (url, _handle) = spawn_scripted_http(vec![http_response(
        200,
        "OK",
        &[("etag", "\"v1\"")],
        RSS_TWO_EPISODES,
    )])
    .await;
    let sub_id = make_subscription(&svc, &url).await;
    let feed_id = bytes_to_feed_id(&sub_id).unwrap();

    let result = svc.refresh_feed(feed_id).await.unwrap();
    assert_eq!(result.new_items, 2);
    assert_eq!(result.total_items, 2);
    assert_eq!(result.feed_id, feed_id);

    let sub = podcast::get_subscription(&svc.db.read, &sub_id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        sub.last_checked_at.is_some(),
        "refresh must update last_checked_at"
    );

    let mut saw_refreshed = false;
    while let Ok(event) = rx.try_recv() {
        if matches!(
            event,
            HarmoniaEvent::FeedRefreshed {
                new_items: 2,
                media_type: MediaType::Podcast,
                ..
            }
        ) {
            saw_refreshed = true;
        }
    }
    assert!(saw_refreshed, "FeedRefreshed event must be emitted");
}

#[tokio::test]
async fn refresh_news_feed_inserts_articles_and_emits_event() {
    let (svc, mut rx) = setup().await;
    let (url, _handle) =
        spawn_scripted_http(vec![http_response(200, "OK", &[], &atom_two_articles())]).await;
    let feed_bytes = make_news_feed(&svc, &url).await;
    let feed_id = bytes_to_feed_id(&feed_bytes).unwrap();

    let result = svc.refresh_feed(feed_id).await.unwrap();
    assert_eq!(result.new_items, 2);
    assert_eq!(result.total_items, 2);

    let feed = news::get_feed(&svc.db.read, &feed_bytes)
        .await
        .unwrap()
        .unwrap();
    assert!(
        feed.last_fetched_at.is_some(),
        "refresh must update last_fetched_at"
    );

    let mut saw_refreshed = false;
    while let Ok(event) = rx.try_recv() {
        if matches!(
            event,
            HarmoniaEvent::FeedRefreshed {
                new_items: 2,
                media_type: MediaType::News,
                ..
            }
        ) {
            saw_refreshed = true;
        }
    }
    assert!(saw_refreshed, "FeedRefreshed event must be emitted");
}

#[tokio::test]
async fn refresh_feed_unknown_id_returns_feed_not_found() {
    let (svc, _rx) = setup().await;
    let result = svc.refresh_feed(FeedId::new()).await;
    assert!(matches!(result, Err(KomideError::FeedNotFound { .. })));
}

#[tokio::test]
async fn refresh_feed_http_500_is_error_and_leaves_db_untouched() {
    let (svc, _rx) = setup().await;
    let (url, _handle) = spawn_scripted_http(vec![http_response(
        500,
        "Internal Server Error",
        &[],
        b"<html>oops</html>",
    )])
    .await;
    let sub_id = make_subscription(&svc, &url).await;

    let result = svc.refresh_feed(bytes_to_feed_id(&sub_id).unwrap()).await;
    assert!(matches!(result, Err(KomideError::FeedFetch { .. })));

    let episodes = podcast::list_episodes(&svc.db.read, &sub_id, 10, 0)
        .await
        .unwrap();
    assert!(episodes.is_empty(), "a 500 must not insert episodes");

    let sub = podcast::get_subscription(&svc.db.read, &sub_id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        sub.last_checked_at.is_none(),
        "a 500 must not update last_checked_at"
    );
}

#[tokio::test]
async fn refresh_podcast_feed_second_call_with_304_returns_zero_new_items() {
    let (svc, _rx) = setup().await;
    let (url, handle) = spawn_scripted_http(vec![
        http_response(200, "OK", &[("etag", "\"v1\"")], RSS_TWO_EPISODES),
        http_response(304, "Not Modified", &[], b""),
    ])
    .await;
    let sub_id = make_subscription(&svc, &url).await;
    let feed_id = bytes_to_feed_id(&sub_id).unwrap();

    let first = svc.refresh_feed(feed_id).await.unwrap();
    assert_eq!(first.new_items, 2);

    let second = svc.refresh_feed(feed_id).await.unwrap();
    assert_eq!(second.new_items, 0);
    assert_eq!(
        second.total_items, 2,
        "total_items on 304 is the stored episode count"
    );

    let requests = handle.await.unwrap();
    assert!(
        !requests[0].to_lowercase().contains("if-none-match"),
        "first request must be unconditional"
    );
    assert!(
        requests[1].to_lowercase().contains("if-none-match: \"v1\""),
        "second request must forward the stored ETag"
    );
}

#[tokio::test]
async fn refresh_news_feed_second_call_with_304_returns_zero_new_items() {
    let (svc, _rx) = setup().await;
    let (url, handle) = spawn_scripted_http(vec![
        http_response(200, "OK", &[("etag", "\"n1\"")], &atom_two_articles()),
        http_response(304, "Not Modified", &[], b""),
    ])
    .await;
    let feed_bytes = make_news_feed(&svc, &url).await;
    let feed_id = bytes_to_feed_id(&feed_bytes).unwrap();

    let first = svc.refresh_feed(feed_id).await.unwrap();
    assert_eq!(first.new_items, 2);

    let second = svc.refresh_feed(feed_id).await.unwrap();
    assert_eq!(second.new_items, 0);
    assert_eq!(
        second.total_items, 2,
        "total_items on 304 is the stored article count"
    );

    let requests = handle.await.unwrap();
    assert!(
        requests[1].to_lowercase().contains("if-none-match: \"n1\""),
        "second request must forward the stored ETag"
    );
}

#[tokio::test]
async fn store_validators_ignores_empty_pair_and_keeps_nonempty() {
    let (svc, _rx) = setup().await;
    let url = "https://example.com/feed.xml";

    svc.store_validators(url, None, None).await;
    assert_eq!(
        svc.cached_validators(url).await,
        (None, None),
        "an empty validator pair must not be cached"
    );

    svc.store_validators(url, Some("\"e1\"".to_string()), None)
        .await;
    assert_eq!(
        svc.cached_validators(url).await,
        (Some("\"e1\"".to_string()), None)
    );

    // A later empty pair must not clobber the stored validators.
    svc.store_validators(url, None, None).await;
    assert_eq!(
        svc.cached_validators(url).await,
        (Some("\"e1\"".to_string()), None)
    );
}

// ── Test helpers ─────────────────────────────────────────────────────────

async fn make_subscription(svc: &FeedSchedulerService, feed_url: &str) -> Vec<u8> {
    let feed_id = FeedId::new();
    let id_bytes = feed_id.as_bytes().to_vec();
    let sub = podcast::PodcastSubscription {
        id: id_bytes.clone(),
        feed_url: feed_url.to_string(),
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

async fn make_news_feed(svc: &FeedSchedulerService, url: &str) -> Vec<u8> {
    let feed_id = FeedId::new();
    let id_bytes = feed_id.as_bytes().to_vec();
    let feed = news::NewsFeed {
        id: id_bytes.clone(),
        title: "Test News".to_string(),
        url: url.to_string(),
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

#[test]
fn bytes_to_feed_id_accepts_valid_uuid_bytes() {
    let uuid = Uuid::new_v4();
    let id = bytes_to_feed_id(uuid.as_bytes()).unwrap();
    assert_eq!(id, FeedId::from_uuid(uuid));
}

#[test]
fn bytes_to_feed_id_rejects_corrupt_bytes() {
    let result = bytes_to_feed_id(&[1, 2, 3]);
    assert!(
        matches!(result, Err(KomideError::CorruptFeedId { .. })),
        "short id bytes must surface as CorruptFeedId, not a phantom FeedId"
    );
}
