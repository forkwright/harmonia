//! Scheduled Tidal want-list sync — a supervisor ticker driving `sync_want_list`.

use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::{ExternalIntegration, ScrobbleClient};

/// Runs the Tidal want-list sync every `interval` until `shutdown` fires.
///
/// The first tick lands one full interval out — the wants persisted by the
/// previous run are the diff baseline, so boot needs no immediate sync
/// (mirrors the zetesis caps-refresh supervisor's tick). Missed ticks Delay
/// rather than Burst: a slow sync must not be chased by a back-to-back
/// catch-up sync. An interval of zero disables the scheduled sync.
pub async fn run_want_list_scheduler(
    service: Arc<ScrobbleClient>,
    interval: Duration,
    shutdown: CancellationToken,
) {
    if interval.is_zero() {
        tracing::info!("tidal want-list scheduler disabled (sync_interval_minutes = 0)");
        return;
    }

    let mut tick = tokio::time::interval_at(tokio::time::Instant::now() + interval, interval);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            _ = tick.tick() => {
                // WHY: the sync runs retry loops with backoff sleeps — raced
                // against the token so shutdown is prompt even mid-retry,
                // instead of waiting out the full backoff.
                tokio::select! {
                    biased;
                    _ = shutdown.cancelled() => break,
                    result = service.sync_tidal_want_list() => {
                        if let Err(e) = result {
                            tracing::warn!(error = %e, "scheduled tidal want-list sync failed");
                        }
                    }
                }
            }
        }
    }
    tracing::info!("tidal want-list scheduler shutting down");
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use aggelmata::create_event_bus;

    use super::*;
    use crate::ScrobbleClientBuilder;
    use crate::tidal::tests::MockTidalApi;
    use crate::tidal::{TidalFavorite, TidalId};

    fn favorite(id: &str) -> TidalFavorite {
        TidalFavorite {
            tidal_id: TidalId(id.to_string()),
            title: format!("Track {id}"),
            artist: "Test Artist".to_string(),
        }
    }

    /// The zetesis caps-refresh supervisor test pattern: a REAL scheduler on
    /// a short test tick, a bounded real wait for the effect to land, then a
    /// clean shutdown join.
    #[tokio::test]
    async fn fires_sync_on_the_configured_interval() {
        let mock = Arc::new(MockTidalApi::new(vec![favorite("t1")]));
        let calls = Arc::clone(&mock.call_count);
        let (tx, _rx) = create_event_bus(32);
        let service = Arc::new(
            ScrobbleClientBuilder::new(tx, crate::test_support::test_pool().await)
                .with_mock_tidal(mock)
                .build(),
        );

        let shutdown = CancellationToken::new();
        let scheduler = tokio::spawn(run_want_list_scheduler(
            service,
            Duration::from_millis(50),
            shutdown.clone(),
        ));

        // WHY: two fires prove the interval repeats, not just the first tick.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while calls.load(Ordering::SeqCst) < 2 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "scheduler never fired a second sync on the 50ms interval"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(2), scheduler)
            .await
            .expect("scheduler joins cleanly after ticks")
            .unwrap();
    }

    #[tokio::test]
    async fn zero_interval_disables_the_scheduler() {
        let mock = Arc::new(MockTidalApi::new(vec![favorite("t1")]));
        let calls = Arc::clone(&mock.call_count);
        let (tx, _rx) = create_event_bus(32);
        let service = Arc::new(
            ScrobbleClientBuilder::new(tx, crate::test_support::test_pool().await)
                .with_mock_tidal(mock)
                .build(),
        );

        let shutdown = CancellationToken::new();
        let scheduler = tokio::spawn(run_want_list_scheduler(service, Duration::ZERO, shutdown));

        tokio::time::timeout(Duration::from_secs(1), scheduler)
            .await
            .expect("a zero interval disables the scheduler instead of spinning")
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 0, "no sync may fire");
    }
}
