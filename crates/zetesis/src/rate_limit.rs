use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tokio::sync::Mutex;
use tokio::time::Instant;

pub struct RateLimiter {
    buckets: Arc<DashMap<i64, Mutex<TokenBucket>>>,
    max_tokens: u32,
    window: Duration,
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
        self.refill();

        if let Some(retry_until) = self.retry_after {
            let now = Instant::now();
            if now < retry_until {
                return Some(retry_until - now);
            }
            self.retry_after = None;
        }

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            None
        } else {
            let wait = (1.0 - self.tokens) / self.refill_rate;
            Some(Duration::from_secs_f64(wait))
        }
    }

    fn set_retry_after(&mut self, duration: Duration) {
        self.retry_after = Some(Instant::now() + duration);
        self.tokens = 0.0;
    }
}

impl RateLimiter {
    pub fn new(max_tokens: u32, window: Duration) -> Self {
        Self {
            buckets: Arc::new(DashMap::new()),
            max_tokens,
            window,
        }
    }

    pub async fn acquire(&self, indexer_id: i64) {
        loop {
            let wait = {
                let entry = self
                    .buckets
                    .entry(indexer_id)
                    .or_insert_with(|| Mutex::new(TokenBucket::new(self.max_tokens, self.window)));
                let mut bucket = entry.value().lock().await;
                bucket.try_acquire()
            };

            match wait {
                None => return,
                Some(duration) => tokio::time::sleep(duration).await,
            }
        }
    }

    pub async fn set_retry_after(&self, indexer_id: i64, duration: Duration) {
        let entry = self
            .buckets
            .entry(indexer_id)
            .or_insert_with(|| Mutex::new(TokenBucket::new(self.max_tokens, self.window)));
        let mut bucket = entry.value().lock().await;
        bucket.set_retry_after(duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn acquire_within_limit() {
        let limiter = RateLimiter::new(5, Duration::from_secs(10));
        for _ in 0..5 {
            limiter.acquire(1).await;
        }
    }

    #[tokio::test]
    async fn acquire_exceeding_limit_delays() {
        let limiter = RateLimiter::new(2, Duration::from_millis(200));
        let start = Instant::now();

        limiter.acquire(1).await;
        limiter.acquire(1).await;
        limiter.acquire(1).await;

        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(50),
            "expected delay for 3rd token, got {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn separate_indexers_independent() {
        let limiter = RateLimiter::new(1, Duration::from_secs(10));
        limiter.acquire(1).await;
        limiter.acquire(2).await;
    }

    #[tokio::test]
    async fn retry_after_respected() {
        let limiter = RateLimiter::new(5, Duration::from_secs(10));
        limiter.set_retry_after(1, Duration::from_millis(100)).await;

        let start = Instant::now();
        limiter.acquire(1).await;
        let elapsed = start.elapsed();

        assert!(
            elapsed >= Duration::from_millis(90),
            "expected retry-after delay, got {elapsed:?}"
        );
    }
}
