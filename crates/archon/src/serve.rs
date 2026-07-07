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
use horismos::{ConfigManager, ConfigOverrides, ReloadOutcome};
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
use zetesis::CardigannRegistry;
use zetesis::SearchIndexerService;
use zetesis::cf_bypass::CloudflareProxy;
use zetesis::cf_bypass::byparr::ByparrProxy;
use zetesis::cf_bypass::noop::NoProxy;

use crate::cli::ServeArgs;
use crate::error::{
    ConfigSnafu, DatabaseSnafu, DownloadEngineSnafu, DownloadQueueSnafu, FeedSchedulerSnafu,
    HostError, ListenAddrSnafu, ReloadTaskPanickedSnafu, ScannerSnafu, ServerSnafu,
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

/// Swappable behind a std `RwLock` (never held across an .await — every
/// method snapshots via `snapshot()`, which clones the Arc and drops the
/// guard before any async work) so a `epignosis.*` config change can rebuild
/// the resolver and swap it in without a service rebuild (#529 step 8).
/// In-flight resolutions already hold their own Arc clone (taken before the
/// call's await) and finish on the OLD instance undisturbed.
struct MetadataAdapter {
    inner: std::sync::RwLock<Arc<ProviderBackedResolver>>,
}

impl MetadataAdapter {
    fn new(resolver: Arc<ProviderBackedResolver>) -> Self {
        Self {
            inner: std::sync::RwLock::new(resolver),
        }
    }

    fn snapshot(&self) -> Arc<ProviderBackedResolver> {
        let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
        Arc::clone(&guard)
    }

    /// Swaps the live resolver. Called by archon's epignosis supervisor
    /// after a rebuild on an `epignosis.*` change.
    fn set_resolver(&self, new: Arc<ProviderBackedResolver>) {
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        *guard = new;
    }
}

impl DynMetadataResolver for MetadataAdapter {
    fn resolve_identity(
        &self,
        item: epignosis::UnidentifiedItem,
    ) -> ServiceFut<epignosis::MediaIdentity> {
        use epignosis::MetadataResolver as _;
        let service = self.snapshot();
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
        let service = self.snapshot();
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
        let service = self.snapshot();
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
        caller_id: themelion::UserId,
    ) -> RequestServiceFut<'_, aitesis::MediaRequest> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .get_request(request_id, caller_id)
                .await
                .map_err(Into::into)
        })
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

    // WHY: compensation for an approval that lost the request-status
    // compare-and-swap — the want must not outlive the refused request.
    // delete_want now returns NotFound on a zero-row match, but the trait's
    // idempotency invariant requires retracting an already-absent want to be a
    // no-op success, so NotFound is mapped to Ok here (a normal caller still
    // gets the stricter NotFound).
    async fn remove_want(
        &self,
        _request: &aitesis::MediaRequest,
        want_id: themelion::WantId,
    ) -> Result<(), aitesis::AitesisError> {
        match apotheke::repo::want::delete_want(&self.db.write, want_id.as_bytes()).await {
            Ok(()) | Err(apotheke::DbError::NotFound { .. }) => Ok(()),
            Err(source) => Err(source).context(aitesis::error::DatabaseSnafu),
        }
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

/// Swappable behind a std `RwLock` (never held across an .await) so the
/// #529 step-8 syndesmos supervisor can swap in a rebuilt client on a
/// `syndesmos.*` change. `DynExternalIntegration` is currently marker-only
/// (no forwarding methods) — the real consumer of `ScrobbleClient`'s
/// behavior is the event handler task, which holds its own Arc clone
/// directly; this field keeps `AppState`'s view in sync for when a future
/// `DynExternalIntegration` method needs the live client.
struct ExternalAdapter {
    inner: std::sync::RwLock<Arc<ScrobbleClient>>,
}

impl ExternalAdapter {
    fn new(client: Arc<ScrobbleClient>) -> Self {
        Self {
            inner: std::sync::RwLock::new(client),
        }
    }

    /// Swaps the live scrobble client. Called by archon's syndesmos
    /// supervisor after a rebuild on a `syndesmos.*` change.
    fn set_client(&self, new: Arc<ScrobbleClient>) {
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        *guard = new;
    }
}

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
    // WHY: a `Section` (not a frozen `ExtractionLimits`) — #529 step 7 makes
    // `ergasia.max_extraction_depth`/`max_decompression_ratio` live: `extract`
    // builds fresh limits FROM a snapshot taken per call.
    extraction_config: horismos::Section<horismos::ErgasiaConfig>,
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
        let limits = ergasia::ExtractionLimits::from(&self.extraction_config.get());
        ergasia::extract_archives(download_path, output_dir, limits).await
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
    let overrides = ConfigOverrides {
        listen_addr: args.listen.clone(),
        port: args.port,
    };
    let (config_manager, config_handle) =
        ConfigManager::new(config.clone(), config_path, overrides);

    // SIGHUP handler for config reload
    let manager_for_reload = config_manager.clone();
    let handle_for_reload = config_handle.clone();
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
                match reload_config(manager_for_reload.clone()).await {
                    Ok(outcome) => log_reload_outcome(&outcome, &handle_for_reload),
                    Err(e) => {
                        tracing::error!("config reload failed: {e}  -  keeping current config");
                    }
                }
            }
        }
        .instrument(tracing::info_span!("sighup_handler")),
    );

    let boot_config = Arc::new(config);

    // 4. Create database pools
    let db_path = boot_config.database.db_path.to_string_lossy();
    let db = Arc::new(
        init_pools(
            &db_path,
            boot_config.database.read_pool_size,
            boot_config.database.write_pool_max,
        )
        .await
        .context(DatabaseSnafu)?,
    );

    // 5. Create Aggelia event bus
    let (event_tx, _event_rx) = create_event_bus(boot_config.aggelia.buffer_size);

    // 6. Create auth service
    // WHY: a Section (not the frozen boot_config) — #529 step 3 makes JWT
    // secret/TTL live: rotation invalidates outstanding bearers immediately,
    // sessions (opaque refresh tokens) survive.
    let auth = Arc::new(ExousiaServiceImpl::new(
        db.clone(),
        config_handle.section(|c| &c.exousia),
    ));

    // 7. First-run admin setup
    ensure_admin_user(&auth, &db, out).await?;

    // 8. Create metadata resolver
    // WHY: swappable behind a MetadataAdapter — #529 step 8 makes epignosis
    // LIVE-B: an epignosis.* change rebuilds the resolver FROM the new
    // config and swaps it; in-flight resolutions finish on the old
    // instance, and the metadata cache resets with the rebuild (logged by
    // the supervisor below).
    let metadata_service = Arc::new(ProviderBackedResolver::new(
        boot_config.epignosis.clone(),
        ProviderCredentials::from(&boot_config.epignosis),
    ));
    let metadata_adapter = Arc::new(MetadataAdapter::new(Arc::clone(&metadata_service)));

    // 9. Create curation service
    // WHY: a Section + LiveGate (not a frozen Semaphore) — #529 step 8 makes
    // `kritike.quality_check_concurrency` live: `assess_quality` re-reads
    // the limit on every admission decision.
    let curation_service = Arc::new(DefaultCurationService::new(
        db.read.clone(),
        event_tx.clone(),
        config_handle.section(|c| &c.kritike),
    ));

    // WHY: created here — earlier than the "Acquisition subsystem startup"
    // block below WHERE it lived pre-#529 — so the step-6 rebuild
    // supervisors spawned by steps 10-11 can take a child token; every
    // consumer further down still gets its own child token exactly as before.
    let shutdown_token = CancellationToken::new();

    // #529 step 8: epignosis is REBUILD-class — a spawned supervisor owns
    // the live resolver FROM here; an epignosis.* config change rebuilds +
    // swaps via MetadataAdapter (no explicit teardown — the resolver has no
    // shutdown hook; its cache-eviction sweeper holds only a Weak reference
    // and exits on its own once the last Arc drops).
    let epignosis_supervisor = tokio::spawn(
        run_epignosis_supervisor(
            Arc::clone(&metadata_adapter),
            config_handle.clone(),
            shutdown_token.child_token(),
        )
        .instrument(tracing::info_span!("epignosis_supervisor")),
    );

    // 10. Start scanner  -  background task. #529 step 6: a spawned
    // supervisor owns it FROM here — a `taxis.*` config change tears the
    // scanner down and rebuilds it FROM the new config; process shutdown
    // performs the final teardown (replaces the old direct
    // `scanner.shutdown().await` call in cleanup below).
    let scanner = ScannerManager::start(&boot_config.taxis, event_tx.clone())
        .await
        .context(ScannerSnafu)?;
    let scanner_supervisor = tokio::spawn(
        run_scanner_supervisor(
            scanner,
            boot_config.taxis.clone(),
            config_handle.clone(),
            event_tx.clone(),
            shutdown_token.child_token(),
        )
        .instrument(tracing::info_span!("scanner_supervisor")),
    );

    // 11. Start feed scheduler  -  background task. #529 step 6: a spawned
    // supervisor owns it FROM here — a `komide.*` config change aborts the
    // old poll tasks and rebuilds the bounded client + service + scheduler
    // together (`FeedSchedulerService` owns its own `KomideConfig`, so
    // rebuilding the scheduler alone would leave it reading stale config);
    // process shutdown performs the final teardown (replaces the old direct
    // `feed_scheduler.shutdown()` call in cleanup below).
    let db_pools = clone_db_pools(&db);
    let feed_scheduler = start_feed_scheduler(&boot_config.komide, &event_tx, &db_pools).await?;
    let feed_supervisor = tokio::spawn(
        run_feed_supervisor(
            feed_scheduler,
            boot_config.komide.clone(),
            config_handle.clone(),
            event_tx.clone(),
            db_pools,
            shutdown_token.child_token(),
        )
        .instrument(tracing::info_span!("feed_supervisor")),
    );

    // ── Pre-flight: acquisition config validation ─────────────────────────
    validate_download_dir(&boot_config)?;

    // ── Acquisition subsystem startup ───────────────────────────────────────

    // Layer 0: Zetesis (indexer protocol)
    let cf_proxy = build_cf_proxy(&boot_config.zetesis)?;
    // WHY: a `Section` (not the frozen boot_config) — #529 step 7 makes the
    // per-op fields live; the zetesis supervisor below handles the
    // remaining LIVE-B mechanisms (rate limiter, cf proxy, cardigann).
    let zetesis = Arc::new(SearchIndexerService::new(
        db.read.clone(),
        db.write.clone(),
        cf_proxy,
        config_handle.section(|c| &c.zetesis),
        event_tx.clone(),
    ));
    info!(
        cloudflare_bypass = boot_config.zetesis.cloudflare_bypass_enabled,
        "zetesis (indexer search) initialized"
    );
    let zetesis_supervisor = tokio::spawn(
        run_zetesis_supervisor(
            Arc::clone(&zetesis),
            boot_config.zetesis.clone(),
            config_handle.clone(),
            shutdown_token.child_token(),
        )
        .instrument(tracing::info_span!("zetesis_supervisor")),
    );

    // Layer 1: Ergasia (download execution)
    let ergasia_session = Arc::new(
        TorrentSession::new(&boot_config.ergasia)
            .await
            .context(DownloadEngineSnafu)?,
    );
    info!("ergasia (download engine) initialized");

    // Layer 2: Syntaxis (queue orchestration, depends on ergasia)
    // WHY: a `Section` (not the frozen boot_config) — #529 step 7 makes
    // `ergasia.max_extraction_depth`/`max_decompression_ratio` live.
    let engine_adapter = Arc::new(SessionEngine {
        session: Arc::clone(&ergasia_session),
        extraction_config: config_handle.section(|c| &c.ergasia),
    });
    let syntaxis_svc = Arc::new(
        DownloadQueue::new(
            db.write.clone(),
            engine_adapter,
            Arc::new(StubImportService),
            boot_config.syntaxis.clone(),
        )
        .await
        .context(DownloadQueueSnafu)?,
    );
    let syntaxis_handle = syntaxis_svc.start(event_tx.subscribe(), shutdown_token.child_token());
    info!("syntaxis (download queue) initialized  -  event listener started");
    // WHY: `retry_count`/`retry_backoff_base_seconds`/the `SlotAllocator`
    // limits go live via `DownloadQueue::update_config` (#529 step 7) — a
    // `syntaxis.*` change is applied in place, never rebuilt.
    let syntaxis_supervisor = tokio::spawn(
        run_syntaxis_supervisor(
            Arc::clone(&syntaxis_svc),
            config_handle.clone(),
            shutdown_token.child_token(),
        )
        .instrument(tracing::info_span!("syntaxis_supervisor")),
    );

    // Layer 4: Syndesmos (external integrations  -  Plex, Last.fm, Tidal)
    // WHY: REBUILD-class (#529 step 8) — a `syndesmos.*` change cancels the
    // event handler, rebuilds the client, respawns the handler, and swaps
    // ExternalAdapter's inner Arc. Two honest costs, logged by the
    // supervisor: circuit breakers reset (fresh breakers), and events
    // published between cancel and re-subscribe are lost (a bounded
    // scrobble-loss window).
    let syndesmos_svc = Arc::new(build_syndesmos(&boot_config, &event_tx, db.read.clone()));
    let external_adapter = Arc::new(ExternalAdapter::new(Arc::clone(&syndesmos_svc)));
    let syndesmos_ct = shutdown_token.child_token();
    let syndesmos_handle = spawn_syndesmos_handler(
        Arc::clone(&syndesmos_svc),
        event_tx.subscribe(),
        syndesmos_ct.clone(),
    );
    let syndesmos_supervisor = tokio::spawn(
        run_syndesmos_supervisor(
            Arc::clone(&external_adapter),
            config_handle.clone(),
            event_tx.clone(),
            db.read.clone(),
            SyndesmosGeneration {
                ct: syndesmos_ct,
                handle: syndesmos_handle,
            },
            shutdown_token.child_token(),
        )
        .instrument(tracing::info_span!("syndesmos_supervisor")),
    );
    info!("syndesmos (external integrations) initialized  -  event listener started");

    // Layer 4: Prostheke (subtitle management)
    // WHY: a Section (not the frozen boot_config) — #529 step 8 makes the
    // per-op language/score preferences live; the prostheke supervisor below
    // handles the provider-set LIVE-B mechanism (the OpenSubtitles rate
    // limiter resets with the provider — logged, not silently accepted).
    let providers = Provider::default_providers(boot_config.prostheke.opensubtitles.clone());
    let prostheke_svc = Arc::new(SubtitleManager::new(
        db.read.clone(),
        db.write.clone(),
        config_handle.section(|c| &c.prostheke),
        providers,
        event_tx.clone(),
    ));
    info!("prostheke (subtitles) initialized");
    let prostheke_supervisor = tokio::spawn(
        run_prostheke_supervisor(
            Arc::clone(&prostheke_svc),
            config_handle.clone(),
            shutdown_token.child_token(),
        )
        .instrument(tracing::info_span!("prostheke_supervisor")),
    );

    // Layer 5: Aitesis (household request workflow)
    // WHY: a Section (not the frozen boot_config) — #529 step 8 makes
    // per-user/per-day request limits and auto-approve live; no supervisor
    // needed — `submit_request` already reads a fresh snapshot per call.
    let request_service = Arc::new(aitesis::AitesisServiceImpl::new(
        db.read.clone(),
        db.write.clone(),
        config_handle.section(|c| &c.aitesis),
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
        &boot_config.paroche.listen_addr,
        boot_config.paroche.renderer_quic_port,
    )?;
    // WHY: the full ConfigHandle (not the frozen boot_config) — #529 steps
    // 4+5 make the renderer server fully live: api key / admission cap /
    // handshake timeout are read per-operation, and a
    // (listen_addr, renderer_quic_port) change rebinds the endpoint
    // make-before-break while established sessions drain with no bound.
    let renderer_config = config_handle.clone();
    let renderer_registry_for_quic = Arc::clone(&renderer_registry);
    let renderer_shutdown = shutdown_token.child_token();
    tokio::spawn(
        async move {
            if let Err(e) = crate::render::server::start_renderer_server(
                renderer_addr,
                &renderer_cert_dir,
                renderer_registry_for_quic,
                renderer_shutdown,
                renderer_config,
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
        config: config_handle.clone(),
        event_tx,
        auth,
        import,
        metadata: metadata_adapter,
        curation: Arc::new(CurationAdapter(curation_service)),
        search: Arc::new(SearchAdapter(zetesis)),
        download_engine: Arc::new(EngineAdapter(ergasia_session)),
        queue: Arc::new(QueueAdapter(Arc::clone(&syntaxis_svc))),
        requests: Arc::new(RequestAdapter(request_service)),
        external: external_adapter,
        subtitles,
        renderers: renderer_registry,
    };
    let router = paroche::build_router(state);

    // 14. Bind + serve — the #529 step-5 supervisor rebinds the listener
    // live on a (paroche.listen_addr, paroche.port) change. The STARTUP bind
    // stays fatal: a server that cannot bind its configured address at boot
    // must not come up half-alive.
    let addr = resolve_listen_addr(&boot_config.paroche.listen_addr, boot_config.paroche.port)?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context(ServerSnafu)?;

    // 15. Graceful shutdown — fanned out through a token so every HTTP
    // generation (active or still draining after a rebind) observes the same
    // process signal.
    let http_shutdown = CancellationToken::new();
    let signal_ct = http_shutdown.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        signal_ct.cancel();
    });
    run_http_supervisor(listener, addr, router, config_handle, http_shutdown).await?;

    // 16. Cleanup  -  reverse startup ORDER
    info!("shutting down subsystems");

    // Cancel all acquisition background tasks (syndesmos event handler, syntaxis listener)
    shutdown_token.cancel();

    // #529 step 8: the syndesmos supervisor now owns the event handler's
    // full lifecycle (its own cancel + await is internal to it); joining the
    // supervisor here replaces the old direct `syndesmos_handle.await`.
    if let Err(e) = syndesmos_supervisor.await {
        tracing::warn!(error = %e, "syndesmos supervisor panicked during shutdown");
    }

    // Wait for the syntaxis event listener to drain (layer 2, after layer 4)
    if let Err(e) = syntaxis_handle.await {
        tracing::warn!(error = %e, "syntaxis event listener panicked during shutdown");
    }

    // #529 step 7: the zetesis and syntaxis LIVE-B supervisors exit on the
    // same shutdown_token child their construction site was handed; joining
    // them here is symmetric with the step-6 rebuild supervisors below.
    if let Err(e) = syntaxis_supervisor.await {
        tracing::warn!(error = %e, "syntaxis supervisor panicked during shutdown");
    }
    if let Err(e) = zetesis_supervisor.await {
        tracing::warn!(error = %e, "zetesis supervisor panicked during shutdown");
    }

    // #529 step 8: prostheke and epignosis are REBUILD-class with no
    // explicit teardown call (neither `SubtitleManager` nor
    // `ProviderBackedResolver` has a shutdown hook) — joining their
    // supervisors here just confirms they observed the same shutdown signal
    // as everything else, symmetric with the step-6/7 supervisors.
    if let Err(e) = prostheke_supervisor.await {
        tracing::warn!(error = %e, "prostheke supervisor panicked during shutdown");
    }
    if let Err(e) = epignosis_supervisor.await {
        tracing::warn!(error = %e, "epignosis supervisor panicked during shutdown");
    }

    // Shutdown core subsystems (reverse of startup) — the #529 step 6
    // supervisors perform the actual teardown internally once their child
    // token (a child of `shutdown_token`, cancelled above) observes the
    // signal; joining them here replaces the old direct `feed_scheduler
    // .shutdown()` / `scanner.shutdown().await` calls.
    if let Err(e) = feed_supervisor.await {
        tracing::warn!(error = %e, "feed supervisor panicked during shutdown");
    }
    if let Err(e) = scanner_supervisor.await {
        tracing::warn!(error = %e, "scanner supervisor panicked during shutdown");
    }

    info!("shutdown complete");
    Ok(())
}

// ── Config reload (SIGHUP) ───────────────────────────────────────────────────

/// Re-reads and re-publishes configuration through `manager`.
///
/// Extracted from the SIGHUP handler so tests can drive a reload without
/// sending a real signal, asserting on the returned `ReloadOutcome` directly
/// rather than scraping log output.
///
/// WHY: `ConfigManager::reload()` does blocking file I/O (figment TOML
/// read); `spawn_blocking` keeps it off the async worker thread.
pub(crate) async fn reload_config(manager: ConfigManager) -> Result<ReloadOutcome, HostError> {
    tokio::task::spawn_blocking(move || manager.reload())
        .await
        .context(ReloadTaskPanickedSnafu)?
        .context(ConfigSnafu)
}

/// Logs a `ReloadOutcome` honestly: live changes actually applied, changes
/// held back until a restart, or neither. `config` supplies the post-publish
/// effective value for change-specific logging (e.g. detecting a cleared
/// renderer key) — `ReloadOutcome` itself carries only dotted paths, not values.
fn log_reload_outcome(outcome: &ReloadOutcome, config: &horismos::ConfigHandle) {
    for w in &outcome.warnings {
        tracing::warn!(field = %w.field, "config reload: {}", w.message);
    }
    if !outcome.applied.is_empty() {
        let n = outcome.applied.len();
        tracing::info!(
            applied = ?outcome.applied,
            "configuration reloaded — {n} live change(s) applied"
        );
    }
    // WHY: jwt_secret rotation is a distinct security event from an ordinary
    // live-config change — it kills every outstanding bearer immediately (no
    // dual-secret grace, operator-locked semantics, #529 step 3).
    if outcome
        .applied
        .iter()
        .any(|path| path == "exousia.jwt_secret")
    {
        tracing::warn!(
            "exousia.jwt_secret rotated — all outstanding access tokens are now invalid"
        );
    }
    // WHY: a reload that CLEARS the renderer key must reproduce the
    // boot-time fail-closed heads-up (render/server.rs's
    // `renderer_api_key not configured` warn) — an operator who blanks the
    // key deserves the same signal immediately, not a slow discovery via a
    // stream of per-connection auth-failure warnings later (#529 step 4).
    // A rotation to a DIFFERENT non-empty key needs no extra warn — new
    // registrations simply start using it.
    if outcome
        .applied
        .iter()
        .any(|path| path == "paroche.renderer_api_key")
        && config
            .current()
            .paroche
            .renderer_api_key
            .as_deref()
            .is_none_or(str::is_empty)
    {
        tracing::warn!(
            "paroche.renderer_api_key not configured; rejecting every renderer registration \
             until a key is SET"
        );
    }
    if !outcome.restart_pending.is_empty() {
        let n = outcome.restart_pending.len();
        tracing::warn!(
            pending = ?outcome.restart_pending,
            "config reload: {n} change(s) require restart and were held back"
        );
    }
    if outcome.is_unchanged() {
        tracing::info!("configuration reloaded — no changes");
    }
}

// ── HTTP listener supervisor (#529 step 5) ──────────────────────────────────

/// Bounded backoff shared by the HTTP and renderer-QUIC rebind fallbacks:
/// after a failed make-before-break bind, the retiring listener's socket is
/// released and the new bind is retried this many times, this far apart.
pub(crate) const REBIND_RETRY_ATTEMPTS: u32 = 5;
pub(crate) const REBIND_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(200);

/// One live HTTP listener generation: the serve task plus the token that
/// triggers ITS graceful drain only.
struct HttpGeneration {
    addr: std::net::SocketAddr,
    drain_ct: CancellationToken,
    task: JoinHandle<Result<(), std::io::Error>>,
}

/// Spawns one `axum::serve` generation on `listener`. Its graceful-shutdown
/// trigger is a child of the process token, so process shutdown drains the
/// active generation through the same path a rebind retirement uses.
fn spawn_http_generation(
    listener: tokio::net::TcpListener,
    addr: std::net::SocketAddr,
    router: axum::Router,
    process_shutdown: &CancellationToken,
) -> HttpGeneration {
    let drain_ct = process_shutdown.child_token();
    let drain = drain_ct.clone();
    let serve = axum::serve(listener, router)
        .with_graceful_shutdown(async move { drain.cancelled().await });
    let task = tokio::spawn(async move { serve.await });
    info!("Harmonia serving on {addr}");
    HttpGeneration {
        addr,
        drain_ct,
        task,
    }
}

/// Stops accepting on a retiring generation (axum drops the listener socket
/// as soon as the drain trigger fires) and awaits its in-flight
/// requests/WebSockets with NO bound, logging when the drain finishes.
fn retire_http(old: HttpGeneration) {
    let HttpGeneration {
        addr,
        drain_ct,
        task,
    } = old;
    drain_ct.cancel();
    info!(addr = %addr, "old HTTP listener retired — draining in-flight requests");
    tokio::spawn(async move {
        match task.await {
            Ok(Ok(())) => info!(addr = %addr, "old HTTP listener drained"),
            Ok(Err(e)) => {
                tracing::warn!(
                    addr = %addr,
                    error = %e,
                    "old HTTP listener ended with an error while draining"
                );
            }
            Err(e) => {
                tracing::warn!(addr = %addr, error = %e, "old HTTP listener drain task panicked");
            }
        }
    });
}

/// Runs the HTTP listener under a live-rebind supervisor.
///
/// On a `(paroche.listen_addr, paroche.port)` change it binds the NEW
/// listener first (make-before-break), then gracefully drains the old
/// generation with no bound. Returns once process shutdown has drained the
/// active generation.
///
/// CLI `--listen`/`--port` pins need no special case here: `ConfigOverrides`
/// re-pins them on every publish, so a pinned field never surfaces as a
/// watcher change.
async fn run_http_supervisor(
    listener: tokio::net::TcpListener,
    addr: std::net::SocketAddr,
    router: axum::Router,
    config: horismos::ConfigHandle,
    shutdown: CancellationToken,
) -> Result<(), HostError> {
    // WHY: the watcher precedes the boot snapshot — a publish landing between
    // the two is either already in the snapshot or surfaces as a watcher
    // event, so no listener-address change can slip through unobserved.
    let mut watcher = config.watch_section(|c| &c.paroche);
    let boot = config.current();
    let mut target = (boot.paroche.listen_addr.clone(), boot.paroche.port);
    let mut generation = Some(spawn_http_generation(
        listener,
        addr,
        router.clone(),
        &shutdown,
    ));

    loop {
        let changed = tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            changed = watcher.changed() => changed,
        };
        let Some(cfg) = changed else {
            // The config owner is gone: no further publishes can arrive, so
            // the active generation serves unchanged until process shutdown.
            shutdown.cancelled().await;
            break;
        };
        let new_target = (cfg.listen_addr.clone(), cfg.port);
        if new_target == target {
            continue;
        }
        let new_addr = match resolve_listen_addr(&new_target.0, new_target.1) {
            Ok(a) => a,
            Err(e) => {
                // Fail-safe: a bad new value never tears down a working
                // listener.
                tracing::error!(
                    listen_addr = %new_target.0,
                    port = new_target.1,
                    error = %e,
                    "HTTP rebind: new listen address is unusable; keeping the current listener"
                );
                continue;
            }
        };

        // Make-before-break: the new listener binds while the old one still
        // accepts, so a working reload has zero downtime.
        match tokio::net::TcpListener::bind(new_addr).await {
            Ok(new_listener) => {
                let new_generation =
                    spawn_http_generation(new_listener, new_addr, router.clone(), &shutdown);
                if let Some(old) = generation.replace(new_generation) {
                    retire_http(old);
                }
                target = new_target;
            }
            Err(e) => {
                tracing::error!(
                    addr = %new_addr,
                    error = %e,
                    "HTTP rebind: make-before-break bind failed; falling back to \
                     break-before-make"
                );
                // Break: retire the old generation — axum drops its listener
                // socket on the drain trigger, freeing the address overlap a
                // wildcard/specific conflict needs before the retry can
                // succeed.
                let old_addr = generation.take().map(|old| {
                    let addr = old.addr;
                    retire_http(old);
                    addr
                });
                let mut bound = None;
                for attempt in 1..=REBIND_RETRY_ATTEMPTS {
                    tokio::time::sleep(REBIND_RETRY_DELAY).await;
                    match tokio::net::TcpListener::bind(new_addr).await {
                        Ok(l) => {
                            bound = Some(l);
                            break;
                        }
                        Err(e) => {
                            tracing::error!(
                                addr = %new_addr,
                                attempt,
                                error = %e,
                                "HTTP rebind: retry bind failed"
                            );
                        }
                    }
                }
                match (bound, old_addr) {
                    (Some(l), _) => {
                        generation = Some(spawn_http_generation(
                            l,
                            new_addr,
                            router.clone(),
                            &shutdown,
                        ));
                        target = new_target;
                    }
                    (None, Some(old_addr)) => {
                        // Rollback: re-bind the previous address so the
                        // server keeps serving.
                        match tokio::net::TcpListener::bind(old_addr).await {
                            Ok(l) => {
                                tracing::error!(
                                    addr = %old_addr,
                                    "HTTP rebind: new address never bound; rolled back to the \
                                     previous address"
                                );
                                generation = Some(spawn_http_generation(
                                    l,
                                    old_addr,
                                    router.clone(),
                                    &shutdown,
                                ));
                                // INVARIANT: `target` keeps the PREVIOUS pair
                                // — it tracks what is actually bound, so a
                                // later publish of any different address
                                // (including a file revert) still registers
                                // as a change.
                            }
                            Err(e) => {
                                tracing::error!(
                                    addr = %old_addr,
                                    error = %e,
                                    "HTTP rebind: rollback bind failed — no HTTP listener is \
                                     accepting; reload with a usable address to recover"
                                );
                                generation = None;
                                target = new_target;
                            }
                        }
                    }
                    (None, None) => {
                        tracing::error!(
                            addr = %new_addr,
                            "HTTP rebind: no HTTP listener is accepting; reload with a usable \
                             address to recover"
                        );
                        target = new_target;
                    }
                }
            }
        }
    }

    // Process shutdown: the active generation's drain trigger is a child of
    // the process token, so its graceful drain is already underway — await
    // it (unbounded, matching the drain posture of a rebind retirement).
    if let Some(HttpGeneration { task, .. }) = generation {
        match task.await {
            Ok(result) => result.context(ServerSnafu)?,
            Err(e) => return Err(std::io::Error::other(e)).context(ServerSnafu),
        }
    }
    Ok(())
}

// ── Rebuild-class supervisors (#529 step 6) ─────────────────────────────────
//
// Unlike the endpoint supervisors above (make-before-break, two generations
// briefly coexisting), the scanner and feed scheduler are REBUILD-class: the
// old instance is fully torn down before the new one is built — neither
// kathodos nor komide supports two live instances at once, and nothing else
// in archon holds a reference to either, so exclusive ownership by the
// supervisor task is sufficient (no `Arc`/`RwLock` sharing needed).

/// Cheap owned copy of `DbPools` (both fields are `SqlitePool`, itself a
/// cheap `Arc`-backed handle) — shared FROM every call site that needs its
/// own copy for a construction outliving the borrow (feed scheduler
/// boot/rebuild).
fn clone_db_pools(db: &apotheke::DbPools) -> apotheke::DbPools {
    apotheke::DbPools {
        read: db.read.clone(),
        write: db.write.clone(),
    }
}

/// Builds the bounded reqwest client + `FeedSchedulerService` + started
/// `FeedScheduler` FROM one komide config — the full construction boot
/// performs at step 11 in `run_serve`, extracted so the rebuild supervisor
/// below and boot share one code path.
async fn start_feed_scheduler(
    config: &horismos::KomideConfig,
    event_tx: &themelion::EventSender,
    db: &apotheke::DbPools,
) -> Result<FeedScheduler, HostError> {
    // WHY a bounded client: an unbounded client left `fetch_timeout_secs`
    // configured but unenforced — a stalled feed host could block
    // `response.chunk().await` forever inside `komide::fetch::fetch_feed`,
    // wedging that feed's poll task.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.fetch_timeout_secs))
        .build()
        .unwrap_or_default(); // WHY: reqwest::Client::default() is a valid fallback; build fails only with invalid TLS config
    let service = Arc::new(FeedSchedulerService::new(
        clone_db_pools(db),
        event_tx.clone(),
        client,
        config.clone(),
    ));
    FeedScheduler::start(service, config.clone(), clone_db_pools(db))
        .await
        .context(FeedSchedulerSnafu)
}

/// Attempts to build a fresh instance FROM `new_cfg`. On failure, logs and
/// attempts a rollback build FROM `old_cfg`; if that ALSO fails the caller is
/// left with no live instance (`None`) — logged loudly — while the rest of
/// the server keeps serving. Returns the config the returned instance (if
/// any) actually reflects, mirroring the `target` tracking idiom in the
/// step-4/5 endpoint supervisors: it tracks what is actually live, not what
/// was requested, so a later config publish is compared against reality.
///
/// Shared by every rebuild-class supervisor (scanner, feeds) — one
/// rebuild/rollback state machine, tested once; this is also the seam the
/// step-6 tests use to exercise the rollback decision without depending on
/// kathodos/komide ever actually failing to start.
// NOTE: bounds live in the generic parameter list rather than a `where`
// clause — kanon's SQL/keyword-case checker mis-flags a standalone `where`
// line here (a false positive: the token is Rust syntax, not SQL).
async fn rebuild_with_rollback<
    T,
    C: Clone,
    E: std::fmt::Display,
    F: Fn(C) -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
>(
    subsystem: &str,
    new_cfg: C,
    old_cfg: C,
    build: F,
) -> (Option<T>, C) {
    match build(new_cfg.clone()).await {
        Ok(instance) => (Some(instance), new_cfg),
        Err(e) => {
            tracing::error!(
                subsystem,
                error = %e,
                "rebuild failed; attempting rollback to the previous config"
            );
            match build(old_cfg.clone()).await {
                Ok(instance) => {
                    tracing::error!(subsystem, "rebuild rolled back to the previous config");
                    (Some(instance), old_cfg)
                }
                Err(e2) => {
                    tracing::error!(
                        subsystem,
                        error = %e2,
                        "rollback ALSO failed  -  subsystem is DOWN; the server keeps serving; \
                         fix the config and reload to recover"
                    );
                    (None, new_cfg)
                }
            }
        }
    }
}

/// Runs the `taxis.*` scanner under a REBUILD-class supervisor: on a section
/// change it tears the old `ScannerManager` down (kathodos joins every
/// watcher/scan task — no statics, so a fresh instance is safe to build
/// immediately after, kathodos/src/scanner/mod.rs:115-126) and starts a new
/// one FROM the changed config. A failed rebuild rolls back to the PREVIOUS
/// taxis config (`rebuild_with_rollback`); if the rollback ALSO fails the
/// scanner stays down — loudly logged — while the rest of the server keeps
/// serving. In-flight scans abort via kathodos's existing shutdown-yield
/// (`scan_yielding_to_shutdown`) and simply re-run on the next
/// interval/trigger of the new instance — acceptable, logged at info above.
async fn run_scanner_supervisor(
    initial: ScannerManager,
    initial_taxis: horismos::TaxisConfig,
    config: horismos::ConfigHandle,
    event_tx: themelion::EventSender,
    shutdown: CancellationToken,
) {
    let mut watcher = config.watch_section(|c| &c.taxis);
    let mut scanner = Some(initial);
    let mut current = initial_taxis;

    loop {
        let changed = tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            changed = watcher.changed() => changed,
        };
        let Some(new_taxis) = changed else {
            // The config owner is gone: no further publishes can arrive, so
            // the active scanner serves unchanged until process shutdown.
            shutdown.cancelled().await;
            break;
        };

        tracing::info!(
            subsystem = "scanner",
            "taxis config changed  -  rebuilding scanner"
        );
        if let Some(old) = scanner.take() {
            old.shutdown().await;
        }
        let event_tx_for_build = event_tx.clone();
        let (rebuilt, effective) =
            rebuild_with_rollback("scanner", new_taxis, current, move |cfg| {
                let event_tx = event_tx_for_build.clone();
                async move { ScannerManager::start(&cfg, event_tx).await }
            })
            .await;
        scanner = rebuilt;
        current = effective;
    }

    // Process shutdown is the final teardown — the one path run_serve used
    // to call directly before this #529 step 6 supervisor owned the
    // lifecycle.
    if let Some(scanner) = scanner {
        scanner.shutdown().await;
    }
}

/// Runs the `komide.*` feed scheduler under a REBUILD-class supervisor: on a
/// section change it aborts the old scheduler's poll tasks and rebuilds the
/// bounded reqwest client + `FeedSchedulerService` + `FeedScheduler` together
/// via `start_feed_scheduler` — `FeedSchedulerService` owns its own
/// `KomideConfig`, so rebuilding the scheduler alone would leave it reading a
/// stale config. A failed rebuild rolls back to the PREVIOUS komide config
/// (`rebuild_with_rollback`); if the rollback ALSO fails the feed scheduler
/// stays down — loudly logged — while the rest of the server keeps serving.
///
/// NOTE: `FeedScheduler::start` re-pages every feed row FROM the DB on every
/// call, so a feed subscribed since boot happens to get a poll loop after ANY
/// rebuild. This is a side effect of the rebuild, NOT a fix for the
/// never-polled runtime-subscribed-feeds defect — that stays open as #577.
async fn run_feed_supervisor(
    initial: FeedScheduler,
    initial_komide: horismos::KomideConfig,
    config: horismos::ConfigHandle,
    event_tx: themelion::EventSender,
    db: apotheke::DbPools,
    shutdown: CancellationToken,
) {
    let mut watcher = config.watch_section(|c| &c.komide);
    let mut scheduler = Some(initial);
    let mut current = initial_komide;

    loop {
        let changed = tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            changed = watcher.changed() => changed,
        };
        let Some(new_komide) = changed else {
            shutdown.cancelled().await;
            break;
        };

        tracing::info!(
            subsystem = "feed_scheduler",
            "komide config changed  -  rebuilding feed scheduler"
        );
        if let Some(old) = scheduler.take() {
            old.shutdown();
        }
        let event_tx_for_build = event_tx.clone();
        let db_for_build = clone_db_pools(&db);
        let (rebuilt, effective) =
            rebuild_with_rollback("feed_scheduler", new_komide, current, move |cfg| {
                let event_tx = event_tx_for_build.clone();
                let db = clone_db_pools(&db_for_build);
                async move { start_feed_scheduler(&cfg, &event_tx, &db).await }
            })
            .await;
        scheduler = rebuilt;
        current = effective;
    }

    if let Some(scheduler) = scheduler {
        scheduler.shutdown();
    }
}

// ── Acquisition-live supervisors (#529 step 7) ──────────────────────────────
//
// Unlike the rebuild-class supervisors above, zetesis and syntaxis need no
// teardown/rebuild: their per-op fields already go live through a `Section`,
// and their remaining LIVE-B mechanisms (rate limiter, cf proxy, cardigann
// registry, `SlotAllocator` limits) are updated IN PLACE behind swappable
// interior mutability — the service instance itself never changes identity.

/// Runs the `syntaxis.*` live-limit supervisor: on a section change it calls
/// `DownloadQueue::update_config`, which replaces the stored config (making
/// `retry_count`/`retry_backoff_base_seconds` reads live) and updates the
/// `SlotAllocator`'s concurrency limits in place. A decrease below the
/// current in-flight count simply stops new dispatch until in-flight drains
/// below the new cap — nothing already dispatched is cancelled.
async fn run_syntaxis_supervisor(
    queue: Arc<DownloadQueue<SessionEngine>>,
    config: horismos::ConfigHandle,
    shutdown: CancellationToken,
) {
    let mut watcher = config.watch_section(|c| &c.syntaxis);
    loop {
        let changed = tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            changed = watcher.changed() => changed,
        };
        let Some(new_cfg) = changed else {
            shutdown.cancelled().await;
            break;
        };

        tracing::info!(
            subsystem = "syntaxis",
            "syntaxis config changed  -  updating live limits"
        );
        queue.update_config(new_cfg).await;
    }
}

/// Runs the zetesis LIVE-B mechanisms (per-indexer rate limits, Cloudflare
/// bypass proxy, Cardigann definitions registry) under a section watcher.
/// The per-op fields (`max_concurrent_searches`, `search_timeout_seconds`,
/// `max_response_body_bytes`, `request_timeout_secs`) need no supervisor
/// action — `SearchIndexerService` already reads them live through its own
/// `Section`.
async fn run_zetesis_supervisor(
    service: Arc<SearchIndexerService>,
    initial: horismos::SearchSubsystemConfig,
    config: horismos::ConfigHandle,
    shutdown: CancellationToken,
) {
    let mut watcher = config.watch_section(|c| &c.zetesis);
    let mut current = initial;

    loop {
        let changed = tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            changed = watcher.changed() => changed,
        };
        let Some(new) = changed else {
            shutdown.cancelled().await;
            break;
        };

        if new.per_indexer_rate_limit_requests != current.per_indexer_rate_limit_requests
            || new.per_indexer_rate_limit_window_seconds
                != current.per_indexer_rate_limit_window_seconds
        {
            tracing::info!(
                subsystem = "zetesis",
                "rate limit config changed  -  reconfiguring live"
            );
            service
                .reconfigure_rate_limiter(
                    new.per_indexer_rate_limit_requests,
                    std::time::Duration::from_secs(new.per_indexer_rate_limit_window_seconds),
                )
                .await;
        }

        if new.cloudflare_bypass_enabled != current.cloudflare_bypass_enabled
            || new.cf_proxy_url != current.cf_proxy_url
            || new.cf_proxy_timeout_seconds != current.cf_proxy_timeout_seconds
        {
            tracing::info!(
                subsystem = "zetesis",
                "cloudflare bypass config changed  -  rebuilding proxy"
            );
            match build_cf_proxy(&new) {
                Ok(proxy) => service.set_cf_proxy(proxy),
                Err(e) => tracing::error!(
                    subsystem = "zetesis",
                    error = %e,
                    "cf proxy rebuild failed  -  keeping the previous proxy"
                ),
            }
        }

        if new.cardigann_definitions_dir != current.cardigann_definitions_dir {
            tracing::info!(
                subsystem = "zetesis",
                "cardigann_definitions_dir changed  -  reloading registry"
            );
            // WHY: `CardigannRegistry::load` never hard-fails (a broken
            // definition file or unreadable dir is logged and skipped
            // internally) — the one failure mode this can preflight is the
            // directory itself being unreadable, checked here so a typo'd
            // path cannot silently blank out a working registry. An
            // existing-but-empty directory is NOT a failure — that is the
            // same state boot accepts.
            match &new.cardigann_definitions_dir {
                Some(dir) if std::fs::metadata(dir).is_err() => {
                    tracing::error!(
                        subsystem = "zetesis",
                        dir = %dir.display(),
                        "cardigann_definitions_dir is not readable  -  keeping the previous registry"
                    );
                }
                _ => {
                    let registry = CardigannRegistry::load(Arc::new(new.clone()));
                    tracing::info!(
                        subsystem = "zetesis",
                        count = registry.len(),
                        "cardigann registry reloaded"
                    );
                    service.set_cardigann_registry(Arc::new(registry));
                }
            }
        }

        current = new;
    }
}

// ── Integration-service supervisors (#529 step 8) ───────────────────────────

/// Runs the `epignosis.*` metadata resolver under a REBUILD-class
/// supervisor: on a section change it builds a fresh `ProviderBackedResolver`
/// FROM the new config and swaps it into `MetadataAdapter` — in-flight
/// resolutions already hold their own Arc clone (taken by
/// `MetadataAdapter::snapshot` before any await) and finish on the OLD
/// instance undisturbed. The resolver's own metadata cache resets on every
/// rebuild — logged, not silently accepted.
async fn run_epignosis_supervisor(
    adapter: Arc<MetadataAdapter>,
    config: horismos::ConfigHandle,
    shutdown: CancellationToken,
) {
    let mut watcher = config.watch_section(|c| &c.epignosis);
    loop {
        let changed = tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            changed = watcher.changed() => changed,
        };
        let Some(new_cfg) = changed else {
            shutdown.cancelled().await;
            break;
        };

        info!("epignosis config changed — resolver rebuilt, metadata cache reset");
        let credentials = ProviderCredentials::from(&new_cfg);
        let resolver = Arc::new(ProviderBackedResolver::new(new_cfg, credentials));
        adapter.set_resolver(resolver);
    }
}

/// Runs the `prostheke.opensubtitles` provider set under a REBUILD-class
/// supervisor: on a subtree change (INCLUDING None↔Some presence) it rebuilds
/// the provider list via `Provider::default_providers` and swaps it into
/// `SubtitleManager`. The OpenSubtitles rate limiter resets with the
/// provider — acceptable (it's a politeness limiter, not a 429 embargo) —
/// logged, not silently accepted. The per-op language/score preferences need
/// no supervisor action — `SubtitleManager` already reads them live through
/// its own `Section`.
async fn run_prostheke_supervisor(
    manager: Arc<SubtitleManager>,
    config: horismos::ConfigHandle,
    shutdown: CancellationToken,
) {
    let mut watcher = config.watch_section(|c| &c.prostheke.opensubtitles);
    loop {
        let changed = tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            changed = watcher.changed() => changed,
        };
        let Some(new_opensubtitles) = changed else {
            shutdown.cancelled().await;
            break;
        };

        info!(
            subsystem = "prostheke",
            "prostheke.opensubtitles config changed  -  rebuilding providers; \
             OpenSubtitles rate limiter reset"
        );
        let providers = Provider::default_providers(new_opensubtitles);
        manager.set_providers(providers);
    }
}

/// One live syndesmos event-handler generation: its child cancellation token
/// (a child of the supervisor's `shutdown`) plus the handler task itself —
/// bundled so `run_syndesmos_supervisor` takes one argument for "the current
/// generation" instead of two.
struct SyndesmosGeneration {
    ct: CancellationToken,
    handle: JoinHandle<()>,
}

/// Runs the `syndesmos.*` external-integrations client under a REBUILD-class
/// supervisor: on a section change it cancels the event handler's child
/// token, awaits it, rebuilds the `ScrobbleClient` via `build_syndesmos`,
/// respawns the handler on a fresh subscription, and swaps `ExternalAdapter`'s
/// inner Arc. Two honest costs, logged: the circuit breakers reset (fresh
/// breakers, `warn!`), and any event published between the cancel and the
/// fresh subscribe is lost — broadcast receivers only see events published
/// after they subscribe — a bounded scrobble-loss window (`warn!`).
async fn run_syndesmos_supervisor(
    external: Arc<ExternalAdapter>,
    config: horismos::ConfigHandle,
    event_tx: themelion::EventSender,
    db: sqlx::SqlitePool,
    mut generation: SyndesmosGeneration,
    shutdown: CancellationToken,
) {
    let mut watcher = config.watch_section(|c| &c.syndesmos);
    loop {
        let changed = tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            changed = watcher.changed() => changed,
        };
        let Some(_new_syndesmos) = changed else {
            shutdown.cancelled().await;
            break;
        };

        tracing::warn!(
            subsystem = "syndesmos",
            "syndesmos config changed  -  rebuilding scrobble client; circuit breakers reset"
        );
        generation.ct.cancel();
        if let Err(e) = generation.handle.await {
            tracing::warn!(error = %e, "syndesmos event handler panicked during rebuild");
        }
        tracing::warn!(
            subsystem = "syndesmos",
            "syndesmos rebuild: events published between handler cancel and resubscribe are \
             not delivered (bounded scrobble-loss window)"
        );

        let cfg = config.current();
        let client = Arc::new(build_syndesmos(&cfg, &event_tx, db.clone()));
        external.set_client(Arc::clone(&client));
        let ct = shutdown.child_token();
        let handle = spawn_syndesmos_handler(client, event_tx.subscribe(), ct.clone());
        generation = SyndesmosGeneration { ct, handle };
    }

    generation.ct.cancel();
    if let Err(e) = generation.handle.await {
        tracing::warn!(error = %e, "syndesmos event handler panicked during shutdown");
    }
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
pub(crate) fn resolve_listen_addr(
    listen_addr: &str,
    port: u16,
) -> Result<std::net::SocketAddr, HostError> {
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
        config.max_response_body_bytes,
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
        let adapter = CurationAdapter(Arc::new(DefaultCurationService::new(
            pool,
            event_tx,
            horismos::Section::fixed(horismos::KritikeConfig {
                quality_check_concurrency: 4,
                ..Default::default()
            }),
        )));

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
        let adapter = CurationAdapter(Arc::new(DefaultCurationService::new(
            pool,
            event_tx,
            horismos::Section::fixed(horismos::KritikeConfig {
                quality_check_concurrency: 4,
                ..Default::default()
            }),
        )));

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
        let adapter = MetadataAdapter::new(Arc::new(ProviderBackedResolver::new(
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
            horismos::Section::fixed(horismos::SearchSubsystemConfig::default()),
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
            horismos::Section::fixed(horismos::ProsthekeConfig::default()),
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

    // ── reload_config ────────────────────────────────────────────────────────

    fn reload_test_toml(port: u16, db_path: &std::path::Path) -> String {
        format!(
            "[exousia]\njwt_secret = \"test-secret-that-is-long-enough-for-hs256\"\n\n[paroche]\nport = {port}\n\n[database]\ndb_path = \"{}\"\n",
            db_path.display()
        )
    }

    #[tokio::test]
    async fn reload_config_reports_applied_and_restart_pending() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("harmonia.toml");
        let db_a = dir.path().join("a.db");
        let db_b = dir.path().join("b.db");

        std::fs::write(&config_path, reload_test_toml(8096, &db_a)).expect("write initial config");
        let (initial_config, _warnings) =
            horismos::load_config(Some(config_path.as_path())).expect("load initial config");
        let (manager, _handle) = ConfigManager::new(
            initial_config,
            config_path.clone(),
            ConfigOverrides::default(),
        );

        // A LIVE change (paroche.port) alongside a RESTART-class change
        // (database.db_path) in the same reload — the outcome must report
        // both correctly rather than collapsing to one bucket.
        std::fs::write(&config_path, reload_test_toml(9090, &db_b)).expect("write changed config");

        let outcome = reload_config(manager)
            .await
            .expect("reload_config succeeds");

        assert_eq!(outcome.applied, vec!["paroche.port".to_string()]);
        assert_eq!(
            outcome.restart_pending,
            vec!["database.db_path".to_string()]
        );
        assert!(outcome.needs_restart());
        assert!(!outcome.is_unchanged());
    }

    #[tokio::test]
    async fn reload_config_reports_unchanged_when_file_unchanged() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("harmonia.toml");
        let db_a = dir.path().join("a.db");

        std::fs::write(&config_path, reload_test_toml(8096, &db_a)).expect("write initial config");
        let (initial_config, _warnings) =
            horismos::load_config(Some(config_path.as_path())).expect("load initial config");
        let (manager, _handle) =
            ConfigManager::new(initial_config, config_path, ConfigOverrides::default());

        let outcome = reload_config(manager)
            .await
            .expect("reload_config succeeds");

        assert!(outcome.applied.is_empty());
        assert!(outcome.restart_pending.is_empty());
        assert!(outcome.is_unchanged());
        assert!(!outcome.needs_restart());
    }
}

#[cfg(test)]
mod http_supervisor_tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use horismos::{Config, ConfigManager, ConfigOverrides};
    use tokio::task::JoinHandle;
    use tokio_util::sync::CancellationToken;

    use super::*;

    fn test_router() -> axum::Router {
        axum::Router::new()
            .route("/ping", axum::routing::get(|| async { "pong" }))
            .route(
                "/slow",
                axum::routing::get(|| async {
                    tokio::time::sleep(Duration::from_millis(800)).await;
                    "slow-done"
                }),
            )
    }

    fn http_test_config(port: u16) -> Config {
        let mut config = Config::default();
        config.exousia.jwt_secret =
            "http-supervisor-test-secret-at-least-32-bytes-long".to_string();
        config.paroche.listen_addr = "127.0.0.1".to_string();
        config.paroche.port = port;
        config
    }

    struct HttpHarness {
        addr: std::net::SocketAddr,
        manager: ConfigManager,
        shutdown: CancellationToken,
        task: JoinHandle<Result<(), HostError>>,
    }

    impl HttpHarness {
        async fn shutdown_and_join(self) {
            self.shutdown.cancel();
            self.task
                .await
                .expect("supervisor task joins")
                .expect("supervisor exits cleanly");
        }
    }

    /// Spawns the real `run_http_supervisor` on an OS-assigned loopback port,
    /// with the live config's `(listen_addr, port)` matching the bound
    /// address so rebind-target tracking reflects the harness's listener.
    async fn spawn_supervisor() -> HttpHarness {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind startup listener");
        let addr = listener.local_addr().expect("local addr");
        let (manager, handle) = ConfigManager::new(
            http_test_config(addr.port()),
            PathBuf::from("unused.toml"),
            ConfigOverrides::default(),
        );
        let shutdown = CancellationToken::new();
        let task = tokio::spawn(run_http_supervisor(
            listener,
            addr,
            test_router(),
            handle,
            shutdown.clone(),
        ));
        HttpHarness {
            addr,
            manager,
            shutdown,
            task,
        }
    }

    /// Reserves an ephemeral TCP port on loopback and releases it so the
    /// supervisor can bind it.
    fn free_tcp_port() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("reserve tcp port");
        listener.local_addr().expect("local addr").port()
    }

    async fn poll_until_served(port: u16) -> bool {
        for _ in 0..100 {
            if let Ok(resp) = reqwest::get(format!("http://127.0.0.1:{port}/ping")).await
                && resp.status() == reqwest::StatusCode::OK
            {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        false
    }

    // ── #529 step 5: HTTP dual-listener rebind ──────────────────────────────

    #[tokio::test]
    async fn rebind_completes_in_flight_on_old_listener_and_serves_new() {
        let harness = spawn_supervisor().await;
        assert!(
            poll_until_served(harness.addr.port()).await,
            "startup listener must serve"
        );

        let slow_url = format!("http://{}/slow", harness.addr);
        let slow = tokio::spawn(async move { reqwest::get(slow_url).await });
        // The slow request must be accepted on the OLD listener before the
        // rebind retires it.
        tokio::time::sleep(Duration::from_millis(150)).await;

        let port_b = free_tcp_port();
        harness
            .manager
            .replace(http_test_config(port_b))
            .expect("replace applies the new port");

        assert!(
            poll_until_served(port_b).await,
            "new requests must be served on the new listener"
        );

        let resp = slow
            .await
            .expect("slow request task joins")
            .expect("the in-flight request on the old listener must complete");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        assert_eq!(resp.text().await.expect("slow body"), "slow-done");

        let refused = reqwest::get(format!("http://{}/ping", harness.addr)).await;
        assert!(
            refused.is_err(),
            "the old listener must refuse new connections after the rebind"
        );

        harness.shutdown_and_join().await;
    }

    #[tokio::test]
    async fn rebind_bind_conflict_rolls_back_to_previous_listener() {
        let harness = spawn_supervisor().await;
        assert!(
            poll_until_served(harness.addr.port()).await,
            "startup listener must serve"
        );

        // Deliberate conflict: a plain TCP listener squats on the target port.
        let blocker = std::net::TcpListener::bind("127.0.0.1:0").expect("blocker socket");
        let port_b = blocker.local_addr().expect("blocker addr").port();

        harness
            .manager
            .replace(http_test_config(port_b))
            .expect("replace applies the conflicting port");

        // Break-before-make retires the original listener first — requests
        // to it stop being accepted.
        let mut retired = false;
        for _ in 0..100 {
            if reqwest::get(format!("http://{}/ping", harness.addr))
                .await
                .is_err()
            {
                retired = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(
            retired,
            "the original listener must retire when the fallback path engages"
        );

        // The retries on the conflicting port fail (the blocker holds it), so
        // the supervisor rolls back to the original address and keeps serving.
        assert!(
            poll_until_served(harness.addr.port()).await,
            "the server must survive a rebind bind-conflict by rolling back to the previous \
             address"
        );
        drop(blocker);
        harness.shutdown_and_join().await;
    }
}

#[cfg(test)]
mod rebuild_supervisor_tests {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use horismos::{Config, ConfigManager, ConfigOverrides};
    use tokio_util::sync::CancellationToken;

    use super::*;

    // ── rebuild_with_rollback: the shared rebuild/rollback decision ─────────
    //
    // Neither kathodos's `ScannerManager::start` nor `start_feed_scheduler`
    // can be made to fail FROM a test without corrupting a live DB/filesystem
    // out FROM under the process, so the rollback decision is exercised here
    // against injected build closures — the escape hatch the #529 step 6 spec
    // calls for when a real failing start isn't reachable.

    #[tokio::test]
    async fn rebuild_with_rollback_recovers_via_previous_config_on_failed_rebuild() {
        let attempts: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
        let attempts_for_build = Arc::clone(&attempts);
        let build = move |cfg: i32| {
            let attempts = Arc::clone(&attempts_for_build);
            async move {
                attempts.lock().expect("lock").push(cfg);
                if cfg == 99 { Err("boom") } else { Ok(cfg) }
            }
        };

        let (result, effective) = rebuild_with_rollback("test", 99, 1, build).await;

        assert_eq!(
            result,
            Some(1),
            "a failed rebuild must roll back to the old value"
        );
        assert_eq!(
            effective, 1,
            "effective config tracks what is actually live"
        );
        assert_eq!(
            *attempts.lock().expect("lock"),
            vec![99, 1],
            "the new config is attempted first, THEN the old one on failure"
        );
    }

    #[tokio::test]
    async fn rebuild_with_rollback_leaves_subsystem_down_when_both_builds_fail() {
        let build = |_cfg: i32| async move { Err::<i32, &str>("boom") };

        let (result, effective) = rebuild_with_rollback("test", 99, 1, build).await;

        assert!(
            result.is_none(),
            "both the rebuild AND the rollback failing must leave no live instance"
        );
        assert_eq!(
            effective, 99,
            "with no live instance, effective tracks the last attempted (new) value"
        );
    }

    // ── scanner supervisor: real ScannerManager + real ConfigManager ────────

    fn scanner_test_config() -> Config {
        let mut config = Config::default();
        config.exousia.jwt_secret =
            "scanner-supervisor-test-secret-at-least-32-bytes-long".to_string();
        config
    }

    /// #529 step 6: a REAL `run_scanner_supervisor`, driven by a REAL
    /// `ConfigManager::replace`, correctly reacts to a `taxis.*` change —
    /// tearing the old (empty) scanner down and rebuilding via
    /// `rebuild_with_rollback` + `ScannerManager::start` — and shuts down
    /// cleanly on process shutdown. Libraries stay empty here so the rebuild
    /// is instantaneous (no scan/watcher tasks to wait on); the NEXT test
    /// proves what a rebuilt instance carrying a library can actually do.
    #[tokio::test]
    async fn scanner_supervisor_rebuilds_on_taxis_change_and_joins_cleanly() {
        let config = scanner_test_config();
        let (manager, handle) = ConfigManager::new(
            config.clone(),
            PathBuf::from("unused.toml"),
            ConfigOverrides::default(),
        );

        let (event_tx, _event_rx) = themelion::create_event_bus(16);
        let initial = ScannerManager::start(&config.taxis, event_tx.clone())
            .await
            .expect("initial scanner starts cleanly");

        let shutdown = CancellationToken::new();
        let supervisor = tokio::spawn(run_scanner_supervisor(
            initial,
            config.taxis.clone(),
            handle,
            event_tx,
            shutdown.clone(),
        ));

        let mut new_config = config.clone();
        new_config.taxis.scan_concurrency = 7;
        manager
            .replace(new_config)
            .expect("replace applies the taxis change");

        // Bounded real wait for the supervisor to observe and process the
        // change — generous relative to in-process channel/task scheduling,
        // nowhere near a scan-interval timescale.
        tokio::time::sleep(Duration::from_millis(200)).await;

        shutdown.cancel();
        supervisor
            .await
            .expect("scanner supervisor joins cleanly after a live rebuild");
    }

    /// #529 step 6: `rebuild_with_rollback` + `ScannerManager::start` — the
    /// EXACT construction the supervisor performs on a `taxis.*` change —
    /// wires a newly added library so it can be scanned, proving the rebuild
    /// uses the NEW `taxis.libraries` map, not the boot-time one. Drives the
    /// scan via `trigger_scan` (kathodos's own test hook, kathodos/src/
    /// scanner/mod.rs's `scanner_detects_new_file_in_watched_directory`) so
    /// the assertion is immediate, not gated on `scan_interval_hours`.
    #[tokio::test]
    async fn scanner_rebuild_scans_newly_added_library() {
        let config = scanner_test_config();
        let (event_tx, mut event_rx) = themelion::create_event_bus(64);

        let dir = tempfile::TempDir::new().expect("tempdir");
        std::fs::write(dir.path().join("track.flac"), b"FLAC").expect("write test file");

        let mut new_taxis = config.taxis.clone();
        new_taxis.libraries.insert(
            "new-lib".to_string(),
            horismos::LibraryConfig {
                path: dir.path().to_path_buf(),
                media_type: horismos::MediaType::Music,
                watcher_mode: horismos::WatcherMode::Poll,
                poll_interval_seconds: 1,
                auto_import: true,
                scan_interval_hours: 9999, // don't auto-scan; trigger_scan drives it
            },
        );

        let old_taxis = config.taxis.clone();
        let (rebuilt, _effective) = rebuild_with_rollback("scanner", new_taxis, old_taxis, |cfg| {
            let event_tx = event_tx.clone();
            async move { ScannerManager::start(&cfg, event_tx).await }
        })
        .await;
        let rebuilt = rebuilt.expect("rebuild succeeds");

        rebuilt
            .trigger_scan("new-lib")
            .await
            .expect("trigger scan on the newly wired library");

        let scanned = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match event_rx.recv().await {
                    Ok(themelion::HarmoniaEvent::LibraryScanCompleted { items_added, .. })
                        if items_added >= 1 =>
                    {
                        return;
                    }
                    Ok(_) => continue,
                    Err(_) => panic!("event bus closed before a scan completed"),
                }
            }
        })
        .await;
        assert!(
            scanned.is_ok(),
            "the rebuilt scanner must scan the newly added library"
        );

        rebuilt.shutdown().await;
    }

    /// #529 step 6: proves `ScannerManager::start` — the exact call the
    /// rebuild supervisor makes on a `taxis.*` change — reflects a changed
    /// `scan_concurrency`. Uses the construction-time accessor (not live
    /// semaphore state), so the assertion is timing-free, not a sleep-race.
    #[tokio::test]
    async fn scanner_rebuild_reflects_changed_scan_concurrency() {
        let (event_tx, _event_rx) = themelion::create_event_bus(16);

        let low = horismos::TaxisConfig {
            scan_concurrency: 2,
            ..Default::default()
        };
        let before = ScannerManager::start(&low, event_tx.clone())
            .await
            .expect("scanner starts");
        assert_eq!(before.scan_concurrency(), 2);
        before.shutdown().await;

        let high = horismos::TaxisConfig {
            scan_concurrency: 9,
            ..Default::default()
        };
        let after = ScannerManager::start(&high, event_tx)
            .await
            .expect("scanner starts");
        assert_eq!(
            after.scan_concurrency(),
            9,
            "a rebuild FROM a changed scan_concurrency must be reflected in the fresh instance"
        );
        after.shutdown().await;
    }

    // ── feed supervisor: real FeedScheduler + real ConfigManager ────────────

    fn feed_test_config(podcast_poll_interval_minutes: u64) -> Config {
        let mut config = Config::default();
        config.exousia.jwt_secret =
            "feed-supervisor-test-secret-at-least-32-bytes-long".to_string();
        config.komide.podcast_poll_interval_minutes = podcast_poll_interval_minutes;
        config.komide.jitter_percent = 0.0;
        config
    }

    async fn seed_podcast_subscription(pool: &sqlx::SqlitePool, feed_url: &str) {
        let sub = apotheke::repo::podcast::PodcastSubscription {
            id: uuid::Uuid::new_v4().as_bytes().to_vec(),
            feed_url: feed_url.to_string(),
            title: Some("Test Feed".to_string()),
            description: None,
            author: None,
            image_url: None,
            language: None,
            last_checked_at: None,
            auto_download: 1,
            quality_profile_id: None,
            added_at: "2024-01-01T00:00:00Z".to_string(),
        };
        apotheke::repo::podcast::insert_subscription(pool, &sub)
            .await
            .expect("seed subscription");
    }

    /// #529 step 6: a REAL `run_feed_supervisor`, driven by a REAL
    /// `ConfigManager::replace` with a DIFFERENT `podcast_poll_interval_minutes`,
    /// correctly reacts to a `komide.*` change — aborting the old (empty)
    /// scheduler and rebuilding via `rebuild_with_rollback` +
    /// `start_feed_scheduler` — and shuts down cleanly on process shutdown.
    /// The DB stays subscription-less here so the rebuild is instantaneous;
    /// `feed_scheduler_rebuild_repages_db_for_new_config` below proves what a
    /// rebuild does with an actual subscription.
    #[tokio::test]
    async fn feed_supervisor_rebuilds_on_komide_change_and_joins_cleanly() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite");
        apotheke::migrate::MIGRATOR
            .run(&pool)
            .await
            .expect("migrate");
        let db = apotheke::DbPools {
            read: pool.clone(),
            write: pool,
        };

        let (event_tx, _event_rx) = themelion::create_event_bus(16);
        let boot_config = feed_test_config(600);
        let (manager, handle) = ConfigManager::new(
            boot_config.clone(),
            PathBuf::from("unused.toml"),
            ConfigOverrides::default(),
        );
        let initial = start_feed_scheduler(&boot_config.komide, &event_tx, &db)
            .await
            .expect("initial scheduler starts cleanly");

        let shutdown = CancellationToken::new();
        let supervisor = tokio::spawn(run_feed_supervisor(
            initial,
            boot_config.komide.clone(),
            handle,
            event_tx,
            clone_db_pools(&db),
            shutdown.clone(),
        ));

        manager
            .replace(feed_test_config(30))
            .expect("replace applies the shorter interval");

        // Bounded real wait for the supervisor to observe and process the
        // change — generous relative to in-process channel/task scheduling,
        // nowhere near a poll-interval timescale.
        tokio::time::sleep(Duration::from_millis(200)).await;

        shutdown.cancel();
        supervisor
            .await
            .expect("feed supervisor joins cleanly after a live rebuild");
    }

    /// #529 step 6: proves `start_feed_scheduler` — the exact call the
    /// rebuild supervisor makes on a `komide.*` change — re-pages the DB, so
    /// a subscription invisible to an earlier instance is picked up by a
    /// fresh one built FROM the same DB. Constructor-visible via
    /// `task_count()`, no polling/timing involved.
    #[tokio::test]
    async fn feed_scheduler_rebuild_repages_db_for_new_config() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite");
        apotheke::migrate::MIGRATOR
            .run(&pool)
            .await
            .expect("migrate");
        let db = apotheke::DbPools {
            read: pool.clone(),
            write: pool.clone(),
        };
        let (event_tx, _event_rx) = themelion::create_event_bus(16);

        let before = start_feed_scheduler(&horismos::KomideConfig::default(), &event_tx, &db)
            .await
            .expect("scheduler starts with zero subscriptions");
        assert_eq!(before.task_count(), 0);
        before.shutdown();

        seed_podcast_subscription(&pool, "https://example.com/feed.xml").await;

        let changed = horismos::KomideConfig {
            podcast_poll_interval_minutes: 45,
            ..Default::default()
        };
        let after = start_feed_scheduler(&changed, &event_tx, &db)
            .await
            .expect("scheduler starts with the new subscription");
        assert_eq!(
            after.task_count(),
            1,
            "a rebuild FROM the new config must re-page the DB and pick up the subscription"
        );
        after.shutdown();
    }

    // ── epignosis supervisor: real MetadataAdapter + real ConfigManager ────

    fn epignosis_test_config() -> Config {
        let mut config = Config::default();
        config.exousia.jwt_secret =
            "epignosis-supervisor-test-secret-at-least-32-bytes-long".to_string();
        config
    }

    /// #529 step 8: a REAL `run_epignosis_supervisor`, driven by a REAL
    /// `ConfigManager::replace`, rebuilds the resolver on an `epignosis.*`
    /// change and swaps it into `MetadataAdapter` — proven by Arc identity
    /// changing (the resolver has no other externally-observable state) —
    /// and shuts down cleanly on process shutdown.
    #[tokio::test]
    async fn epignosis_supervisor_rebuilds_resolver_on_config_change() {
        let config = epignosis_test_config();
        let (manager, handle) = ConfigManager::new(
            config.clone(),
            PathBuf::from("unused.toml"),
            ConfigOverrides::default(),
        );

        let initial = Arc::new(ProviderBackedResolver::new(
            config.epignosis.clone(),
            ProviderCredentials::default(),
        ));
        let adapter = Arc::new(MetadataAdapter::new(Arc::clone(&initial)));

        let shutdown = CancellationToken::new();
        let supervisor = tokio::spawn(run_epignosis_supervisor(
            Arc::clone(&adapter),
            handle,
            shutdown.clone(),
        ));

        // WHY: let the supervisor's first poll run BEFORE the replace below —
        // `watch_section`'s baseline is established at first-poll time (a
        // clone of a never-advanced `watch::Receiver` starts "unseen," so a
        // replace landing before the first poll would be silently folded
        // into that baseline instead of surfacing as a change).
        tokio::task::yield_now().await;

        let mut changed = config.clone();
        changed.epignosis.provider_timeout_secs += 5;
        manager
            .replace(changed)
            .expect("replace applies the epignosis change");

        // Bounded real wait for the supervisor to observe and process the
        // change — generous relative to in-process channel/task scheduling.
        tokio::time::sleep(Duration::from_millis(200)).await;

        let after = adapter.snapshot();
        assert!(
            !Arc::ptr_eq(&initial, &after),
            "an epignosis.* change must rebuild the resolver (new Arc identity)"
        );

        shutdown.cancel();
        supervisor
            .await
            .expect("epignosis supervisor joins cleanly after a live rebuild");
    }

    /// #578: a published provider-credential change (not just a tuning knob
    /// like `provider_timeout_secs`) must reach the same rebuild path — the
    /// supervisor re-derives `ProviderCredentials::from(&new_cfg)` on every
    /// `epignosis.*` change, so key rotation is live for free. `Arc` identity
    /// is the only externally-observable proof available from archon (the
    /// resolver's provider clients are epignosis-crate-private); that the
    /// derived credentials actually carry the new key value into the
    /// outbound provider request is covered by epignosis's own
    /// `provider_credentials_from_config_maps_present_keys` (resolver.rs) and
    /// `configured_api_key_reaches_lookup_request` (providers/acoustid.rs).
    #[tokio::test]
    async fn epignosis_supervisor_rebuilds_resolver_on_credential_change() {
        let config = epignosis_test_config();
        let (manager, handle) = ConfigManager::new(
            config.clone(),
            PathBuf::from("unused.toml"),
            ConfigOverrides::default(),
        );

        let initial = Arc::new(ProviderBackedResolver::new(
            config.epignosis.clone(),
            ProviderCredentials::default(),
        ));
        let adapter = Arc::new(MetadataAdapter::new(Arc::clone(&initial)));

        let shutdown = CancellationToken::new();
        let supervisor = tokio::spawn(run_epignosis_supervisor(
            Arc::clone(&adapter),
            handle,
            shutdown.clone(),
        ));

        tokio::task::yield_now().await;

        let mut changed = config.clone();
        changed.epignosis.acoustid_key = Some("rotated-acoustid-key".to_string());
        manager
            .replace(changed)
            .expect("replace applies the epignosis credential change");

        tokio::time::sleep(Duration::from_millis(200)).await;

        let after = adapter.snapshot();
        assert!(
            !Arc::ptr_eq(&initial, &after),
            "a published epignosis credential change must rebuild the resolver"
        );

        shutdown.cancel();
        supervisor
            .await
            .expect("epignosis supervisor joins cleanly after a credential rebuild");
    }
}
