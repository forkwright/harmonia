//! Retry with exponential backoff and per-service circuit breakers.

use std::future::Future;
use std::sync::Mutex; // kanon:ignore RUST/std-mutex-in-async -- guards Option<Instant>, never held across await; lock scopes are microseconds inside sync methods
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use horismos::SyndesmosConfig;
use tokio::time::Instant;
use tracing::instrument;

use crate::error::{ExternalServiceDownSnafu, SyndesmodError};

/// Delays before each successive retry attempt: 2 s, 8 s, 32 s.
const RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(2),
    Duration::from_secs(8),
    Duration::from_secs(32),
];

/// Tracks consecutive failures for a single external service.
///
/// After `failure_threshold` consecutive failures, all calls are short-circuited
/// until `cooldown` elapses. A single success resets the counter.
pub struct CircuitBreaker {
    consecutive_failures: AtomicU32,
    tripped_at: Mutex<Option<Instant>>,
    failure_threshold: u32,
    cooldown: Duration,
    service_name: String,
}

impl CircuitBreaker {
    pub fn new(
        service_name: impl Into<String>,
        failure_threshold: u32,
        cooldown: Duration,
    ) -> Self {
        Self {
            consecutive_failures: AtomicU32::new(0),
            tripped_at: Mutex::new(None),
            failure_threshold,
            cooldown,
            service_name: service_name.into(),
        }
    }

    pub fn with_defaults(service_name: impl Into<String>, circuit_break_minutes: u64) -> Self {
        Self::new(
            service_name,
            SyndesmosConfig::default().circuit_break_failure_threshold,
            Duration::from_secs(circuit_break_minutes * 60),
        )
    }

    /// Build a circuit breaker FROM the operator-supplied SyndesmosConfig.
    pub fn from_config(service_name: impl Into<String>, config: &SyndesmosConfig) -> Self {
        Self::new(
            service_name,
            config.circuit_break_failure_threshold,
            Duration::from_secs(config.circuit_break_minutes * 60),
        )
    }

    /// Returns true when the circuit is open and calls should be short-circuited.
    pub fn is_open(&self) -> bool {
        let guard = self.tripped_at.lock().unwrap();
        match *guard {
            None => false,
            Some(tripped) => tripped.elapsed() < self.cooldown,
        }
    }

    pub fn on_success(&self) {
        self.consecutive_failures.store(0, Ordering::Relaxed);
        let mut guard = self.tripped_at.lock().unwrap();
        *guard = None;
    }

    pub fn on_failure(&self) {
        let prev = self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
        if prev + 1 >= self.failure_threshold {
            let mut guard = self.tripped_at.lock().unwrap();
            if guard.is_none() {
                *guard = Some(Instant::now());
            }
        }
    }

    pub fn service_name(&self) -> &str {
        &self.service_name
    }
}

/// Calls `f` up to 4 times (1 initial + 3 retries) with exponential backoff,
/// subject to the `circuit` breaker.
///
/// Returns `ExternalServiceDown` immediately when the circuit is open.
/// Resets the failure counter on success; increments it on every failure.
#[instrument(skip(f, circuit), fields(service = %circuit.service_name()))]
pub async fn with_retry<F, T, Fut>(f: F, circuit: &CircuitBreaker) -> Result<T, SyndesmodError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, SyndesmodError>>,
{
    if circuit.is_open() {
        return ExternalServiceDownSnafu {
            service: circuit.service_name().to_string(),
        }
        .fail();
    }

    let mut last_err = None;

    for attempt in 0..=RETRY_DELAYS.len() {
        if attempt > 0 {
            tokio::time::sleep(RETRY_DELAYS[attempt - 1]).await;

            if circuit.is_open() {
                return ExternalServiceDownSnafu {
                    service: circuit.service_name().to_string(),
                }
                .fail();
            }
        }

        match f().await {
            Ok(value) => {
                circuit.on_success();
                return Ok(value);
            }
            Err(err) => {
                circuit.on_failure();
                tracing::warn!(
                    attempt,
                    service = %circuit.service_name(),
                    "external call failed, will retry"
                );
                last_err = Some(err);
            }
        }
    }

    // All attempts exhausted.
    Err(last_err.unwrap())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;

    fn make_breaker(threshold: u32, cooldown_secs: u64) -> CircuitBreaker {
        CircuitBreaker::new(
            "test-service",
            threshold,
            Duration::from_secs(cooldown_secs),
        )
    }

    #[tokio::test(start_paused = true)]
    async fn succeeds_on_first_attempt() {
        let circuit = make_breaker(5, 300);
        let result: Result<u32, SyndesmodError> =
            with_retry(|| async { Ok(42u32) }, &circuit).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test(start_paused = true)]
    async fn retries_on_transient_failure_then_succeeds() {
        let call_count = std::sync::Arc::new(AtomicUsize::new(0));
        let circuit = make_breaker(5, 300);

        let cc = call_count.clone();
        let result: Result<u32, SyndesmodError> = with_retry(
            move || {
                let cc = cc.clone();
                async move {
                    let n = cc.fetch_add(1, Ordering::SeqCst);
                    if n < 2 {
                        Err(SyndesmodError::RateLimitExceeded {
                            service: "test".to_string(),
                            location: snafu::location!(),
                        })
                    } else {
                        Ok(99u32)
                    }
                }
            },
            &circuit,
        )
        .await;

        assert_eq!(result.unwrap(), 99);
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn all_retries_exhausted_returns_last_error() {
        let circuit = make_breaker(10, 300);

        let result: Result<u32, SyndesmodError> = with_retry(
            || async {
                Err::<u32, SyndesmodError>(SyndesmodError::RateLimitExceeded {
                    service: "test".to_string(),
                    location: snafu::location!(),
                })
            },
            &circuit,
        )
        .await;

        assert!(matches!(
            result,
            Err(SyndesmodError::RateLimitExceeded { .. })
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn circuit_breaker_trips_after_threshold_failures() {
        let circuit = make_breaker(5, 300);

        for _ in 0..5 {
            circuit.on_failure();
        }

        assert!(circuit.is_open());
    }

    #[tokio::test(start_paused = true)]
    async fn circuit_breaker_short_circuits_when_open() {
        let circuit = make_breaker(5, 300);
        for _ in 0..5 {
            circuit.on_failure();
        }

        let result: Result<u32, SyndesmodError> = with_retry(|| async { Ok(1u32) }, &circuit).await;

        assert!(matches!(
            result,
            Err(SyndesmodError::ExternalServiceDown { .. })
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn circuit_breaker_resets_after_cooldown() {
        let circuit = make_breaker(5, 10);
        for _ in 0..5 {
            circuit.on_failure();
        }

        assert!(circuit.is_open());

        tokio::time::advance(Duration::from_secs(11)).await;

        assert!(!circuit.is_open());
    }

    #[tokio::test(start_paused = true)]
    async fn custom_failure_threshold_observed() {
        // WHY: non-default config — threshold of 2 trips the circuit after 2 failures.
        let cfg = SyndesmosConfig {
            circuit_break_failure_threshold: 2,
            ..SyndesmosConfig::default()
        };
        let circuit = CircuitBreaker::from_config("svc", &cfg);
        circuit.on_failure();
        assert!(!circuit.is_open());
        circuit.on_failure();
        assert!(
            circuit.is_open(),
            "circuit should trip after custom threshold reached"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn success_resets_failure_counter() {
        let circuit = make_breaker(5, 300);

        circuit.on_failure();
        circuit.on_failure();
        circuit.on_failure();
        circuit.on_success();

        assert!(!circuit.is_open());
        assert_eq!(circuit.consecutive_failures.load(Ordering::Relaxed), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn with_retry_makes_exactly_four_attempts_on_all_failures() {
        let circuit = make_breaker(20, 300);
        let call_count = std::sync::Arc::new(AtomicUsize::new(0));
        let cc = call_count.clone();

        let _result: Result<u32, SyndesmodError> = with_retry(
            move || {
                let cc = cc.clone();
                async move {
                    cc.fetch_add(1, Ordering::SeqCst);
                    Err::<u32, _>(SyndesmodError::RateLimitExceeded {
                        service: "test".to_string(),
                        location: snafu::location!(),
                    })
                }
            },
            &circuit,
        )
        .await;

        // 1 initial + 3 retries = 4 total
        assert_eq!(call_count.load(Ordering::SeqCst), 4);
    }
}
