//! Syntaxis: download queue orchestration and post-processing for Harmonia.
//!
//! Syntaxis owns the download queue, priority rules, concurrency control, and
//! the post-processing pipeline that runs after each download completes.

pub mod error;
pub mod pipeline;
pub mod types;

pub(crate) mod dispatch;
pub(crate) mod queue;
pub(crate) mod recovery;
pub(crate) mod repo;
pub(crate) mod retry;

pub use error::SyntaxisError;
pub use pipeline::ImportService;
pub use types::{CompletedDownload, DownloadProtocol, QueueItem, QueuePosition, QueueSnapshot};

use std::collections::HashMap;
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, error, info, instrument, warn};

use harmonia_common::ids::DownloadId;
use harmonia_common::{EventReceiver, HarmoniaEvent};
use horismos::SyntaxisConfig;

use ergasia::DownloadEngine;

use crate::dispatch::SlotAllocator;
use crate::pipeline::PipelineItem;
use crate::queue::PriorityQueue;
use crate::retry::{FailureKind, backoff_seconds, classify_failure};

/// Public trait surface for queue management.
pub trait QueueManager: Send + Sync {
    fn enqueue(
        &self,
        item: QueueItem,
    ) -> impl std::future::Future<Output = Result<QueuePosition, SyntaxisError>> + Send;

    fn cancel(
        &self,
        download_id: DownloadId,
    ) -> impl std::future::Future<Output = Result<(), SyntaxisError>> + Send;

    fn reprioritize(
        &self,
        download_id: DownloadId,
        new_priority: u8,
    ) -> impl std::future::Future<Output = Result<(), SyntaxisError>> + Send;

    fn get_queue_state(
        &self,
    ) -> impl std::future::Future<Output = Result<QueueSnapshot, SyntaxisError>> + Send;
}

/// Metadata for a dispatched download; needed for slot release and retry.
#[derive(Debug, Clone)]
struct ActiveEntry {
    queue_id: uuid::Uuid,
    protocol: DownloadProtocol,
    tracker_id: Option<i64>,
    want_id: harmonia_common::ids::WantId,
    release_id: harmonia_common::ids::ReleaseId,
    download_url: String,
    retry_count: u32,
}

/// Internal mutable state guarded by a single Mutex.
struct Inner {
    queue: PriorityQueue,
    allocator: SlotAllocator,
    /// Active downloads keyed by DownloadId string representation.
    active: HashMap<String, ActiveEntry>,
    config: SyntaxisConfig,
}

/// The concrete Syntaxis service, generic over the download engine type.
///
/// Construct via `SyntaxisService::new`, then call `start` to launch the
/// event-listener task that processes Ergasia broadcast events.
pub struct SyntaxisService<E: DownloadEngine + 'static> {
    pool: SqlitePool,
    engine: Arc<E>,
    import_svc: Arc<dyn ImportService>,
    inner: Arc<Mutex<Inner>>,
}

impl<E: DownloadEngine + 'static> SyntaxisService<E> {
    /// Creates a new service and runs startup reconciliation.
    pub async fn new(
        pool: SqlitePool,
        engine: Arc<E>,
        import_svc: Arc<dyn ImportService>,
        config: SyntaxisConfig,
    ) -> Result<Self, SyntaxisError> {
        let mut pq = PriorityQueue::new();
        let recovered = recovery::reload_queue(&pool, &mut pq).await?;
        if recovered > 0 {
            info!(count = recovered, "recovered queue items from database");
        }

        let allocator = SlotAllocator::new(config.max_concurrent_downloads, config.max_per_tracker);

        let inner = Arc::new(Mutex::new(Inner {
            queue: pq,
            allocator,
            active: HashMap::new(),
            config,
        }));

        Ok(Self {
            pool,
            engine,
            import_svc,
            inner,
        })
    }

    /// Launches the event-listener task that processes `DownloadCompleted` and
    /// `DownloadFailed` events from the Ergasia broadcast bus.
    ///
    /// The task runs until `shutdown` is cancelled.
    pub fn start(self: &Arc<Self>, mut event_rx: EventReceiver, shutdown: CancellationToken) {
        let svc = Arc::clone(self);
        let span = tracing::Span::current();
        tokio::spawn(
            async move {
                loop {
                    tokio::select! {
                        biased;
                        _ = shutdown.cancelled() => break,
                        result = event_rx.recv() => {
                            match result {
                                Ok(event) => svc.handle_event(event).await,
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                    warn!(skipped = n, "event bus lagged; some events were dropped");
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                            }
                        }
                    }
                }
                info!("syntaxis event listener stopped");
            }
            .instrument(span),
        );
    }

    async fn handle_event(&self, event: HarmoniaEvent) {
        match event {
            HarmoniaEvent::DownloadCompleted { download_id, path } => {
                self.on_download_completed(download_id, path).await;
            }
            HarmoniaEvent::DownloadFailed {
                download_id,
                reason,
            } => {
                self.on_download_failed(download_id, reason).await;
            }
            _ => {}
        }
    }

    async fn on_download_completed(&self, download_id: DownloadId, path: std::path::PathBuf) {
        let entry = {
            let mut inner = self.inner.lock().await;
            let key = download_id.to_string();
            if let Some(entry) = inner.active.remove(&key) {
                inner.allocator.release(entry.protocol, entry.tracker_id);
                Some(entry)
            } else {
                warn!(%download_id, "DownloadCompleted for unknown download_id");
                None
            }
        };

        if let Some(entry) = entry {
            // Dispatch next eligible item now that a slot freed up.
            self.try_dispatch_next().await;

            let pool = self.pool.clone();
            let engine = Arc::clone(&self.engine);
            let import_svc = Arc::clone(&self.import_svc);
            let span = tracing::Span::current();

            tokio::spawn(
                async move {
                    if let Err(e) = pipeline::run_pipeline(
                        &pool,
                        &engine,
                        &import_svc,
                        download_id,
                        &path,
                        PipelineItem {
                            queue_id: entry.queue_id,
                            want_id: entry.want_id,
                            release_id: entry.release_id,
                            protocol: entry.protocol,
                            tracker_id: entry.tracker_id,
                        },
                    )
                    .await
                    {
                        error!(error = %e, "post-processing pipeline failed");
                    }
                }
                .instrument(span),
            );
        }
    }

    async fn on_download_failed(&self, download_id: DownloadId, reason: String) {
        let entry = {
            let mut inner = self.inner.lock().await;
            let key = download_id.to_string();
            inner.active.remove(&key).inspect(|e| {
                inner.allocator.release(e.protocol, e.tracker_id);
            })
        };

        let Some(entry) = entry else {
            warn!(%download_id, "DownloadFailed for unknown download_id");
            return;
        };

        match classify_failure(&reason) {
            FailureKind::Permanent => {
                error!(%download_id, %reason, "permanent download failure");
                if let Err(e) = repo::mark_failed(&self.pool, entry.queue_id, &reason).await {
                    error!(error = %e, "failed to persist failure status");
                }
            }
            FailureKind::Transient => {
                let retry_count = entry.retry_count;
                let (max_retries, backoff_base) = {
                    let inner = self.inner.lock().await;
                    (
                        inner.config.retry_count,
                        inner.config.retry_backoff_base_seconds,
                    )
                };

                if retry_count >= max_retries {
                    error!(%download_id, attempts = retry_count, "retry budget exhausted");
                    if let Err(e) = repo::mark_failed(
                        &self.pool,
                        entry.queue_id,
                        &format!("retry budget exhausted after {retry_count} attempts: {reason}"),
                    )
                    .await
                    {
                        error!(error = %e, "failed to persist exhausted retry status");
                    }
                } else {
                    let backoff = backoff_seconds(retry_count, backoff_base);
                    info!(
                        %download_id,
                        retry = retry_count + 1,
                        backoff_secs = backoff,
                        "scheduling retry"
                    );

                    if let Err(e) = repo::increment_retry_count(&self.pool, entry.queue_id).await {
                        error!(error = %e, "failed to increment retry count");
                    }
                    if let Err(e) = repo::update_status(&self.pool, entry.queue_id, "queued").await
                    {
                        error!(error = %e, "failed to reset status for retry");
                    }

                    let inner = Arc::clone(&self.inner);
                    let span = tracing::Span::current();
                    let queue_item = QueueItem {
                        id: entry.queue_id,
                        want_id: entry.want_id,
                        release_id: entry.release_id,
                        download_url: entry.download_url,
                        protocol: entry.protocol,
                        priority: 2,
                        tracker_id: entry.tracker_id,
                        info_hash: None,
                    };

                    tokio::spawn(
                        async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(backoff)).await;
                            inner.lock().await.queue.insert(queue_item);
                        }
                        .instrument(span),
                    );
                }
            }
        }

        self.try_dispatch_next().await;
    }

    /// Attempts to dispatch the next eligible item from the queue to Ergasia.
    async fn try_dispatch_next(&self) {
        let item = {
            let mut inner = self.inner.lock().await;
            if !inner.allocator.global_slot_available() {
                return;
            }

            let max_per_tracker = inner.config.max_per_tracker;
            // WHY: Snapshot tracker counts before mutably borrowing the queue.
            // The closure passed to dequeue_eligible cannot hold a reference into
            // `inner` while `inner.queue` is mutably borrowed.
            let tracker_counts = inner.allocator.per_tracker_snapshot();
            let item = inner.queue.dequeue_eligible(|tracker_id| {
                if let Some(id) = tracker_id {
                    tracker_counts.get(&id).copied().unwrap_or(0) < max_per_tracker
                } else {
                    true
                }
            });

            let Some(item) = item else {
                return;
            };

            inner.allocator.acquire(&item);

            let new_dl_id = DownloadId::new();
            inner.active.insert(
                new_dl_id.to_string(),
                ActiveEntry {
                    queue_id: item.id,
                    protocol: item.protocol,
                    tracker_id: item.tracker_id,
                    want_id: item.want_id,
                    release_id: item.release_id,
                    download_url: item.download_url.clone(),
                    retry_count: 0,
                },
            );

            (item, new_dl_id)
        };

        let (queue_item, new_dl_id) = item;
        let engine = Arc::clone(&self.engine);
        let pool = self.pool.clone();
        let span = tracing::Span::current();

        tokio::spawn(
            async move {
                if let Err(e) = repo::update_status(&pool, queue_item.id, "downloading").await {
                    error!(error = %e, "failed to update status to downloading");
                }
                let request = ergasia::DownloadRequest {
                    download_url: queue_item.download_url,
                    protocol: ergasia::DownloadProtocol::Torrent,
                    download_id: new_dl_id,
                    want_id: queue_item.want_id,
                };
                if let Err(e) = engine.start_download(request).await {
                    error!(error = %e, "failed to dispatch download to engine");
                }
            }
            .instrument(span),
        );
    }
}

impl<E: DownloadEngine + 'static> QueueManager for Arc<SyntaxisService<E>> {
    #[instrument(skip(self))]
    async fn enqueue(&self, item: QueueItem) -> Result<QueuePosition, SyntaxisError> {
        // Persist to DB first for durability.
        repo::insert_queue_item(
            &self.pool,
            item.id,
            item.want_id.as_uuid().as_bytes(),
            item.release_id.as_uuid().as_bytes(),
            &item.download_url,
            item.protocol.as_db_str(),
            item.priority,
            item.tracker_id,
            item.info_hash.as_deref(),
        )
        .await
        .map_err(|source| SyntaxisError::Database {
            source,
            location: snafu::location!(),
        })?;

        if item.priority == 4 {
            // Interactive bypass: try to acquire a slot and dispatch immediately.
            let slot_available = {
                let inner = self.inner.lock().await;
                inner.allocator.has_slot(&item)
            };

            if slot_available {
                let new_dl_id = DownloadId::new();
                {
                    let mut inner = self.inner.lock().await;
                    inner.allocator.acquire(&item);
                    inner.active.insert(
                        new_dl_id.to_string(),
                        ActiveEntry {
                            queue_id: item.id,
                            protocol: item.protocol,
                            tracker_id: item.tracker_id,
                            want_id: item.want_id,
                            release_id: item.release_id,
                            download_url: item.download_url.clone(),
                            retry_count: 0,
                        },
                    );
                }

                let pool = self.pool.clone();
                let engine = Arc::clone(&self.engine);
                let queue_id = item.id;
                let span = tracing::Span::current();
                tokio::spawn(
                    async move {
                        if let Err(e) = repo::update_status(&pool, queue_id, "downloading").await {
                            error!(error = %e, "failed to update status");
                        }
                        let request = ergasia::DownloadRequest {
                            download_url: item.download_url,
                            protocol: ergasia::DownloadProtocol::Torrent,
                            download_id: new_dl_id,
                            want_id: item.want_id,
                        };
                        if let Err(e) = engine.start_download(request).await {
                            error!(error = %e, "failed to dispatch interactive download");
                        }
                    }
                    .instrument(span),
                );

                return Ok(QueuePosition {
                    position: 0,
                    estimated_wait_secs: Some(0),
                });
            }
            // No slot available: fall through and queue at priority 3.
        }

        let position = {
            let mut inner = self.inner.lock().await;
            let pos = inner.queue.len();
            inner.queue.insert(item);
            pos
        };

        self.try_dispatch_next().await;

        Ok(QueuePosition {
            position,
            estimated_wait_secs: None,
        })
    }

    #[instrument(skip(self))]
    async fn cancel(&self, download_id: DownloadId) -> Result<(), SyntaxisError> {
        let key = download_id.to_string();
        let entry = {
            let mut inner = self.inner.lock().await;
            inner.active.remove(&key).inspect(|e| {
                inner.allocator.release(e.protocol, e.tracker_id);
            })
        };

        if let Some(entry) = entry {
            self.engine
                .cancel_download(download_id)
                .await
                .map_err(|_| SyntaxisError::DispatchFailed {
                    location: snafu::location!(),
                })?;
            repo::mark_failed(&self.pool, entry.queue_id, "cancelled by user")
                .await
                .map_err(|source| SyntaxisError::Database {
                    source,
                    location: snafu::location!(),
                })?;
            self.try_dispatch_next().await;
            return Ok(());
        }

        Err(SyntaxisError::ItemNotFound {
            id: key,
            location: snafu::location!(),
        })
    }

    #[instrument(skip(self))]
    async fn reprioritize(
        &self,
        download_id: DownloadId,
        new_priority: u8,
    ) -> Result<(), SyntaxisError> {
        let id_str = download_id.to_string();

        {
            let inner = self.inner.lock().await;
            // If already active, re-prioritization is a no-op.
            if inner.active.contains_key(&id_str) {
                return Ok(());
            }
        }

        // Try to parse as Uuid and find in queue.
        let uuid = uuid::Uuid::parse_str(&id_str).map_err(|_| SyntaxisError::ItemNotFound {
            id: id_str.clone(),
            location: snafu::location!(),
        })?;

        let found = {
            let mut inner = self.inner.lock().await;
            inner.queue.reprioritize(uuid, new_priority)
        };

        if !found {
            return Err(SyntaxisError::ItemNotFound {
                id: id_str,
                location: snafu::location!(),
            });
        }

        repo::update_priority(&self.pool, uuid, new_priority)
            .await
            .map_err(|source| SyntaxisError::Database {
                source,
                location: snafu::location!(),
            })?;

        if new_priority == 4 {
            self.try_dispatch_next().await;
        }
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_queue_state(&self) -> Result<QueueSnapshot, SyntaxisError> {
        let (queued_items, active_downloads) = {
            let inner = self.inner.lock().await;
            let queued: Vec<QueueItem> = inner.queue.items().cloned().collect();
            let active: Vec<QueueItem> = inner
                .active
                .values()
                .map(|e| QueueItem {
                    id: e.queue_id,
                    want_id: e.want_id,
                    release_id: e.release_id,
                    download_url: e.download_url.clone(),
                    protocol: e.protocol,
                    priority: 4,
                    tracker_id: e.tracker_id,
                    info_hash: None,
                })
                .collect();
            (queued, active)
        };

        let completed_count = repo::count_by_status(&self.pool, "completed")
            .await
            .map_err(|source| SyntaxisError::Database {
                source,
                location: snafu::location!(),
            })?;
        let failed_count = repo::count_by_status(&self.pool, "failed")
            .await
            .map_err(|source| SyntaxisError::Database {
                source,
                location: snafu::location!(),
            })?;

        Ok(QueueSnapshot {
            active_downloads,
            queued_items,
            completed_count,
            failed_count,
        })
    }
}
