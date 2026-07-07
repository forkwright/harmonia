use std::collections::HashMap;

use apotheke::DbPools;
use apotheke::repo::{news, podcast};
use horismos::KomideConfig;
use snafu::{OptionExt, ResultExt};
use themelion::aggelia::EventSender;
use themelion::ids::{EpisodeId, FeedId, MediaId};
use themelion::media::MediaType;
use tracing::{Instrument, debug, info, instrument, warn};
use uuid::Uuid;

use crate::error::{
    CorruptFeedIdSnafu, DatabaseSnafu, EpisodeIoSnafu, EpisodeNotDownloadableSnafu,
    EpisodeNotFoundSnafu, FeedNotFoundSnafu, InvalidUrlSnafu, KomideError,
};
use crate::fetch::{FetchResult, fetch_feed};
use crate::news::apply_retention;
use crate::parser::parse_feed;
use crate::podcast::extract_audio_enclosure;

// WHY: pure data — result of a feed refresh operation.
pub struct FeedRefreshResult {
    pub feed_id: FeedId,
    pub new_items: usize,
    pub total_items: usize,
}

// WHY: pure data — summary of a podcast feed.
pub struct FeedSummary {
    pub id: FeedId,
    pub title: String,
    pub url: String,
    pub media_type: MediaType,
    pub last_fetched_at: Option<String>,
    pub is_active: bool,
}

pub struct FeedSchedulerService {
    pub(crate) db: DbPools,
    event_tx: EventSender,
    client: reqwest::Client,
    pub(crate) config: KomideConfig,
    /// Stores (etag, last_modified) keyed by feed URL for conditional requests.
    cache_validators: tokio::sync::Mutex<HashMap<String, (Option<String>, Option<String>)>>,
}

impl FeedSchedulerService {
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
    ///
    /// `auto_download` sets the subscription's episode-download COUNT:
    /// `Some(false)` stores 0 (poll the feed, download nothing), anything
    /// else stores the configured `auto_download_latest_n`. Polling never
    /// depends on this value — every subscription gets a poll loop.
    #[instrument(skip(self), fields(url))]
    pub async fn subscribe_podcast(
        &self,
        url: &str,
        label: Option<&str>,
        auto_download: Option<bool>,
    ) -> Result<FeedId, KomideError> {
        validate_url(url)?;

        // Check for existing subscription at this URL
        if let Some(existing) = podcast::subscription_by_url(&self.db.read, url)
            .await
            .context(DatabaseSnafu)?
        {
            let id = bytes_to_feed_id(&existing.id)?;
            return Ok(id);
        }

        // Fetch and parse to populate metadata
        let feed_bytes = fetch_bytes(&self.client, url, self.config.max_feed_bytes).await?;
        let parsed = parse_feed(&feed_bytes)?;

        let feed_id = FeedId::new();
        let id_bytes = feed_id.as_bytes().to_vec();
        let now = now_iso8601();

        let auto_download_latest_n = match auto_download {
            Some(false) => 0,
            _ => self.config.auto_download_latest_n,
        };
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
            auto_download: i64::try_from(auto_download_latest_n).unwrap_or_default(), // WHY: auto_download_latest_n is a small config value; bounded within i64
            quality_profile_id: None,
            added_at: now.clone(),
        };
        podcast::insert_subscription(&self.db.write, &sub)
            .await
            .context(DatabaseSnafu)?;

        self.emit_feed_set_changed(feed_id, MediaType::Podcast);

        // Insert initial episodes
        let new_count = self
            .insert_new_podcast_episodes(&id_bytes, &parsed.entries, &now)
            .await?;

        self.spawn_auto_downloads(id_bytes, auto_download_latest_n);

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
        category: Option<&str>,
    ) -> Result<FeedId, KomideError> {
        validate_url(url)?;

        // Check for existing feed at this URL
        if let Some(existing) = news::feed_by_url(&self.db.read, url)
            .await
            .context(DatabaseSnafu)?
        {
            return bytes_to_feed_id(&existing.id);
        }

        let feed_bytes = fetch_bytes(&self.client, url, self.config.max_feed_bytes).await?;
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
            category: category.map(str::to_owned),
            icon_url: parsed.image_url.clone(),
            last_fetched_at: Some(now.clone()),
            fetch_interval_minutes: i64::try_from(self.config.news_poll_interval_minutes)
                .unwrap_or_default(), // WHY: news_poll_interval_minutes is a small config value; bounded within i64
            is_active: 1,
            added_at: now.clone(),
            updated_at: now.clone(),
        };
        news::insert_feed(&self.db.write, &feed)
            .await
            .context(DatabaseSnafu)?;

        self.emit_feed_set_changed(feed_id, MediaType::News);

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
            self.emit_feed_set_changed(feed_id, MediaType::Podcast);
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
            self.emit_feed_set_changed(feed_id, MediaType::News);
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
                subs.into_iter()
                    .map(|s| {
                        Ok(FeedSummary {
                            id: bytes_to_feed_id(&s.id)?,
                            title: s.title.unwrap_or_default(),
                            url: s.feed_url,
                            media_type: MediaType::Podcast,
                            last_fetched_at: s.last_checked_at,
                            is_active: true,
                        })
                    })
                    .collect()
            }
            MediaType::News => {
                let feeds = news::list_feeds(&self.db.read, 1000, 0)
                    .await
                    .context(DatabaseSnafu)?;
                feeds
                    .into_iter()
                    .map(|f| {
                        Ok(FeedSummary {
                            id: bytes_to_feed_id(&f.id)?,
                            title: f.title,
                            url: f.url,
                            media_type: MediaType::News,
                            last_fetched_at: f.last_fetched_at,
                            is_active: f.is_active != 0,
                        })
                    })
                    .collect()
            }
            other => {
                warn!(media_type = %other, "list_feeds called for non-feed media type");
                Ok(vec![])
            }
        }
    }

    /// Check an episode exists and carries a downloadable audio enclosure —
    /// the cheap preflight paroche's episode-download route runs before
    /// spawning the actual transfer.
    pub async fn episode_downloadable(&self, episode_id: EpisodeId) -> Result<(), KomideError> {
        self.episode_for_download(episode_id).await.map(|_| ())
    }

    /// Download one episode's audio enclosure into `podcast_dir`, persisting
    /// the file location and size on the episode row. Returns bytes written.
    #[instrument(skip(self))]
    pub async fn download_episode_by_id(&self, episode_id: EpisodeId) -> Result<u64, KomideError> {
        let (ep, url) = self.episode_for_download(episode_id).await?;
        transfer_episode(&self.db, &self.client, &self.config, &ep, &url).await
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

        // Not found in either table  -  silently succeed (idempotent)
        debug!(item_id = %item_id, "mark_consumed: item not found, ignoring");
        Ok(())
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    async fn episode_for_download(
        &self,
        episode_id: EpisodeId,
    ) -> Result<(podcast::PodcastEpisode, String), KomideError> {
        let ep = podcast::get_episode(&self.db.read, episode_id.as_bytes())
            .await
            .context(DatabaseSnafu)?
            .context(EpisodeNotFoundSnafu {
                episode_id: episode_id.to_string(),
            })?;
        let url = ep
            .enclosure_url
            .clone()
            .context(EpisodeNotDownloadableSnafu {
                episode_id: episode_id.to_string(),
            })?;
        Ok((ep, url))
    }

    /// Fire-and-forget the initial episode downloads for a new subscription.
    ///
    /// WHY spawned: subscribe is HTTP-facing (paroche delegates to it) and
    /// must return before the API timeout, while episode audio can be
    /// arbitrarily large. The scheduler's refresh path downloads INLINE
    /// instead — its poll task is the natural backpressure.
    fn spawn_auto_downloads(&self, subscription_id: Vec<u8>, latest_n: u64) {
        if latest_n == 0 {
            return;
        }
        let db = DbPools {
            read: self.db.read.clone(),
            write: self.db.write.clone(),
        };
        let client = self.client.clone();
        let config = self.config.clone();
        tokio::spawn(
            async move {
                match download_latest_episodes(&db, &client, &config, &subscription_id, latest_n)
                    .await
                {
                    Ok(count) => {
                        info!(episodes = count, "initial episode auto-download complete");
                    }
                    Err(e) => warn!(error = %e, "initial episode auto-download failed"),
                }
            }
            .instrument(tracing::info_span!("episode_auto_download")),
        );
    }

    async fn refresh_podcast_feed(
        &self,
        feed_id: FeedId,
        sub: &podcast::PodcastSubscription,
    ) -> Result<FeedRefreshResult, KomideError> {
        let url = &sub.feed_url;
        let (etag, last_modified) = self.cached_validators(url).await;

        let fetch_result = fetch_feed(
            &self.client,
            url,
            etag.as_deref(),
            last_modified.as_deref(),
            self.config.max_feed_bytes,
        )
        .await?;

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

                if new_count > 0 {
                    let latest_n = u64::try_from(sub.auto_download).unwrap_or_default(); // WHY: auto_download is a small episode count; a corrupt negative row means "download nothing"
                    // WHY inline (not spawned): the caller is the scheduler's
                    // per-feed poll task — awaiting here is the natural
                    // backpressure for bulk audio transfers. A failed
                    // download must not fail the refresh: the feed fetch
                    // itself succeeded.
                    if let Err(e) = download_latest_episodes(
                        &self.db,
                        &self.client,
                        &self.config,
                        &sub.id,
                        latest_n,
                    )
                    .await
                    {
                        warn!(feed_id = %feed_id, error = %e, "episode auto-download after refresh failed");
                    }
                }

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

        let fetch_result = fetch_feed(
            &self.client,
            url,
            etag.as_deref(),
            last_modified.as_deref(),
            self.config.max_feed_bytes,
        )
        .await?;

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
            .send(themelion::aggelia::HarmoniaEvent::FeedRefreshed {
                feed_id,
                new_items,
                media_type,
            });
    }

    fn emit_feed_set_changed(&self, feed_id: FeedId, media_type: MediaType) {
        let _ = self
            .event_tx
            .send(themelion::aggelia::HarmoniaEvent::FeedSetChanged {
                feed_id,
                media_type,
            });
    }

    fn emit_episode_available(&self, subscription_id: FeedId, episode_id: EpisodeId, title: &str) {
        let _ = self
            .event_tx
            .send(themelion::aggelia::HarmoniaEvent::EpisodeAvailable {
                subscription_id,
                episode_id,
                title: title.to_string(),
            });
    }
}

/// Download the audio enclosures of a subscription's most recent episodes
/// (up to `latest_n`) that have no local file yet. A single failed episode
/// is logged and skipped — one dead enclosure must not block the rest.
/// Returns how many episode files were written.
pub(crate) async fn download_latest_episodes(
    db: &DbPools,
    client: &reqwest::Client,
    config: &KomideConfig,
    subscription_id: &[u8],
    latest_n: u64,
) -> Result<usize, KomideError> {
    if latest_n == 0 {
        return Ok(0);
    }
    let total = podcast::count_episodes_for_subscription(&db.read, subscription_id)
        .await
        .context(DatabaseSnafu)?;
    let total = usize::try_from(total).unwrap_or_default(); // WHY: a COUNT(*) is non-negative; a corrupt value degrades to "nothing to download"
    let want = crate::podcast::episodes_to_download(total, latest_n);
    if want == 0 {
        return Ok(0);
    }

    let recent = podcast::list_recent_episodes(
        &db.read,
        subscription_id,
        i64::try_from(want).unwrap_or(i64::MAX), // WHY: want is bounded by latest_n, a small config value
    )
    .await
    .context(DatabaseSnafu)?;

    let mut downloaded = 0;
    for ep in recent {
        if ep.file_path.is_some() {
            continue;
        }
        let Some(url) = ep.enclosure_url.clone() else {
            continue;
        };
        match transfer_episode(db, client, config, &ep, &url).await {
            Ok(_) => downloaded += 1,
            Err(e) => {
                warn!(
                    episode_title = ep.title.as_deref().unwrap_or(""),
                    url,
                    error = %e,
                    "episode auto-download failed; continuing with the rest"
                );
            }
        }
    }
    Ok(downloaded)
}

/// Fetch one episode's enclosure into `podcast_dir` (capped at
/// `max_episode_bytes`) and persist the file location on the episode row.
/// Returns the bytes written.
async fn transfer_episode(
    db: &DbPools,
    client: &reqwest::Client,
    config: &KomideConfig,
    ep: &podcast::PodcastEpisode,
    url: &str,
) -> Result<u64, KomideError> {
    tokio::fs::create_dir_all(&config.podcast_dir)
        .await
        .context(EpisodeIoSnafu {
            path: config.podcast_dir.display().to_string(),
        })?;
    let dest = config.podcast_dir.join(episode_file_name(ep, url));
    let written =
        crate::fetch::download_episode(client, url, &dest, config.max_episode_bytes).await?;
    podcast::set_episode_file(
        &db.write,
        &ep.id,
        &dest.to_string_lossy(),
        i64::try_from(written).unwrap_or(i64::MAX), // WHY: written is capped by max_episode_bytes, well within i64
    )
    .await
    .context(DatabaseSnafu)?;
    info!(path = %dest.display(), bytes = written, "episode downloaded");
    Ok(written)
}

/// `{episode-uuid}.{ext}` — the uuid keeps names collision-free; the
/// extension comes from the enclosure URL's path, falling back to `mp3`
/// (the dominant podcast enclosure format).
fn episode_file_name(ep: &podcast::PodcastEpisode, url: &str) -> String {
    let id = Uuid::from_slice(&ep.id).unwrap_or(Uuid::nil());
    format!("{id}.{}", enclosure_extension(url))
}

fn enclosure_extension(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|parsed| {
            let path = parsed.path().to_string();
            let name = path.rsplit('/').next()?.to_string();
            let (_, ext) = name.rsplit_once('.')?;
            let ext = ext.to_ascii_lowercase();
            (!ext.is_empty() && ext.len() <= 4 && ext.chars().all(|c| c.is_ascii_alphanumeric()))
                .then_some(ext)
        })
        .unwrap_or_else(|| "mp3".to_string())
}

fn validate_url(input: &str) -> Result<(), KomideError> {
    let parsed = url::Url::parse(input).map_err(|_| {
        InvalidUrlSnafu {
            url: input.to_string(),
        }
        .build()
    })?;
    match parsed.scheme() {
        "http" | "https" if parsed.has_host() => Ok(()),
        _ => InvalidUrlSnafu {
            url: input.to_string(),
        }
        .fail(),
    }
}

async fn fetch_bytes(
    client: &reqwest::Client,
    url: &str,
    max_bytes: u64,
) -> Result<Vec<u8>, KomideError> {
    use crate::error::FeedFetchSnafu;
    let response = client
        .get(url)
        .send()
        .await
        .context(FeedFetchSnafu {
            url: url.to_string(),
        })?
        .error_for_status()
        .context(FeedFetchSnafu {
            url: url.to_string(),
        })?;
    crate::fetch::read_body_capped(response, url, max_bytes).await
}

// WHY: fallible by design — silently minting a fresh FeedId on corrupt DB
// bytes masked data corruption as a legitimately new feed.
pub(crate) fn bytes_to_feed_id(bytes: &[u8]) -> Result<FeedId, KomideError> {
    Uuid::from_slice(bytes)
        .map(FeedId::from_uuid)
        .context(CorruptFeedIdSnafu)
}

fn now_iso8601() -> String {
    jiff::Timestamp::now()
        .strftime("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
