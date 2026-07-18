use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

use dashmap::DashMap;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

/// Per-indexer rate limiting with a live-reconfigurable ceiling.
///
/// `max_tokens`/`window` are atomics (not plain fields) because `reconfigure`
/// mutates them through a shared `&self` — the same handle `search()` reads
/// through concurrently.
pub struct RateLimiter {
    buckets: Arc<DashMap<i64, Mutex<TokenBucket>>>,
    max_tokens: AtomicU32,
    window_millis: AtomicU64,
}

struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    last_refill: Instant,
    refill_rate: f64,
    retry_after: Option<Instant>,
}

impl TokenBucket {
    fn new(max_tokens: u32, window: Duration) -> Self {
        let max = f64::from(max_tokens);
        Self {
            tokens: max,
            max_tokens: max,
            last_refill: Instant::now(),
            refill_rate: max / window.as_secs_f64(),
            retry_after: None,
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    fn try_acquire(&mut self) -> Option<Duration> {
        // WHY: refill() must not run while an embargo is active — otherwise
        // tokens re-accumulate for the whole back-off window and fire a
        // full-`max_tokens` burst the instant the embargo clears, risking an
        // immediate re-429. Checking retry_after first and only refilling
        // once the embargo has genuinely cleared (last_refill is frozen at
        // retry_until by set_retry_after) means accrual resumes strictly
        // after the embargo, not from whenever the last real refill ran.
        if let Some(retry_until) = self.retry_after {
            let now = Instant::now();
            if now < retry_until {
                return Some(retry_until - now);
            }
            self.retry_after = None;
        }

        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            None
        } else {
            let wait = (1.0 - self.tokens) / self.refill_rate;
            Some(Duration::from_secs_f64(wait))
        }
    }

    fn set_retry_after(&mut self, duration: Duration) {
        let retry_until = Instant::now() + duration;
        self.retry_after = Some(retry_until);
        // WHY: freeze last_refill at retry_until (not now) so refill() only
        // accrues tokens for time elapsed AFTER the embargo clears — without
        // this, tokens would re-accumulate across the whole back-off window
        // and the bucket would be full again the instant retry_after expires.
        self.last_refill = retry_until;
        self.tokens = 0.0;
    }
}

impl RateLimiter {
    pub fn new(max_tokens: u32, window: Duration) -> Self {
        Self {
            buckets: Arc::new(DashMap::new()),
            max_tokens: AtomicU32::new(max_tokens),
            window_millis: AtomicU64::new(u64::try_from(window.as_millis()).unwrap_or(u64::MAX)),
        }
    }

    fn current_limits(&self) -> (u32, Duration) {
        let max_tokens = self.max_tokens.load(Ordering::Relaxed);
        let window = Duration::from_millis(self.window_millis.load(Ordering::Relaxed));
        (max_tokens, window)
    }

    /// Waits for a token, racing the back-off sleep against cancellation.
    ///
    /// Returns `true` when a token was acquired, `false` when the token was
    /// cancelled first — callers must skip the guarded work on `false`.
    #[must_use = "a false return means cancellation — the guarded work must be skipped"]
    pub async fn acquire(&self, indexer_id: i64, ct: &CancellationToken) -> bool {
        loop {
            let wait = {
                let (max_tokens, window) = self.current_limits();
                let entry = self
                    .buckets
                    .entry(indexer_id)
                    .or_insert_with(|| Mutex::new(TokenBucket::new(max_tokens, window)));
                let mut bucket = entry.value().lock().await;
                bucket.try_acquire()
            };

            match wait {
                None => return true,
                Some(duration) => {
                    // WHY: a cancelled search must not park fan-out tasks for the
                    // full back-off window — race the sleep against the token.
                    tokio::select! {
                        () = tokio::time::sleep(duration) => {}
                        () = ct.cancelled() => return false,
                    }
                }
            }
        }
    }

    pub async fn set_retry_after(&self, indexer_id: i64, duration: Duration) {
        let (max_tokens, window) = self.current_limits();
        let entry = self
            .buckets
            .entry(indexer_id)
            .or_insert_with(|| Mutex::new(TokenBucket::new(max_tokens, window)));
        let mut bucket = entry.value().lock().await;
        bucket.set_retry_after(duration);
    }

    /// Live-reconfigures the rate limit: updates the ceiling used for any
    /// bucket created FROM here on, and updates every EXISTING bucket's
    /// `max_tokens`/`refill_rate` in place, clamping its available token
    /// count to the new max.
    ///
    /// CRITICAL: `retry_after` and `last_refill` are left UNTOUCHED — the
    /// #533 fix (an active embargo freezes accrual until `retry_after`
    /// clears) must survive a reconfigure. A reload must never un-embargo an
    /// indexer that returned 429.
    pub async fn reconfigure(&self, requests_per: u32, window: Duration) {
        self.max_tokens.store(requests_per, Ordering::Relaxed);
        self.window_millis.store(
            u64::try_from(window.as_millis()).unwrap_or(u64::MAX),
            Ordering::Relaxed,
        );

        let new_max = f64::from(requests_per);
        let new_refill_rate = new_max / window.as_secs_f64();
        for entry in self.buckets.iter() {
            let mut bucket = entry.value().lock().await;
            bucket.max_tokens = new_max;
            bucket.refill_rate = new_refill_rate;
            // WHY: clamp only — a lowered ceiling must not leave a bucket
            // holding more tokens than the new max allows. `retry_after` and
            // `last_refill` are untouched (see doc comment above).
            bucket.tokens = bucket.tokens.min(new_max);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn acquire_within_limit() {
        let limiter = RateLimiter::new(5, Duration::from_secs(10));
        for _ in 0..5 {
            assert!(limiter.acquire(1, &CancellationToken::new()).await);
        }
    }

    #[tokio::test]
    async fn acquire_exceeding_limit_delays() {
        let limiter = RateLimiter::new(2, Duration::from_millis(200));
        let start = Instant::now();

        assert!(limiter.acquire(1, &CancellationToken::new()).await);
        assert!(limiter.acquire(1, &CancellationToken::new()).await);
        assert!(limiter.acquire(1, &CancellationToken::new()).await);

        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(50),
            "expected delay for 3rd token, got {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn separate_indexers_independent() {
        let limiter = RateLimiter::new(1, Duration::from_secs(10));
        assert!(limiter.acquire(1, &CancellationToken::new()).await);
        assert!(limiter.acquire(2, &CancellationToken::new()).await);
    }

    #[tokio::test]
    async fn acquire_unblocks_on_cancellation_before_refill() {
        let limiter = RateLimiter::new(1, Duration::from_secs(600));
        assert!(limiter.acquire(1, &CancellationToken::new()).await);

        let ct = CancellationToken::new();
        ct.cancel();
        let start = Instant::now();
        let acquired = limiter.acquire(1, &ct).await;
        let elapsed = start.elapsed();

        assert!(!acquired, "expected cancellation, not acquisition");
        assert!(
            elapsed < Duration::from_secs(1),
            "expected prompt unblock on cancel, got {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn retry_after_respected() {
        let limiter = RateLimiter::new(5, Duration::from_secs(10));
        limiter.set_retry_after(1, Duration::from_millis(100)).await;

        let start = Instant::now();
        assert!(limiter.acquire(1, &CancellationToken::new()).await);
        let elapsed = start.elapsed();

        assert!(
            elapsed >= Duration::from_millis(90),
            "expected retry-after delay, got {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn reconfigure_preserves_active_embargo() {
        let limiter = RateLimiter::new(5, Duration::from_secs(10));
        limiter.set_retry_after(1, Duration::from_millis(300)).await;

        // A reconfigure racing the embargo must not clear it.
        limiter.reconfigure(20, Duration::from_secs(5)).await;

        let start = Instant::now();
        assert!(limiter.acquire(1, &CancellationToken::new()).await);
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(250),
            "reconfigure must not un-embargo an indexer that returned 429, got {elapsed:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn reconfigure_preserves_accrual_state_across_the_embargo_window() {
        // Mirrors `embargo_freezes_accrual_until_retry_after_clears`, but with
        // a `reconfigure` call injected mid-embargo — the #533 semantics
        // (accrual frozen until retry_after clears) must survive it.
        let limiter = RateLimiter::new(10, Duration::from_secs(10));
        limiter.set_retry_after(1, Duration::from_secs(20)).await;

        // Reconfigure partway through the embargo window — same ceiling, so
        // this isolates "does reconfigure preserve last_refill/retry_after"
        // from "does it also update the ceiling".
        tokio::time::advance(Duration::from_secs(5)).await;
        limiter.reconfigure(10, Duration::from_secs(10)).await;

        tokio::time::advance(Duration::from_secs(20)).await;

        let start = Instant::now();
        assert!(limiter.acquire(1, &CancellationToken::new()).await);
        let elapsed = start.elapsed();
        assert!(
            elapsed.is_zero(),
            "embargo should have cleared by now, got {elapsed:?} wait"
        );
    }

    #[tokio::test]
    async fn reconfigure_clamps_existing_tokens_to_lowered_max() {
        let limiter = RateLimiter::new(10, Duration::from_secs(10));
        // Touch the bucket once so it exists with a full 10-token bank.
        assert!(limiter.acquire(1, &CancellationToken::new()).await);

        limiter.reconfigure(2, Duration::from_secs(10)).await;

        let bucket = limiter
            .buckets
            .get(&1)
            .expect("bucket exists after acquire");
        let bucket = bucket.lock().await;
        assert!(
            bucket.tokens <= 2.0,
            "expected tokens clamped to the new max of 2, got {}",
            bucket.tokens
        );
    }

    #[tokio::test]
    async fn reconfigure_applies_new_ceiling_to_newly_seen_indexers() {
        let limiter = RateLimiter::new(5, Duration::from_secs(10));
        limiter.reconfigure(1, Duration::from_millis(500)).await;

        // A brand-new indexer bucket must be created with the RECONFIGURED
        // ceiling, not the constructor's original value.
        assert!(limiter.acquire(99, &CancellationToken::new()).await);
        let start = Instant::now();
        assert!(limiter.acquire(99, &CancellationToken::new()).await);
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(400),
            "expected the reconfigured (lower) ceiling to apply, got {elapsed:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn embargo_freezes_accrual_until_retry_after_clears() {
        // refill_rate = 10 tokens / 10s = 1 token/sec.
        let mut bucket = TokenBucket::new(10, Duration::from_secs(10));

        // retry_after >= the refill window — the regime the bug fired in:
        // an unfrozen last_refill accrues a full max_tokens burst by the
        // time the embargo clears.
        bucket.set_retry_after(Duration::from_secs(20));

        // Advance 5s past retry_until.
        tokio::time::advance(Duration::from_secs(25)).await;

        let wait = bucket.try_acquire();
        assert!(wait.is_none(), "expected a token once the embargo cleared");
        // Only the 5s elapsed AFTER retry_until should have accrued (5
        // tokens, minus the 1 just spent = 4). A full burst (the bug) would
        // leave max_tokens - 1 = 9.
        assert!(
            bucket.tokens <= 4.5,
            "expected accrual limited to post-embargo elapsed time, got {} tokens",
            bucket.tokens
        );
    }
}
