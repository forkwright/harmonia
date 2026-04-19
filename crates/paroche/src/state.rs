use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use apotheke::DbPools;
use axum::extract::FromRef;
use exousia::ExousiaServiceImpl;
use horismos::Config;
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

/// Dyn-compatible interface for services not yet used in route handlers.
pub trait DynCurationService: Send + Sync {}

pub trait DynMetadataResolver: Send + Sync {}

/// Boxed future type for dyn-safe acquisition service methods.
pub type ServiceFut<T> = Pin<Box<dyn Future<Output = Result<T, ServiceError>> + Send>>;

/// Error type returned by acquisition service trait methods.
#[derive(Debug)]
pub enum ServiceError {
    /// The backing service is not wired up.
    NotAvailable,
    /// The requested resource was not found by the service.
    NotFound,
    /// An internal service error.
    Internal(String),
}

/// Search across indexers via zetesis.
pub trait DynSearchService: Send + Sync {
    fn search(&self, query: serde_json::Value) -> ServiceFut<serde_json::Value>;
    fn test_indexer(&self, indexer_id: i64) -> ServiceFut<serde_json::Value>;
    fn refresh_caps(&self, indexer_id: i64) -> ServiceFut<serde_json::Value>;
}

pub trait DynDownloadEngine: Send + Sync {}
pub trait DynQueueManager: Send + Sync {}
pub trait DynRequestService: Send + Sync {}
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
pub(crate) struct ImportQueueFn(pub Arc<dyn Fn() -> ImportQueueFut + Send + Sync>);

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
impl DynCurationService for NullCuration {}

struct NullMetadata;
impl DynMetadataResolver for NullMetadata {}

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
}

struct NullDownloadEngine;
impl DynDownloadEngine for NullDownloadEngine {}

struct NullQueueManager;
impl DynQueueManager for NullQueueManager {}

struct NullRequestService;
impl DynRequestService for NullRequestService {}

struct NullExternalIntegration;
impl DynExternalIntegration for NullExternalIntegration {}

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
    pub config: Arc<Config>,
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
        config: Arc<Config>,
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
        }
    }
}

impl FromRef<AppState> for Arc<ExousiaServiceImpl> {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
    }
}
