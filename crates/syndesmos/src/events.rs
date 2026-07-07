//! Aggelia event handler — subscribes to HarmoniaEvent and dispatches to integrations.

use std::sync::Arc;

use themelion::{EventReceiver, HarmoniaEvent};
use tokio::sync::broadcast::error::RecvError;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::{ExternalIntegration, ScrobbleClient};

/// Runs the event handler loop for Syndesmos.
///
/// Subscribes to `rx`, handles `PlexNotifyRequired` and `ScrobbleRequired`,
/// and shuts down cleanly on `ct` cancellation or channel close.
///
/// `RecvError::Lagged` is logged as a warning — missed events are acceptable
/// when the service falls behind; integration calls are best-effort.
#[instrument(skip(service, rx, ct))]
pub async fn run_event_handler(
    service: Arc<ScrobbleClient>,
    mut rx: EventReceiver,
    ct: CancellationToken,
) {
    loop {
        tokio::select! {
            biased;
            _ = ct.cancelled() => {
                tracing::info!("syndesmos event handler shutting down");
                break;
            }
            result = rx.recv() => {
                match result {
                    Ok(event) => {
                        // WHY: handle_event runs retry loops with backoff sleeps
                        // — raced against the token so shutdown is prompt even
                        // mid-retry, instead of waiting out the full backoff.
                        tokio::select! {
                            biased;
                            _ = ct.cancelled() => {
                                tracing::info!(
                                    "syndesmos event handler cancelled mid-dispatch; shutting down"
                                );
                                break;
                            }
                            () = handle_event(&service, event) => {}
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        tracing::warn!(missed = n, "syndesmos event receiver lagged; events skipped");
                    }
                    Err(RecvError::Closed) => {
                        tracing::info!("syndesmos event channel closed; shutting down");
                        break;
                    }
                }
            }
        }
    }
}

async fn handle_event(service: &ScrobbleClient, event: HarmoniaEvent) {
    match event {
        HarmoniaEvent::PlexNotifyRequired { media_id } => {
            if let Err(err) = service.notify_plex_import(media_id).await {
                tracing::warn!(
                    error = %err,
                    media_id = %media_id,
                    "plex library notify failed"
                );
            }
        }
        HarmoniaEvent::ScrobbleRequired { track_id, user_id } => {
            if let Err(err) = service.scrobble(track_id, user_id).await {
                tracing::warn!(
                    error = %err,
                    track_id = %track_id,
                    user_id = %user_id,
                    "last.fm scrobble failed"
                );
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use themelion::{MediaId, UserId, create_event_bus};
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::ScrobbleClientBuilder;

    #[tokio::test]
    async fn plex_notify_required_calls_plex_notify() {
        use std::sync::Arc;

        use crate::plex::tests::MockPlexApi;

        let mock_plex = Arc::new(MockPlexApi::new());
        let sections_ref = mock_plex.sections_refreshed.clone();

        let (tx, rx) = create_event_bus(32);
        let ct = CancellationToken::new();

        // Configure with music section 1
        let mut sections = std::collections::HashMap::new();
        sections.insert(themelion::MediaType::Music, 1u32);

        let service = Arc::new(
            ScrobbleClientBuilder::new(tx.clone(), crate::test_support::test_pool().await)
                .with_mock_plex(mock_plex.clone(), sections)
                .build(),
        );

        let ct_clone = ct.clone();
        let svc_clone = service.clone();
        let handler = tokio::spawn(async move {
            run_event_handler(svc_clone, rx, ct_clone).await;
        });

        let media_id = MediaId::new();
        tx.send(HarmoniaEvent::PlexNotifyRequired { media_id })
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;
        ct.cancel();
        handler.await.unwrap();

        assert_eq!(*sections_ref.lock().unwrap(), vec![1u32]);
    }

    #[tokio::test]
    async fn scrobble_required_calls_lastfm_scrobble() {
        use std::sync::Arc;

        use crate::lastfm::tests::MockLastfmApi;

        let mock_lastfm = Arc::new(MockLastfmApi::new());
        let submitted_ref = mock_lastfm.scrobbles_submitted.clone();

        let (tx, rx) = create_event_bus(32);
        let ct = CancellationToken::new();

        let pool = crate::test_support::test_pool().await;
        let track_id =
            crate::test_support::seed_scrobble_track(&pool, "Autechre", "Gantz Graf", "Gantz Graf")
                .await;

        let service = Arc::new(
            ScrobbleClientBuilder::new(tx.clone(), pool)
                .with_mock_lastfm(mock_lastfm.clone())
                .build(),
        );

        let ct_clone = ct.clone();
        let svc_clone = service.clone();
        let handler = tokio::spawn(async move {
            run_event_handler(svc_clone, rx, ct_clone).await;
        });

        let user_id = UserId::new();
        tx.send(HarmoniaEvent::ScrobbleRequired { track_id, user_id })
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;
        ct.cancel();
        handler.await.unwrap();

        let submitted = submitted_ref.lock().unwrap();
        assert_eq!(submitted.len(), 1);
        assert_eq!(submitted[0].artist, "Autechre");
    }

    #[tokio::test]
    async fn handler_continues_after_lag() {
        use std::sync::Arc;

        use crate::plex::tests::MockPlexApi;

        let mock_plex = Arc::new(MockPlexApi::new());
        let sections_ref = mock_plex.sections_refreshed.clone();

        // WHY: a small bus overflowed before the handler starts forces the
        // first recv to hit RecvError::Lagged — the loop must survive it.
        let (tx, rx) = create_event_bus(4);
        let ct = CancellationToken::new();

        for _ in 0..32 {
            tx.send(HarmoniaEvent::SearchCompleted {
                query_id: themelion::QueryId::new(),
                result_count: 0,
            })
            .unwrap();
        }

        let mut sections = std::collections::HashMap::new();
        sections.insert(themelion::MediaType::Music, 7u32);
        let service = Arc::new(
            ScrobbleClientBuilder::new(tx.clone(), crate::test_support::test_pool().await)
                .with_mock_plex(mock_plex.clone(), sections)
                .build(),
        );

        let ct_clone = ct.clone();
        let handler = tokio::spawn(async move {
            run_event_handler(service, rx, ct_clone).await;
        });

        // The handler drains the lag, then must still process a fresh event.
        tokio::time::sleep(Duration::from_millis(50)).await;
        tx.send(HarmoniaEvent::PlexNotifyRequired {
            media_id: MediaId::new(),
        })
        .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;
        ct.cancel();
        handler.await.unwrap();

        assert_eq!(
            *sections_ref.lock().unwrap(),
            vec![7u32],
            "the loop must keep dispatching after a Lagged error"
        );
    }

    #[tokio::test]
    async fn cancellation_interrupts_in_flight_event_dispatch() {
        use std::sync::Arc;

        use crate::plex::tests::MockPlexApi;

        // WHY: 30s dwarfs the assertion window — if cancellation waited for
        // handle_event to finish, the join below would time out.
        let mock_plex = Arc::new(MockPlexApi::with_delay_ms(30_000));

        let (tx, rx) = create_event_bus(32);
        let ct = CancellationToken::new();

        let mut sections = std::collections::HashMap::new();
        sections.insert(themelion::MediaType::Music, 1u32);
        let service = Arc::new(
            ScrobbleClientBuilder::new(tx.clone(), crate::test_support::test_pool().await)
                .with_mock_plex(mock_plex.clone(), sections)
                .build(),
        );

        let ct_clone = ct.clone();
        let handler = tokio::spawn(async move {
            run_event_handler(service, rx, ct_clone).await;
        });

        tx.send(HarmoniaEvent::PlexNotifyRequired {
            media_id: MediaId::new(),
        })
        .unwrap();

        // Let the handler enter the slow dispatch, then cancel.
        tokio::time::sleep(Duration::from_millis(100)).await;
        ct.cancel();

        tokio::time::timeout(Duration::from_secs(2), handler)
            .await
            .expect("handler must exit promptly despite the in-flight dispatch")
            .unwrap();
    }

    #[tokio::test]
    async fn handler_exits_on_cancellation() {
        let (tx, rx) = create_event_bus(32);
        let ct = CancellationToken::new();
        let service = Arc::new(
            ScrobbleClientBuilder::new(tx, crate::test_support::test_pool().await).build(),
        );

        let ct_clone = ct.clone();
        let handler = tokio::spawn(async move {
            run_event_handler(service, rx, ct_clone).await;
        });

        ct.cancel();
        tokio::time::timeout(Duration::from_secs(1), handler)
            .await
            .expect("handler should exit after cancellation")
            .unwrap();
    }

    // #529 step 8: mirrors EXACTLY the mechanic `run_syndesmos_supervisor`
    // (archon) performs on a `syndesmos.*` change — cancel the old handler's
    // token, await it, then respawn on a FRESH subscription with a REBUILT
    // client. Proves a post-change event reaches the rebuilt client, not the
    // retired one.
    #[tokio::test]
    async fn handler_respawn_processes_post_change_event_with_the_rebuilt_client() {
        use std::sync::Arc;

        use crate::plex::tests::MockPlexApi;

        // Generation A: section 1.
        let mock_a = Arc::new(MockPlexApi::new());
        let sections_a = mock_a.sections_refreshed.clone();
        let mut sections_map_a = std::collections::HashMap::new();
        sections_map_a.insert(themelion::MediaType::Music, 1u32);

        let (tx, rx_a) = create_event_bus(32);
        let service_a = Arc::new(
            ScrobbleClientBuilder::new(tx.clone(), crate::test_support::test_pool().await)
                .with_mock_plex(mock_a.clone(), sections_map_a)
                .build(),
        );

        let ct_a = CancellationToken::new();
        let ct_a_clone = ct_a.clone();
        let handler_a = tokio::spawn(async move {
            run_event_handler(service_a, rx_a, ct_a_clone).await;
        });

        tx.send(HarmoniaEvent::PlexNotifyRequired {
            media_id: MediaId::new(),
        })
        .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(*sections_a.lock().unwrap(), vec![1u32]);

        // The supervisor mechanic: cancel + await the OLD handler BEFORE
        // rebuilding.
        ct_a.cancel();
        handler_a.await.unwrap();

        // Generation B: a REBUILT client (different Plex section mapping),
        // respawned on a FRESH subscription — exactly what
        // `run_syndesmos_supervisor` does after the cancel+await above.
        let mock_b = Arc::new(MockPlexApi::new());
        let sections_b = mock_b.sections_refreshed.clone();
        let mut sections_map_b = std::collections::HashMap::new();
        sections_map_b.insert(themelion::MediaType::Music, 2u32);

        let rx_b = tx.subscribe();
        let service_b = Arc::new(
            ScrobbleClientBuilder::new(tx.clone(), crate::test_support::test_pool().await)
                .with_mock_plex(mock_b.clone(), sections_map_b)
                .build(),
        );

        let ct_b = CancellationToken::new();
        let ct_b_clone = ct_b.clone();
        let handler_b = tokio::spawn(async move {
            run_event_handler(service_b, rx_b, ct_b_clone).await;
        });

        // A POST-change event must be processed by the rebuilt client
        // (section 2) — the old (cancelled) client must see nothing further.
        tx.send(HarmoniaEvent::PlexNotifyRequired {
            media_id: MediaId::new(),
        })
        .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        ct_b.cancel();
        handler_b.await.unwrap();

        assert_eq!(
            *sections_a.lock().unwrap(),
            vec![1u32],
            "the OLD (cancelled) client must not see the post-change event"
        );
        assert_eq!(
            *sections_b.lock().unwrap(),
            vec![2u32],
            "the respawned handler must dispatch the post-change event through the rebuilt client"
        );
    }
}
