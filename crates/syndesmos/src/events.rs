//! Aggelia event handler — subscribes to HarmoniaEvent and dispatches to integrations.

use std::sync::Arc;

use themelion::{EventReceiver, HarmoniaEvent};
use tokio::sync::broadcast::error::RecvError;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::{ExternalIntegration, SyndesmosService};

/// Runs the event handler loop for Syndesmos.
///
/// Subscribes to `rx`, handles `PlexNotifyRequired` and `ScrobbleRequired`,
/// and shuts down cleanly on `ct` cancellation or channel close.
///
/// `RecvError::Lagged` is logged as a warning — missed events are acceptable
/// when the service falls behind; integration calls are best-effort.
#[instrument(skip(service, rx, ct))]
pub async fn run_event_handler(
    service: Arc<SyndesmosService>,
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
                    Ok(event) => handle_event(&service, event).await,
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

async fn handle_event(service: &SyndesmosService, event: HarmoniaEvent) {
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
    use crate::SyndesmosServiceBuilder;

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
            SyndesmosServiceBuilder::new(tx.clone())
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

        let service = Arc::new(
            SyndesmosServiceBuilder::new(tx.clone())
                .with_mock_lastfm(mock_lastfm.clone())
                .build(),
        );

        let ct_clone = ct.clone();
        let svc_clone = service.clone();
        let handler = tokio::spawn(async move {
            run_event_handler(svc_clone, rx, ct_clone).await;
        });

        let track_id = MediaId::new();
        let user_id = UserId::new();
        tx.send(HarmoniaEvent::ScrobbleRequired { track_id, user_id })
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;
        ct.cancel();
        handler.await.unwrap();

        let submitted = submitted_ref.lock().unwrap();
        assert_eq!(submitted.len(), 1);
    }

    #[tokio::test]
    async fn handler_exits_on_cancellation() {
        let (tx, rx) = create_event_bus(32);
        let ct = CancellationToken::new();
        let service = Arc::new(SyndesmosServiceBuilder::new(tx).build());

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
}
