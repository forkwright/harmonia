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
use komide::FeedSchedulerService;
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
use syndesmos::{ScrobbleClient, ScrobbleClientBuilder};
use syntaxis::{CompletedDownload, DownloadQueue, QueueManager};
use themelion::{MediaId, MediaType, create_event_bus};
use tokio::signal::unix::SignalKind;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, info};
use zetesis::SearchIndexerService;
use zetesis::cf_bypass::CloudflareProxy;
use zetesis::cf_bypass::byparr::ByparrProxy;
use zetesis::cf_bypass::noop::NoProxy;

use crate::cli::ServeArgs;
use crate::error::{
    ConfigSnafu, DatabaseSnafu, DownloadEngineSnafu, DownloadQueueSnafu, FeedSchedulerSnafu,
    HostError, ListenAddrSnafu, ScannerSnafu, ServerSnafu,
};
use crate::shutdown::shutdown_signal;
use crate::startup::{ensure_admin_user, init_tracing};

// ── Dyn-trait adapters ──────────────────────────────────────────────────────

struct CurationAdapter(Arc<DefaultCurationService>);

impl DynCurationService for CurationAdapter {
    fn assess_quality(
        &self,
        media_type: themelion::MediaType,
        item_metadata: kritike::QualityMetadata,
    ) -> ServiceFut<kritike::QualityAssessment> {
        use kritike::CurationService as _;
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .assess_quality(media_type, &item_metadata)
                .await
                .map_err(curation_error)
        })
    }

    fn check_upgrade_eligibility(
        &self,
        have_id: themelion::HaveId,
        candidate_score: i32,
    ) -> ServiceFut<kritike::UpgradeDecision> {
        use kritike::CurationService as _;
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .check_upgrade_eligibility(have_id, candidate_score)
                .await
                .map_err(curation_error)
        })
    }

    fn health_report(&self) -> ServiceFut<kritike::HealthReport> {
        use kritike::CurationService as _;
        let service = Arc::clone(&self.0);
        Box::pin(async move { service.health_report().await.map_err(curation_error) })
    }
}

fn curation_error(error: kritike::KritikeError) -> ServiceError {
    match error {
        kritike::KritikeError::ProfileNotFound { .. } => ServiceError::NotFound,
        other => ServiceError::Internal(other.to_string()),
    }
}

struct MetadataAdapter(Arc<ProviderBackedResolver>);

impl DynMetadataResolver for MetadataAdapter {
    fn resolve_identity(
        &self,
        item: epignosis::UnidentifiedItem,
    ) -> ServiceFut<epignosis::MediaIdentity> {
        use epignosis::MetadataResolver as _;
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .resolve_identity(&item, CancellationToken::new())
                .await
                .map_err(metadata_error)
        })
    }

    fn enrich(
        &self,
        identity: epignosis::MediaIdentity,
    ) -> ServiceFut<epignosis::EnrichedMetadata> {
        use epignosis::MetadataResolver as _;
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .enrich(&identity, CancellationToken::new())
                .await
                .map_err(metadata_error)
        })
    }

    fn fingerprint_audio(
        &self,
        file_path: std::path::PathBuf,
    ) -> ServiceFut<epignosis::FingerprintResult> {
        use epignosis::MetadataResolver as _;
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .fingerprint_audio(&file_path, CancellationToken::new())
                .await
                .map_err(metadata_error)
        })
    }
}

fn metadata_error(error: epignosis::EpignosisError) -> ServiceError {
    match error {
        epignosis::EpignosisError::IdentityNotResolved { .. } => ServiceError::NotFound,
        other => ServiceError::Internal(other.to_string()),
    }
}

struct SearchAdapter(Arc<SearchIndexerService>);
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
        .unwrap_or_default(); // WHY: transpose()? yields Option<T>; unwrap_or_default is on Option, not Result

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
            .unwrap_or_default(), // WHY: .and_then().map() produces Option<Vec<_>>; unwrap_or_default is on Option, not Result
        imdb_id: json_string(&value, "imdb_id"),
        tvdb_id: json_u32(&value, "tvdb_id"),
        tmdb_id: json_u32(&value, "tmdb_id"),
        artist: json_string(&value, "artist"),
        album: json_string(&value, "album"),
        author: json_string(&value, "author"),
        season: json_u32(&value, "season"),
        episode: json_u32(&value, "episode"),
        limit: json_u32(&value, "limit").unwrap_or(100),
        offset: json_u32(&value, "offset").unwrap_or_default(), // WHY: json_u32 returns Option<u32>; unwrap_or_default is on Option, not Result
    })
}

fn parse_search_media_type(media_type: &str) -> Result<zetesis::SearchMediaType, ServiceError> {
    match media_type {
        "any" => Ok(zetesis::SearchMediaType::Any),
        "tv" | "series" => Ok(zetesis::SearchMediaType::Tv),
        "movie" | "movies" => Ok(zetesis::SearchMediaType::Movie),
        "music" | "album" | "music_album" => Ok(zetesis::SearchMediaType::Music),
        "book" | "books" | "audiobook" | "comic" => Ok(zetesis::SearchMediaType::Book),
        other => Err(ServiceError::InvalidInput(format!(
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

fn search_error(error: zetesis::SearchIndexerError) -> ServiceError {
    match error {
        zetesis::SearchIndexerError::IndexerNotFound { .. } => ServiceError::NotFound,
        other => ServiceError::Internal(other.to_string()),
    }
}

struct EngineAdapter(#[expect(dead_code)] Arc<TorrentSession>);
impl DynDownloadEngine for EngineAdapter {}

/// Bridges the running `DownloadQueue` to paroche's queue-manager trait so an
/// API enqueue/cancel/reprioritize reaches the live dispatcher and engine.
struct QueueAdapter<E: ergasia::DownloadEngine + 'static>(Arc<DownloadQueue<E>>);

impl<E: ergasia::DownloadEngine + 'static> DynQueueManager for QueueAdapter<E> {
    fn enqueue(&self, item: paroche::state::EnqueueItem) -> ServiceFut<()> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            let protocol = syntaxis::DownloadProtocol::parse(&item.protocol).ok_or_else(|| {
                ServiceError::InvalidInput(format!(
                    "unsupported download protocol: {}",
                    item.protocol
                ))
            })?;
            service
                .enqueue(syntaxis::QueueItem {
                    id: item.queue_id,
                    want_id: themelion::WantId::from_uuid(item.want_id),
                    release_id: themelion::ReleaseId::from_uuid(item.release_id),
                    download_url: item.download_url,
                    protocol,
                    priority: item.priority,
                    tracker_id: None,
                    info_hash: item.info_hash,
                    retry_count: 0,
                })
                .await
                .map(|_| ())
                .map_err(queue_error)
        })
    }

    fn cancel(&self, queue_id: uuid::Uuid) -> ServiceFut<()> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .cancel_by_queue_id(queue_id)
                .await
                .map_err(queue_error)
        })
    }

    fn reprioritize(&self, queue_id: uuid::Uuid, priority: u8) -> ServiceFut<()> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .reprioritize_by_queue_id(queue_id, priority)
                .await
                .map_err(queue_error)
        })
    }
}

fn queue_error(error: syntaxis::SyntaxisError) -> ServiceError {
    match error {
        syntaxis::SyntaxisError::ItemNotFound { .. } => ServiceError::NotFound,
        other => ServiceError::Internal(other.to_string()),
    }
}

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
        caller_id: themelion::UserId,
        user_id: Option<themelion::UserId>,
        status: Option<aitesis::RequestStatus>,
        limit: u32,
        offset: u32,
    ) -> RequestServiceFut<'_, Vec<aitesis::MediaRequest>> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .list_requests(caller_id, user_id, status, limit, offset)
                .await
                .map_err(Into::into)
        })
    }

    fn count_requests(
        &self,
        caller_id: themelion::UserId,
        user_id: Option<themelion::UserId>,
        status: Option<aitesis::RequestStatus>,
    ) -> RequestServiceFut<'_, u64> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .count_requests(caller_id, user_id, status)
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

        // WHY: upsert keyed on (source='request', source_ref=request id) —
        // MonitorService::create_want must be idempotent per request so a
        // retried approval resolves to the existing want instead of
        // double-inserting one.
        let want_id = themelion::WantId::new();
        let source_ref = request.id.as_uuid().to_string();
        let stored_id = apotheke::repo::want::upsert_want_by_source_ref(
            &self.db.write,
            "request",
            &source_ref,
            &apotheke::repo::want::Want {
                id: want_id.as_bytes().to_vec(),
                media_type: want_media_type.to_string(),
                title: request.title.clone(),
                registry_id: None,
                quality_profile_id: profile.id,
                status: "searching".to_string(),
                source: Some("request".to_string()),
                source_ref: Some(source_ref.clone()),
                added_at: jiff::Timestamp::now().to_string(),
                fulfilled_at: None,
            },
        )
        .await
        .context(aitesis::error::DatabaseSnafu)?;
        let stored_uuid = uuid::Uuid::from_slice(&stored_id).map_err(|_| {
            aitesis::error::MediaIdentityInvalidSnafu {
                detail: format!(
                    "stored want id for request {} is not a valid uuid",
                    request.id
                ),
            }
            .build()
        })?;
        Ok(themelion::WantId::from_uuid(stored_uuid))
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

struct ExternalAdapter(#[expect(dead_code)] Arc<ScrobbleClient>);
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
    extraction_limits: ergasia::ExtractionLimits,
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

    async fn extract(
        &self,
        download_path: &std::path::Path,
        output_dir: &std::path::Path,
    ) -> Result<Option<ergasia::ExtractionResult>, ergasia::ErgasiaError> {
        ergasia::extract_archives(download_path, output_dir, self.extraction_limits).await
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
                // WHY: reload() does blocking file I/O (figment TOML read);
                // spawn_blocking keeps it off the async worker thread.
                let manager = manager_for_reload.clone();
                match tokio::task::spawn_blocking(move || manager.reload()).await {
                    Ok(Ok(reload_warnings)) => {
                        for w in reload_warnings {
                            tracing::warn!(field = %w.field, "config reload: {}", w.message);
                        }
                        tracing::info!("configuration reloaded");
                    }
                    Ok(Err(e)) => {
                        tracing::error!("config reload failed: {e}  -  keeping current config");
                    }
                    Err(e) => {
                        tracing::error!(
                            "config reload task panicked: {e}  -  keeping current config"
                        );
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
    let komide_service = Arc::new(FeedSchedulerService::new(
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
    let cf_proxy = build_cf_proxy(&config.zetesis)?;
    let zetesis = Arc::new(SearchIndexerService::new(
        db.read.clone(),
        db.write.clone(),
        cf_proxy,
        config.zetesis.clone(),
        event_tx.clone(),
    ));
    info!(
        cloudflare_bypass = config.zetesis.cloudflare_bypass_enabled,
        "zetesis (indexer search) initialized"
    );

    // Layer 1: Ergasia (download execution)
    let ergasia_session = Arc::new(
        TorrentSession::new(&config.ergasia)
            .await
            .context(DownloadEngineSnafu)?,
    );
    info!("ergasia (download engine) initialized");

    // Layer 2: Syntaxis (queue orchestration, depends on ergasia)
    let engine_adapter = Arc::new(SessionEngine {
        session: Arc::clone(&ergasia_session),
        extraction_limits: ergasia::ExtractionLimits::from(&config.ergasia),
    });
    let syntaxis_svc = Arc::new(
        DownloadQueue::new(
            db.write.clone(),
            engine_adapter,
            Arc::new(StubImportService),
            config.syntaxis.clone(),
        )
        .await
        .context(DownloadQueueSnafu)?,
    );
    let syntaxis_handle = syntaxis_svc.start(event_tx.subscribe(), shutdown_token.child_token());
    info!("syntaxis (download queue) initialized  -  event listener started");

    // Layer 4: Syndesmos (external integrations  -  Plex, Last.fm, Tidal)
    let syndesmos_svc = Arc::new(build_syndesmos(&config, &event_tx, db.read.clone()));
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
    let renderer_cert_dir = crate::paths::dirs_config_path().join("certs");
    let renderer_addr = resolve_listen_addr(
        &config.paroche.listen_addr,
        crate::render::server::DEFAULT_QUIC_PORT,
    )?;
    let renderer_api_key = config.paroche.renderer_api_key.clone();
    let renderer_registry_for_quic = Arc::clone(&renderer_registry);
    let renderer_shutdown = shutdown_token.child_token();
    tokio::spawn(
        async move {
            if let Err(e) = crate::render::server::start_renderer_server(
                renderer_addr,
                &renderer_cert_dir,
                renderer_registry_for_quic,
                renderer_shutdown,
                renderer_api_key,
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
        queue: Arc::new(QueueAdapter(Arc::clone(&syntaxis_svc))),
        requests: Arc::new(RequestAdapter(request_service)),
        external: Arc::new(ExternalAdapter(syndesmos_svc)),
        subtitles,
        renderers: renderer_registry,
    };
    let router = paroche::build_router(state);

    // 14. Bind + serve
    let addr = resolve_listen_addr(&config.paroche.listen_addr, config.paroche.port)?;
    let listener = tokio::net::TcpListener::bind(addr)
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

    // Wait for the syntaxis event listener to drain (layer 2, after layer 4)
    if let Err(e) = syntaxis_handle.await {
        tracing::warn!(error = %e, "syntaxis event listener panicked during shutdown");
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
    db: sqlx::SqlitePool,
) -> ScrobbleClient {
    let mut builder = ScrobbleClientBuilder::new(event_tx.clone(), db)
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
    service: Arc<ScrobbleClient>,
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

/// Parse `listen_addr` as a bare IP and pair it with `port`.
///
/// Failing loudly here replaces the old `format!("{listen_addr}:{port}").parse()`
/// round-trip, which mangled IPv6 literals (`::` became the unparseable `:::4433`)
/// and silently fell back to binding 0.0.0.0 on every parse failure.
fn resolve_listen_addr(listen_addr: &str, port: u16) -> Result<std::net::SocketAddr, HostError> {
    let ip = listen_addr
        .parse::<std::net::IpAddr>()
        .context(ListenAddrSnafu { addr: listen_addr })
        .inspect_err(|_| {
            tracing::error!(
                listen_addr,
                "listen_addr does not parse as an IP address; refusing to bind"
            );
        })?;
    Ok(std::net::SocketAddr::new(ip, port))
}

/// Builds the Cloudflare-bypass proxy from config.
///
/// WHY: fail loud on `cloudflare_bypass_enabled` without a proxy URL —
/// silently falling back to `NoProxy` would turn every cf_bypass indexer
/// into a permanent `NoCfBypass` failure at search time.
fn build_cf_proxy(
    config: &horismos::SearchSubsystemConfig,
) -> Result<Arc<dyn CloudflareProxy>, HostError> {
    if !config.cloudflare_bypass_enabled {
        return Ok(Arc::new(NoProxy));
    }
    let url = config
        .cf_proxy_url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .ok_or_else(|| HostError::Config {
            source: horismos::HorismosError::Validation {
                message: "zetesis.cloudflare_bypass_enabled requires zetesis.cf_proxy_url"
                    .to_string(),
                location: snafu::location!(),
            },
            location: snafu::location!(),
        })?;
    Ok(Arc::new(ByparrProxy::new(
        url.to_string(),
        std::time::Duration::from_secs(config.cf_proxy_timeout_seconds),
    )))
}

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
mod service_adapter_tests {
    use std::sync::Arc;

    use apotheke::migrate::MIGRATOR;
    use paroche::state::{DynCurationService, DynMetadataResolver, DynQueueManager, ServiceError};
    use sqlx::SqlitePool;
    use syntaxis::QueueManager;
    use themelion::create_event_bus;
    use themelion::ids::{DownloadId, ReleaseId, WantId};

    use super::*;

    async fn migrated_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite opens");
        MIGRATOR.run(&pool).await.expect("migrations run");
        pool
    }

    // ── #470: CurationAdapter delegates to the live kritike service ────────

    #[tokio::test]
    async fn curation_adapter_calls_live_kritike_service() {
        let pool = migrated_pool().await;
        let (event_tx, _) = create_event_bus(64);
        let adapter = CurationAdapter(Arc::new(DefaultCurationService::new(pool, event_tx)));

        let report = adapter
            .health_report()
            .await
            .expect("live kritike health report succeeds on an empty library");
        assert_eq!(report.total_items, 0);

        let decision = adapter
            .check_upgrade_eligibility(themelion::HaveId::new(), 90)
            .await
            .expect("live kritike upgrade check succeeds");
        assert_eq!(
            decision,
            kritike::UpgradeDecision::Skip,
            "a missing have skips per kritike's real logic"
        );
    }

    #[tokio::test]
    async fn curation_adapter_maps_profile_not_found() {
        let pool = migrated_pool().await;
        let (event_tx, _) = create_event_bus(64);
        let adapter = CurationAdapter(Arc::new(DefaultCurationService::new(pool, event_tx)));

        let error = adapter
            .assess_quality(
                themelion::MediaType::Music,
                kritike::QualityMetadata {
                    format: "FLAC_24BIT".to_string(),
                    custom_format_score: 0,
                    profile_id: 999_999,
                    codec: None,
                    bit_depth: None,
                    sample_rate: None,
                    file_size: None,
                    channels: None,
                },
            )
            .await
            .expect_err("a missing profile must not assess");
        assert!(matches!(error, ServiceError::NotFound));
    }

    // ── #468: MetadataAdapter delegates to the live epignosis resolver ─────

    #[tokio::test]
    async fn metadata_adapter_calls_live_epignosis_resolver() {
        let adapter = MetadataAdapter(Arc::new(ProviderBackedResolver::new(
            horismos::EpignosisConfig::default(),
            ProviderCredentials::default(),
        )));

        // WHY: fingerprint_audio is the resolver's only network-free method;
        // its distinctive fpcalc error proves the call reaches the real
        // ProviderBackedResolver, not a stub.
        let error = adapter
            .fingerprint_audio(std::path::PathBuf::from("/nonexistent/track.flac"))
            .await
            .expect_err("fingerprinting without fpcalc fails in the real resolver");
        let ServiceError::Internal(message) = error else {
            panic!("expected the real resolver error to map to Internal");
        };
        assert!(
            message.contains("fpcalc"),
            "the real epignosis error must surface, got: {message}"
        );
    }

    // ── #469: QueueAdapter reaches the live queue and engine ───────────────

    struct RecordingEngine {
        started: std::sync::Mutex<Vec<DownloadId>>,
        cancelled: std::sync::Mutex<Vec<DownloadId>>,
    }

    impl ergasia::DownloadEngine for RecordingEngine {
        async fn start_download(
            &self,
            request: ergasia::DownloadRequest,
        ) -> Result<DownloadId, ergasia::ErgasiaError> {
            self.started.lock().expect("lock").push(request.download_id);
            Ok(request.download_id)
        }

        async fn cancel_download(
            &self,
            download_id: DownloadId,
        ) -> Result<(), ergasia::ErgasiaError> {
            self.cancelled.lock().expect("lock").push(download_id);
            Ok(())
        }

        async fn get_progress(
            &self,
            download_id: DownloadId,
        ) -> Result<ergasia::DownloadProgress, ergasia::ErgasiaError> {
            Ok(ergasia::DownloadProgress {
                download_id,
                state: ergasia::DownloadState::Downloading,
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
        ) -> Result<Option<ergasia::ExtractionResult>, ergasia::ErgasiaError> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn queue_adapter_cancel_stops_live_engine_download() {
        let pool = migrated_pool().await;
        let engine = Arc::new(RecordingEngine {
            started: std::sync::Mutex::new(Vec::new()),
            cancelled: std::sync::Mutex::new(Vec::new()),
        });
        let queue = Arc::new(
            DownloadQueue::new(
                pool,
                Arc::clone(&engine),
                Arc::new(StubImportService),
                horismos::SyntaxisConfig {
                    max_concurrent_downloads: 2,
                    max_per_tracker: 3,
                    retry_count: 1,
                    retry_backoff_base_seconds: 0,
                    stalled_download_timeout_hours: 24,
                },
            )
            .await
            .expect("queue constructs"),
        );

        let queue_id = uuid::Uuid::now_v7();
        queue
            .enqueue(syntaxis::QueueItem {
                id: queue_id,
                want_id: WantId::new(),
                release_id: ReleaseId::new(),
                download_url: "magnet:?xt=urn:btih:adapter".to_string(),
                protocol: syntaxis::DownloadProtocol::Torrent,
                priority: 4,
                tracker_id: None,
                info_hash: None,
                retry_count: 0,
            })
            .await
            .expect("enqueue succeeds");

        // Wait for the spawned dispatch to reach the engine before cancelling.
        for _ in 0..500 {
            if !engine.started.lock().expect("lock").is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let started = engine.started.lock().expect("lock").clone();
        assert_eq!(started.len(), 1, "precondition: the download is live");

        let adapter = QueueAdapter(Arc::clone(&queue));
        adapter
            .cancel(queue_id)
            .await
            .expect("adapter cancel succeeds");

        assert_eq!(
            engine.cancelled.lock().expect("lock").clone(),
            started,
            "the API-facing cancel must stop the live engine download"
        );
    }

    #[tokio::test]
    async fn queue_adapter_maps_unknown_item_to_not_found() {
        let pool = migrated_pool().await;
        let engine = Arc::new(RecordingEngine {
            started: std::sync::Mutex::new(Vec::new()),
            cancelled: std::sync::Mutex::new(Vec::new()),
        });
        let queue = Arc::new(
            DownloadQueue::new(
                pool,
                engine,
                Arc::new(StubImportService),
                horismos::SyntaxisConfig {
                    max_concurrent_downloads: 2,
                    max_per_tracker: 3,
                    retry_count: 1,
                    retry_backoff_base_seconds: 0,
                    stalled_download_timeout_hours: 24,
                },
            )
            .await
            .expect("queue constructs"),
        );

        let adapter = QueueAdapter(queue);
        let error = adapter
            .cancel(uuid::Uuid::now_v7())
            .await
            .expect_err("an unknown id must not succeed");
        assert!(matches!(error, ServiceError::NotFound));
    }

    // ── #499: QueueAdapter enqueue reaches the live queue ──────────────────

    fn enqueue_item(protocol: &str) -> paroche::state::EnqueueItem {
        paroche::state::EnqueueItem {
            queue_id: uuid::Uuid::now_v7(),
            want_id: uuid::Uuid::now_v7(),
            release_id: uuid::Uuid::now_v7(),
            download_url: "magnet:?xt=urn:btih:adapter-enqueue".to_string(),
            protocol: protocol.to_string(),
            priority: 4,
            info_hash: None,
        }
    }

    #[tokio::test]
    async fn queue_adapter_enqueue_persists_and_dispatches_to_live_engine() {
        let pool = migrated_pool().await;
        let engine = Arc::new(RecordingEngine {
            started: std::sync::Mutex::new(Vec::new()),
            cancelled: std::sync::Mutex::new(Vec::new()),
        });
        let queue = Arc::new(
            DownloadQueue::new(
                pool.clone(),
                Arc::clone(&engine),
                Arc::new(StubImportService),
                horismos::SyntaxisConfig {
                    max_concurrent_downloads: 2,
                    max_per_tracker: 3,
                    retry_count: 1,
                    retry_backoff_base_seconds: 0,
                    stalled_download_timeout_hours: 24,
                },
            )
            .await
            .expect("queue constructs"),
        );

        let adapter = QueueAdapter(Arc::clone(&queue));
        let item = enqueue_item("torrent");
        let queue_id = item.queue_id;
        adapter
            .enqueue(item)
            .await
            .expect("adapter enqueue succeeds");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM download_queue WHERE id = ?")
            .bind(queue_id.as_bytes().as_slice())
            .fetch_one(&pool)
            .await
            .expect("row count query");
        assert_eq!(count, 1, "the service must persist the enqueued row");

        // The interactive dispatch is spawned; wait for it to reach the engine.
        for _ in 0..500 {
            if !engine.started.lock().expect("lock").is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(
            engine.started.lock().expect("lock").len(),
            1,
            "the API-facing enqueue must dispatch to the live engine"
        );
    }

    #[tokio::test]
    async fn queue_adapter_enqueue_rejects_unknown_protocol() {
        let pool = migrated_pool().await;
        let engine = Arc::new(RecordingEngine {
            started: std::sync::Mutex::new(Vec::new()),
            cancelled: std::sync::Mutex::new(Vec::new()),
        });
        let queue = Arc::new(
            DownloadQueue::new(
                pool.clone(),
                Arc::clone(&engine),
                Arc::new(StubImportService),
                horismos::SyntaxisConfig {
                    max_concurrent_downloads: 2,
                    max_per_tracker: 3,
                    retry_count: 1,
                    retry_backoff_base_seconds: 0,
                    stalled_download_timeout_hours: 24,
                },
            )
            .await
            .expect("queue constructs"),
        );

        let adapter = QueueAdapter(queue);
        let error = adapter
            .enqueue(enqueue_item("ftp"))
            .await
            .expect_err("an unknown protocol must not enqueue");
        // WHY: user-supplied protocol must map to a 400-class error, never
        // fold into Internal (which the HTTP layer reports as a 500).
        assert!(matches!(error, ServiceError::InvalidInput(_)));

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM download_queue")
            .fetch_one(&pool)
            .await
            .expect("row count query");
        assert_eq!(count, 0, "a rejected protocol must not be persisted");
        assert!(
            engine.started.lock().expect("lock").is_empty(),
            "a rejected protocol must not reach the engine"
        );
    }
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
    fn parse_search_media_type_rejects_unknown_values_as_invalid_input() {
        // WHY: user-supplied media_type must map to a 400-class error, never
        // fold into Internal (which the HTTP layer reports as a 500).
        let error = parse_search_media_type("podcast").expect_err("unsupported media type");
        assert!(matches!(error, ServiceError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn search_adapter_calls_live_zetesis_service() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite opens");
        MIGRATOR.run(&pool).await.expect("migrations run");
        let (event_tx, _) = create_event_bus(64);
        let service = zetesis::SearchIndexerService::new(
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

    async fn test_pools() -> Arc<apotheke::DbPools> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite opens");
        MIGRATOR.run(&pool).await.expect("migrations run");
        Arc::new(apotheke::DbPools {
            read: pool.clone(),
            write: pool,
        })
    }

    #[tokio::test]
    async fn role_of_rejects_inactive_user() {
        let db = test_pools().await;
        let user_id = themelion::UserId::new();
        let user = apotheke::repo::user::User {
            id: user_id.as_bytes().to_vec(),
            username: "dormant".to_string(),
            display_name: "Dormant".to_string(),
            password_hash: "x".to_string(),
            role: "member".to_string(),
            is_active: 0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            last_login_at: None,
        };
        apotheke::repo::user::insert_user(&db.write, &user)
            .await
            .expect("insert user");

        let provider = RequestRoleProvider { db };
        let result = provider.role_of(user_id).await;
        assert!(
            matches!(
                result,
                Err(aitesis::AitesisError::InsufficientPermission { .. })
            ),
            "inactive user must be rejected, got {result:?}"
        );
    }

    fn media_request(media_type: themelion::MediaType) -> aitesis::MediaRequest {
        aitesis::MediaRequest {
            id: themelion::RequestId::new(),
            user_id: themelion::UserId::new(),
            media_type,
            title: "Kind of Blue".to_string(),
            external_id: None,
            status: aitesis::RequestStatus::Approved,
            decided_by: None,
            decided_at: None,
            deny_reason: None,
            want_id: None,
            created_at: jiff::Timestamp::now(),
        }
    }

    #[tokio::test]
    async fn create_want_persists_want_with_first_quality_profile() {
        let db = test_pools().await;
        // WHY: migrations seed default profiles — the monitor must select the
        // first music profile by name, so the expectation derives from the
        // same repo query rather than a hand-seeded row.
        let profile_id = apotheke::repo::quality::list_profiles_for_type(&db.read, "music")
            .await
            .expect("profiles query")
            .into_iter()
            .next()
            .expect("a default music quality profile exists")
            .id;

        let monitor = RequestMonitor {
            db: Arc::clone(&db),
        };
        let request = media_request(themelion::MediaType::Music);
        let want_id = monitor.create_want(&request).await.expect("want created");

        #[derive(sqlx::FromRow)]
        struct WantRow {
            media_type: String,
            title: String,
            quality_profile_id: i64,
            status: String,
            source: Option<String>,
        }
        let row = sqlx::query_as::<_, WantRow>(
            "SELECT media_type, title, quality_profile_id, status, source FROM wants WHERE id = ?",
        )
        .bind(want_id.as_bytes().to_vec())
        .fetch_one(&db.read)
        .await
        .expect("persisted want row");

        assert_eq!(row.media_type, "music_album");
        assert_eq!(row.title, "Kind of Blue");
        assert_eq!(row.quality_profile_id, profile_id);
        assert_eq!(row.status, "searching");
        assert_eq!(row.source.as_deref(), Some("request"));
    }

    #[tokio::test]
    async fn create_want_rejects_news_media_type() {
        let db = test_pools().await;
        let monitor = RequestMonitor { db };
        let request = media_request(themelion::MediaType::News);
        let result = monitor.create_want(&request).await;
        assert!(
            matches!(
                result,
                Err(aitesis::AitesisError::MediaIdentityInvalid { .. })
            ),
            "news must be rejected, got {result:?}"
        );
    }

    #[test]
    fn request_media_types_maps_each_variant() {
        assert_eq!(
            request_media_types(themelion::MediaType::Music),
            Some(("music_album", "music"))
        );
        assert_eq!(
            request_media_types(themelion::MediaType::Audiobook),
            Some(("audiobook", "audiobook"))
        );
        assert_eq!(
            request_media_types(themelion::MediaType::Book),
            Some(("book", "book"))
        );
        assert_eq!(
            request_media_types(themelion::MediaType::Comic),
            Some(("comic", "comic"))
        );
        assert_eq!(
            request_media_types(themelion::MediaType::Podcast),
            Some(("podcast", "podcast"))
        );
        assert_eq!(
            request_media_types(themelion::MediaType::Movie),
            Some(("movie", "movie"))
        );
        assert_eq!(
            request_media_types(themelion::MediaType::Tv),
            Some(("tv_series", "tv"))
        );
        assert_eq!(request_media_types(themelion::MediaType::News), None);
    }

    fn config_with_download_dir(dir: PathBuf) -> horismos::Config {
        let mut config = horismos::Config::default();
        config.ergasia.download_dir = dir;
        config
    }

    #[test]
    fn validate_download_dir_rejects_missing_dir() {
        let config = config_with_download_dir(PathBuf::from("/nonexistent/harmonia-dl"));
        let error = validate_download_dir(&config).expect_err("missing dir must fail");
        assert!(error.to_string().contains("does not exist"), "{error}");
    }

    #[test]
    fn validate_download_dir_rejects_unwritable_dir() {
        use std::os::unix::fs::PermissionsExt;

        // WHY: root bypasses permission bits — the assertion would be
        // meaningless, so the case is skipped for uid 0.
        let dir = tempfile::TempDir::new().expect("tempdir");
        let probe = dir.path().join(".probe");
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555))
            .expect("chmod");
        if std::fs::write(&probe, b"").is_ok() {
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755))
                .expect("chmod back");
            return; // running as root (or an ACL grants write) — not testable
        }

        let config = config_with_download_dir(dir.path().to_path_buf());
        let error = validate_download_dir(&config).expect_err("unwritable dir must fail");
        assert!(error.to_string().contains("not writable"), "{error}");

        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755))
            .expect("chmod back for cleanup");
    }

    #[test]
    fn validate_download_dir_accepts_writable_dir_and_cleans_up() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let config = config_with_download_dir(dir.path().to_path_buf());
        validate_download_dir(&config).expect("writable dir passes");
        assert!(
            !dir.path().join(".harmonia-write-test").exists(),
            "the write-probe file must be cleaned up"
        );
    }

    #[test]
    fn resolve_listen_addr_accepts_ipv4() {
        let addr = resolve_listen_addr("0.0.0.0", 4433).expect("ipv4 wildcard parses");
        assert_eq!(addr, "0.0.0.0:4433".parse().expect("expected addr"));
    }

    #[test]
    fn resolve_listen_addr_accepts_ipv6_wildcard() {
        let addr = resolve_listen_addr("::", 4433).expect("ipv6 wildcard parses");
        assert_eq!(addr, "[::]:4433".parse().expect("expected addr"));
        assert!(addr.is_ipv6());
    }

    #[test]
    fn resolve_listen_addr_accepts_ipv6_loopback() {
        let addr = resolve_listen_addr("::1", 8096).expect("ipv6 loopback parses");
        assert_eq!(addr, "[::1]:8096".parse().expect("expected addr"));
    }

    #[test]
    fn resolve_listen_addr_rejects_garbage_and_errors() {
        for bad in ["not-an-ip", "", "0.0.0.0:9999", "example.com", "[::]"] {
            let result = resolve_listen_addr(bad, 4433);
            assert!(
                matches!(result, Err(HostError::ListenAddr { .. })),
                "expected ListenAddr error for {bad:?}"
            );
        }
    }

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

    // ── Cloudflare-bypass proxy wiring ──────────────────────────────────────

    #[tokio::test]
    async fn build_cf_proxy_disabled_returns_no_proxy() {
        let config = horismos::SearchSubsystemConfig::default();
        assert!(!config.cloudflare_bypass_enabled);

        let proxy = build_cf_proxy(&config).expect("disabled bypass builds NoProxy");
        let err = proxy
            .get("https://example.com", CancellationToken::new())
            .await
            .expect_err("NoProxy always errors");
        assert!(matches!(
            err,
            zetesis::SearchIndexerError::NoCfBypass { .. }
        ));
    }

    #[tokio::test]
    async fn build_cf_proxy_enabled_without_url_errors() {
        let config = horismos::SearchSubsystemConfig {
            cloudflare_bypass_enabled: true,
            cf_proxy_url: None,
            ..horismos::SearchSubsystemConfig::default()
        };

        let Err(err) = build_cf_proxy(&config) else {
            panic!("enabled bypass without URL must fail loud");
        };
        assert!(matches!(err, HostError::Config { .. }));
        assert!(err.to_string().contains("cf_proxy_url"));
    }

    #[tokio::test]
    async fn build_cf_proxy_enabled_with_blank_url_errors() {
        let config = horismos::SearchSubsystemConfig {
            cloudflare_bypass_enabled: true,
            cf_proxy_url: Some("   ".to_string()),
            ..horismos::SearchSubsystemConfig::default()
        };

        let Err(err) = build_cf_proxy(&config) else {
            panic!("blank URL must fail loud");
        };
        assert!(err.to_string().contains("cf_proxy_url"));
    }

    #[tokio::test]
    async fn build_cf_proxy_enabled_posts_to_byparr_endpoint() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // One-shot Byparr stub: answers a single POST /v1 with a solved page.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind stub listener");
        let endpoint = format!("http://{}", listener.local_addr().expect("local addr"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = vec![0u8; 8192];
            let n = stream.read(&mut buf).await.expect("read request head");
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            let body = r#"{"status":"ok","message":"solved","solution":{"url":"https://example.com","status":200,"response":"<html>ok</html>","cookies":[],"userAgent":"UA"}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write response");
            request
        });

        let config = horismos::SearchSubsystemConfig {
            cloudflare_bypass_enabled: true,
            cf_proxy_url: Some(endpoint),
            cf_proxy_timeout_seconds: 5,
            ..horismos::SearchSubsystemConfig::default()
        };

        let proxy = build_cf_proxy(&config).expect("enabled bypass builds ByparrProxy");
        let response = proxy
            .get("https://cf-protected.example.com", CancellationToken::new())
            .await
            .expect("Byparr stub answers");

        assert_eq!(response.status, 200);
        assert_eq!(response.body, "<html>ok</html>");
        let request = server.await.expect("stub request captured");
        assert!(
            request.starts_with("POST /v1"),
            "expected POST /v1, got: {}",
            request.lines().next().unwrap_or_default()
        );
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
