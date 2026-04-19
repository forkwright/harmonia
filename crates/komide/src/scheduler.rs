use std::time::Duration;

use apotheke::DbPools;
use horismos::KomideConfig;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{Instrument, debug, error, info, instrument};

use crate::error::KomideError;
use crate::service::KomideService;

const MAX_BACKOFF_MINUTES: u64 = 240; // 4 hours
const JITTER_PERCENT: f64 = 10.0;

/// Per-feed polling state tracked by the scheduler.
#[derive(Debug)]
struct FeedState {
    base_interval_minutes: u64,
    failure_count: u32,
}

impl FeedState {
    fn new(base_interval_minutes: u64) -> Self {
        Self {
            base_interval_minutes,
            failure_count: 0,
        }
    }

    /// Compute polling interval with exponential backoff on failures.
    ///
    /// On success the base interval is used. Each consecutive failure doubles
    /// the interval up to `MAX_BACKOFF_MINUTES`.
    fn current_interval_minutes(&self) -> u64 {
        if self.failure_count == 0 {
            return self.base_interval_minutes;
        }
        let backed_off = self.base_interval_minutes * 2u64.pow(self.failure_count);
        backed_off.min(MAX_BACKOFF_MINUTES)
    }

    /// Apply ±10% jitter to a duration to avoid thundering-herd fetches.
    fn with_jitter(minutes: u64) -> Duration {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Deterministic but spread jitter: seed FROM current thread id + time
        let mut hasher = DefaultHasher::new();
        std::time::SystemTime::now().hash(&mut hasher);
        std::thread::current().id().hash(&mut hasher);
        let hash = hasher.finish();

        let jitter_range = (minutes as f64 * JITTER_PERCENT / 100.0) as u64;
        let jitter = if jitter_range == 0 {
            0
        } else {
            // Map hash INTO [0, 2*jitter_range] then subtract jitter_range → [-range, +range]
            (hash % (jitter_range * 2 + 1)) as i64 - i64::try_from(jitter_range).unwrap_or_default()
        };

        let adjusted = (i64::try_from(minutes).unwrap_or_default() + jitter).max(1) as u64;
        Duration::from_secs(adjusted * 60)
    }
}

/// Manages background polling tasks for all active feeds.
pub struct FeedScheduler {
    handles: Vec<JoinHandle<()>>,
}

impl FeedScheduler {
    /// Start scheduler tasks for all active podcast and news feeds.
    ///
    /// Loads feeds FROM the database, then spawns a tokio task per feed that
    /// polls on the configured interval (with jitter and backoff).
    #[instrument(skip_all)]
    pub async fn start(
        service: std::sync::Arc<KomideService>,
        config: KomideConfig,
        db: DbPools,
    ) -> Result<Self, KomideError> {
        let mut handles = Vec::new();

        let podcast_subs = apotheke::repo::podcast::list_subscriptions(&db.read, 1000, 0)
            .await
            .map_err(|source| KomideError::Database {
                source,
                location: snafu::location!(),
            })?;

        for sub in podcast_subs {
            if sub.auto_download == 0 {
                continue;
            }
            let interval = config.podcast_poll_interval_minutes;
            let state = FeedState::new(interval);
            let svc = service.clone();
            let feed_id_uuid = uuid::Uuid::from_slice(&sub.id).ok();

            let handle = tokio::spawn(
                async move {
                    poll_feed_loop(svc, state, feed_id_uuid).await;
                }
                .instrument(tracing::info_span!("poll_feed", feed_id = ?feed_id_uuid)),
            );
            handles.push(handle);
        }

        let news_feeds = apotheke::repo::news::list_feeds(&db.read, 1000, 0)
            .await
            .map_err(|source| KomideError::Database {
                source,
                location: snafu::location!(),
            })?;

        for feed in news_feeds {
            if feed.is_active == 0 {
                continue;
            }
            let interval = u64::try_from(feed.fetch_interval_minutes).unwrap_or_default();
            let state = FeedState::new(interval);
            let svc = service.clone();
            let feed_id_uuid = uuid::Uuid::from_slice(&feed.id).ok();

            let handle = tokio::spawn(
                async move {
                    poll_feed_loop(svc, state, feed_id_uuid).await;
                }
                .instrument(tracing::info_span!("poll_feed", feed_id = ?feed_id_uuid)),
            );
            handles.push(handle);
        }

        info!(feeds = handles.len(), "feed scheduler started");
        Ok(Self { handles })
    }

    /// Abort all scheduled polling tasks.
    pub fn shutdown(self) {
        for handle in self.handles {
            handle.abort();
        }
    }
}

async fn poll_feed_loop(
    service: std::sync::Arc<KomideService>,
    mut state: FeedState,
    feed_id_uuid: Option<uuid::Uuid>,
) {
    let Some(uuid) = feed_id_uuid else {
        error!("invalid feed id bytes, skipping poll loop");
        return;
    };
    let feed_id = themelion::ids::FeedId::from_uuid(uuid);

    loop {
        let interval = FeedState::with_jitter(state.current_interval_minutes());
        debug!(
            feed_id = %feed_id,
            interval_secs = interval.as_secs(),
            failures = state.failure_count,
            "scheduling next feed poll"
        );
        sleep(interval).await;

        match service.refresh_feed(feed_id).await {
            Ok(result) => {
                info!(
                    feed_id = %feed_id,
                    new_items = result.new_items,
                    "feed refreshed"
                );
                state.failure_count = 0;
            }
            Err(e) => {
                error!(feed_id = %feed_id, error = %e, "feed refresh failed");
                state.failure_count = state.failure_count.saturating_add(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_on_each_failure() {
        let mut state = FeedState::new(30);
        assert_eq!(state.current_interval_minutes(), 30);

        state.failure_count = 1;
        assert_eq!(state.current_interval_minutes(), 60);

        state.failure_count = 2;
        assert_eq!(state.current_interval_minutes(), 120);

        state.failure_count = 3;
        assert_eq!(state.current_interval_minutes(), 240);
    }

    #[test]
    fn backoff_capped_at_four_hours() {
        let mut state = FeedState::new(30);
        state.failure_count = 10;
        assert_eq!(state.current_interval_minutes(), MAX_BACKOFF_MINUTES);
    }

    #[test]
    fn jitter_within_ten_percent() {
        let base_minutes = 30u64;
        let expected_min = (base_minutes as f64 * 0.9) as u64;
        let expected_max = (base_minutes as f64 * 1.1) as u64;

        // Run multiple samples to check spread
        let mut seen_durations = std::collections::HashSet::new();
        for _ in 0..20 {
            let d = FeedState::with_jitter(base_minutes);
            let minutes = d.as_secs() / 60;
            assert!(
                minutes >= expected_min && minutes <= expected_max,
                "jitter out of range: {minutes}m, expected [{expected_min}, {expected_max}]"
            );
            seen_durations.insert(d.as_secs());
        }
        // Jitter should produce some variation across 20 samples
        assert!(
            seen_durations.len() > 1,
            "jitter should vary across samples"
        );
    }

    #[test]
    fn failure_count_reset_on_success() {
        let mut state = FeedState::new(30);
        state.failure_count = 3;
        assert_eq!(state.current_interval_minutes(), 240);

        state.failure_count = 0;
        assert_eq!(state.current_interval_minutes(), 30);
    }
}
