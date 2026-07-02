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

use std::collections::HashMap;
use std::sync::Arc;

use ergasia::{DownloadEngine, DownloadState};
pub use error::SyntaxisError;
use horismos::SyntaxisConfig;
pub use pipeline::ImportService;
use sqlx::SqlitePool;
use themelion::ids::DownloadId;
use themelion::{EventReceiver, HarmoniaEvent};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, error, info, instrument, warn};
pub use types::{CompletedDownload, DownloadProtocol, QueueItem, QueuePosition, QueueSnapshot};

use crate::dispatch::SlotAllocator;
use crate::pipeline::PipelineItem;
use crate::queue::PriorityQueue;
use crate::retry::{FailureKind, backoff_seconds, classify_failure};

/// Interval between periodic active-download reconciliation passes.
///
/// WHY: broadcast `Lagged` detection alone can miss a terminal event that
/// races the lag window; a periodic pass bounds any slot leak to one interval.
const RECONCILE_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_secs(60);

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
    download_id: DownloadId,
    queue_id: uuid::Uuid,
    protocol: DownloadProtocol,
    tracker_id: Option<i64>,
    want_id: themelion::ids::WantId,
    release_id: themelion::ids::ReleaseId,
    download_url: String,
    retry_count: u32,
}

impl ActiveEntry {
    fn from_item(item: &QueueItem, download_id: DownloadId) -> Self {
        Self {
            download_id,
            queue_id: item.id,
            protocol: item.protocol,
            tracker_id: item.tracker_id,
            want_id: item.want_id,
            release_id: item.release_id,
            download_url: item.download_url.clone(),
            retry_count: item.retry_count,
        }
    }
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
/// Construct via `DownloadQueue::new`, then call `start` to launch the
/// event-listener task that processes Ergasia broadcast events.
pub struct DownloadQueue<E: DownloadEngine + 'static> {
    pool: SqlitePool,
    engine: Arc<E>,
    import_svc: Arc<dyn ImportService>,
    inner: Arc<Mutex<Inner>>,
}

impl<E: DownloadEngine + 'static> DownloadQueue<E> {
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
            info!(count = recovered, "recovered queue items FROM database");
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
    /// The task also reconciles in-memory active downloads against
    /// engine-reported state whenever the broadcast bus lags (dropped events
    /// would otherwise leak slots permanently) and on a periodic interval.
    ///
    /// The task runs until `shutdown` is cancelled. The returned handle lets
    /// the host await the listener's drain during graceful shutdown (or abort
    /// it); dropping the handle detaches the task.
    pub fn start(
        self: &Arc<Self>,
        mut event_rx: EventReceiver,
        shutdown: CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        let svc = Arc::clone(self);
        let span = tracing::Span::current();
        tokio::spawn(
            async move {
                let mut reconcile_tick = tokio::time::interval_at(
                    tokio::time::Instant::now() + RECONCILE_INTERVAL,
                    RECONCILE_INTERVAL,
                );
                reconcile_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                loop {
                    tokio::select! {
                        biased;
                        _ = shutdown.cancelled() => break,
                        result = event_rx.recv() => {
                            match result {
                                Ok(event) => svc.handle_event(event).await,
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                    warn!(skipped = n, "event bus lagged; reconciling active downloads");
                                    svc.reconcile_active().await;
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                            }
                        }
                        _ = reconcile_tick.tick() => {
                            svc.reconcile_active().await;
                        }
                    }
                }
                info!("syntaxis event listener stopped");
            }
            .instrument(span),
        )
    }

    /// Cancels a queue item by its `download_queue` row id, stopping the live
    /// engine download when one is active.
    ///
    /// Resolution order: an active download is claimed (slot released) and
    /// cancelled on the engine; a queued item is removed from the in-memory
    /// tier queue; a DB-only row (persisted by the HTTP API before this
    /// service observed it) is still deleted. The row is deleted in every
    /// successful branch so a cancelled item can never be re-dispatched by
    /// startup recovery.
    pub async fn cancel_by_queue_id(&self, queue_id: uuid::Uuid) -> Result<(), SyntaxisError> {
        // INVARIANT: claim-and-release is one critical section, so a
        // concurrent completion/failure event cannot double-release the slot.
        let active_entry = {
            let mut inner = self.inner.lock().await;
            let key = inner
                .active
                .iter()
                .find(|(_, e)| e.queue_id == queue_id)
                .map(|(k, _)| k.clone());
            key.and_then(|key| {
                inner.active.remove(&key).inspect(|e| {
                    inner.allocator.release(e.protocol, e.tracker_id);
                })
            })
        };

        if let Some(entry) = active_entry {
            if let Err(e) = self.engine.cancel_download(entry.download_id).await {
                // WHY: the entry is already claimed; surface the failure loud
                // and leave the row terminal so recovery cannot re-dispatch a
                // torrent the engine may still be running.
                error!(error = %e, %queue_id, "engine cancel failed for active download");
                repo::mark_failed(
                    &self.pool,
                    queue_id,
                    &format!("cancel requested; engine cancel failed: {e}"),
                )
                .await
                .map_err(|source| SyntaxisError::Database {
                    source,
                    location: snafu::location!(),
                })?;
                self.try_dispatch_next().await;
                return Err(SyntaxisError::DispatchFailed {
                    location: snafu::location!(),
                });
            }
            repo::delete_queue_item(&self.pool, queue_id)
                .await
                .map_err(|source| SyntaxisError::Database {
                    source,
                    location: snafu::location!(),
                })?;
            self.try_dispatch_next().await;
            return Ok(());
        }

        let removed_from_queue = {
            let mut inner = self.inner.lock().await;
            inner.queue.remove(queue_id).is_some()
        };
        let deleted = repo::delete_queue_item(&self.pool, queue_id)
            .await
            .map_err(|source| SyntaxisError::Database {
                source,
                location: snafu::location!(),
            })?;

        if removed_from_queue || deleted > 0 {
            return Ok(());
        }
        Err(SyntaxisError::ItemNotFound {
            id: queue_id.to_string(),
            location: snafu::location!(),
        })
    }

    /// Changes the dispatch priority of a queue item by its `download_queue`
    /// row id, re-bucketing the live in-memory queue.
    ///
    /// Priority 4 (interactive) pulls the item out of the queue and
    /// dispatches it immediately when a slot is free, demoting to tier 3
    /// otherwise — mirroring `enqueue`'s interactive path. An active download
    /// is untouched: priority is a queue-ordering concern with no effect on a
    /// running transfer. A DB-only row still has its persisted priority
    /// updated.
    pub async fn reprioritize_by_queue_id(
        &self,
        queue_id: uuid::Uuid,
        new_priority: u8,
    ) -> Result<(), SyntaxisError> {
        let new_priority = new_priority.clamp(1, 4);

        enum Placement {
            Active,
            Requeued(u8),
            Dispatch(QueueItem, DownloadId),
            NotInMemory,
        }

        let placement = {
            let mut inner = self.inner.lock().await;
            if inner.active.values().any(|e| e.queue_id == queue_id) {
                Placement::Active
            } else if new_priority == 4 {
                match inner.queue.reprioritize_to_interactive(queue_id) {
                    Some(mut item) => {
                        // WHY: check-and-acquire under one critical section
                        // (same TOCTOU guard as the interactive enqueue path).
                        if inner.allocator.has_slot(&item) {
                            inner.allocator.acquire(&item);
                            let download_id = DownloadId::new();
                            inner.active.insert(
                                download_id.to_string(),
                                ActiveEntry::from_item(&item, download_id),
                            );
                            Placement::Dispatch(item, download_id)
                        } else {
                            // No slot: demote to tier 3 and keep it queued —
                            // the tier queue only holds priorities 1-3.
                            item.priority = 3;
                            inner.queue.insert(item);
                            Placement::Requeued(3)
                        }
                    }
                    None => Placement::NotInMemory,
                }
            } else if inner.queue.reprioritize(queue_id, new_priority) {
                Placement::Requeued(new_priority)
            } else {
                Placement::NotInMemory
            }
        };

        match placement {
            Placement::Active => Ok(()),
            Placement::Requeued(priority) => {
                repo::update_priority(&self.pool, queue_id, priority)
                    .await
                    .map_err(|source| SyntaxisError::Database {
                        source,
                        location: snafu::location!(),
                    })?;
                Ok(())
            }
            Placement::Dispatch(item, download_id) => {
                repo::update_priority(&self.pool, queue_id, 4)
                    .await
                    .map_err(|source| SyntaxisError::Database {
                        source,
                        location: snafu::location!(),
                    })?;
                if !Self::dispatch_active(&self.inner, &self.engine, &self.pool, item, download_id)
                    .await
                {
                    // WHY: the failed interactive dispatch released its slot;
                    // hand the freed capacity to queued work.
                    Self::dispatch_next_inner(&self.inner, &self.engine, &self.pool).await;
                }
                Ok(())
            }
            Placement::NotInMemory => {
                let updated = repo::update_priority(&self.pool, queue_id, new_priority)
                    .await
                    .map_err(|source| SyntaxisError::Database {
                        source,
                        location: snafu::location!(),
                    })?;
                if updated > 0 {
                    Ok(())
                } else {
                    Err(SyntaxisError::ItemNotFound {
                        id: queue_id.to_string(),
                        location: snafu::location!(),
                    })
                }
            }
        }
    }

    /// Reconciles in-memory active entries against engine-reported state.
    ///
    /// Any download the engine reports as terminal while still registered as
    /// active has lost its completion/failure event; it is routed through the
    /// failure path so its slot is released and its retry budget decides
    /// between re-queue and permanent failure.
    async fn reconcile_active(&self) {
        let snapshot: Vec<DownloadId> = {
            let inner = self.inner.lock().await;
            inner.active.values().map(|e| e.download_id).collect()
        };

        for download_id in snapshot {
            let state = match self.engine.get_progress(download_id).await {
                Ok(progress) => progress.state,
                Err(e) => {
                    // WHY: a get_progress error cannot distinguish a lost
                    // download from a dispatch still registering with the
                    // engine; reaping here could kill a healthy in-flight
                    // dispatch. Keep the entry; a later pass or a real event
                    // settles it.
                    warn!(%download_id, error = %e, "reconciliation could not query engine state");
                    continue;
                }
            };
            match state {
                DownloadState::Completed
                | DownloadState::Seeding
                | DownloadState::SeedPolicySatisfied
                | DownloadState::Failed
                | DownloadState::Deleted => {
                    warn!(%download_id, %state, "terminal engine state with no processed event; reconciling");
                    // WHY: get_progress carries no completion path, so the
                    // post-processing pipeline cannot be synthesized here.
                    // The failure path releases the slot and re-queues within
                    // the retry budget; an engine-side resume of a finished
                    // download then re-emits the real completion event.
                    self.on_download_failed(
                        download_id,
                        format!(
                            "terminal engine state {state} observed during reconciliation; event was dropped"
                        ),
                    )
                    .await;
                }
                DownloadState::Queued
                | DownloadState::Initializing
                | DownloadState::Downloading => {}
                // WHY: DownloadState is non_exhaustive upstream; an unknown
                // variant is treated as in-flight so the slot is never
                // released for a state this crate cannot classify.
                _ => {}
            }
        }
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
                    let engine = Arc::clone(&self.engine);
                    let pool = self.pool.clone();
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
                        retry_count: retry_count + 1,
                    };

                    tokio::spawn(
                        async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(backoff)).await;
                            inner.lock().await.queue.insert(queue_item);
                            // WHY: the dispatcher only wakes on events and
                            // enqueues; the re-queued item must trigger its own
                            // dispatch pass or it sits until unrelated activity
                            // arrives.
                            Self::dispatch_next_inner(&inner, &engine, &pool).await;
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
        Self::dispatch_next_inner(&self.inner, &self.engine, &self.pool).await;
    }

    /// Dequeue-and-dispatch pass callable from tasks that hold only the shared
    /// state handles (retry timers, spawned dispatch tasks).
    async fn dispatch_next_inner(inner: &Arc<Mutex<Inner>>, engine: &Arc<E>, pool: &SqlitePool) {
        let Some((queue_item, download_id)) = Self::next_dispatchable(inner).await else {
            return;
        };

        let inner = Arc::clone(inner);
        let engine = Arc::clone(engine);
        let pool = pool.clone();
        let span = tracing::Span::current();
        tokio::spawn(
            async move {
                let mut current = (queue_item, download_id);
                loop {
                    if Self::dispatch_active(&inner, &engine, &pool, current.0, current.1).await {
                        break;
                    }
                    // WHY: the failed dispatch released its slot; pull the next
                    // eligible item so the queue does not stall until unrelated
                    // activity wakes the dispatcher.
                    match Self::next_dispatchable(&inner).await {
                        Some(next) => current = next,
                        None => break,
                    }
                }
            }
            .instrument(span),
        );
    }

    /// Pops the next eligible queue item, acquires its slot, and registers the
    /// active entry, all under one critical section.
    async fn next_dispatchable(inner: &Arc<Mutex<Inner>>) -> Option<(QueueItem, DownloadId)> {
        let mut inner = inner.lock().await;
        if !inner.allocator.global_slot_available() {
            return None;
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
        })?;

        inner.allocator.acquire(&item);
        let download_id = DownloadId::new();
        inner.active.insert(
            download_id.to_string(),
            ActiveEntry::from_item(&item, download_id),
        );
        Some((item, download_id))
    }

    /// Marks the item downloading and hands it to the engine.
    ///
    /// Returns `false` when the item never reached the engine: the slot is
    /// released, the active entry removed, and the row marked failed, so the
    /// caller can dispatch a replacement.
    async fn dispatch_active(
        inner: &Arc<Mutex<Inner>>,
        engine: &Arc<E>,
        pool: &SqlitePool,
        queue_item: QueueItem,
        download_id: DownloadId,
    ) -> bool {
        let queue_id = queue_item.id;
        let protocol = queue_item.protocol;
        let tracker_id = queue_item.tracker_id;

        if let Err(e) = repo::update_status(pool, queue_id, "downloading").await {
            error!(error = %e, "failed to UPDATE status to downloading");
        }

        let engine_protocol = match protocol {
            DownloadProtocol::Torrent => ergasia::DownloadProtocol::Torrent,
            // WHY: the engine only speaks BitTorrent today. Failing loud here
            // prevents a Usenet-tagged item from being silently pushed onto a
            // torrent swarm (wrong network, wrong protocol, leaked intent).
            DownloadProtocol::Usenet => {
                error!(%queue_id, %protocol, "download engine does not support protocol; marking failed");
                Self::rollback_dispatch(inner, protocol, tracker_id, download_id).await;
                if let Err(e) = repo::mark_failed(
                    pool,
                    queue_id,
                    &format!("protocol {protocol} not supported by download engine"),
                )
                .await
                {
                    error!(error = %e, "failed to persist unsupported-protocol failure");
                }
                return false;
            }
        };

        let request = ergasia::DownloadRequest {
            download_url: queue_item.download_url,
            protocol: engine_protocol,
            download_id,
            want_id: queue_item.want_id,
        };
        if let Err(e) = engine.start_download(request).await {
            error!(error = %e, %queue_id, "failed to dispatch download to engine");
            Self::rollback_dispatch(inner, protocol, tracker_id, download_id).await;
            if let Err(db_e) =
                repo::mark_failed(pool, queue_id, &format!("engine dispatch failed: {e}")).await
            {
                error!(error = %db_e, "failed to persist dispatch failure");
            }
            return false;
        }
        true
    }

    /// Reverts the slot acquisition and active registration of a dispatch that
    /// never reached the engine.
    async fn rollback_dispatch(
        inner: &Arc<Mutex<Inner>>,
        protocol: DownloadProtocol,
        tracker_id: Option<i64>,
        download_id: DownloadId,
    ) {
        let mut inner = inner.lock().await;
        // INVARIANT: release only while the entry is still registered, so a
        // concurrent completion/failure event cannot double-release the slot.
        if inner.active.remove(&download_id.to_string()).is_some() {
            inner.allocator.release(protocol, tracker_id);
        }
    }
}

impl<E: DownloadEngine + 'static> QueueManager for Arc<DownloadQueue<E>> {
    #[instrument(skip(self))]
    async fn enqueue(&self, mut item: QueueItem) -> Result<QueuePosition, SyntaxisError> {
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
            let download_id = DownloadId::new();
            // WHY: check-and-acquire must be one critical section; a second
            // enqueue interleaving between a separate has_slot and acquire
            // could push active_total past max_concurrent (TOCTOU).
            let acquired = {
                let mut inner = self.inner.lock().await;
                if inner.allocator.has_slot(&item) {
                    inner.allocator.acquire(&item);
                    inner.active.insert(
                        download_id.to_string(),
                        ActiveEntry::from_item(&item, download_id),
                    );
                    true
                } else {
                    false
                }
            };

            if acquired {
                let inner = Arc::clone(&self.inner);
                let engine = Arc::clone(&self.engine);
                let pool = self.pool.clone();
                let span = tracing::Span::current();
                tokio::spawn(
                    async move {
                        if !DownloadQueue::dispatch_active(
                            &inner,
                            &engine,
                            &pool,
                            item,
                            download_id,
                        )
                        .await
                        {
                            // WHY: the interactive dispatch failed and released
                            // its slot; hand the freed capacity to queued work.
                            DownloadQueue::dispatch_next_inner(&inner, &engine, &pool).await;
                        }
                    }
                    .instrument(span),
                );

                return Ok(QueuePosition {
                    position: 0,
                    estimated_wait_secs: Some(0),
                });
            }
            // No slot available: demote to priority 3 and fall through to the
            // queued path (the in-memory queue only holds tiers 1-3).
            item.priority = 3;
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
        {
            let inner = self.inner.lock().await;
            // If already active, re-prioritization is a no-op.
            if inner.active.contains_key(&download_id.to_string()) {
                return Ok(());
            }
        }

        // WHY: queued items are keyed by their queue row uuid; the prior
        // Uuid::parse_str on the Display form always failed on the "dl-"
        // prefix, making this method unreachable for queued items.
        self.reprioritize_by_queue_id(*download_id.as_uuid(), new_priority)
            .await
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
                    retry_count: e.retry_count,
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

#[cfg(test)]
mod tests {
    use std::sync::Mutex as StdMutex;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use apotheke::migrate::MIGRATOR;
    use ergasia::{DownloadProgress, ErgasiaError, ExtractionResult};
    use themelion::ids::{ReleaseId, WantId};
    use tokio::sync::mpsc;

    use super::*;

    struct MockEngine {
        started_tx: mpsc::UnboundedSender<(DownloadId, String)>,
        calls: AtomicUsize,
        fail_remaining: AtomicUsize,
        progress_state: StdMutex<DownloadState>,
        progress_unavailable: AtomicBool,
        cancelled: StdMutex<Vec<DownloadId>>,
        fail_cancels: AtomicBool,
    }

    impl MockEngine {
        fn create() -> (Arc<Self>, mpsc::UnboundedReceiver<(DownloadId, String)>) {
            let (tx, rx) = mpsc::unbounded_channel();
            (
                Arc::new(Self {
                    started_tx: tx,
                    calls: AtomicUsize::new(0),
                    fail_remaining: AtomicUsize::new(0),
                    progress_state: StdMutex::new(DownloadState::Downloading),
                    progress_unavailable: AtomicBool::new(false),
                    cancelled: StdMutex::new(Vec::new()),
                    fail_cancels: AtomicBool::new(false),
                }),
                rx,
            )
        }

        fn fail_next_starts(&self, n: usize) {
            self.fail_remaining.store(n, Ordering::SeqCst);
        }

        fn set_progress_state(&self, state: DownloadState) {
            *self.progress_state.lock().unwrap() = state;
        }

        fn set_progress_unavailable(&self) {
            self.progress_unavailable.store(true, Ordering::SeqCst);
        }

        fn start_calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }

        fn fail_cancels(&self) {
            self.fail_cancels.store(true, Ordering::SeqCst);
        }

        fn cancelled_ids(&self) -> Vec<DownloadId> {
            self.cancelled.lock().unwrap().clone()
        }
    }

    impl DownloadEngine for MockEngine {
        async fn start_download(
            &self,
            request: ergasia::DownloadRequest,
        ) -> Result<DownloadId, ErgasiaError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let should_fail = self
                .fail_remaining
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                .is_ok();
            if should_fail {
                return Err(ErgasiaError::AddTorrent {
                    reason: "injected start failure".to_string(),
                    error: "injected".to_string(),
                    location: snafu::location!(),
                });
            }
            self.started_tx
                .send((request.download_id, request.download_url))
                .ok();
            Ok(request.download_id)
        }

        async fn cancel_download(&self, download_id: DownloadId) -> Result<(), ErgasiaError> {
            if self.fail_cancels.load(Ordering::SeqCst) {
                return Err(ErgasiaError::TorrentNotFound {
                    download_id,
                    location: snafu::location!(),
                });
            }
            self.cancelled.lock().unwrap().push(download_id);
            Ok(())
        }

        async fn get_progress(
            &self,
            download_id: DownloadId,
        ) -> Result<DownloadProgress, ErgasiaError> {
            if self.progress_unavailable.load(Ordering::SeqCst) {
                return Err(ErgasiaError::TorrentNotFound {
                    download_id,
                    location: snafu::location!(),
                });
            }
            Ok(DownloadProgress {
                download_id,
                state: *self.progress_state.lock().unwrap(),
                percent_complete: 0,
                download_speed_bps: 0,
                upload_speed_bps: 0,
                peers_connected: 0,
                seeders: 0,
                eta_seconds: None,
            })
        }

        async fn extract(
            &self,
            _download_path: &std::path::Path,
            _output_dir: &std::path::Path,
        ) -> Result<Option<ExtractionResult>, ErgasiaError> {
            Ok(None)
        }
    }

    struct NoopImportService;

    impl ImportService for NoopImportService {
        fn import(
            &self,
            _completed: CompletedDownload,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>>
        {
            Box::pin(async { Ok(()) })
        }
    }

    const RECV_TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(5);

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    fn test_config(max_concurrent: usize, retry_count: u32, backoff_base: u64) -> SyntaxisConfig {
        SyntaxisConfig {
            max_concurrent_downloads: max_concurrent,
            max_per_tracker: 3,
            retry_count,
            retry_backoff_base_seconds: backoff_base,
            stalled_download_timeout_hours: 24,
        }
    }

    fn make_item(protocol: DownloadProtocol, priority: u8) -> QueueItem {
        QueueItem {
            id: uuid::Uuid::now_v7(),
            want_id: WantId::new(),
            release_id: ReleaseId::new(),
            download_url: format!("magnet:?xt=urn:btih:{}", uuid::Uuid::now_v7()),
            protocol,
            priority,
            tracker_id: None,
            info_hash: None,
            retry_count: 0,
        }
    }

    async fn make_service(
        pool: SqlitePool,
        engine: Arc<MockEngine>,
        config: SyntaxisConfig,
    ) -> Arc<DownloadQueue<MockEngine>> {
        let import_svc: Arc<dyn ImportService> = Arc::new(NoopImportService);
        Arc::new(
            DownloadQueue::new(pool, engine, import_svc, config)
                .await
                .unwrap(),
        )
    }

    async fn active_total(svc: &DownloadQueue<MockEngine>) -> usize {
        svc.inner.lock().await.allocator.active_total()
    }

    async fn active_contains(svc: &DownloadQueue<MockEngine>, download_id: DownloadId) -> bool {
        svc.inner
            .lock()
            .await
            .active
            .contains_key(&download_id.to_string())
    }

    async fn queued_len(svc: &DownloadQueue<MockEngine>) -> usize {
        svc.inner.lock().await.queue.len()
    }

    async fn row_state(pool: &SqlitePool, id: uuid::Uuid) -> (String, Option<String>, i64) {
        let row = repo::get_queue_item(pool, id).await.unwrap().unwrap();
        (row.status, row.failed_reason, row.retry_count)
    }

    async fn wait_for_status(
        pool: &SqlitePool,
        id: uuid::Uuid,
        status: &str,
    ) -> (String, Option<String>, i64) {
        for _ in 0..500 {
            let row = row_state(pool, id).await;
            if row.0 == status {
                return row;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
        panic!("row {id} never reached status {status}");
    }

    // WHY: paused-clock tests must not await tokio timers while waiting on the
    // sqlite worker thread (auto-advance would fire unrelated timers); a real
    // thread sleep between yields lets external work land without moving the
    // paused clock.
    async fn settle_until(mut cond: impl FnMut() -> bool) {
        for _ in 0..5000 {
            if cond() {
                return;
            }
            tokio::task::yield_now().await;
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        panic!("condition not reached within settle budget");
    }

    // ── #393: dispatch failure rolls back slot, entry, and DB row ──────────

    #[tokio::test]
    async fn dispatch_failure_releases_slot_and_marks_failed() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        engine.fail_next_starts(1);
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 3, 0)).await;

        let item = make_item(DownloadProtocol::Torrent, 2);
        let queue_id = item.id;
        svc.enqueue(item).await.unwrap();

        let (_, reason, _) = wait_for_status(&pool, queue_id, "failed").await;
        assert!(
            reason
                .unwrap_or_default()
                .contains("engine dispatch failed"),
            "failure reason must name the dispatch failure"
        );
        assert_eq!(active_total(&svc).await, 0, "slot must be released");
        assert_eq!(queued_len(&svc).await, 0);

        // A freed slot must accept new work.
        let second = make_item(DownloadProtocol::Torrent, 2);
        svc.enqueue(second).await.unwrap();
        let (dl_id, _) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(active_contains(&svc, dl_id).await);
    }

    #[tokio::test]
    async fn interactive_dispatch_failure_releases_slot_and_marks_failed() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        engine.fail_next_starts(1);
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(1, 3, 0)).await;

        let item = make_item(DownloadProtocol::Torrent, 4);
        let queue_id = item.id;
        let pos = svc.enqueue(item).await.unwrap();
        assert_eq!(pos.position, 0);

        let (_, reason, _) = wait_for_status(&pool, queue_id, "failed").await;
        assert!(
            reason
                .unwrap_or_default()
                .contains("engine dispatch failed")
        );
        assert_eq!(
            active_total(&svc).await,
            0,
            "interactive slot must be released"
        );

        // With max_concurrent = 1, a second interactive item only dispatches
        // if the failed dispatch really freed its slot.
        let second = make_item(DownloadProtocol::Torrent, 4);
        let pos = svc.enqueue(second).await.unwrap();
        assert_eq!(pos.estimated_wait_secs, Some(0));
        let (dl_id, _) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(active_contains(&svc, dl_id).await);
    }

    #[tokio::test]
    async fn dispatch_failure_dispatches_replacement_from_queue() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        engine.fail_next_starts(1);
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(1, 3, 0)).await;

        let first = make_item(DownloadProtocol::Torrent, 2);
        let first_id = first.id;
        let second = make_item(DownloadProtocol::Torrent, 2);
        let second_url = second.download_url.clone();
        svc.enqueue(first).await.unwrap();
        svc.enqueue(second).await.unwrap();

        let (_, url) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(url, second_url, "the queued item must take the freed slot");
        wait_for_status(&pool, first_id, "failed").await;
        assert_eq!(active_total(&svc).await, 1);
        assert_eq!(queued_len(&svc).await, 0);
    }

    // ── #424: non-torrent protocols fail loud, never mis-route ─────────────

    #[tokio::test]
    async fn usenet_item_never_reaches_torrent_engine() {
        let pool = test_pool().await;
        let (engine, _started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 3, 0)).await;

        let item = make_item(DownloadProtocol::Usenet, 2);
        let queue_id = item.id;
        svc.enqueue(item).await.unwrap();

        let (_, reason, _) = wait_for_status(&pool, queue_id, "failed").await;
        assert!(
            reason.unwrap_or_default().contains("not supported"),
            "failure reason must name the unsupported protocol"
        );
        assert_eq!(
            engine.start_calls(),
            0,
            "a usenet item must never be handed to the torrent engine"
        );
        assert_eq!(
            active_total(&svc).await,
            0,
            "the guard must release the slot"
        );
    }

    #[tokio::test]
    async fn interactive_usenet_item_never_reaches_torrent_engine() {
        let pool = test_pool().await;
        let (engine, _started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 3, 0)).await;

        let item = make_item(DownloadProtocol::Usenet, 4);
        let queue_id = item.id;
        svc.enqueue(item).await.unwrap();

        let (_, reason, _) = wait_for_status(&pool, queue_id, "failed").await;
        assert!(reason.unwrap_or_default().contains("not supported"));
        assert_eq!(engine.start_calls(), 0);
        assert_eq!(active_total(&svc).await, 0);
    }

    #[tokio::test]
    async fn torrent_item_still_dispatches_unaffected() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 3, 0)).await;

        let item = make_item(DownloadProtocol::Torrent, 2);
        let queue_id = item.id;
        let url = item.download_url.clone();
        svc.enqueue(item).await.unwrap();

        let (dl_id, started_url) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(started_url, url);
        assert!(active_contains(&svc, dl_id).await);
        assert_eq!(active_total(&svc).await, 1);
        let (status, _, _) = row_state(&pool, queue_id).await;
        assert_eq!(status, "downloading");
    }

    // ── #425: interactive fast-path check-and-acquire is atomic ────────────

    #[tokio::test]
    async fn concurrent_priority4_enqueue_never_exceeds_max_concurrent() {
        let pool = test_pool().await;
        let (engine, _started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(1, 3, 0)).await;

        let mut handles = Vec::new();
        for _ in 0..8 {
            let svc = Arc::clone(&svc);
            handles.push(tokio::spawn(async move {
                svc.enqueue(make_item(DownloadProtocol::Torrent, 4)).await
            }));
        }

        let mut dispatched = 0usize;
        let mut queued = 0usize;
        for handle in handles {
            let pos = handle.await.unwrap().unwrap();
            if pos.estimated_wait_secs == Some(0) {
                dispatched += 1;
            } else {
                queued += 1;
            }
        }

        assert_eq!(dispatched, 1, "exactly one enqueue may win the single slot");
        assert_eq!(queued, 7);
        assert_eq!(
            active_total(&svc).await,
            1,
            "active count must never exceed max_concurrent"
        );
        assert_eq!(queued_len(&svc).await, 7);

        settle_until(|| engine.start_calls() == 1).await;
        for _ in 0..50 {
            tokio::task::yield_now().await;
        }
        assert_eq!(
            engine.start_calls(),
            1,
            "no completions were sent, so only one start is legal"
        );

        let snapshot = svc.get_queue_state().await.unwrap();
        assert!(
            snapshot.queued_items.iter().all(|i| i.priority == 3),
            "losers of the interactive race must queue at priority 3"
        );
    }

    #[tokio::test]
    async fn priority4_without_slot_queues_at_priority_3() {
        let pool = test_pool().await;
        let (engine, _started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(0, 3, 0)).await;

        svc.enqueue(make_item(DownloadProtocol::Torrent, 4))
            .await
            .unwrap();

        assert_eq!(engine.start_calls(), 0);
        assert_eq!(queued_len(&svc).await, 1);
        let snapshot = svc.get_queue_state().await.unwrap();
        assert_eq!(
            snapshot.queued_items.first().map(|i| i.priority),
            Some(3),
            "an interactive item without a slot must demote to tier 3"
        );
    }

    // ── #394 + #428: a retried download re-dispatches after backoff ────────

    #[tokio::test]
    async fn retry_reenqueue_dispatches_after_backoff() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 3, 30)).await;

        let item = make_item(DownloadProtocol::Torrent, 2);
        let queue_id = item.id;
        let url = item.download_url.clone();
        svc.enqueue(item).await.unwrap();

        let (first_id, first_url) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first_url, url);

        svc.on_download_failed(first_id, "connection timeout".to_string())
            .await;

        // WHY: pause AFTER the failure bookkeeping so no sqlx acquire awaits
        // under a paused clock (auto-advance would fire its acquire-timeout
        // timer while the sqlite worker thread is still responding).
        tokio::time::pause();
        // WHY: yield so the spawned retry task registers its backoff timer
        // before the clock is advanced.
        for _ in 0..20 {
            tokio::task::yield_now().await;
        }
        assert_eq!(
            engine.start_calls(),
            1,
            "the retry must not dispatch before the backoff elapses"
        );

        tokio::time::advance(tokio::time::Duration::from_secs(29)).await;
        for _ in 0..20 {
            tokio::task::yield_now().await;
        }
        assert_eq!(
            engine.start_calls(),
            1,
            "backoff is 30s; 29s must not trigger the retry"
        );

        tokio::time::advance(tokio::time::Duration::from_secs(2)).await;
        settle_until(|| engine.start_calls() == 2).await;
        tokio::time::resume();
        let (second_id, second_url) = started_rx.try_recv().unwrap();
        assert_ne!(
            second_id, first_id,
            "the retry must dispatch under a fresh download id"
        );
        assert_eq!(second_url, url, "the retry must carry the same download");
        assert!(active_contains(&svc, second_id).await);
        assert!(!active_contains(&svc, first_id).await);
        assert_eq!(active_total(&svc).await, 1);
        assert_eq!(
            queued_len(&svc).await,
            0,
            "the re-queued item must not linger in the queue"
        );

        let (status, _, retry_count) = row_state(&pool, queue_id).await;
        assert_eq!(status, "downloading");
        assert_eq!(retry_count, 1);
    }

    // ── #427: retry budget flows through dispatch and survives recovery ────

    #[tokio::test]
    async fn retry_budget_exhausts_after_max_retries() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 2, 0)).await;

        let item = make_item(DownloadProtocol::Torrent, 2);
        let queue_id = item.id;
        svc.enqueue(item).await.unwrap();

        // Initial dispatch plus two allowed retries; the third failure exhausts.
        for _ in 0..3 {
            let (dl_id, _) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
                .await
                .unwrap()
                .unwrap();
            svc.on_download_failed(dl_id, "connection timeout".to_string())
                .await;
        }

        let (status, reason, _) = row_state(&pool, queue_id).await;
        assert_eq!(status, "failed");
        assert!(
            reason
                .unwrap_or_default()
                .contains("retry budget exhausted after 2 attempts"),
            "exhaustion must report the true retry count"
        );
        assert_eq!(
            engine.start_calls(),
            3,
            "budget of 2 retries = 3 dispatches total"
        );
        assert_eq!(active_total(&svc).await, 0);
        assert_eq!(queued_len(&svc).await, 0);
    }

    #[tokio::test]
    async fn recovered_item_carries_prior_retry_count() {
        let pool = test_pool().await;
        let queue_id = uuid::Uuid::now_v7();
        let want_id = uuid::Uuid::now_v7().as_bytes().to_vec();
        let release_id = uuid::Uuid::now_v7().as_bytes().to_vec();
        repo::insert_queue_item(
            &pool,
            queue_id,
            &want_id,
            &release_id,
            "magnet:?xt=urn:btih:recovered",
            "torrent",
            2,
            None,
            None,
        )
        .await
        .unwrap();
        // Simulate a pre-restart process that already consumed the full budget.
        repo::increment_retry_count(&pool, queue_id).await.unwrap();
        repo::increment_retry_count(&pool, queue_id).await.unwrap();

        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 2, 0)).await;
        svc.try_dispatch_next().await;

        let (dl_id, _) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();
        svc.on_download_failed(dl_id, "connection timeout".to_string())
            .await;

        let (status, reason, _) = row_state(&pool, queue_id).await;
        assert_eq!(
            status, "failed",
            "a recovered exhausted budget must fail, not retry"
        );
        assert!(
            reason
                .unwrap_or_default()
                .contains("retry budget exhausted after 2 attempts")
        );
        assert_eq!(
            engine.start_calls(),
            1,
            "the persisted retry count must gate the retry decision"
        );
    }

    // ── #426: dropped events no longer leak slots ───────────────────────────

    #[tokio::test]
    async fn reconcile_releases_slot_for_terminal_state() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        // Retry budget 0: the synthesized failure exhausts immediately.
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 0, 0)).await;

        let item = make_item(DownloadProtocol::Torrent, 2);
        let queue_id = item.id;
        svc.enqueue(item).await.unwrap();
        let (dl_id, _) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(active_total(&svc).await, 1);

        engine.set_progress_state(DownloadState::Completed);
        svc.reconcile_active().await;

        assert_eq!(
            active_total(&svc).await,
            0,
            "the leaked slot must be reclaimed"
        );
        assert!(!active_contains(&svc, dl_id).await);
        let (status, reason, _) = row_state(&pool, queue_id).await;
        assert_eq!(status, "failed");
        assert!(
            reason.unwrap_or_default().contains("reconciliation"),
            "the failure reason must name the reconciliation origin"
        );
    }

    #[tokio::test]
    async fn reconcile_requeues_terminal_download_within_budget() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 3, 0)).await;

        let item = make_item(DownloadProtocol::Torrent, 2);
        let queue_id = item.id;
        svc.enqueue(item).await.unwrap();
        let (first_id, _) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();

        engine.set_progress_state(DownloadState::Failed);
        svc.reconcile_active().await;

        let (second_id, _) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_ne!(second_id, first_id);
        assert!(!active_contains(&svc, first_id).await);
        assert!(active_contains(&svc, second_id).await);
        assert_eq!(
            active_total(&svc).await,
            1,
            "release + re-acquire must not leak or double-count"
        );
        let (_, _, retry_count) = row_state(&pool, queue_id).await;
        assert_eq!(retry_count, 1);
    }

    #[tokio::test]
    async fn reconcile_skips_in_flight_downloads() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 3, 0)).await;

        let item = make_item(DownloadProtocol::Torrent, 2);
        let queue_id = item.id;
        svc.enqueue(item).await.unwrap();
        let (dl_id, _) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();

        // Engine reports Downloading (the mock default).
        svc.reconcile_active().await;

        assert!(
            active_contains(&svc, dl_id).await,
            "an in-flight download must be untouched"
        );
        assert_eq!(active_total(&svc).await, 1);
        let (status, _, _) = row_state(&pool, queue_id).await;
        assert_eq!(status, "downloading");
    }

    #[tokio::test]
    async fn reconcile_keeps_entry_when_engine_state_unavailable() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 3, 0)).await;

        svc.enqueue(make_item(DownloadProtocol::Torrent, 2))
            .await
            .unwrap();
        let (dl_id, _) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();

        engine.set_progress_unavailable();
        svc.reconcile_active().await;

        assert!(
            active_contains(&svc, dl_id).await,
            "an unqueryable download must not be reaped (could be a dispatch in flight)"
        );
        assert_eq!(active_total(&svc).await, 1);
    }

    #[tokio::test]
    async fn lagged_event_bus_reconciles_leaked_slot() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 0, 0)).await;

        let item = make_item(DownloadProtocol::Torrent, 2);
        let queue_id = item.id;
        svc.enqueue(item).await.unwrap();
        tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();
        engine.set_progress_state(DownloadState::Completed);

        // Overflow a capacity-1 bus before the listener starts so its first
        // recv observes Lagged (the dropped events include the completion).
        let (event_tx, event_rx) = themelion::create_event_bus(1);
        for _ in 0..3 {
            event_tx
                .send(HarmoniaEvent::LibraryScanCompleted {
                    items_scanned: 0,
                    items_added: 0,
                    items_removed: 0,
                })
                .unwrap();
        }
        let shutdown = CancellationToken::new();
        let listener = svc.start(event_rx, shutdown.clone());

        let (_, reason, _) = wait_for_status(&pool, queue_id, "failed").await;
        assert!(reason.unwrap_or_default().contains("reconciliation"));
        assert_eq!(
            active_total(&svc).await,
            0,
            "Lagged must trigger slot reclamation"
        );
        shutdown.cancel();
        listener.await.unwrap();
    }

    #[tokio::test]
    async fn periodic_reconciliation_catches_missed_completion() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 0, 0)).await;

        // Healthy bus: no Lagged signal will ever fire; only the periodic
        // pass can observe the missed completion.
        let (_event_tx, event_rx) = themelion::create_event_bus(64);
        let shutdown = CancellationToken::new();
        let listener = svc.start(event_rx, shutdown.clone());

        let item = make_item(DownloadProtocol::Torrent, 2);
        let queue_id = item.id;
        svc.enqueue(item).await.unwrap();
        tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();
        let (status, _, _) = row_state(&pool, queue_id).await;
        assert_eq!(
            status, "downloading",
            "precondition: item is active before the tick"
        );
        engine.set_progress_state(DownloadState::Completed);

        // WHY: pause only around the interval jump; sqlx acquires must never
        // await under a paused clock (auto-advance fires their acquire timeout).
        tokio::time::pause();
        tokio::time::advance(RECONCILE_INTERVAL + tokio::time::Duration::from_secs(1)).await;
        tokio::time::resume();

        let (_, reason, _) = wait_for_status(&pool, queue_id, "failed").await;
        assert!(
            reason.unwrap_or_default().contains("reconciliation"),
            "the periodic pass must reconcile without a Lagged signal"
        );
        assert_eq!(active_total(&svc).await, 0);
        shutdown.cancel();
        listener.await.unwrap();
    }

    // ── #412: start returns an awaitable handle for graceful shutdown ──────

    #[tokio::test]
    async fn start_handle_resolves_on_shutdown_after_draining_event() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 3, 0)).await;

        let (event_tx, event_rx) = themelion::create_event_bus(64);
        let shutdown = CancellationToken::new();
        let listener = svc.start(event_rx, shutdown.clone());

        svc.enqueue(make_item(DownloadProtocol::Torrent, 2))
            .await
            .unwrap();
        let (dl_id, _) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();

        event_tx
            .send(HarmoniaEvent::DownloadCompleted {
                download_id: dl_id,
                path: std::path::PathBuf::from("/tmp/none"),
            })
            .unwrap();

        // The listener must process the completion (slot released) before the
        // handle is awaited — proving events are not silently dropped.
        settle_until(|| {
            svc.inner
                .try_lock()
                .map(|inner| inner.allocator.active_total() == 0)
                .unwrap_or(false)
        })
        .await;

        shutdown.cancel();
        tokio::time::timeout(RECV_TIMEOUT, listener)
            .await
            .expect("the listener handle must resolve on shutdown, not hang")
            .expect("the listener task must not panic");
    }

    // ── #469: cancel/reprioritize reach the live queue and engine ──────────

    #[tokio::test]
    async fn cancel_by_queue_id_stops_live_engine_download() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 3, 0)).await;

        let item = make_item(DownloadProtocol::Torrent, 2);
        let queue_id = item.id;
        svc.enqueue(item).await.unwrap();
        let (dl_id, _) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();

        svc.cancel_by_queue_id(queue_id).await.unwrap();

        assert_eq!(
            engine.cancelled_ids(),
            vec![dl_id],
            "the live engine download must be cancelled, not just the DB row"
        );
        assert_eq!(active_total(&svc).await, 0, "the slot must be released");
        assert!(!active_contains(&svc, dl_id).await);
        assert!(
            repo::get_queue_item(&pool, queue_id)
                .await
                .unwrap()
                .is_none(),
            "the cancelled row must be gone so recovery cannot re-dispatch it"
        );
    }

    #[tokio::test]
    async fn cancel_by_queue_id_frees_slot_for_next_queued_item() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(1, 3, 0)).await;

        let first = make_item(DownloadProtocol::Torrent, 2);
        let first_queue_id = first.id;
        let second = make_item(DownloadProtocol::Torrent, 2);
        let second_url = second.download_url.clone();
        svc.enqueue(first).await.unwrap();
        svc.enqueue(second).await.unwrap();
        tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(queued_len(&svc).await, 1, "precondition: second is queued");

        svc.cancel_by_queue_id(first_queue_id).await.unwrap();

        let (_, url) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(url, second_url, "the freed slot must dispatch queued work");
    }

    #[tokio::test]
    async fn cancel_by_queue_id_removes_queued_item_without_engine_call() {
        let pool = test_pool().await;
        let (engine, _started_rx) = MockEngine::create();
        // max_concurrent 0: the item can never dispatch, so it stays queued.
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(0, 3, 0)).await;

        let item = make_item(DownloadProtocol::Torrent, 2);
        let queue_id = item.id;
        svc.enqueue(item).await.unwrap();
        assert_eq!(queued_len(&svc).await, 1);

        svc.cancel_by_queue_id(queue_id).await.unwrap();

        assert_eq!(queued_len(&svc).await, 0);
        assert!(
            engine.cancelled_ids().is_empty(),
            "a never-dispatched item has nothing to cancel on the engine"
        );
        assert!(
            repo::get_queue_item(&pool, queue_id)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn cancel_by_queue_id_deletes_db_only_row() {
        let pool = test_pool().await;
        let (engine, _started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 3, 0)).await;

        // A row persisted by the HTTP API without going through this service.
        let queue_id = uuid::Uuid::now_v7();
        repo::insert_queue_item(
            &pool,
            queue_id,
            uuid::Uuid::now_v7().as_bytes().as_ref(),
            uuid::Uuid::now_v7().as_bytes().as_ref(),
            "magnet:?xt=urn:btih:dbonly",
            "torrent",
            2,
            None,
            None,
        )
        .await
        .unwrap();

        svc.cancel_by_queue_id(queue_id).await.unwrap();

        assert!(
            repo::get_queue_item(&pool, queue_id)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn cancel_by_queue_id_unknown_id_errors() {
        let pool = test_pool().await;
        let (engine, _started_rx) = MockEngine::create();
        let svc = make_service(pool, Arc::clone(&engine), test_config(2, 3, 0)).await;

        let result = svc.cancel_by_queue_id(uuid::Uuid::now_v7()).await;
        assert!(matches!(result, Err(SyntaxisError::ItemNotFound { .. })));
    }

    #[tokio::test]
    async fn cancel_by_queue_id_engine_failure_surfaces_and_marks_row() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 3, 0)).await;

        let item = make_item(DownloadProtocol::Torrent, 2);
        let queue_id = item.id;
        svc.enqueue(item).await.unwrap();
        tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();

        engine.fail_cancels();
        let result = svc.cancel_by_queue_id(queue_id).await;

        assert!(
            matches!(result, Err(SyntaxisError::DispatchFailed { .. })),
            "an engine cancel failure must not report success"
        );
        let (status, reason, _) = row_state(&pool, queue_id).await;
        assert_eq!(status, "failed", "the row must be terminal, not deleted");
        assert!(reason.unwrap_or_default().contains("engine cancel failed"));
        assert_eq!(active_total(&svc).await, 0, "the claimed slot stays freed");
    }

    #[tokio::test]
    async fn reprioritize_by_queue_id_moves_queued_item_tier() {
        let pool = test_pool().await;
        let (engine, _started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(0, 3, 0)).await;

        let low = make_item(DownloadProtocol::Torrent, 1);
        let low_id = low.id;
        let high = make_item(DownloadProtocol::Torrent, 3);
        svc.enqueue(low).await.unwrap();
        svc.enqueue(high).await.unwrap();

        svc.reprioritize_by_queue_id(low_id, 3).await.unwrap();

        let snapshot = svc.get_queue_state().await.unwrap();
        let moved = snapshot
            .queued_items
            .iter()
            .find(|i| i.id == low_id)
            .expect("the item stays queued");
        assert_eq!(moved.priority, 3, "the live tier must change");
        let row = repo::get_queue_item(&pool, low_id).await.unwrap().unwrap();
        assert_eq!(row.priority, 3, "the persisted priority must match");
    }

    #[tokio::test]
    async fn reprioritize_by_queue_id_to_interactive_dispatches_immediately() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(1, 3, 0)).await;

        // Place a queued item directly (slot free, nothing dispatched yet).
        let item = make_item(DownloadProtocol::Torrent, 2);
        let queue_id = item.id;
        repo::insert_queue_item(
            &pool,
            queue_id,
            item.want_id.as_uuid().as_bytes(),
            item.release_id.as_uuid().as_bytes(),
            &item.download_url,
            "torrent",
            2,
            None,
            None,
        )
        .await
        .unwrap();
        svc.inner.lock().await.queue.insert(item);

        svc.reprioritize_by_queue_id(queue_id, 4).await.unwrap();

        let (dl_id, _) = tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(
            active_contains(&svc, dl_id).await,
            "interactive upgrade must dispatch through the live engine"
        );
        assert_eq!(queued_len(&svc).await, 0);
        let row = repo::get_queue_item(&pool, queue_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.priority, 4);
        assert_eq!(row.status, "downloading");
    }

    #[tokio::test]
    async fn reprioritize_to_interactive_without_slot_requeues_at_tier3() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(1, 3, 0)).await;

        // Occupy the single slot.
        svc.enqueue(make_item(DownloadProtocol::Torrent, 2))
            .await
            .unwrap();
        tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();

        let queued = make_item(DownloadProtocol::Torrent, 2);
        let queued_id = queued.id;
        svc.enqueue(queued).await.unwrap();
        assert_eq!(queued_len(&svc).await, 1);

        svc.reprioritize_by_queue_id(queued_id, 4).await.unwrap();

        // WHY: pre-fix, reprioritize-to-4 dropped the item from memory
        // entirely (removed but never dispatched or re-inserted).
        let snapshot = svc.get_queue_state().await.unwrap();
        let kept = snapshot
            .queued_items
            .iter()
            .find(|i| i.id == queued_id)
            .expect("without a slot the item must stay queued, not vanish");
        assert_eq!(kept.priority, 3, "no-slot interactive demotes to tier 3");
        assert_eq!(engine.start_calls(), 1, "no second dispatch may happen");
    }

    #[tokio::test]
    async fn reprioritize_by_queue_id_active_item_is_noop() {
        let pool = test_pool().await;
        let (engine, mut started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 3, 0)).await;

        let item = make_item(DownloadProtocol::Torrent, 2);
        let queue_id = item.id;
        svc.enqueue(item).await.unwrap();
        tokio::time::timeout(RECV_TIMEOUT, started_rx.recv())
            .await
            .unwrap()
            .unwrap();

        svc.reprioritize_by_queue_id(queue_id, 1).await.unwrap();

        let row = repo::get_queue_item(&pool, queue_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            row.status, "downloading",
            "an active download must be untouched"
        );
        assert_eq!(row.priority, 2, "priority of a running transfer stays put");
    }

    #[tokio::test]
    async fn reprioritize_by_queue_id_db_only_row_updates_priority() {
        let pool = test_pool().await;
        let (engine, _started_rx) = MockEngine::create();
        let svc = make_service(pool.clone(), Arc::clone(&engine), test_config(2, 3, 0)).await;

        let queue_id = uuid::Uuid::now_v7();
        repo::insert_queue_item(
            &pool,
            queue_id,
            uuid::Uuid::now_v7().as_bytes().as_ref(),
            uuid::Uuid::now_v7().as_bytes().as_ref(),
            "magnet:?xt=urn:btih:dbonly",
            "torrent",
            1,
            None,
            None,
        )
        .await
        .unwrap();

        svc.reprioritize_by_queue_id(queue_id, 3).await.unwrap();

        let row = repo::get_queue_item(&pool, queue_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.priority, 3);
    }

    #[tokio::test]
    async fn reprioritize_by_queue_id_unknown_id_errors() {
        let pool = test_pool().await;
        let (engine, _started_rx) = MockEngine::create();
        let svc = make_service(pool, Arc::clone(&engine), test_config(2, 3, 0)).await;

        let result = svc.reprioritize_by_queue_id(uuid::Uuid::now_v7(), 3).await;
        assert!(matches!(result, Err(SyntaxisError::ItemNotFound { .. })));
    }
}
