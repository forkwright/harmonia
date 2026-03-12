use std::{future::Future, pin::Pin, sync::Arc};

use axum::extract::FromRef;
use exousia::ExousiaServiceImpl;
use harmonia_common::EventSender;
use harmonia_db::DbPools;
use horismos::Config;

type ImportQueueFut = Pin<
    Box<
        dyn Future<Output = Result<Vec<taxis::import::PendingImport>, taxis::error::TaxisError>>
            + Send,
    >,
>;

/// Dyn-compatible interface for the parts of ImportService used in route handlers.
pub trait DynImportService: Send + Sync {
    fn get_import_queue_boxed(&self) -> ImportQueueFut;
}

/// Dyn-compatible interface for services not yet used in route handlers.
pub trait DynCurationService: Send + Sync {}

pub trait DynMetadataResolver: Send + Sync {}

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
    Fut: Future<Output = Result<Vec<taxis::import::PendingImport>, taxis::error::TaxisError>>
        + Send
        + 'static,
{
    Arc::new(ImportQueueFn(Arc::new(move || Box::pin(f()))))
}

struct NullCuration;
impl DynCurationService for NullCuration {}

struct NullMetadata;
impl DynMetadataResolver for NullMetadata {}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DbPools>,
    pub config: Arc<Config>,
    pub event_tx: EventSender,
    pub auth: Arc<ExousiaServiceImpl>,
    pub import: Arc<dyn DynImportService>,
    pub metadata: Arc<dyn DynMetadataResolver>,
    pub curation: Arc<dyn DynCurationService>,
}

impl AppState {
    pub async fn get_import_queue(
        &self,
    ) -> Result<Vec<taxis::import::PendingImport>, taxis::error::TaxisError> {
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
        }
    }
}

impl FromRef<AppState> for Arc<ExousiaServiceImpl> {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
    }
}
