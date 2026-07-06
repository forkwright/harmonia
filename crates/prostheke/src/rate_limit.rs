//! Token-bucket rate limiter bounding request throughput to a subtitle provider.

use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::Instant;

/// Spaces calls to at most `requests_per_second`, allowing a burst of one
/// immediately after construction (or after an idle gap) before throttling
/// kicks in.
///
/// WHY: mirrors `epignosis::rate_limit::ProviderQueue`'s spacing approach (the
/// existing per-provider metadata rate limiter in this workspace), but tracks
/// the next eligible slot directly instead of a background task + channel —
/// construction must stay runtime-agnostic, since `OpenSubtitlesProvider::new`
/// is exercised FROM plain (non-`tokio::test`) unit tests.
pub struct RateLimiter {
    interval: Duration,
    next_slot: Mutex<Instant>,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        // WHY: 0 has no documented "unlimited" meaning for this field —
        // clamp rather than let the interval computation divide by zero or
        // degenerate into a permanently-stuck limiter.
        let requests_per_second = requests_per_second.max(1);
        let interval = Duration::from_secs(1) / requests_per_second;
        Self {
            interval,
            next_slot: Mutex::new(Instant::now()),
        }
    }

    /// Blocks until the next slot in the configured rate is available.
    pub async fn acquire(&self) {
        let wait_until = {
            let mut next_slot = self.next_slot.lock().await;
            let now = Instant::now();
            let slot = if *next_slot > now { *next_slot } else { now };
            *next_slot = slot + self.interval;
            slot
        };
        tokio::time::sleep_until(wait_until).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn first_call_is_not_throttled() {
        let limiter = RateLimiter::new(5);
        let start = Instant::now();
        limiter.acquire().await;
        assert_eq!(start.elapsed(), Duration::ZERO);
    }

    /// 5 req/s → 200ms interval. The first call is free (burst of one); each
    /// of the next 4 is spaced by one interval.
    ///
    /// WHY `start_paused`: runs the limiter's `tokio::time::sleep_until` on
    /// the mock clock, so the schedule is exact logical time — no wall-clock
    /// slack, no flake under load.
    #[tokio::test(start_paused = true)]
    async fn rapid_calls_are_throttled_to_the_configured_rate() {
        let limiter = RateLimiter::new(5);
        let start = Instant::now();

        for _ in 0..5 {
            limiter.acquire().await;
        }

        assert_eq!(start.elapsed(), Duration::from_millis(800));
    }

    #[tokio::test]
    async fn zero_rate_clamps_to_one_request_per_second() {
        let limiter = RateLimiter::new(0);
        assert_eq!(limiter.interval, Duration::from_secs(1));
    }
}
