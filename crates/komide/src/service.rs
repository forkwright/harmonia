use std::collections::HashMap;

use snafu::ResultExt;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use harmonia_common::aggelia::EventSender;
use harmonia_common::ids::{EpisodeId, FeedId, MediaId};
use harmonia_common::media::MediaType;
use harmonia_db::DbPools;
use harmonia_db::repo::{news, podcast};
use horismos::KomideConfig;

use crate::error::{DatabaseSnafu, FeedNotFoundSnafu, InvalidUrlSnafu, KomideError};
use crate::fetch::{FetchResult, fetch_feed};
use crate::news::apply_retention;
use crate::parser::parse_feed;
use crate::podcast::extract_audio_enclosure;

pub struct FeedRefreshResult {
    pub feed_id: FeedId,
    pub new_items: usize,
    pub total_items: usize,
}

pub struct FeedSummary {
    pub id: FeedId,
    pub title: String,
    pub url: String,
    pub media_type: MediaType,
    pub last_fetched_at: Option<String>,
    pub is_active: bool,
}

pub struct KomideService {
    pub(crate) db: DbPools,
    event_tx: EventSender,
    client: reqwest::Client,
    pub(crate) config: KomideConfig,
    /// Stores (etag, last_modified) keyed by feed URL for conditional requests.
    #[expect(
        clippy::type_complexity,
        reason = "map of (etag, last-modified) pairs; extracting a named struct would add indirection without clarity"
    )]
    cache_validators: tokio::sync::Mutex<HashMap<String, (Option<String>, Option<String>)>>,
}

impl KomideService {
    pub fn new(
        db: DbPools,
        event_tx: EventSender,
        client: reqwest::Client,
        config: KomideConfig,
    ) -> Self {
        Self {
            db,
            event_tx,
            client,
            config,
            cache_validators: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Add a new podcast subscription.
    #[instrument(skip(self), fields(url))]
    pub async fn subscribe_podcast(
        &self,
        url: &str,
        label: Option<&str>,
    ) -> Result<FeedId, KomideError> {
        validate_url(url)?;

        // Check for existing subscription at this URL
        if let Some(existing) = podcast::subscription_by_url(&self.db.read, url)
            .await
            .context(DatabaseSnafu)?
        {
            let id = bytes_to_feed_id(&existing.id);
            return Ok(id);
        }

        // Fetch and parse to populate metadata
        let feed_bytes = fetch_bytes(&self.client, url).await?;
        let parsed = parse_feed(&feed_bytes)?;

        let feed_id = FeedId::new();
        let id_bytes = feed_id.as_bytes().to_vec();
        let now = now_iso8601();

        let sub = podcast::PodcastSubscription {
            id: id_bytes.clone(),
            feed_url: url.to_string(),
            title: label
                .map(str::to_owned)
                .or_else(|| Some(parsed.title.clone())),
            description: parsed.description.clone(),
            author: None,
            image_url: parsed.image_url.clone(),
            language: None,
            last_checked_at: Some(now.clone()),
            auto_download: self.config.auto_download_latest_n as i64,
            quality_profile_id: None,
            added_at: now.clone(),
        };
        podcast::insert_subscription(&self.db.write, &sub)
            .await
            .context(DatabaseSnafu)?;

        // Insert initial episodes
        let new_count = self
            .insert_new_podcast_episodes(&id_bytes, &parsed.entries, &now)
            .await?;

        self.emit_feed_refreshed(feed_id, new_count, MediaType::Podcast);
        info!(feed_id = %feed_id, episodes = new_count, "podcast subscribed");
        Ok(feed_id)
    }

    /// Add a new news feed subscription.
    #[instrument(skip(self), fields(url))]
    pub async fn subscribe_news(
        &self,
        url: &str,
        label: Option<&str>,
    ) -> Result<FeedId, KomideError> {
        validate_url(url)?;

        // Check for existing feed at this URL
        if let Some(existing) = news::feed_by_url(&self.db.read, url)
            .await
            .context(DatabaseSnafu)?
        {
            return Ok(bytes_to_feed_id(&existing.id));
        }

        let feed_bytes = fetch_bytes(&self.client, url).await?;
        let parsed = parse_feed(&feed_bytes)?;

        let feed_id = FeedId::new();
        let id_bytes = feed_id.as_bytes().to_vec();
        let now = now_iso8601();

        let feed = news::NewsFeed {
            id: id_bytes.clone(),
            title: label
                .map(str::to_owned)
                .unwrap_or_else(|| parsed.title.clone()),
            url: url.to_string(),
            site_url: parsed.link.clone(),
            description: parsed.description.clone(),
            category: None,
            icon_url: parsed.image_url.clone(),
            last_fetched_at: Some(now.clone()),
            fetch_interval_minutes: self.config.news_poll_interval_minutes as i64,
            is_active: 1,
            added_at: now.clone(),
            updated_at: now.clone(),
        };
        news::insert_feed(&self.db.write, &feed)
            .await
            .context(DatabaseSnafu)?;

        let new_count = self
            .insert_new_articles(&id_bytes, &parsed.entries, &now)
            .await?;

        self.emit_feed_refreshed(feed_id, new_count, MediaType::News);
        info!(feed_id = %feed_id, articles = new_count, "news feed subscribed");
        Ok(feed_id)
    }

    /// Remove a subscription. Tries podcast subscriptions then news feeds.
    #[instrument(skip(self))]
    pub async fn unsubscribe(&self, feed_id: FeedId) -> Result<(), KomideError> {
        let id_bytes = feed_id.as_bytes().to_vec();

        // Try podcast subscription first
        if podcast::get_subscription(&self.db.read, &id_bytes)
            .await
            .context(DatabaseSnafu)?
            .is_some()
        {
            podcast::delete_subscription(&self.db.write, &id_bytes)
                .await
                .context(DatabaseSnafu)?;
            return Ok(());
        }

        // Try news feed
        if news::get_feed(&self.db.read, &id_bytes)
            .await
            .context(DatabaseSnafu)?
            .is_some()
        {
            news::delete_feed(&self.db.write, &id_bytes)
                .await
                .context(DatabaseSnafu)?;
            return Ok(());
        }

        FeedNotFoundSnafu {
            feed_id: feed_id.to_string(),
        }
        .fail()
    }

    /// Force immediate refresh of a feed (podcast or news).
    #[instrument(skip(self))]
    pub async fn refresh_feed(&self, feed_id: FeedId) -> Result<FeedRefreshResult, KomideError> {
        let id_bytes = feed_id.as_bytes().to_vec();

        // Determine feed type and URL
        if let Some(sub) = podcast::get_subscription(&self.db.read, &id_bytes)
            .await
            .context(DatabaseSnafu)?
        {
            return self.refresh_podcast_feed(feed_id, &sub).await;
        }

        if let Some(feed) = news::get_feed(&self.db.read, &id_bytes)
            .await
            .context(DatabaseSnafu)?
        {
            return self.refresh_news_feed(feed_id, &feed).await;
        }

        FeedNotFoundSnafu {
            feed_id: feed_id.to_string(),
        }
        .fail()
    }

    /// List all subscriptions of the given media type.
    pub async fn list_feeds(&self, media_type: MediaType) -> Result<Vec<FeedSummary>, KomideError> {
        match media_type {
            MediaType::Podcast => {
                let subs = podcast::list_subscriptions(&self.db.read, 1000, 0)
                    .await
                    .context(DatabaseSnafu)?;
                Ok(subs
                    .into_iter()
                    .map(|s| FeedSummary {
                        id: bytes_to_feed_id(&s.id),
                        title: s.title.unwrap_or_default(),
                        url: s.feed_url,
                        media_type: MediaType::Podcast,
                        last_fetched_at: s.last_checked_at,
                        is_active: true,
                    })
                    .collect())
            }
            MediaType::News => {
                let feeds = news::list_feeds(&self.db.read, 1000, 0)
                    .await
                    .context(DatabaseSnafu)?;
                Ok(feeds
                    .into_iter()
                    .map(|f| FeedSummary {
                        id: bytes_to_feed_id(&f.id),
                        title: f.title,
                        url: f.url,
                        media_type: MediaType::News,
                        last_fetched_at: f.last_fetched_at,
                        is_active: f.is_active != 0,
                    })
                    .collect())
            }
            other => {
                warn!(media_type = %other, "list_feeds called for non-feed media type");
                Ok(vec![])
            }
        }
    }

    /// Mark a podcast episode or news article as consumed (listened/read).
    #[instrument(skip(self))]
    pub async fn mark_consumed(&self, item_id: MediaId) -> Result<(), KomideError> {
        let id_bytes = item_id.as_bytes().to_vec();

        // Try episode first
        if podcast::get_episode(&self.db.read, &id_bytes)
            .await
            .context(DatabaseSnafu)?
            .is_some()
        {
            podcast::update_episode(&self.db.write, &id_bytes, 1, None, None)
                .await
                .context(DatabaseSnafu)?;
            return Ok(());
        }

        // Try article
        if news::get_article(&self.db.read, &id_bytes)
            .await
            .context(DatabaseSnafu)?
            .is_some()
        {
            news::update_article(&self.db.write, &id_bytes, 1, 0)
                .await
                .context(DatabaseSnafu)?;
            return Ok(());
        }

        // Not found in either table — silently succeed (idempotent)
        debug!(item_id = %item_id, "mark_consumed: item not found, ignoring");
        Ok(())
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    async fn refresh_podcast_feed(
        &self,
        feed_id: FeedId,
        sub: &podcast::PodcastSubscription,
    ) -> Result<FeedRefreshResult, KomideError> {
        let url = &sub.feed_url;
        let (etag, last_modified) = self.cached_validators(url).await;

        let fetch_result =
            fetch_feed(&self.client, url, etag.as_deref(), last_modified.as_deref()).await?;

        match fetch_result {
            FetchResult::NotModified => {
                debug!(feed_id = %feed_id, "podcast feed not modified (304)");
                let total =
                    podcast::count_episodes_for_subscription(&self.db.read, sub.id.as_slice())
                        .await
                        .context(DatabaseSnafu)? as usize;
                Ok(FeedRefreshResult {
                    feed_id,
                    new_items: 0,
                    total_items: total,
                })
            }
            FetchResult::Content {
                bytes,
                etag: new_etag,
                last_modified: new_lm,
            } => {
                self.store_validators(url, new_etag, new_lm).await;
                let parsed = parse_feed(&bytes)?;
                let now = now_iso8601();

                let new_count = self
                    .insert_new_podcast_episodes(&sub.id, &parsed.entries, &now)
                    .await?;

                podcast::update_subscription(
                    &self.db.write,
                    &sub.id,
                    sub.title.as_deref(),
                    sub.auto_download,
                    Some(&now),
                )
                .await
                .context(DatabaseSnafu)?;

                let total = podcast::count_episodes_for_subscription(&self.db.read, &sub.id)
                    .await
                    .context(DatabaseSnafu)? as usize;

                self.emit_feed_refreshed(feed_id, new_count, MediaType::Podcast);
                Ok(FeedRefreshResult {
                    feed_id,
                    new_items: new_count,
                    total_items: total,
                })
            }
        }
    }

    async fn refresh_news_feed(
        &self,
        feed_id: FeedId,
        feed: &news::NewsFeed,
    ) -> Result<FeedRefreshResult, KomideError> {
        let url = &feed.url;
        let (etag, last_modified) = self.cached_validators(url).await;

        let fetch_result =
            fetch_feed(&self.client, url, etag.as_deref(), last_modified.as_deref()).await?;

        match fetch_result {
            FetchResult::NotModified => {
                debug!(feed_id = %feed_id, "news feed not modified (304)");
                let total = news::count_articles_for_feed(&self.db.read, feed.id.as_slice())
                    .await
                    .context(DatabaseSnafu)? as usize;
                Ok(FeedRefreshResult {
                    feed_id,
                    new_items: 0,
                    total_items: total,
                })
            }
            FetchResult::Content {
                bytes,
                etag: new_etag,
                last_modified: new_lm,
            } => {
                self.store_validators(url, new_etag, new_lm).await;
                let parsed = parse_feed(&bytes)?;
                let now = now_iso8601();

                let new_count = self
                    .insert_new_articles(&feed.id, &parsed.entries, &now)
                    .await?;

                news::update_feed(
                    &self.db.write,
                    &feed.id,
                    &parsed.title,
                    feed.is_active,
                    Some(&now),
                    &now,
                )
                .await
                .context(DatabaseSnafu)?;

                // Apply retention after inserting
                apply_retention(
                    &self.db,
                    &feed.id,
                    self.config.news_retention_days,
                    self.config.news_retention_articles,
                )
                .await?;

                let total = news::count_articles_for_feed(&self.db.read, &feed.id)
                    .await
                    .context(DatabaseSnafu)? as usize;

                self.emit_feed_refreshed(feed_id, new_count, MediaType::News);
                Ok(FeedRefreshResult {
                    feed_id,
                    new_items: new_count,
                    total_items: total,
                })
            }
        }
    }

    async fn insert_new_podcast_episodes(
        &self,
        subscription_id: &[u8],
        entries: &[crate::parser::NormalizedEntry],
        now: &str,
    ) -> Result<usize, KomideError> {
        let mut count = 0;

        for entry in entries {
            if podcast::episode_guid_exists(&self.db.read, subscription_id, &entry.guid)
                .await
                .context(DatabaseSnafu)?
            {
                continue;
            }

            let enclosure = extract_audio_enclosure(entry);
            let episode_id = EpisodeId::new();
            let ep_bytes = episode_id.as_bytes().to_vec();

            let ep = podcast::PodcastEpisode {
                id: ep_bytes,
                subscription_id: subscription_id.to_vec(),
                guid: entry.guid.clone(),
                title: Some(entry.title.clone()),
                description: entry.summary.clone(),
                episode_number: None,
                season_number: None,
                publication_date: entry.published.clone(),
                duration_ms: None,
                enclosure_url: enclosure.map(|e| e.url.clone()),
                file_path: None,
                file_size_bytes: None,
                file_format: enclosure.and_then(|e| e.content_type.clone()),
                quality_score: None,
                source_type: "rss".to_string(),
                listened: 0,
                added_at: now.to_string(),
            };
            podcast::insert_episode(&self.db.write, &ep)
                .await
                .context(DatabaseSnafu)?;

            // Emit event for new episode
            let sub_id =
                FeedId::from_uuid(Uuid::from_slice(subscription_id).unwrap_or(Uuid::nil()));
            self.emit_episode_available(sub_id, episode_id, &entry.title);

            count += 1;
        }

        Ok(count)
    }

    async fn insert_new_articles(
        &self,
        feed_id: &[u8],
        entries: &[crate::parser::NormalizedEntry],
        now: &str,
    ) -> Result<usize, KomideError> {
        let mut count = 0;

        for entry in entries {
            if news::article_guid_exists(&self.db.read, feed_id, &entry.guid)
                .await
                .context(DatabaseSnafu)?
            {
                continue;
            }

            let article = news::NewsArticle {
                id: MediaId::new().as_bytes().to_vec(),
                feed_id: feed_id.to_vec(),
                guid: entry.guid.clone(),
                title: entry.title.clone(),
                url: entry.link.clone().unwrap_or_default(),
                author: None,
                content_html: entry.content.clone(),
                summary: entry.summary.clone(),
                published_at: entry.published.clone(),
                is_read: 0,
                is_starred: 0,
                source_type: "rss".to_string(),
                added_at: now.to_string(),
            };
            news::insert_article(&self.db.write, &article)
                .await
                .context(DatabaseSnafu)?;

            count += 1;
        }

        Ok(count)
    }

    async fn cached_validators(&self, url: &str) -> (Option<String>, Option<String>) {
        let cache = self.cache_validators.lock().await;
        cache.get(url).cloned().unwrap_or((None, None))
    }

    async fn store_validators(
        &self,
        url: &str,
        etag: Option<String>,
        last_modified: Option<String>,
    ) {
        if etag.is_some() || last_modified.is_some() {
            let mut cache = self.cache_validators.lock().await;
            cache.insert(url.to_string(), (etag, last_modified));
        }
    }

    fn emit_feed_refreshed(&self, feed_id: FeedId, new_items: usize, media_type: MediaType) {
        let _ = self
            .event_tx
            .send(harmonia_common::aggelia::HarmoniaEvent::FeedRefreshed {
                feed_id,
                new_items,
                media_type,
            });
    }

    fn emit_episode_available(&self, subscription_id: FeedId, episode_id: EpisodeId, title: &str) {
        let _ = self
            .event_tx
            .send(harmonia_common::aggelia::HarmoniaEvent::EpisodeAvailable {
                subscription_id,
                episode_id,
                title: title.to_string(),
            });
    }
}

fn validate_url(url: &str) -> Result<(), KomideError> {
    if url.is_empty() || (!url.starts_with("http://") && !url.starts_with("https://")) {
        return InvalidUrlSnafu {
            url: url.to_string(),
        }
        .fail();
    }
    Ok(())
}

async fn fetch_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, KomideError> {
    use crate::error::FeedFetchSnafu;
    client
        .get(url)
        .send()
        .await
        .context(FeedFetchSnafu {
            url: url.to_string(),
        })?
        .bytes()
        .await
        .context(FeedFetchSnafu {
            url: url.to_string(),
        })
        .map(|b| b.to_vec())
}

pub(crate) fn bytes_to_feed_id(bytes: &[u8]) -> FeedId {
    Uuid::from_slice(bytes)
        .map(FeedId::from_uuid)
        .unwrap_or_else(|_| FeedId::new())
}

fn now_iso8601() -> String {
    jiff::Timestamp::now()
        .strftime("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use harmonia_common::aggelia::create_event_bus;
    use harmonia_db::{DbPools, migrate::MIGRATOR};
    use sqlx::SqlitePool;

    async fn setup() -> (KomideService, harmonia_common::aggelia::EventReceiver) {
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
            harmonia_common::aggelia::HarmoniaEvent::EpisodeAvailable { .. }
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
}
