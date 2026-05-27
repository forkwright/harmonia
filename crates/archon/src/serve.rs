use std::io::Write;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use aitesis::{IdentityValidator, MonitorService, RequestService, UserRoleProvider};
use apotheke::init_pools;
use epignosis::ProviderBackedResolver;
use epignosis::resolver::ProviderCredentials;
use ergasia::TorrentSession;
use exousia::ExousiaServiceImpl;
use horismos::ConfigManager;
use kathodos::ScannerManager;
use komide::KomideService;
use komide::scheduler::FeedScheduler;
use kritike::DefaultCurationService;
use paroche::state::{
    AppState, DynCurationService, DynDownloadEngine, DynExternalIntegration, DynMetadataResolver,
    DynQueueManager, DynRequestService, DynSearchService, DynSubtitleService, RequestServiceFut,
    ServiceError, ServiceFut,
};
use prostheke::providers::Provider;
use prostheke::{SubtitleManager, SubtitleService};
use snafu::ResultExt;
use syndesmos::{SyndesmosService, SyndesmosServiceBuilder};
use syntaxis::{CompletedDownload, SyntaxisService};
use themelion::{MediaId, MediaType, create_event_bus};
use tokio::signal::unix::SignalKind;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, info};
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

struct CurationAdapter(#[expect(dead_code)] Arc<DefaultCurationService>);
impl DynCurationService for CurationAdapter {}

struct MetadataAdapter(#[expect(dead_code)] Arc<ProviderBackedResolver>);
impl DynMetadataResolver for MetadataAdapter {}

struct SearchAdapter(Arc<ZetesisService>);
impl DynSearchService for SearchAdapter {
    fn search(&self, query: serde_json::Value) -> ServiceFut<serde_json::Value> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            let query = search_query_from_json(query)?;
            let results = service
                .search(query, CancellationToken::new())
                .await
                .map_err(search_error)?;
            serde_json::to_value(serde_json::json!({ "results": results }))
                .map_err(|error| ServiceError::Internal(error.to_string()))
        })
    }

    fn test_indexer(&self, indexer_id: i64) -> ServiceFut<serde_json::Value> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            let status = service
                .test_indexer(indexer_id, CancellationToken::new())
                .await
                .map_err(search_error)?;
            serde_json::to_value(status).map_err(|error| ServiceError::Internal(error.to_string()))
        })
    }

    fn refresh_caps(&self, indexer_id: i64) -> ServiceFut<serde_json::Value> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            let caps = service
                .refresh_caps(indexer_id, CancellationToken::new())
                .await
                .map_err(search_error)?;
            serde_json::to_value(caps).map_err(|error| ServiceError::Internal(error.to_string()))
        })
    }
}

fn search_query_from_json(value: serde_json::Value) -> Result<zetesis::SearchQuery, ServiceError> {
    if value.get("query_id").is_some() {
        return Err(ServiceError::NotFound);
    }

    let media_type = value
        .get("media_type")
        .and_then(serde_json::Value::as_str)
        .map(parse_search_media_type)
        .transpose()?
        .unwrap_or_default();

    Ok(zetesis::SearchQuery {
        query_text: json_string(&value, "query_text"),
        media_type,
        category_ids: value
            .get("category_ids")
            .and_then(serde_json::Value::as_array)
            .map(|ids| {
                ids.iter()
                    .filter_map(serde_json::Value::as_u64)
                    .filter_map(|id| u32::try_from(id).ok())
                    .collect()
            })
            .unwrap_or_default(),
        imdb_id: json_string(&value, "imdb_id"),
        tvdb_id: json_u32(&value, "tvdb_id"),
        tmdb_id: json_u32(&value, "tmdb_id"),
        artist: json_string(&value, "artist"),
        album: json_string(&value, "album"),
        author: json_string(&value, "author"),
        season: json_u32(&value, "season"),
        episode: json_u32(&value, "episode"),
        limit: json_u32(&value, "limit").unwrap_or(100),
        offset: json_u32(&value, "offset").unwrap_or_default(),
    })
}

fn parse_search_media_type(media_type: &str) -> Result<zetesis::SearchMediaType, ServiceError> {
    match media_type {
        "any" => Ok(zetesis::SearchMediaType::Any),
        "tv" | "series" => Ok(zetesis::SearchMediaType::Tv),
        "movie" | "movies" => Ok(zetesis::SearchMediaType::Movie),
        "music" | "album" | "music_album" => Ok(zetesis::SearchMediaType::Music),
        "book" | "books" | "audiobook" | "comic" => Ok(zetesis::SearchMediaType::Book),
        other => Err(ServiceError::Internal(format!(
            "unsupported search media_type: {other}"
        ))),
    }
}

fn json_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
}

fn json_u32(value: &serde_json::Value, key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
}

fn search_error(error: zetesis::ZetesisError) -> ServiceError {
    match error {
        zetesis::ZetesisError::IndexerNotFound { .. } => ServiceError::NotFound,
        other => ServiceError::Internal(other.to_string()),
    }
}

struct EngineAdapter(#[expect(dead_code)] Arc<TorrentSession>);
impl DynDownloadEngine for EngineAdapter {}

struct QueueAdapter;
impl DynQueueManager for QueueAdapter {}

type LiveRequestService =
    aitesis::AitesisServiceImpl<RequestRoleProvider, RequestIdentityValidator, RequestMonitor>;

struct RequestAdapter(Arc<LiveRequestService>);
impl DynRequestService for RequestAdapter {
    fn submit_request(
        &self,
        user_id: themelion::UserId,
        input: aitesis::CreateRequestInput,
    ) -> RequestServiceFut<'_, aitesis::MediaRequest> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .submit_request(user_id, input)
                .await
                .map_err(Into::into)
        })
    }

    fn approve(
        &self,
        request_id: themelion::RequestId,
        admin_id: themelion::UserId,
    ) -> RequestServiceFut<'_, aitesis::MediaRequest> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .approve(request_id, admin_id)
                .await
                .map_err(Into::into)
        })
    }

    fn deny(
        &self,
        request_id: themelion::RequestId,
        admin_id: themelion::UserId,
        reason: Option<String>,
    ) -> RequestServiceFut<'_, aitesis::MediaRequest> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .deny(request_id, admin_id, reason)
                .await
                .map_err(Into::into)
        })
    }

    fn get_request(
        &self,
        request_id: themelion::RequestId,
    ) -> RequestServiceFut<'_, aitesis::MediaRequest> {
        let service = Arc::clone(&self.0);
        Box::pin(async move { service.get_request(request_id).await.map_err(Into::into) })
    }

    fn list_requests(
        &self,
        user_id: Option<themelion::UserId>,
        status: Option<aitesis::RequestStatus>,
    ) -> RequestServiceFut<'_, Vec<aitesis::MediaRequest>> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .list_requests(user_id, status)
                .await
                .map_err(Into::into)
        })
    }

    fn cancel_request(
        &self,
        request_id: themelion::RequestId,
        user_id: themelion::UserId,
    ) -> RequestServiceFut<'_, ()> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .cancel_request(request_id, user_id)
                .await
                .map_err(Into::into)
        })
    }
}

struct RequestRoleProvider {
    db: Arc<apotheke::DbPools>,
}

impl UserRoleProvider for RequestRoleProvider {
    async fn role_of(
        &self,
        user_id: themelion::UserId,
    ) -> Result<aitesis::UserRole, aitesis::AitesisError> {
        let user = apotheke::repo::user::get_user(&self.db.read, user_id.as_bytes().as_slice())
            .await
            .context(aitesis::error::DatabaseSnafu)?;
        let Some(user) = user else {
            return aitesis::error::InsufficientPermissionSnafu.fail();
        };
        if user.is_active == 0 {
            return aitesis::error::InsufficientPermissionSnafu.fail();
        }
        match exousia::UserRole::parse(&user.role) {
            Some(exousia::UserRole::Admin) => Ok(aitesis::UserRole::Admin),
            Some(exousia::UserRole::Member) => Ok(aitesis::UserRole::Member),
            Some(_) | None => aitesis::error::InsufficientPermissionSnafu.fail(),
        }
    }
}

struct RequestIdentityValidator;

impl IdentityValidator for RequestIdentityValidator {
    async fn validate(
        &self,
        media_type: themelion::MediaType,
        title: &str,
        _external_id: Option<&str>,
    ) -> Result<(), aitesis::AitesisError> {
        if title.trim().is_empty() {
            return aitesis::error::MediaIdentityInvalidSnafu {
                detail: "title is required".to_string(),
            }
            .fail();
        }
        if matches!(media_type, themelion::MediaType::News) {
            return aitesis::error::MediaIdentityInvalidSnafu {
                detail: "news requests cannot be handed off to wanted media".to_string(),
            }
            .fail();
        }
        Ok(())
    }
}

struct RequestMonitor {
    db: Arc<apotheke::DbPools>,
}

impl MonitorService for RequestMonitor {
    async fn create_want(
        &self,
        request: &aitesis::MediaRequest,
    ) -> Result<themelion::WantId, aitesis::AitesisError> {
        let Some((want_media_type, quality_media_type)) = request_media_types(request.media_type)
        else {
            return aitesis::error::MediaIdentityInvalidSnafu {
                detail: format!(
                    "{} requests cannot be handed off to wanted media",
                    request.media_type
                ),
            }
            .fail();
        };
        let profile =
            apotheke::repo::quality::list_profiles_for_type(&self.db.read, quality_media_type)
                .await
                .context(aitesis::error::DatabaseSnafu)?
                .into_iter()
                .next()
                .ok_or_else(|| {
                    aitesis::error::MediaIdentityInvalidSnafu {
                        detail: format!("no quality profile for {quality_media_type}"),
                    }
                    .build()
                })?;

        let want_id = themelion::WantId::new();
        apotheke::repo::want::insert_want(
            &self.db.write,
            &apotheke::repo::want::Want {
                id: want_id.as_bytes().to_vec(),
                media_type: want_media_type.to_string(),
                title: request.title.clone(),
                registry_id: None,
                quality_profile_id: profile.id,
                status: "searching".to_string(),
                source: Some("request".to_string()),
                source_ref: Some(request.id.as_uuid().to_string()),
                added_at: jiff::Timestamp::now().to_string(),
                fulfilled_at: None,
            },
        )
        .await
        .context(aitesis::error::DatabaseSnafu)?;
        Ok(want_id)
    }
}

fn request_media_types(media_type: themelion::MediaType) -> Option<(&'static str, &'static str)> {
    match media_type {
        themelion::MediaType::Music => Some(("music_album", "music")),
        themelion::MediaType::Audiobook => Some(("audiobook", "audiobook")),
        themelion::MediaType::Book => Some(("book", "book")),
        themelion::MediaType::Comic => Some(("comic", "comic")),
        themelion::MediaType::Podcast => Some(("podcast", "podcast")),
        themelion::MediaType::Movie => Some(("movie", "movie")),
        themelion::MediaType::Tv => Some(("tv_series", "tv")),
        themelion::MediaType::News => None,
        _ => None,
    }
}

struct ExternalAdapter(#[expect(dead_code)] Arc<SyndesmosService>);
impl DynExternalIntegration for ExternalAdapter {}

struct SubtitleAdapter {
    service: Arc<SubtitleManager>,
    read: sqlx::SqlitePool,
}

impl DynSubtitleService for SubtitleAdapter {
    fn search_for_media(
        &self,
        media_id: Vec<u8>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(), paroche::state::ServiceError>> + Send>>
    {
        let service = Arc::clone(&self.service);
        let read = self.read.clone();
        Box::pin(async move {
            let media_id = media_id_from_bytes(media_id)?;
            let (media_type, file_path) = subtitle_target(&read, media_id).await?;
            service
                .acquire_subtitles(media_id, media_type, &file_path)
                .await
                .map_err(|error| paroche::state::ServiceError::Internal(error.to_string()))
        })
    }
}

fn media_id_from_bytes(media_id: Vec<u8>) -> Result<MediaId, paroche::state::ServiceError> {
    uuid::Uuid::from_slice(&media_id)
        .map(MediaId::from_uuid)
        .map_err(|_| paroche::state::ServiceError::NotFound)
}

async fn subtitle_target(
    pool: &sqlx::SqlitePool,
    media_id: MediaId,
) -> Result<(MediaType, PathBuf), paroche::state::ServiceError> {
    let id = media_id.as_bytes().as_slice();
    let movie = apotheke::repo::movie::get_movie(pool, id)
        .await
        .map_err(|error| paroche::state::ServiceError::Internal(error.to_string()))?;
    if let Some(movie) = movie {
        let path = movie
            .file_path
            .ok_or(paroche::state::ServiceError::NotFound)?;
        return Ok((MediaType::Movie, PathBuf::from(path)));
    }

    let episode = apotheke::repo::tv::get_episode(pool, id)
        .await
        .map_err(|error| paroche::state::ServiceError::Internal(error.to_string()))?;
    if let Some(episode) = episode {
        let path = episode
            .file_path
            .ok_or(paroche::state::ServiceError::NotFound)?;
        return Ok((MediaType::Tv, PathBuf::from(path)));
    }

    Err(paroche::state::ServiceError::NotFound)
}

// ── DownloadEngine adapter ──────────────────────────────────────────────────

/// Bridges `TorrentSession` (torrent client) to the `DownloadEngine` trait
/// that Syntaxis expects for dispatching downloads.
struct SessionEngine {
    session: Arc<TorrentSession>,
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

// TODO[deliberate-prudent] #300: replace with real ImportService that calls kathodos // kanon:ignore RUST/todo-no-issue -- richer quadrant rule covers this // kanon:ignore META/rule-todo-without-issue -- richer quadrant rule covers this
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
            Err("import pipeline not wired".to_string())
        })
    }
}

// ── Serve entry point ───────────────────────────────────────────────────────

pub async fn run_serve(args: ServeArgs, out: &mut impl Write) -> Result<(), HostError> {
    // 1. Load config
    let (mut config, warnings) =
        horismos::load_config(Some(args.config.as_path())).context(ConfigSnafu)?;

    for w in &warnings {
        // WHY: writeln! to stdout is non-fatal; broken pipe on exit is expected behavior
        writeln!(out, "config warning: [{}] {}", w.field, w.message).ok();
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
    ensure_admin_user(&auth, &db, out).await?;

    // 8. Create metadata resolver
    let metadata_service = Arc::new(ProviderBackedResolver::new(
        config.epignosis.clone(),
        ProviderCredentials::default(),
    ));

    // 9. Create curation service
    let curation_service = Arc::new(DefaultCurationService::new(
        db.read.clone(),
        event_tx.clone(),
    ));

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
        TorrentSession::new(&config.ergasia)
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
    let prostheke_svc = Arc::new(SubtitleManager::new(
        db.read.clone(),
        db.write.clone(),
        config.prostheke.clone(),
        providers,
        event_tx.clone(),
    ));
    info!("prostheke (subtitles) initialized");

    // Layer 5: Aitesis (household request workflow)
    let request_service = Arc::new(aitesis::AitesisServiceImpl::new(
        db.read.clone(),
        db.write.clone(),
        config.aitesis.clone(),
        RequestRoleProvider { db: db.clone() },
        RequestIdentityValidator,
        RequestMonitor { db: db.clone() },
    ));
    info!("aitesis (media requests) initialized");

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

    let subtitles = Arc::new(SubtitleAdapter {
        service: prostheke_svc,
        read: db.read.clone(),
    });

    // 13. Build HTTP router
    let state = AppState {
        db,
        config: config.clone(),
        event_tx,
        auth,
        import,
        metadata: Arc::new(MetadataAdapter(metadata_service)),
        curation: Arc::new(CurationAdapter(curation_service)),
        search: Arc::new(SearchAdapter(zetesis)),
        download_engine: Arc::new(EngineAdapter(ergasia_session)),
        queue: Arc::new(QueueAdapter),
        requests: Arc::new(RequestAdapter(request_service)),
        external: Arc::new(ExternalAdapter(syndesmos_svc)),
        subtitles,
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
    // WHY: temp file cleanup failure is non-fatal; OS will reclaim on exit
    std::fs::remove_file(&test_file).ok();
    Ok(())
}

#[cfg(test)]
mod search_adapter_tests {
    use std::sync::Arc;

    use apotheke::migrate::MIGRATOR;
    use paroche::state::{DynSearchService, ServiceError};
    use serde_json::json;
    use sqlx::SqlitePool;
    use themelion::create_event_bus;

    use super::{SearchAdapter, parse_search_media_type, search_query_from_json};

    #[test]
    fn search_query_from_json_maps_route_payload_to_zetesis_query() {
        let query = search_query_from_json(json!({
            "query_text": "Kind of Blue",
            "media_type": "music",
            "category_ids": [3000, 3010],
            "artist": "Miles Davis",
            "limit": 25,
            "offset": 5
        }))
        .expect("valid search query");

        assert_eq!(query.query_text.as_deref(), Some("Kind of Blue"));
        assert_eq!(query.media_type, zetesis::SearchMediaType::Music);
        assert_eq!(query.category_ids, vec![3000, 3010]);
        assert_eq!(query.artist.as_deref(), Some("Miles Davis"));
        assert_eq!(query.limit, 25);
        assert_eq!(query.offset, 5);
    }

    #[test]
    fn search_query_from_json_rejects_cached_result_lookup() {
        let error = search_query_from_json(json!({ "query_id": "q-1" }))
            .expect_err("cached result lookup is not backed by zetesis search fan-out");
        assert!(matches!(error, ServiceError::NotFound));
    }

    #[test]
    fn parse_search_media_type_rejects_unknown_values() {
        let error = parse_search_media_type("podcast").expect_err("unsupported media type");
        assert!(matches!(error, ServiceError::Internal(_)));
    }

    #[tokio::test]
    async fn search_adapter_calls_live_zetesis_service() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite opens");
        MIGRATOR.run(&pool).await.expect("migrations run");
        let (event_tx, _) = create_event_bus(64);
        let service = zetesis::ZetesisService::new(
            pool.clone(),
            pool,
            Arc::new(zetesis::cf_bypass::noop::NoProxy),
            horismos::SearchSubsystemConfig::default(),
            event_tx,
        );
        let adapter = SearchAdapter(Arc::new(service));

        let result = adapter
            .search(json!({ "query_text": "empty library", "media_type": "music" }))
            .await
            .expect("live zetesis search should return an empty result set without indexers");

        assert_eq!(
            result["results"].as_array().expect("results array").len(),
            0
        );
    }
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use apotheke::migrate::MIGRATOR;
    use paroche::state::{DynSubtitleService, ServiceError};
    use sqlx::SqlitePool;
    use syntaxis::ImportService;
    use themelion::ids::{DownloadId, ReleaseId, WantId};

    use super::*;

    #[tokio::test]
    async fn run_serve_output_param_accepted() {
        let mut out = Vec::new();
        let args = ServeArgs {
            config: std::path::PathBuf::from("/nonexistent/config.toml"),
            listen: None,
            port: None,
        };
        // The function should accept a Vec<u8> writer and fail on missing config.
        let result = run_serve(args, &mut out).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn stub_import_service_fails_until_import_pipeline_is_wired() {
        let completed = CompletedDownload {
            download_id: DownloadId::new(),
            download_path: PathBuf::from("/data/downloads/album"),
            source_path: PathBuf::from("/data/downloads/album"),
            want_id: WantId::new(),
            release_id: ReleaseId::new(),
            protocol: syntaxis::DownloadProtocol::Torrent,
            requires_copy: false,
        };

        let result = StubImportService.import(completed).await;

        assert_eq!(result, Err("import pipeline not wired".to_string()));
    }

    #[tokio::test]
    async fn subtitle_adapter_calls_live_prostheke_for_movie_path() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite opens");
        MIGRATOR.run(&pool).await.expect("migrations run");

        let media_id = MediaId::new();
        apotheke::repo::movie::insert_movie(
            &pool,
            &apotheke::repo::movie::Movie {
                id: media_id.as_bytes().as_slice().to_vec(),
                registry_id: None,
                title: "Dune".to_string(),
                original_title: None,
                year: Some(2021),
                tmdb_id: None,
                imdb_id: None,
                runtime_min: None,
                overview: None,
                certification: None,
                file_path: Some("/library/movies/Dune.mkv".to_string()),
                file_format: Some("mkv".to_string()),
                file_size_bytes: None,
                resolution: None,
                codec: None,
                hdr_type: None,
                quality_score: None,
                quality_profile_id: None,
                source_type: "local".to_string(),
                added_at: "2026-01-01T00:00:00Z".to_string(),
            },
        )
        .await
        .expect("movie inserted");

        let (event_tx, _) = create_event_bus(64);
        let service = Arc::new(SubtitleManager::new(
            pool.clone(),
            pool.clone(),
            horismos::ProsthekeConfig::default(),
            Vec::<Provider>::new(),
            event_tx,
        ));
        let adapter = SubtitleAdapter {
            service,
            read: pool,
        };

        adapter
            .search_for_media(media_id.as_bytes().as_slice().to_vec())
            .await
            .expect("empty provider set is still a live Prostheke call");
    }

    #[tokio::test]
    async fn subtitle_target_rejects_non_video_or_missing_media() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite opens");
        MIGRATOR.run(&pool).await.expect("migrations run");

        let error = subtitle_target(&pool, MediaId::new())
            .await
            .expect_err("missing media cannot be searched for subtitles");

        assert!(matches!(error, ServiceError::NotFound));
    }
}
