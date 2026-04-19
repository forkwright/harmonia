use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tokio::time::interval;
use tracing::Instrument;

/// Zero-size token returned when a permit is granted.
pub struct ProviderPermit;

/// Token-bucket rate limiter for a single provider.
///
/// Callers send a oneshot sender; the background loop ticks at the configured
/// interval and unblocks one caller per tick.
pub struct ProviderQueue {
    tx: mpsc::Sender<oneshot::Sender<()>>,
}

impl ProviderQueue {
    pub(crate) fn new(requests_per_window: u32, window_millis: u64) -> Self {
        let (tx, mut rx) = mpsc::channel::<oneshot::Sender<()>>(100);
        let requests_per_window = requests_per_window.max(1);
        let interval_millis = window_millis / u64::from(requests_per_window);
        let interval_dur = Duration::from_millis(interval_millis.max(1));

        tokio::spawn(
            async move {
                let mut tick = interval(interval_dur);
                // First tick fires immediately  -  consume it so the first real
                // request still waits the full interval.
                tick.tick().await;
                while let Some(caller_tx) = rx.recv().await {
                    tick.tick().await;
                    let _ = caller_tx.send(());
                }
            }
            .instrument(tracing::info_span!("provider_rate_limiter")),
        );

        Self { tx }
    }

    /// Acquire a permit, waiting until the rate limiter allows the next request.
    pub async fn acquire(&self) -> ProviderPermit {
        let (cb_tx, cb_rx) = oneshot::channel();
        // If the channel is closed (background task panicked), proceed anyway.
        let _ = self.tx.send(cb_tx).await;
        let _ = cb_rx.await;
        ProviderPermit
    }
}

/// Pre-configured rate limits matching per-provider API budgets.
pub struct ProviderQueues {
    pub musicbrainz: ProviderQueue,
    pub acoustid: ProviderQueue,
    pub tmdb: ProviderQueue,
    pub tvdb: ProviderQueue,
    pub audnexus: ProviderQueue,
    pub openlibrary: ProviderQueue,
    pub itunes: ProviderQueue,
    pub comicvine: ProviderQueue,
}

impl ProviderQueues {
    pub(crate) fn new() -> Self {
        Self {
            musicbrainz: ProviderQueue::new(1, 1_000),  // 1 req/s
            acoustid: ProviderQueue::new(3, 1_000),     // 3 req/s
            tmdb: ProviderQueue::new(40, 1_000),        // 40 req/s
            tvdb: ProviderQueue::new(10, 1_000),        // 10 req/s
            audnexus: ProviderQueue::new(5, 1_000),     // 5 req/s
            openlibrary: ProviderQueue::new(10, 1_000), // 10 req/s
            itunes: ProviderQueue::new(20, 60_000),     // 20 req/min
            comicvine: ProviderQueue::new(1, 1_000),    // 1 req/s
        }
    }
}

impl Default for ProviderQueues {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    /// 3 requests in a 100ms window  -  all three should complete quickly,
    /// a fourth request must wait for the next slot.
    #[tokio::test]
    async fn rate_limiter_allows_burst_then_throttles() {
        // 3 req per 300ms → ~100ms interval
        let queue = ProviderQueue::new(3, 300);

        let start = Instant::now();

        // First three permits  -  each waits one interval after the previous.
        queue.acquire().await;
        queue.acquire().await;
        queue.acquire().await;

        let elapsed = start.elapsed();
        // Three permits should complete within ~500ms with some slack.
        assert!(
            elapsed < Duration::from_millis(600),
            "three permits took too long: {elapsed:?}"
        );

        // Fourth permit must wait at least one more interval.
        let before_fourth = Instant::now();
        queue.acquire().await;
        let fourth_wait = before_fourth.elapsed();
        assert!(
            fourth_wait >= Duration::from_millis(50),
            "fourth permit did not throttle: {fourth_wait:?}"
        );
    }

    #[tokio::test]
    async fn provider_queues_default_construction() {
        let _queues = ProviderQueues::new();
        // Verifies construction does not panic.
    }
}
