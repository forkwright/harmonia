//! Event handler: subscribes to ImportCompleted and triggers subtitle acquisition
//! for video media types (Movie, Tv).

use std::sync::Arc;

use harmonia_common::{EventReceiver, HarmoniaEvent, MediaType};
use tokio_util::sync::CancellationToken;
use tracing::{instrument, warn};

use crate::SubtitleService;

/// Drive the subtitle acquisition event loop.
///
/// Subscribes to `ImportCompleted` events. Silently ignores non-video types.
/// Emits `SubtitleAcquired` events on success (handled inside `SubtitleService`).
///
/// The loop exits cleanly when the cancellation token fires or the broadcast
/// channel closes.
pub async fn run_event_handler<S>(mut rx: EventReceiver, service: Arc<S>, ct: CancellationToken)
where
    S: SubtitleService,
{
    loop {
        tokio::select! {
            biased;
            _ = ct.cancelled() => break,
            result = rx.recv() => {
                match result {
                    Ok(event) => handle_event(event, Arc::clone(&service)).await,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = n, "subtitle event receiver lagged — some ImportCompleted events may be missed");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

#[instrument(skip(service))]
async fn handle_event<S: SubtitleService>(event: HarmoniaEvent, service: Arc<S>) {
    if let HarmoniaEvent::ImportCompleted {
        media_id,
        media_type,
        path,
    } = event
    {
        // Non-video types do not get subtitles — silently ignored.
        if matches!(media_type, MediaType::Movie | MediaType::Tv)
            && let Err(e) = service.acquire_subtitles(media_id, media_type, &path).await
        {
            warn!(
                media_id = %media_id,
                error = %e,
                "subtitle acquisition failed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Mutex;

    use harmonia_common::{MediaId, MediaType, create_event_bus};

    use super::*;
    use crate::error::ProsthekeError;

    // ── Mock SubtitleService ──────────────────────────────────────────────────

    struct RecordingService {
        calls: Mutex<Vec<(MediaId, MediaType)>>,
    }

    impl RecordingService {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                calls: Mutex::new(vec![]),
            })
        }

        fn recorded(&self) -> Vec<(MediaId, MediaType)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl SubtitleService for RecordingService {
        async fn acquire_subtitles(
            &self,
            media_id: MediaId,
            media_type: MediaType,
            _path: &std::path::Path,
        ) -> Result<(), ProsthekeError> {
            self.calls.lock().unwrap().push((media_id, media_type));
            Ok(())
        }

        async fn list_for_media(
            &self,
            _media_id: MediaId,
        ) -> Result<Vec<crate::types::SubtitleTrack>, ProsthekeError> {
            Ok(vec![])
        }
    }

    async fn send_import(
        tx: &harmonia_common::EventSender,
        media_id: MediaId,
        media_type: MediaType,
    ) {
        tx.send(HarmoniaEvent::ImportCompleted {
            media_id,
            media_type,
            path: PathBuf::from("/library/file.mkv"),
        })
        .unwrap();
    }

    #[tokio::test]
    async fn import_completed_triggers_for_movie() {
        let (tx, rx) = create_event_bus(16);
        let service = RecordingService::new();
        let ct = CancellationToken::new();

        let service_clone = Arc::clone(&service);
        let ct_clone = ct.clone();
        let handle =
            tokio::spawn(async move { run_event_handler(rx, service_clone, ct_clone).await });

        let media_id = MediaId::new();
        send_import(&tx, media_id, MediaType::Movie).await;

        // Give the handler time to process.
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        ct.cancel();
        handle.await.unwrap();

        let calls = service.recorded();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, media_id);
        assert_eq!(calls[0].1, MediaType::Movie);
    }

    #[tokio::test]
    async fn import_completed_triggers_for_tv() {
        let (tx, rx) = create_event_bus(16);
        let service = RecordingService::new();
        let ct = CancellationToken::new();

        let service_clone = Arc::clone(&service);
        let ct_clone = ct.clone();
        let handle =
            tokio::spawn(async move { run_event_handler(rx, service_clone, ct_clone).await });

        let media_id = MediaId::new();
        send_import(&tx, media_id, MediaType::Tv).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        ct.cancel();
        handle.await.unwrap();

        let calls = service.recorded();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, MediaType::Tv);
    }

    #[tokio::test]
    async fn import_completed_ignored_for_music() {
        let (tx, rx) = create_event_bus(16);
        let service = RecordingService::new();
        let ct = CancellationToken::new();

        let service_clone = Arc::clone(&service);
        let ct_clone = ct.clone();
        let handle =
            tokio::spawn(async move { run_event_handler(rx, service_clone, ct_clone).await });

        send_import(&tx, MediaId::new(), MediaType::Music).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        ct.cancel();
        handle.await.unwrap();

        assert!(service.recorded().is_empty());
    }

    #[tokio::test]
    async fn import_completed_ignored_for_audiobook() {
        let (tx, rx) = create_event_bus(16);
        let service = RecordingService::new();
        let ct = CancellationToken::new();

        let service_clone = Arc::clone(&service);
        let ct_clone = ct.clone();
        let handle =
            tokio::spawn(async move { run_event_handler(rx, service_clone, ct_clone).await });

        send_import(&tx, MediaId::new(), MediaType::Audiobook).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        ct.cancel();
        handle.await.unwrap();

        assert!(service.recorded().is_empty());
    }

    #[tokio::test]
    async fn import_completed_ignored_for_book() {
        let (tx, rx) = create_event_bus(16);
        let service = RecordingService::new();
        let ct = CancellationToken::new();

        let service_clone = Arc::clone(&service);
        let ct_clone = ct.clone();
        let handle =
            tokio::spawn(async move { run_event_handler(rx, service_clone, ct_clone).await });

        send_import(&tx, MediaId::new(), MediaType::Book).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        ct.cancel();
        handle.await.unwrap();

        assert!(service.recorded().is_empty());
    }
}
