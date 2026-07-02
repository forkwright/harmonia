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
    pub fn new(requests_per_window: u32, window_millis: u64) -> Self {
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
                    // WHY: send fails only when caller dropped; rate-limit permit delivery is best-effort
                    caller_tx.send(()).ok();
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
        // WHY: send fails only when background task has shut down; proceed without rate-limiting
        self.tx.send(cb_tx).await.ok();
        // WHY: permit release send fails only when channel closed; intentional on shutdown
        cb_rx.await.ok();
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
    pub google_books: ProviderQueue,
    pub itunes: ProviderQueue,
    pub comicvine: ProviderQueue,
}

impl ProviderQueues {
    pub fn new() -> Self {
        Self {
            musicbrainz: ProviderQueue::new(1, 1_000),  // 1 req/s
            acoustid: ProviderQueue::new(3, 1_000),     // 3 req/s
            tmdb: ProviderQueue::new(40, 1_000),        // 40 req/s
            tvdb: ProviderQueue::new(10, 1_000),        // 10 req/s
            audnexus: ProviderQueue::new(5, 1_000),     // 5 req/s
            openlibrary: ProviderQueue::new(10, 1_000), // 10 req/s
            google_books: ProviderQueue::new(1, 1_000), // 1 req/s
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
    use tokio::time::Instant;

    use super::*;

    /// 3 requests per 300ms window — each permit waits one 100ms interval.
    ///
    /// WHY: `start_paused` runs the limiter's `tokio::time::interval` on the
    /// mock clock, so the schedule is exact logical time — no wall-clock
    /// slack, no flake under load.
    #[tokio::test(start_paused = true)]
    async fn rate_limiter_allows_burst_then_throttles() {
        // 3 req per 300ms → 100ms interval
        let queue = ProviderQueue::new(3, 300);

        let start = Instant::now();

        queue.acquire().await;
        queue.acquire().await;
        queue.acquire().await;

        assert_eq!(
            start.elapsed(),
            Duration::from_millis(300),
            "each of the first three permits waits exactly one interval"
        );

        let before_fourth = Instant::now();
        queue.acquire().await;
        assert_eq!(
            before_fourth.elapsed(),
            Duration::from_millis(100),
            "the fourth permit waits exactly one more interval"
        );
    }

    #[tokio::test]
    async fn provider_queues_default_construction() {
        let _queues = ProviderQueues::new();
        // Verifies construction does not panic.
    }
}
