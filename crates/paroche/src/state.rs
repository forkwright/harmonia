use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use apotheke::DbPools;
use axum::extract::FromRef;
use exousia::ExousiaServiceImpl;
use horismos::ConfigHandle;
use themelion::EventSender;

type ImportQueueFut = Pin<
    Box<
        dyn Future<
                Output = Result<Vec<kathodos::import::PendingImport>, kathodos::error::TaxisError>,
            > + Send,
    >,
>;

/// Dyn-compatible interface for the parts of ImportService used in route handlers.
pub trait DynImportService: Send + Sync {
    fn get_import_queue_boxed(&self) -> ImportQueueFut;
}

/// Library curation via kritike: quality assessment, upgrade eligibility,
/// and library health reporting.
pub trait DynCurationService: Send + Sync {
    /// Assess the quality score of an item's metadata against its profile.
    fn assess_quality(
        &self,
        media_type: themelion::MediaType,
        item_metadata: kritike::QualityMetadata,
    ) -> ServiceFut<kritike::QualityAssessment>;

    /// Decide whether an existing have should be upgraded by a candidate.
    fn check_upgrade_eligibility(
        &self,
        have_id: themelion::HaveId,
        candidate_score: i32,
    ) -> ServiceFut<kritike::UpgradeDecision>;

    /// Generate the library-wide health report.
    fn health_report(&self) -> ServiceFut<kritike::HealthReport>;
}

/// Metadata identification and enrichment via epignosis' provider-backed
/// resolver (mirrors `epignosis::MetadataResolver`).
pub trait DynMetadataResolver: Send + Sync {
    /// Resolve a media item's canonical identity via the provider fan-out.
    fn resolve_identity(
        &self,
        item: epignosis::UnidentifiedItem,
    ) -> ServiceFut<epignosis::MediaIdentity>;

    /// Enrich a resolved identity with provider metadata.
    fn enrich(&self, identity: epignosis::MediaIdentity)
    -> ServiceFut<epignosis::EnrichedMetadata>;

    /// Compute an audio fingerprint for a file on the server.
    fn fingerprint_audio(
        &self,
        file_path: std::path::PathBuf,
    ) -> ServiceFut<epignosis::FingerprintResult>;
}

/// Boxed future type for dyn-safe acquisition service methods.
pub type ServiceFut<T> = Pin<Box<dyn Future<Output = Result<T, ServiceError>> + Send>>;

/// Boxed future type for dyn-safe request service methods.
pub type RequestServiceFut<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, RequestServiceError>> + Send + 'a>>;

/// Error type returned by acquisition service trait methods.
#[derive(Debug)]
#[non_exhaustive]
pub enum ServiceError {
    /// The backing service is not wired up.
    NotAvailable,
    /// The requested resource was not found by the service.
    NotFound,
    /// The caller's input was rejected by validation (HTTP 400, never 500).
    InvalidInput(String),
    /// An internal service error.
    Internal(String),
}

/// Error type returned by request service trait methods.
#[derive(Debug)]
#[non_exhaustive]
pub enum RequestServiceError {
    /// The backing request service is not wired up.
    NotAvailable,
    /// The Aitesis domain service rejected the operation.
    Domain(aitesis::AitesisError),
}

impl From<aitesis::AitesisError> for RequestServiceError {
    fn from(error: aitesis::AitesisError) -> Self {
        Self::Domain(error)
    }
}

/// Server-side-only resolution of a cached search result's REAL,
/// credentialed download URL — the enqueue-by-reference join
/// `enqueue_download` uses so a Torznab/Newznab indexer credential never
/// crosses the HTTP boundary to a client (#608). Local mirror of
/// `eksetasis::ResolvedRelease`, decoupled the same way `EnqueueItem` and
/// `DynQueueManager` stay decoupled from syntaxis types.
pub struct ResolvedRelease {
    pub download_url: String,
    /// Wire-form protocol (`"torrent"` / `"nzb"`) — the SAME form
    /// `EnqueueItem::protocol` expects.
    pub protocol: String,
    pub info_hash: Option<String>,
    pub indexer_id: i64,
    /// The catalogued result's title — a caller persists a `releases` row
    /// keyed by (want_id, release_id) using these fields BEFORE enqueueing,
    /// so the row exists once the transfer completes (#651).
    pub title: String,
    pub size_bytes: Option<u64>,
}

impl std::fmt::Debug for ResolvedRelease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedRelease")
            .field(
                "download_url",
                &crate::redact::redact_download_url(&self.download_url),
            )
            .field("protocol", &self.protocol)
            .field("info_hash", &self.info_hash)
            .field("indexer_id", &self.indexer_id)
            .field("title", &self.title)
            .field("size_bytes", &self.size_bytes)
            .finish()
    }
}

/// Search across indexers via eksetasis.
pub trait DynSearchService: Send + Sync {
    fn search(&self, query: serde_json::Value) -> ServiceFut<serde_json::Value>;
    fn test_indexer(&self, indexer_id: i64) -> ServiceFut<serde_json::Value>;
    fn refresh_caps(&self, indexer_id: i64) -> ServiceFut<serde_json::Value>;
    /// Retrieves a prior search's cached, redaction-pending results by
    /// `query_id` — backs `GET /api/v1/search/{query_id}/results`. The
    /// caller MUST redact `download_url` before this reaches an HTTP
    /// response; `NotFound` on an unknown or expired query id.
    fn cached_results(&self, query_id: uuid::Uuid) -> ServiceFut<serde_json::Value>;
    /// Resolves a cached release's UNREDACTED download URL for
    /// enqueue-by-reference. `NotFound` on an unknown or expired release id.
    fn resolve_release(&self, release_id: uuid::Uuid) -> ServiceFut<ResolvedRelease>;
}

/// Feed subscription lifecycle via komide's `FeedSchedulerService`
/// (fetch + parse + episode/article seeding + `FeedSetChanged` emission) —
/// the rich path the raw-row route handlers used to bypass (#577).
pub trait DynFeedService: Send + Sync {
    /// Subscribe to a podcast feed. Fetches and parses the feed, seeds its
    /// episodes, and returns the subscription's feed id. `auto_download`
    /// maps to komide's episode-download COUNT (`false` → 0, otherwise the
    /// configured `auto_download_latest_n`).
    fn subscribe_podcast(
        &self,
        url: String,
        title: Option<String>,
        auto_download: Option<bool>,
    ) -> ServiceFut<themelion::FeedId>;

    /// Subscribe to a news feed. Fetches and parses the feed, seeds its
    /// articles, and returns the feed id.
    fn subscribe_news(
        &self,
        url: String,
        title: Option<String>,
        category: Option<String>,
    ) -> ServiceFut<themelion::FeedId>;

    /// Remove a podcast or news subscription.
    fn unsubscribe(&self, feed_id: themelion::FeedId) -> ServiceFut<()>;

    /// Start a background download of one episode's audio enclosure.
    /// Resolves once the download is validated and started, not when it
    /// completes.
    fn download_episode(&self, episode_id: themelion::EpisodeId) -> ServiceFut<()>;
}

pub trait DynDownloadEngine: Send + Sync {}

// WHY: pure data — enqueue parameters for the live download queue, kept as
// plain uuid/string fields so paroche stays decoupled from syntaxis types.
/// A new item for the live download queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnqueueItem {
    /// `download_queue` row id (UUIDv7), chosen by the caller so the HTTP
    /// response can reference the persisted row.
    pub queue_id: uuid::Uuid,
    pub want_id: uuid::Uuid,
    pub release_id: uuid::Uuid,
    pub download_url: String,
    /// Protocol in its wire form (`"torrent"` / `"nzb"` / `"usenet"`); an
    /// unknown value is rejected with `ServiceError::InvalidInput`.
    pub protocol: String,
    /// 1-4; priority 4 dispatches immediately when a slot is free.
    pub priority: u8,
    pub info_hash: Option<String>,
}

/// Download queue control via the running syntaxis service.
///
/// Items are keyed by their `download_queue` row id — the identifier the
/// HTTP API exposes.
pub trait DynQueueManager: Send + Sync {
    /// Persists the item to `download_queue` and schedules it in the live
    /// queue, so it dispatches without waiting for a process restart.
    fn enqueue(&self, item: EnqueueItem) -> ServiceFut<()>;

    /// Cancels the queue item, stopping the live download when one is active.
    fn cancel(&self, queue_id: uuid::Uuid) -> ServiceFut<()>;

    /// Changes the dispatch priority (1-4) of the queue item in the live
    /// queue; priority 4 dispatches immediately when a slot is free.
    fn reprioritize(&self, queue_id: uuid::Uuid, priority: u8) -> ServiceFut<()>;
}

/// Media-request lifecycle via Aitesis.
pub trait DynRequestService: Send + Sync {
    fn submit_request(
        &self,
        user_id: themelion::UserId,
        input: aitesis::CreateRequestInput,
    ) -> RequestServiceFut<'_, aitesis::MediaRequest>;

    fn approve(
        &self,
        request_id: themelion::RequestId,
        admin_id: themelion::UserId,
    ) -> RequestServiceFut<'_, aitesis::MediaRequest>;

    fn deny(
        &self,
        request_id: themelion::RequestId,
        admin_id: themelion::UserId,
        reason: Option<String>,
    ) -> RequestServiceFut<'_, aitesis::MediaRequest>;

    fn get_request(
        &self,
        request_id: themelion::RequestId,
        caller_id: themelion::UserId,
    ) -> RequestServiceFut<'_, aitesis::MediaRequest>;

    fn list_requests(
        &self,
        caller_id: themelion::UserId,
        user_id: Option<themelion::UserId>,
        status: Option<aitesis::RequestStatus>,
        limit: u32,
        offset: u32,
    ) -> RequestServiceFut<'_, Vec<aitesis::MediaRequest>>;

    fn count_requests(
        &self,
        caller_id: themelion::UserId,
        user_id: Option<themelion::UserId>,
        status: Option<aitesis::RequestStatus>,
    ) -> RequestServiceFut<'_, u64>;

    fn cancel_request(
        &self,
        request_id: themelion::RequestId,
        user_id: themelion::UserId,
    ) -> RequestServiceFut<'_, ()>;
}
pub trait DynExternalIntegration: Send + Sync {}

/// Subtitle acquisition via prostheke.
pub trait DynSubtitleService: Send + Sync {
    fn search_for_media(&self, media_id: Vec<u8>) -> ServiceFut<()>;
}

/// Serializable renderer status for the REST API.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RendererInfo {
    pub name: String,
    pub session_id: String,
    pub connected_secs: u64,
    pub buffer_depth_ms: f64,
    pub latency_ms: f64,
    pub state: String,
    pub underrun_count: u64,
}

/// Connected renderer listing via the renderer QUIC server.
pub trait DynRendererRegistry: Send + Sync {
    fn list_renderers(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<RendererInfo>> + Send + '_>>;
}

/// Adapter around a closure for import queue retrieval.
pub struct ImportQueueFn(pub Arc<dyn Fn() -> ImportQueueFut + Send + Sync>);

impl DynImportService for ImportQueueFn {
    fn get_import_queue_boxed(&self) -> ImportQueueFut {
        (self.0)()
    }
}

/// Helper to construct an `Arc<dyn DynImportService>` from any function.
pub fn make_import_service<F, Fut>(f: F) -> Arc<dyn DynImportService>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Vec<kathodos::import::PendingImport>, kathodos::error::TaxisError>>
        + Send
        + 'static,
{
    Arc::new(ImportQueueFn(Arc::new(move || Box::pin(f()))))
}

struct NullCuration;
impl DynCurationService for NullCuration {
    fn assess_quality(
        &self,
        _media_type: themelion::MediaType,
        _item_metadata: kritike::QualityMetadata,
    ) -> ServiceFut<kritike::QualityAssessment> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }

    fn check_upgrade_eligibility(
        &self,
        _have_id: themelion::HaveId,
        _candidate_score: i32,
    ) -> ServiceFut<kritike::UpgradeDecision> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }

    fn health_report(&self) -> ServiceFut<kritike::HealthReport> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }
}

struct NullMetadata;
impl DynMetadataResolver for NullMetadata {
    fn resolve_identity(
        &self,
        _item: epignosis::UnidentifiedItem,
    ) -> ServiceFut<epignosis::MediaIdentity> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }

    fn enrich(
        &self,
        _identity: epignosis::MediaIdentity,
    ) -> ServiceFut<epignosis::EnrichedMetadata> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }

    fn fingerprint_audio(
        &self,
        _file_path: std::path::PathBuf,
    ) -> ServiceFut<epignosis::FingerprintResult> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }
}

struct NullSearch;
impl DynSearchService for NullSearch {
    fn search(&self, _query: serde_json::Value) -> ServiceFut<serde_json::Value> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }
    fn test_indexer(&self, _indexer_id: i64) -> ServiceFut<serde_json::Value> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }
    fn refresh_caps(&self, _indexer_id: i64) -> ServiceFut<serde_json::Value> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }
    fn cached_results(&self, _query_id: uuid::Uuid) -> ServiceFut<serde_json::Value> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }
    fn resolve_release(&self, _release_id: uuid::Uuid) -> ServiceFut<ResolvedRelease> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }
}

struct NullDownloadEngine;
impl DynDownloadEngine for NullDownloadEngine {}

struct NullQueueManager;
impl DynQueueManager for NullQueueManager {
    fn enqueue(&self, _item: EnqueueItem) -> ServiceFut<()> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }

    fn cancel(&self, _queue_id: uuid::Uuid) -> ServiceFut<()> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }

    fn reprioritize(&self, _queue_id: uuid::Uuid, _priority: u8) -> ServiceFut<()> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }
}

struct NullRequestService;
impl DynRequestService for NullRequestService {
    fn submit_request(
        &self,
        _user_id: themelion::UserId,
        _input: aitesis::CreateRequestInput,
    ) -> RequestServiceFut<'_, aitesis::MediaRequest> {
        Box::pin(async { Err(RequestServiceError::NotAvailable) })
    }

    fn approve(
        &self,
        _request_id: themelion::RequestId,
        _admin_id: themelion::UserId,
    ) -> RequestServiceFut<'_, aitesis::MediaRequest> {
        Box::pin(async { Err(RequestServiceError::NotAvailable) })
    }

    fn deny(
        &self,
        _request_id: themelion::RequestId,
        _admin_id: themelion::UserId,
        _reason: Option<String>,
    ) -> RequestServiceFut<'_, aitesis::MediaRequest> {
        Box::pin(async { Err(RequestServiceError::NotAvailable) })
    }

    fn get_request(
        &self,
        _request_id: themelion::RequestId,
        _caller_id: themelion::UserId,
    ) -> RequestServiceFut<'_, aitesis::MediaRequest> {
        Box::pin(async { Err(RequestServiceError::NotAvailable) })
    }

    fn list_requests(
        &self,
        _caller_id: themelion::UserId,
        _user_id: Option<themelion::UserId>,
        _status: Option<aitesis::RequestStatus>,
        _limit: u32,
        _offset: u32,
    ) -> RequestServiceFut<'_, Vec<aitesis::MediaRequest>> {
        Box::pin(async { Err(RequestServiceError::NotAvailable) })
    }

    fn count_requests(
        &self,
        _caller_id: themelion::UserId,
        _user_id: Option<themelion::UserId>,
        _status: Option<aitesis::RequestStatus>,
    ) -> RequestServiceFut<'_, u64> {
        Box::pin(async { Err(RequestServiceError::NotAvailable) })
    }

    fn cancel_request(
        &self,
        _request_id: themelion::RequestId,
        _user_id: themelion::UserId,
    ) -> RequestServiceFut<'_, ()> {
        Box::pin(async { Err(RequestServiceError::NotAvailable) })
    }
}

struct NullExternalIntegration;
impl DynExternalIntegration for NullExternalIntegration {}

struct NullFeedService;
impl DynFeedService for NullFeedService {
    fn subscribe_podcast(
        &self,
        _url: String,
        _title: Option<String>,
        _auto_download: Option<bool>,
    ) -> ServiceFut<themelion::FeedId> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }

    fn subscribe_news(
        &self,
        _url: String,
        _title: Option<String>,
        _category: Option<String>,
    ) -> ServiceFut<themelion::FeedId> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }

    fn unsubscribe(&self, _feed_id: themelion::FeedId) -> ServiceFut<()> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }

    fn download_episode(&self, _episode_id: themelion::EpisodeId) -> ServiceFut<()> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }
}

struct NullSubtitleService;
impl DynSubtitleService for NullSubtitleService {
    fn search_for_media(&self, _media_id: Vec<u8>) -> ServiceFut<()> {
        Box::pin(async { Err(ServiceError::NotAvailable) })
    }
}

struct NullRendererRegistry;
impl DynRendererRegistry for NullRendererRegistry {
    fn list_renderers(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<RendererInfo>> + Send + '_>> {
        Box::pin(async { Vec::new() })
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DbPools>,
    pub config: ConfigHandle,
    pub event_tx: EventSender,
    pub auth: Arc<ExousiaServiceImpl>,
    pub import: Arc<dyn DynImportService>,
    pub metadata: Arc<dyn DynMetadataResolver>,
    pub curation: Arc<dyn DynCurationService>,
    pub search: Arc<dyn DynSearchService>,
    pub download_engine: Arc<dyn DynDownloadEngine>,
    pub queue: Arc<dyn DynQueueManager>,
    pub requests: Arc<dyn DynRequestService>,
    pub external: Arc<dyn DynExternalIntegration>,
    pub subtitles: Arc<dyn DynSubtitleService>,
    pub renderers: Arc<dyn DynRendererRegistry>,
    pub feeds: Arc<dyn DynFeedService>,
}

impl AppState {
    pub async fn get_import_queue(
        &self,
    ) -> Result<Vec<kathodos::import::PendingImport>, kathodos::error::TaxisError> {
        self.import.get_import_queue_boxed().await
    }

    /// Build a new AppState with stub service impls for testing.
    pub fn with_stubs(
        db: Arc<DbPools>,
        config: ConfigHandle,
        event_tx: EventSender,
        auth: Arc<ExousiaServiceImpl>,
        import: Arc<dyn DynImportService>,
    ) -> Self {
        Self {
            db,
            config,
            event_tx,
            auth,
            import,
            metadata: Arc::new(NullMetadata),
            curation: Arc::new(NullCuration),
            search: Arc::new(NullSearch),
            download_engine: Arc::new(NullDownloadEngine),
            queue: Arc::new(NullQueueManager),
            requests: Arc::new(NullRequestService),
            external: Arc::new(NullExternalIntegration),
            subtitles: Arc::new(NullSubtitleService),
            renderers: Arc::new(NullRendererRegistry),
            feeds: Arc::new(NullFeedService),
        }
    }
}

impl FromRef<AppState> for Arc<ExousiaServiceImpl> {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
    }
}
