use std::pin::Pin;
use std::sync::Arc;

use snafu::ResultExt;
use tokio::signal::unix::SignalKind;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, info};

use apotheke::init_pools;
use epignosis::{EpignosisService, resolver::ProviderCredentials};
use ergasia::ErgasiaSession;
use exousia::ExousiaServiceImpl;
use horismos::ConfigManager;
use kathodos::ScannerManager;
use komide::{KomideService, scheduler::FeedScheduler};
use kritike::DefaultCurationService;
use paroche::state::{
    AppState, DynCurationService, DynDownloadEngine, DynExternalIntegration, DynMetadataResolver,
    DynQueueManager, DynRequestService, DynSearchService, DynSubtitleService,
};
use prostheke::ProsthekeService;
use prostheke::providers::Provider;
use syndesmos::{SyndesmosService, SyndesmosServiceBuilder};
use syntaxis::{CompletedDownload, SyntaxisService};
use themelion::create_event_bus;
use zetesis::ZetesisService;
use zetesis::cf_bypass::noop::NoProxy;

use crate::cli::ServeArgs;
use crate::error::{
    ConfigSnafu, DatabaseSnafu, DownloadEngineSnafu, DownloadQueueSnafu, FeedSchedulerSnafu,
    HostError, ScannerSnafu, ServerSnafu,
};
use crate::shutdown::shutdown_signal;
use crate::startup::{ensure_admin_user, init_tracing};

// ── Dyn-trait adapters ──────────────────────────────────────────────────────

struct NullCuration;
impl DynCurationService for NullCuration {}

struct NullMetadata;
impl DynMetadataResolver for NullMetadata {}

// WHY: Adapter structs hold Arc handles to keep acquisition subsystems alive
// for the lifetime of AppState. The INNER fields are read once route handlers
// are wired in prompt 102.
struct SearchAdapter(#[expect(dead_code)] Arc<ZetesisService>);
impl DynSearchService for SearchAdapter {
    fn search(
        &self,
        _query: serde_json::Value,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<serde_json::Value, paroche::state::ServiceError>,
                > + Send,
        >,
    > {
        Box::pin(async { Err(paroche::state::ServiceError::NotAvailable) })
    }
    fn test_indexer(
        &self,
        _indexer_id: i64,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<serde_json::Value, paroche::state::ServiceError>,
                > + Send,
        >,
    > {
        Box::pin(async { Err(paroche::state::ServiceError::NotAvailable) })
    }
    fn refresh_caps(
        &self,
        _indexer_id: i64,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<serde_json::Value, paroche::state::ServiceError>,
                > + Send,
        >,
    > {
        Box::pin(async { Err(paroche::state::ServiceError::NotAvailable) })
    }
}

struct EngineAdapter(#[expect(dead_code)] Arc<ErgasiaSession>);
impl DynDownloadEngine for EngineAdapter {}

struct QueueAdapter;
impl DynQueueManager for QueueAdapter {}

struct RequestAdapter;
impl DynRequestService for RequestAdapter {}

struct ExternalAdapter(#[expect(dead_code)] Arc<SyndesmosService>);
impl DynExternalIntegration for ExternalAdapter {}

struct SubtitleAdapter;
impl DynSubtitleService for SubtitleAdapter {
    fn search_for_media(
        &self,
        _media_id: Vec<u8>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(), paroche::state::ServiceError>> + Send>>
    {
        Box::pin(async { Err(paroche::state::ServiceError::NotAvailable) })
    }
}

// ── DownloadEngine adapter ──────────────────────────────────────────────────

/// Bridges `ErgasiaSession` (torrent client) to the `DownloadEngine` trait
/// that Syntaxis expects for dispatching downloads.
struct SessionEngine {
    session: Arc<ErgasiaSession>,
}

impl ergasia::DownloadEngine for SessionEngine {
    async fn start_download(
        &self,
        request: ergasia::DownloadRequest,
    ) -> Result<themelion::ids::DownloadId, ergasia::ErgasiaError> {
        self.session
            .add_torrent_from_magnet(request.download_id, &request.download_url)
            .await?;
        Ok(request.download_id)
    }

    async fn cancel_download(
        &self,
        download_id: themelion::ids::DownloadId,
    ) -> Result<(), ergasia::ErgasiaError> {
        self.session.delete_torrent(download_id).await
    }

    async fn get_progress(
        &self,
        download_id: themelion::ids::DownloadId,
    ) -> Result<ergasia::DownloadProgress, ergasia::ErgasiaError> {
        let stats = self.session.get_stats(download_id)?;
        let total = stats.total_bytes;
        let downloaded = stats.progress_bytes;
        let pct = if total > 0 {
            ((downloaded as f64 / total as f64) * 100.0) as u8
        } else {
            0
        };
        let (dl_speed, ul_speed) = match &stats.live {
            Some(live) => (
                live.download_speed.mbps * 125_000.0,
                live.upload_speed.mbps * 125_000.0,
            ),
            None => (0.0, 0.0),
        };
        Ok(ergasia::DownloadProgress {
            download_id,
            state: ergasia::DownloadState::Downloading,
            percent_complete: pct,
            download_speed_bps: dl_speed as u64,
            upload_speed_bps: ul_speed as u64,
            peers_connected: 0,
            seeders: 0,
            eta_seconds: None,
        })
    }

    fn extract(
        &self,
        download_path: &std::path::Path,
        output_dir: &std::path::Path,
    ) -> Result<Option<ergasia::ExtractionResult>, ergasia::ErgasiaError> {
        ergasia::extract_archives(download_path, output_dir, 3)
    }
}

// ── ImportService stub ──────────────────────────────────────────────────────

/// Stub ImportService for Syntaxis. The real import pipeline wiring is done
/// in a follow-up prompt; this stub accepts completed downloads and logs them.
struct StubImportService;

impl syntaxis::ImportService for StubImportService {
    fn import(
        &self,
        completed: CompletedDownload,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async move {
            tracing::info!(
                download_id = %completed.download_id,
                "import stub: download completed, import pipeline not yet wired"
            );
            Ok(())
        })
    }
}

// ── Serve entry point ───────────────────────────────────────────────────────

pub async fn run_serve(args: ServeArgs) -> Result<(), HostError> {
    // 1. Load config
    let (mut config, warnings) =
        horismos::load_config(Some(args.config.as_path())).context(ConfigSnafu)?;

    for w in &warnings {
        eprintln!("config warning: [{}] {}", w.field, w.message);
    }

    // Apply CLI overrides
    if let Some(ref listen) = args.listen {
        config.paroche.listen_addr = listen.clone();
    }
    if let Some(port) = args.port {
        config.paroche.port = port;
    }

    // 2. Initialize tracing
    init_tracing(&config)?;

    for w in &warnings {
        tracing::warn!(field = %w.field, "{}", w.message);
    }

    // 3. Set up ConfigManager for hot-reload
    let config_path = args.config.clone();
    let (config_manager, _config_handle) = ConfigManager::new(config.clone(), config_path);

    // SIGHUP handler for config reload
    let manager_for_reload = config_manager.clone();
    tokio::spawn(
        async move {
            let mut sighup = match tokio::signal::unix::signal(SignalKind::hangup()) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(
                        "failed to register SIGHUP handler: {e}; config reload via SIGHUP disabled"
                    );
                    return;
                }
            };
            loop {
                sighup.recv().await;
                tracing::info!("SIGHUP received  -  reloading configuration");
                match manager_for_reload.reload() {
                    Ok(reload_warnings) => {
                        for w in reload_warnings {
                            tracing::warn!(field = %w.field, "config reload: {}", w.message);
                        }
                        tracing::info!("configuration reloaded");
                    }
                    Err(e) => {
                        tracing::error!("config reload failed: {e}  -  keeping current config");
                    }
                }
            }
        }
        .instrument(tracing::info_span!("sighup_handler")),
    );

    let config = Arc::new(config);

    // 4. Create database pools
    let db_path = config.database.db_path.to_string_lossy();
    let db = Arc::new(init_pools(&db_path).await.context(DatabaseSnafu)?);

    // 5. Create Aggelia event bus
    let (event_tx, _event_rx) = create_event_bus(config.aggelia.buffer_size);

    // 6. Create auth service
    let auth = Arc::new(ExousiaServiceImpl::new(db.clone(), config.exousia.clone()));

    // 7. First-run admin setup
    ensure_admin_user(&auth, &db).await?;

    // 8. Create metadata resolver
    let _metadata_service =
        EpignosisService::new(config.epignosis.clone(), ProviderCredentials::default());

    // 9. Create curation service
    let _curation_service = DefaultCurationService::new(db.read.clone(), event_tx.clone());

    // 10. Start scanner  -  background task
    let scanner = ScannerManager::start(&config.taxis, event_tx.clone())
        .await
        .context(ScannerSnafu)?;

    // 11. Start feed scheduler  -  background task
    let komide_service = Arc::new(KomideService::new(
        apotheke::DbPools {
            read: db.read.clone(),
            write: db.write.clone(),
        },
        event_tx.clone(),
        reqwest::Client::new(),
        config.komide.clone(),
    ));
    let feed_scheduler = FeedScheduler::start(
        komide_service,
        config.komide.clone(),
        apotheke::DbPools {
            read: db.read.clone(),
            write: db.write.clone(),
        },
    )
    .await
    .context(FeedSchedulerSnafu)?;

    // ── Pre-flight: acquisition config validation ─────────────────────────
    validate_download_dir(&config)?;

    // ── Acquisition subsystem startup ───────────────────────────────────────

    let shutdown_token = CancellationToken::new();

    // Layer 0: Zetesis (indexer protocol)
    let zetesis = Arc::new(ZetesisService::new(
        db.read.clone(),
        db.write.clone(),
        Arc::new(NoProxy),
        config.zetesis.clone(),
        event_tx.clone(),
    ));
    info!("zetesis (indexer search) initialized");

    // Layer 1: Ergasia (download execution)
    let ergasia_session = Arc::new(
        ErgasiaSession::new(&config.ergasia)
            .await
            .context(DownloadEngineSnafu)?,
    );
    ergasia_session.reconcile_persisted_torrents();
    info!("ergasia (download engine) initialized");

    // Layer 2: Syntaxis (queue orchestration, depends on ergasia)
    let engine_adapter = Arc::new(SessionEngine {
        session: Arc::clone(&ergasia_session),
    });
    let syntaxis_svc = Arc::new(
        SyntaxisService::new(
            db.write.clone(),
            engine_adapter,
            Arc::new(StubImportService),
            config.syntaxis.clone(),
        )
        .await
        .context(DownloadQueueSnafu)?,
    );
    syntaxis_svc.start(event_tx.subscribe(), shutdown_token.child_token());
    info!("syntaxis (download queue) initialized  -  event listener started");

    // Layer 4: Syndesmos (external integrations  -  Plex, Last.fm, Tidal)
    let syndesmos_svc = Arc::new(build_syndesmos(&config, &event_tx));
    let syndesmos_handle = spawn_syndesmos_handler(
        Arc::clone(&syndesmos_svc),
        event_tx.subscribe(),
        shutdown_token.child_token(),
    );
    info!("syndesmos (external integrations) initialized  -  event listener started");

    // Layer 4: Prostheke (subtitle management)
    let providers = Provider::default_providers(config.prostheke.opensubtitles.clone());
    let _prostheke_svc = ProsthekeService::new(
        db.read.clone(),
        db.write.clone(),
        config.prostheke.clone(),
        providers,
        event_tx.clone(),
    );
    info!("prostheke (subtitles) initialized");

    // ── End acquisition startup ─────────────────────────────────────────────

    // 12. Start renderer QUIC server
    let renderer_registry = Arc::new(crate::render::RendererRegistry::new());
    let renderer_cert_dir = dirs_config_path().join("certs");
    let renderer_addr: std::net::SocketAddr = format!(
        "{}:{}",
        config.paroche.listen_addr,
        crate::render::server::DEFAULT_QUIC_PORT
    )
    .parse()
    .unwrap_or_else(|_| {
        std::net::SocketAddr::from(([0, 0, 0, 0], crate::render::server::DEFAULT_QUIC_PORT))
    });
    let renderer_registry_for_quic = Arc::clone(&renderer_registry);
    let renderer_shutdown = shutdown_token.child_token();
    tokio::spawn(
        async move {
            if let Err(e) = crate::render::server::start_renderer_server(
                renderer_addr,
                &renderer_cert_dir,
                renderer_registry_for_quic,
                renderer_shutdown,
            )
            .await
            {
                tracing::error!(error = %e, "renderer QUIC server failed");
            }
        }
        .instrument(tracing::info_span!("renderer_server")),
    );

    // 13. Build import service adapter for paroche
    let import = paroche::state::make_import_service(|| async { Ok(vec![]) });

    // 13. Build HTTP router
    let state = AppState {
        db,
        config: config.clone(),
        event_tx,
        auth,
        import,
        metadata: Arc::new(NullMetadata),
        curation: Arc::new(NullCuration),
        search: Arc::new(SearchAdapter(zetesis)),
        download_engine: Arc::new(EngineAdapter(ergasia_session)),
        queue: Arc::new(QueueAdapter),
        requests: Arc::new(RequestAdapter),
        external: Arc::new(ExternalAdapter(syndesmos_svc)),
        subtitles: Arc::new(SubtitleAdapter),
        renderers: renderer_registry,
    };
    let router = paroche::build_router(state);

    // 14. Bind + serve
    let addr = format!("{}:{}", config.paroche.listen_addr, config.paroche.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context(ServerSnafu)?;
    info!("Harmonia serving on {addr}");

    // 15. Graceful shutdown
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context(ServerSnafu)?;

    // 16. Cleanup  -  reverse startup ORDER
    info!("shutting down subsystems");

    // Cancel all acquisition background tasks (syndesmos event handler, syntaxis listener)
    shutdown_token.cancel();

    // Wait for syndesmos event handler to drain
    if let Err(e) = syndesmos_handle.await {
        tracing::warn!(error = %e, "syndesmos event handler panicked during shutdown");
    }

    // Shutdown core subsystems (reverse of startup)
    feed_scheduler.shutdown();
    scanner.shutdown().await;

    info!("shutdown complete");
    Ok(())
}

// ── Syndesmos construction ──────────────────────────────────────────────────

fn build_syndesmos(
    config: &horismos::Config,
    event_tx: &themelion::EventSender,
) -> SyndesmosService {
    let mut builder = SyndesmosServiceBuilder::new(event_tx.clone())
        .circuit_break_minutes(config.syndesmos.circuit_break_minutes);

    if let Some(ref plex_config) = config.syndesmos.plex {
        let client = syndesmos::plex::PlexClient::new(plex_config.clone());
        builder = builder.with_plex(client);
    }

    if let Some(ref lastfm_config) = config.syndesmos.lastfm {
        let client = syndesmos::lastfm::LastfmClient::new(lastfm_config.clone());
        builder = builder.with_lastfm(client);
    }

    if let Some(ref tidal_config) = config.syndesmos.tidal {
        let client = syndesmos::tidal::TidalClient::new(tidal_config.clone());
        builder = builder.with_tidal(client);
    }

    builder.build()
}

fn spawn_syndesmos_handler(
    service: Arc<SyndesmosService>,
    event_rx: themelion::EventReceiver,
    ct: CancellationToken,
) -> JoinHandle<()> {
    let span = tracing::info_span!("syndesmos_event_handler");
    tokio::spawn(
        async move {
            syndesmos::events::run_event_handler(service, event_rx, ct).await;
        }
        .instrument(span),
    )
}

// ── Config pre-flight ───────────────────────────────────────────────────────

fn validate_download_dir(config: &horismos::Config) -> Result<(), HostError> {
    let dir = &config.ergasia.download_dir;
    if !dir.exists() {
        return Err(HostError::Config {
            source: horismos::HorismosError::Validation {
                message: format!(
                    "ergasia.download_dir '{}' does not exist  -  CREATE it before starting",
                    dir.display()
                ),
                location: snafu::location!(),
            },
            location: snafu::location!(),
        });
    }
    let test_file = dir.join(".harmonia-write-test");
    if let Err(e) = std::fs::write(&test_file, b"") {
        return Err(HostError::Config {
            source: horismos::HorismosError::Validation {
                message: format!(
                    "ergasia.download_dir '{}' is not writable: {e}",
                    dir.display()
                ),
                location: snafu::location!(),
            },
            location: snafu::location!(),
        });
    }
    let _ = std::fs::remove_file(&test_file);
    Ok(())
}

fn dirs_config_path() -> std::path::PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".config"))
                .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        })
        .join("harmonia")
}
