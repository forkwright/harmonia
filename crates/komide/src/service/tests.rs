use aggelmata::aggelia::{HarmoniaEvent, create_event_bus};
use apotheke::DbPools;
use apotheke::migrate::MIGRATOR;
use sqlx::SqlitePool;

use super::*;
use crate::test_support::{http_response, install_test_crypto_provider, spawn_scripted_http};

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

async fn setup() -> (FeedSchedulerService, aggelmata::aggelia::EventReceiver) {
    setup_with_config(KomideConfig::default()).await
}

async fn setup_with_config(
    config: KomideConfig,
) -> (FeedSchedulerService, aggelmata::aggelia::EventReceiver) {
    // WHY: reqwest::Client::new() below eagerly builds its TLS connector;
    // see test_support::install_test_crypto_provider's WHY note.
    install_test_crypto_provider();
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    let db = DbPools {
        read: pool.clone(),
        write: pool,
    };
    let (tx, rx) = create_event_bus(64);
    let client = reqwest::Client::new();
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
    install_test_crypto_provider();
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
        aggelmata::aggelia::HarmoniaEvent::EpisodeAvailable { .. }
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

// ── subscribe / unsubscribe (FeedSetChanged + auto_download semantics) ───

/// RSS fixture whose items carry NO enclosures — subscribe tests that leave
/// auto-download enabled use it so the spawned download task finds nothing
/// to fetch (no network reachout from a unit test).
const RSS_TWO_EPISODES_NO_ENCLOSURES: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test Podcast</title>
    <description>A test podcast feed</description>
    <item>
      <title>Episode 1</title>
      <guid>ep-001</guid>
      <pubDate>Mon, 01 Jan 2024 00:00:00 +0000</pubDate>
    </item>
    <item>
      <title>Episode 2</title>
      <guid>ep-002</guid>
      <pubDate>Tue, 02 Jan 2024 00:00:00 +0000</pubDate>
    </item>
  </channel>
</rss>"#;

/// RSS fixture with both enclosures pointing at the scripted test server.
fn rss_two_episodes_with_enclosures(base: &str) -> Vec<u8> {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test Podcast</title>
    <description>A test podcast feed</description>
    <item>
      <title>Episode 1</title>
      <guid>ep-001</guid>
      <pubDate>Mon, 01 Jan 2024 00:00:00 +0000</pubDate>
      <enclosure url="{base}/ep1.mp3" type="audio/mpeg" length="4"/>
    </item>
    <item>
      <title>Episode 2</title>
      <guid>ep-002</guid>
      <pubDate>Tue, 02 Jan 2024 00:00:00 +0000</pubDate>
      <enclosure url="{base}/ep2.mp3" type="audio/mpeg" length="4"/>
    </item>
  </channel>
</rss>"#
    )
    .into_bytes()
}

fn drain_events(rx: &mut aggelmata::aggelia::EventReceiver) -> Vec<HarmoniaEvent> {
    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    events
}

#[tokio::test]
async fn subscribe_podcast_emits_feed_set_changed() {
    let (svc, mut rx) = setup().await;
    let (url, _handle) =
        spawn_scripted_http(vec![http_response(200, "OK", &[], RSS_TWO_EPISODES)]).await;

    let feed_id = svc
        .subscribe_podcast(&url, None, Some(false))
        .await
        .unwrap();

    let events = drain_events(&mut rx);
    assert!(
        events.iter().any(|e| matches!(
            e,
            HarmoniaEvent::FeedSetChanged {
                feed_id: id,
                media_type: MediaType::Podcast,
            } if *id == feed_id
        )),
        "subscribe must emit FeedSetChanged so the supervisor re-enumerates"
    );

    let sub = podcast::get_subscription(&svc.db.read, feed_id.as_bytes())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        sub.auto_download, 0,
        "auto_download=false must store an episode count of 0"
    );
}

#[tokio::test]
async fn subscribe_podcast_defaults_auto_download_count_from_config() {
    let (svc, _rx) = setup().await;
    let (url, _handle) = spawn_scripted_http(vec![http_response(
        200,
        "OK",
        &[],
        RSS_TWO_EPISODES_NO_ENCLOSURES,
    )])
    .await;

    let feed_id = svc.subscribe_podcast(&url, None, None).await.unwrap();

    let sub = podcast::get_subscription(&svc.db.read, feed_id.as_bytes())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        sub.auto_download,
        i64::try_from(KomideConfig::default().auto_download_latest_n).unwrap(),
        "the stored auto_download is the configured episode COUNT, not a 0/1 bool"
    );
}

#[tokio::test]
async fn subscribe_podcast_downloads_initial_episodes_in_background() {
    let dir = tempfile::TempDir::new().unwrap();
    let config = KomideConfig {
        podcast_dir: dir.path().to_path_buf(),
        auto_download_latest_n: 2,
        ..KomideConfig::default()
    };
    let (svc, _rx) = setup_with_config(config).await;

    let (base, _handle) = spawn_scripted_http(vec![
        http_response(200, "OK", &[], b"AUD1"),
        http_response(200, "OK", &[], b"AUD2"),
    ])
    .await;
    // WHY two servers: the feed body must embed the audio server's base URL,
    // so the audio server exists first and a second server serves the feed.
    let feed_body = rss_two_episodes_with_enclosures(&base);
    let (feed_url, _feed_handle) =
        spawn_scripted_http(vec![http_response(200, "OK", &[], &feed_body)]).await;

    let feed_id = svc.subscribe_podcast(&feed_url, None, None).await.unwrap();

    // Bounded wait for the spawned download task to finish both transfers.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let episodes = podcast::list_episodes(&svc.db.read, feed_id.as_bytes(), 10, 0)
            .await
            .unwrap();
        if episodes.len() == 2 && episodes.iter().all(|e| e.file_path.is_some()) {
            for ep in &episodes {
                let path = std::path::Path::new(ep.file_path.as_deref().unwrap());
                assert!(
                    path.starts_with(dir.path()),
                    "file must land in podcast_dir"
                );
                assert!(path.exists(), "downloaded file must exist on disk");
                assert_eq!(ep.file_size_bytes, Some(4));
            }
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "spawned initial downloads did not complete in time: {episodes:?}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn subscribe_news_emits_feed_set_changed_and_stores_category() {
    let (svc, mut rx) = setup().await;
    let (url, _handle) =
        spawn_scripted_http(vec![http_response(200, "OK", &[], &atom_two_articles())]).await;

    let feed_id = svc
        .subscribe_news(&url, Some("My News"), Some("tech"))
        .await
        .unwrap();

    let events = drain_events(&mut rx);
    assert!(
        events.iter().any(|e| matches!(
            e,
            HarmoniaEvent::FeedSetChanged {
                feed_id: id,
                media_type: MediaType::News,
            } if *id == feed_id
        )),
        "news subscribe must emit FeedSetChanged"
    );

    let feed = news::get_feed(&svc.db.read, feed_id.as_bytes())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(feed.title, "My News");
    assert_eq!(feed.category.as_deref(), Some("tech"));
    assert_eq!(
        feed.fetch_interval_minutes,
        i64::try_from(KomideConfig::default().news_poll_interval_minutes).unwrap(),
        "the poll interval comes FROM config, never a hardcoded literal"
    );
}

#[tokio::test]
async fn unsubscribe_emits_feed_set_changed() {
    let (svc, mut rx) = setup().await;
    let sub_id = make_subscription(&svc, "https://example.com/bye.xml").await;
    let feed_id = bytes_to_feed_id(&sub_id).unwrap();

    svc.unsubscribe(feed_id).await.unwrap();

    let events = drain_events(&mut rx);
    assert!(
        events.iter().any(|e| matches!(
            e,
            HarmoniaEvent::FeedSetChanged {
                feed_id: id,
                media_type: MediaType::Podcast,
            } if *id == feed_id
        )),
        "unsubscribe must emit FeedSetChanged"
    );
}

// ── episode download (#575 wire: podcast_dir + max_episode_bytes) ────────

#[tokio::test]
async fn refresh_downloads_latest_n_new_episodes_inline() {
    let dir = tempfile::TempDir::new().unwrap();
    let config = KomideConfig {
        podcast_dir: dir.path().to_path_buf(),
        ..KomideConfig::default()
    };
    let (svc, _rx) = setup_with_config(config).await;

    let (base, _audio_handle) =
        spawn_scripted_http(vec![http_response(200, "OK", &[], b"AUDIO")]).await;
    let feed_body = rss_two_episodes_with_enclosures(&base);
    let (feed_url, _feed_handle) =
        spawn_scripted_http(vec![http_response(200, "OK", &[], &feed_body)]).await;

    // auto_download = 1 → only the most recent episode (ep-002) downloads.
    let sub_id = make_subscription_with_auto_download(&svc, &feed_url, 1).await;
    let feed_id = bytes_to_feed_id(&sub_id).unwrap();

    let result = svc.refresh_feed(feed_id).await.unwrap();
    assert_eq!(result.new_items, 2);

    let episodes = podcast::list_episodes(&svc.db.read, &sub_id, 10, 0)
        .await
        .unwrap();
    let ep2 = episodes.iter().find(|e| e.guid == "ep-002").unwrap();
    let ep1 = episodes.iter().find(|e| e.guid == "ep-001").unwrap();
    assert!(
        ep2.file_path.is_some(),
        "the most recent episode must be downloaded inline by the refresh"
    );
    assert_eq!(ep2.file_size_bytes, Some(5));
    assert!(
        std::path::Path::new(ep2.file_path.as_deref().unwrap()).exists(),
        "downloaded audio must exist under podcast_dir"
    );
    assert!(
        ep1.file_path.is_none(),
        "episodes beyond auto_download's count must not download"
    );
}

#[tokio::test]
async fn download_episode_by_id_writes_file_and_updates_row() {
    let dir = tempfile::TempDir::new().unwrap();
    let config = KomideConfig {
        podcast_dir: dir.path().to_path_buf(),
        ..KomideConfig::default()
    };
    let (svc, _rx) = setup_with_config(config).await;

    let (base, _handle) =
        spawn_scripted_http(vec![http_response(200, "OK", &[], b"AUDIOBYTES")]).await;
    let sub_id = make_subscription(&svc, "https://example.com/pod.xml").await;
    let entry =
        make_podcast_entry_with_enclosure("ep-dl", "Download Me", &format!("{base}/ep.m4a"));
    svc.insert_new_podcast_episodes(&sub_id, &[entry], &now_iso8601())
        .await
        .unwrap();
    let ep = podcast::list_episodes(&svc.db.read, &sub_id, 1, 0)
        .await
        .unwrap()[0]
        .clone();
    let episode_id = EpisodeId::from_uuid(Uuid::from_slice(&ep.id).unwrap());

    let written = svc.download_episode_by_id(episode_id).await.unwrap();
    assert_eq!(written, 10);

    let ep = podcast::get_episode(&svc.db.read, &ep.id)
        .await
        .unwrap()
        .unwrap();
    let path = std::path::PathBuf::from(ep.file_path.as_deref().unwrap());
    assert!(
        path.starts_with(dir.path()),
        "file must land in podcast_dir"
    );
    assert_eq!(
        path.extension().and_then(|e| e.to_str()),
        Some("m4a"),
        "the file extension comes FROM the enclosure URL"
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"AUDIOBYTES");
    assert_eq!(ep.file_size_bytes, Some(10));
}

#[tokio::test]
async fn download_episode_by_id_respects_max_episode_bytes() {
    let dir = tempfile::TempDir::new().unwrap();
    let config = KomideConfig {
        podcast_dir: dir.path().to_path_buf(),
        max_episode_bytes: 4,
        ..KomideConfig::default()
    };
    let (svc, _rx) = setup_with_config(config).await;

    let (base, _handle) =
        spawn_scripted_http(vec![http_response(200, "OK", &[], b"WAY TOO LARGE")]).await;
    let sub_id = make_subscription(&svc, "https://example.com/cap.xml").await;
    let entry = make_podcast_entry_with_enclosure("ep-cap", "Too Big", &format!("{base}/big.mp3"));
    svc.insert_new_podcast_episodes(&sub_id, &[entry], &now_iso8601())
        .await
        .unwrap();
    let ep = podcast::list_episodes(&svc.db.read, &sub_id, 1, 0)
        .await
        .unwrap()[0]
        .clone();
    let episode_id = EpisodeId::from_uuid(Uuid::from_slice(&ep.id).unwrap());

    let result = svc.download_episode_by_id(episode_id).await;
    assert!(
        matches!(result, Err(KomideError::ResponseTooLarge { .. })),
        "an over-cap enclosure must be rejected: {result:?}"
    );

    let leftovers: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
    assert!(leftovers.is_empty(), "no partial file may remain on disk");
    let ep = podcast::get_episode(&svc.db.read, &ep.id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        ep.file_path.is_none(),
        "a failed download must not set file_path"
    );
}

#[tokio::test]
async fn download_episode_unknown_id_returns_episode_not_found() {
    let (svc, _rx) = setup().await;
    let result = svc.download_episode_by_id(EpisodeId::new()).await;
    assert!(matches!(result, Err(KomideError::EpisodeNotFound { .. })));
}

#[tokio::test]
async fn download_episode_without_enclosure_returns_not_downloadable() {
    let (svc, _rx) = setup().await;
    let sub_id = make_subscription(&svc, "https://example.com/noenc.xml").await;
    let entry = crate::parser::NormalizedEntry {
        guid: "ep-noenc".to_string(),
        title: "No Enclosure".to_string(),
        published: Some("2026-01-01T00:00:00Z".to_string()),
        summary: None,
        content: None,
        enclosures: vec![],
        link: None,
    };
    svc.insert_new_podcast_episodes(&sub_id, &[entry], &now_iso8601())
        .await
        .unwrap();
    let ep = podcast::list_episodes(&svc.db.read, &sub_id, 1, 0)
        .await
        .unwrap()[0]
        .clone();
    let episode_id = EpisodeId::from_uuid(Uuid::from_slice(&ep.id).unwrap());

    let result = svc.episode_downloadable(episode_id).await;
    assert!(matches!(
        result,
        Err(KomideError::EpisodeNotDownloadable { .. })
    ));
}

#[test]
fn enclosure_extension_falls_back_to_mp3() {
    assert_eq!(enclosure_extension("https://x.com/audio/ep.M4A"), "m4a");
    assert_eq!(enclosure_extension("https://x.com/audio/ep"), "mp3");
    assert_eq!(enclosure_extension("https://x.com/ep.mp3?tk=1"), "mp3");
    assert_eq!(enclosure_extension("not a url"), "mp3");
    assert_eq!(
        enclosure_extension("https://x.com/ep.tooLong"),
        "mp3",
        "an implausibly long extension must not be trusted"
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

// NOTE: auto_download 0 — refresh tests exercising feed parsing/eventing
// must never spill into real enclosure fetches (the fixture enclosure URLs
// point at example.com). Download tests seed a nonzero count explicitly.
async fn make_subscription(svc: &FeedSchedulerService, feed_url: &str) -> Vec<u8> {
    make_subscription_with_auto_download(svc, feed_url, 0).await
}

async fn make_subscription_with_auto_download(
    svc: &FeedSchedulerService,
    feed_url: &str,
    auto_download: i64,
) -> Vec<u8> {
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
        auto_download,
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
    make_podcast_entry_with_enclosure(guid, title, &format!("https://example.com/{guid}.mp3"))
}

fn make_podcast_entry_with_enclosure(
    guid: &str,
    title: &str,
    enclosure_url: &str,
) -> crate::parser::NormalizedEntry {
    crate::parser::NormalizedEntry {
        guid: guid.to_string(),
        title: title.to_string(),
        published: Some("2026-01-01T00:00:00Z".to_string()),
        summary: None,
        content: None,
        enclosures: vec![crate::parser::Enclosure {
            url: enclosure_url.to_string(),
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
