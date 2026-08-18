/// Download queue endpoints.
use apotheke::DbPools;
use apotheke::repo::want;
use axum::{
    Json,
    extract::{Path, State},
};
use exousia::{AuthenticatedUser, RequireAdmin};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing;
use uuid::Uuid;

use crate::error::ParocheError;
use crate::response::{ApiResponse, deleted};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Row / response types
// ---------------------------------------------------------------------------

fn bytes_to_uuid_str(bytes: &[u8]) -> String {
    Uuid::from_slice(bytes)
        .map(|u| u.to_string())
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, len = bytes.len(), "malformed UUID bytes in db row");
            String::new()
        })
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct DownloadRow {
    id: Vec<u8>,
    want_id: Vec<u8>,
    release_id: Vec<u8>,
    download_url: String,
    protocol: String,
    priority: i64,
    info_hash: Option<String>,
    status: String,
    added_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    failed_reason: Option<String>,
    retry_count: i64,
}

const SELECT_DOWNLOAD: &str = "\
    SELECT id, want_id, release_id, download_url, protocol, priority, \
           info_hash, status, added_at, started_at, completed_at, \
           failed_reason, retry_count \
    FROM download_queue";

#[derive(Serialize)]
pub struct DownloadResponse {
    pub id: String,
    pub want_id: String,
    pub release_id: String,
    pub download_url: String,
    pub protocol: String,
    pub priority: i64,
    pub info_hash: Option<String>,
    pub status: String,
    pub added_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub failed_reason: Option<String>,
    pub retry_count: i64,
}

impl From<DownloadRow> for DownloadResponse {
    fn from(r: DownloadRow) -> Self {
        Self {
            id: bytes_to_uuid_str(&r.id),
            want_id: bytes_to_uuid_str(&r.want_id),
            release_id: bytes_to_uuid_str(&r.release_id),
            // WHY: the stored URL embeds the indexer apikey/passkey
            // (Torznab/Newznab convention); the queue snapshot is
            // member-visible, so raw URLs hand out the operator's
            // private-tracker credentials. The DB row and the enqueue path
            // keep the real URL — only the outbound response is redacted.
            download_url: crate::redact::redact_download_url(&r.download_url),
            protocol: r.protocol,
            priority: r.priority,
            info_hash: r.info_hash,
            status: r.status,
            added_at: r.added_at,
            started_at: r.started_at,
            completed_at: r.completed_at,
            failed_reason: r.failed_reason,
            retry_count: r.retry_count,
        }
    }
}

#[derive(Serialize)]
pub struct QueueSnapshotResponse {
    pub active: Vec<DownloadResponse>,
    pub queued: Vec<DownloadResponse>,
    pub completed_count: i64,
    pub failed_count: i64,
}

// ---------------------------------------------------------------------------
// Shared row queries (#609: reused by the MCP acquisition bridge — a
// single query path avoids `download_queue` SQL drifting between the HTTP
// surface and the bridge).
// ---------------------------------------------------------------------------

/// Fetches one persisted download row by its `download_queue` id, redacted
/// exactly like every other outbound download row.
pub async fn fetch_download(
    pool: &SqlitePool,
    id: Uuid,
) -> Result<Option<DownloadResponse>, sqlx::Error> {
    let q = format!("{SELECT_DOWNLOAD} WHERE id = ?");
    let row = sqlx::query_as::<_, DownloadRow>(&q)
        .bind(id.as_bytes().as_slice())
        .fetch_optional(pool)
        .await?;
    Ok(row.map(Into::into))
}

/// Lists persisted download rows, optionally filtered by `status` and/or
/// `id`, highest priority first. Backs the MCP `harmonia_list_downloads`
/// tool's flat filtered view (the HTTP `GET /api/v1/downloads` snapshot
/// keeps its own active/queued split — a different shape, not a filter).
pub async fn list_downloads(
    pool: &SqlitePool,
    status: Option<&str>,
    id: Option<Uuid>,
    limit: u32,
) -> Result<Vec<DownloadResponse>, sqlx::Error> {
    let id_bytes = id.map(|u| u.as_bytes().to_vec());
    let q = format!(
        "{SELECT_DOWNLOAD} WHERE (?1 IS NULL OR status = ?1) AND (?2 IS NULL OR id = ?2) \
         ORDER BY priority DESC, added_at DESC LIMIT ?3"
    );
    let rows = sqlx::query_as::<_, DownloadRow>(&q)
        .bind(status)
        .bind(id_bytes)
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

// ---------------------------------------------------------------------------
// Release persistence ahead of enqueue (#651: reused by the MCP acquisition
// bridge — one definition of the persist-before-enqueue invariant so the
// HTTP route and the bridge cannot drift back apart on this rule).
// ---------------------------------------------------------------------------

// WHY: `releases.indexer_id` is a plain NOT NULL i64, not an FK — a
// self-supplied URL (magnet or a direct file link) never came from a
// cataloged indexer, so it gets this sentinel rather than a fabricated real
// indexer id. Real `indexers.id` values are SQLite rowids starting at 1,
// never 0.
pub const MANUAL_RELEASE_INDEXER_ID: i64 = 0;

/// Extracts a magnet URI's own `dn` (display name) / `xl` (exact length)
/// query params — the ONLY release metadata available for a self-supplied
/// magnet that never went through `resolve_release`'s indexer catalog.
pub fn magnet_release_metadata(magnet: &url::Url) -> (Option<String>, Option<u64>) {
    let mut title = None;
    let mut size_bytes = None;
    for (key, value) in magnet.query_pairs() {
        match key.as_ref() {
            "dn" => title = Some(value.into_owned()),
            "xl" => size_bytes = value.parse::<u64>().ok(),
            _ => {}
        }
    }
    (title, size_bytes)
}

/// Everything needed to persist a `releases` row ahead of enqueue — either
/// resolved server-side (a cataloged indexer hit via `resolve_release`) or
/// derived from a self-supplied URL with no catalog entry (a magnet's own
/// `dn`/`xl` params, or a neutral placeholder for a plain file URL).
pub struct ReleaseMetadata {
    pub indexer_id: i64,
    pub title: String,
    pub size_bytes: Option<u64>,
    pub download_url: String,
    pub protocol: String,
    pub info_hash: Option<String>,
}

impl ReleaseMetadata {
    /// The one construction path for a releases row. `title` and
    /// `download_url` must be non-empty — every call site arrives here with
    /// an SSRF-validated URL and a resolved-or-placeholder title, and this
    /// is where that contract is enforced rather than re-stated.
    pub fn new(
        indexer_id: i64,
        title: String,
        size_bytes: Option<u64>,
        download_url: String,
        protocol: String,
        info_hash: Option<String>,
    ) -> Result<Self, ReleasePersistError> {
        if title.is_empty() || download_url.is_empty() {
            return Err(ReleasePersistError::EmptyMetadata);
        }
        Ok(Self {
            indexer_id,
            title,
            size_bytes,
            download_url,
            protocol,
            info_hash,
        })
    }
}

/// Outcomes of `persist_release_before_enqueue` other than success. Kept
/// distinct from `ParocheError`/the MCP bridge's `tool_error` so this
/// module stays decoupled from either surface's error-rendering convention
/// — each caller maps these to its own shape.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReleasePersistError {
    /// `want_id` has no row in `wants` — the caller must create it first.
    WantNotFound,
    /// `release_id` already exists but is recorded under a DIFFERENT want.
    ReleaseWantConflict,
    /// Title or download URL arrived empty — the constructor's contract.
    EmptyMetadata,
    Database(apotheke::DbError),
}

impl From<apotheke::DbError> for ReleasePersistError {
    fn from(source: apotheke::DbError) -> Self {
        Self::Database(source)
    }
}

/// Persists the `releases` row (and validates `want_id`) that MUST exist
/// before `queue.enqueue()` is called — shared by the HTTP route and the
/// MCP acquisition bridge (#651). `syntaxis::ImportService`'s production
/// adapter resolves both `want_id` and `release_id` against
/// `apotheke::repo::want::{get_want,get_release}` once the transfer
/// completes; a search result's `release_id` otherwise lives only in
/// eksetasis's in-memory cache, which does not survive a restart.
///
/// Idempotent: a retry with the SAME (release_id, want_id) pair reuses the
/// existing row rather than erroring on a duplicate insert.
///
/// INVARIANT: callers must `.await` this call BEFORE `queue.enqueue()`,
/// never after — that ordering is what makes the row durable ahead of the
/// transfer rather than racing it.
pub async fn persist_release_before_enqueue(
    db: &DbPools,
    want_id: Uuid,
    release_id: Uuid,
    metadata: ReleaseMetadata,
) -> Result<(), ReleasePersistError> {
    match want::get_want(&db.read, want_id.as_bytes().as_ref()).await? {
        Some(_) => {}
        None => return Err(ReleasePersistError::WantNotFound),
    }

    match want::get_release(&db.read, release_id.as_bytes().as_ref()).await? {
        Some(existing) if existing.want_id == want_id.as_bytes().to_vec() => {
            // NOTE: idempotent retry — this exact (release, want) pair is
            // already persisted (e.g. a prior enqueue attempt failed after
            // this insert but before the queue write) — reuse it.
        }
        Some(_other_want) => return Err(ReleasePersistError::ReleaseWantConflict),
        None => {
            let release = want::Release {
                id: release_id.as_bytes().to_vec(),
                want_id: want_id.as_bytes().to_vec(),
                indexer_id: metadata.indexer_id,
                title: metadata.title,
                size_bytes: i64::try_from(metadata.size_bytes.unwrap_or(0)).unwrap_or(i64::MAX),
                // WHY: no pre-download quality-assessment pipeline scores a
                // raw search hit or self-supplied URL yet — kritike::assessment
                // scores already-imported file tags (kathodos::sidecar
                // output), not release-name text. 0 is a neutral placeholder,
                // not a measured quality claim; upgrade decisions over this
                // row are unaffected since they compare against OTHER
                // releases of the same want, not an absolute threshold.
                quality_score: 0,
                custom_format_score: 0,
                download_url: metadata.download_url,
                protocol: metadata.protocol,
                info_hash: metadata.info_hash,
                found_at: jiff::Timestamp::now().to_string(),
                grabbed_at: None,
                rejected_reason: None,
            };
            want::insert_release(&db.write, &release).await?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct EnqueueRequest {
    pub want_id: String,
    pub release_id: String,
    /// Absent (or omitted) resolves the RELEASE server-side via
    /// `release_id` — the enqueue-by-reference path for a credentialed
    /// Torznab/Newznab search hit (#608). A magnet URI or a manual raw URL
    /// still goes here directly.
    pub download_url: Option<String>,
    #[serde(default = "default_protocol")]
    pub protocol: String,
    #[serde(default = "default_interactive_priority")]
    pub priority: u8,
    pub info_hash: Option<String>,
}
fn default_protocol() -> String {
    "torrent".to_string()
}
fn default_interactive_priority() -> u8 {
    4
}

#[derive(Deserialize)]
pub struct ReprioritizeRequest {
    pub priority: u8,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn get_queue_snapshot(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let active_q = format!(
        "{SELECT_DOWNLOAD} WHERE status IN ('downloading', 'post_processing', 'importing') \
         ORDER BY priority DESC, added_at ASC"
    );
    let active = sqlx::query_as::<_, DownloadRow>(&active_q)
        .fetch_all(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?;

    let queued_q =
        format!("{SELECT_DOWNLOAD} WHERE status = 'queued' ORDER BY priority DESC, added_at ASC");
    let queued = sqlx::query_as::<_, DownloadRow>(&queued_q)
        .fetch_all(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?;

    let completed_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM download_queue WHERE status = 'completed'")
            .fetch_one(&state.db.read)
            .await
            .map_err(|_| ParocheError::Internal)?;

    let failed_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM download_queue WHERE status = 'failed'")
            .fetch_one(&state.db.read)
            .await
            .map_err(|_| ParocheError::Internal)?;

    let snapshot = QueueSnapshotResponse {
        active: active.into_iter().map(Into::into).collect(),
        queued: queued.into_iter().map(Into::into).collect(),
        completed_count,
        failed_count,
    };

    Ok(ApiResponse::ok(snapshot))
}

// WHY: admin-only — enqueueing hands an arbitrary URL to the server-side
// download engine; member-level access is an SSRF primitive.
// WHY: enqueueing goes through the running syntaxis service, which persists
// the row and schedules it live — a raw DB INSERT here left the download
// invisible to the running queue until the next process restart.
pub async fn enqueue_download(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Json(body): Json<EnqueueRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let want_id = Uuid::parse_str(&body.want_id).map_err(|_| ParocheError::InvalidId)?;
    let release_id = Uuid::parse_str(&body.release_id).map_err(|_| ParocheError::InvalidId)?;

    // WHY: a present-but-empty/whitespace download_url stays a 422 (today's
    // behavior, unchanged); an ABSENT download_url resolves the release
    // server-side (#608) — the client never sees the indexer's credentialed
    // URL. The resolved protocol wins over the body's default; info_hash
    // falls back to the body's only when the resolved release carries none.
    // A self-supplied URL (this arm) never came from a cataloged indexer, so
    // it carries no title/size_bytes from the search cache — a magnet's own
    // dn/xl query params are the only metadata available; a plain file URL
    // gets a neutral placeholder (#651, mirrors the MCP bridge's manual-
    // magnet arm).
    let (download_url, protocol, info_hash, indexer_id, title, size_bytes) = if let Some(url) = body
        .download_url
        .as_deref()
        .filter(|url| !url.trim().is_empty())
    {
        let (title, size_bytes) = url::Url::parse(url)
            .ok()
            .filter(|parsed| parsed.scheme() == "magnet")
            .map(|parsed| magnet_release_metadata(&parsed))
            .unwrap_or((None, None));
        (
            url.to_string(),
            body.protocol.clone(),
            body.info_hash.clone(),
            MANUAL_RELEASE_INDEXER_ID,
            title.unwrap_or_else(|| "manual download".to_string()),
            size_bytes,
        )
    } else if body.download_url.is_some() {
        return Err(ParocheError::Validation {
            message: "download_url is required".to_string(),
        });
    } else {
        let resolved = state.search.resolve_release(release_id).await?;
        (
            resolved.download_url,
            resolved.protocol,
            resolved.info_hash.or_else(|| body.info_hash.clone()),
            resolved.indexer_id,
            resolved.title,
            resolved.size_bytes,
        )
    };

    // SAFETY: the by-reference path must not become an SSRF bypass — the
    // RESOLVED url is validated the same as a client-supplied one, never
    // trusted just because it came from the server-side cache.
    crate::net_validate::validate_download_url(&download_url).await?;

    // INVARIANT: the `releases` row (and `want_id`'s existence) must be
    // persisted BEFORE `queue.enqueue()` below — see
    // `persist_release_before_enqueue` (#651).
    persist_release_before_enqueue(
        &state.db,
        want_id,
        release_id,
        ReleaseMetadata::new(
            indexer_id,
            title,
            size_bytes,
            download_url.clone(),
            protocol.clone(),
            info_hash.clone(),
        )?,
    )
    .await?;

    let queue_id = Uuid::now_v7();

    state
        .queue
        .enqueue(crate::state::EnqueueItem {
            queue_id,
            want_id,
            release_id,
            download_url,
            protocol,
            priority: body.priority.clamp(1, 4),
            info_hash,
        })
        .await?;

    let q = format!("{SELECT_DOWNLOAD} WHERE id = ?");
    let row = sqlx::query_as::<_, DownloadRow>(&q)
        .bind(queue_id.as_bytes().as_slice())
        .fetch_one(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?;

    Ok(ApiResponse::created(DownloadResponse::from(row)))
}

// WHY: cancellation goes through the running syntaxis service so a live
// torrent session is actually stopped — a raw DB DELETE here left the
// download running in the engine while the API reported success.
pub async fn cancel_download(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;

    state.queue.cancel(uuid).await?;

    Ok(deleted())
}

// WHY: re-prioritization goes through the running syntaxis service so the
// live in-memory tier queue re-orders — a raw DB UPDATE here never changed
// what actually dispatched next.
pub async fn reprioritize_download(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
    Json(body): Json<ReprioritizeRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let priority = body.priority.clamp(1, 4);

    state.queue.reprioritize(uuid, priority).await?;

    let q = format!("{SELECT_DOWNLOAD} WHERE id = ?");
    let row = sqlx::query_as::<_, DownloadRow>(&q)
        .bind(uuid.as_bytes().as_slice())
        .fetch_optional(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?
        .ok_or(ParocheError::NotFound)?;

    Ok(ApiResponse::ok(DownloadResponse::from(row)))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn download_routes() -> axum::Router<AppState> {
    use axum::routing::{get, patch};
    axum::Router::new()
        .route("/", get(get_queue_snapshot).post(enqueue_download))
        .route("/{id}", axum::routing::delete(cancel_download))
        .route("/{id}/priority", patch(reprioritize_download))
}

#[cfg(test)]
mod tests {
    use apotheke::repo::want::{self, Want};
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use exousia::AuthService;
    use exousia::user::{CreateUserRequest, UserRole};
    use tower::ServiceExt;

    use super::*;
    use crate::state::{DynSearchService, ResolvedRelease, ServiceError, ServiceFut};
    use crate::test_helpers::test_state;

    async fn token_for(
        auth: &std::sync::Arc<exousia::ExousiaServiceImpl>,
        username: &str,
        role: UserRole,
    ) -> String {
        auth.create_user(CreateUserRequest {
            username: username.to_string(),
            display_name: username.to_string(),
            password: "password123".to_string(),
            role,
        })
        .await
        .unwrap();
        auth.login(username, "password123")
            .await
            .unwrap()
            .access_token
    }

    fn enqueue_body(want_id: &str, download_url: &str) -> String {
        serde_json::json!({
            "want_id": want_id,
            "release_id": uuid::Uuid::now_v7().to_string(),
            "download_url": download_url,
        })
        .to_string()
    }

    async fn post_enqueue(
        app: &axum::Router,
        token: &str,
        want_id: &str,
        download_url: &str,
    ) -> axum::response::Response {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/downloads")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(enqueue_body(want_id, download_url)))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn enqueue_download_rejects_unauthenticated() {
        let (state, _auth) = test_state().await;
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/downloads")
                    .header("content-type", "application/json")
                    .body(Body::from(enqueue_body(
                        &uuid::Uuid::now_v7().to_string(),
                        "http://203.0.113.10/f.torrent",
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn enqueue_download_requires_admin() {
        let (state, auth) = test_state().await;
        let token = token_for(&auth, "member", UserRole::Member).await;
        let app = crate::build_router(state);
        let resp = post_enqueue(
            &app,
            &token,
            &uuid::Uuid::now_v7().to_string(),
            "http://203.0.113.10/f.torrent",
        )
        .await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn enqueue_download_rejects_private_hosts() {
        let (state, auth) = test_state().await;
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let app = crate::build_router(state);

        for url in [
            "http://127.0.0.1/f.torrent",
            "http://169.254.169.254/latest/meta-data/",
            "http://10.0.0.5/f.torrent",
            "http://192.168.1.10/f.torrent",
            "http://[::1]/f.torrent",
        ] {
            let resp = post_enqueue(&app, &token, &uuid::Uuid::now_v7().to_string(), url).await;
            assert_eq!(
                resp.status(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "expected 422 for {url}"
            );
        }

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM download_queue")
            .fetch_one(&app_pool(&auth))
            .await
            .unwrap();
        assert_eq!(count, 0, "no rejected URL may reach the queue");
    }

    fn app_pool(auth: &std::sync::Arc<exousia::ExousiaServiceImpl>) -> sqlx::SqlitePool {
        auth.pools().read.clone()
    }

    /// Persists a real `wants` row (#651: `enqueue_download` now validates
    /// `want_id` before enqueueing) using the migrator's seeded default
    /// music quality profile.
    async fn seed_want(pool: &sqlx::SqlitePool) -> Uuid {
        let profile_id: i64 = sqlx::query_scalar(
            "SELECT id FROM quality_profiles WHERE media_type = 'music' LIMIT 1",
        )
        .fetch_one(pool)
        .await
        .expect("migrator seeds a default music quality profile");
        let want_id = Uuid::now_v7();
        want::insert_want(
            pool,
            &Want {
                id: want_id.as_bytes().to_vec(),
                media_type: "music_album".to_string(),
                title: "Test Want".to_string(),
                registry_id: None,
                quality_profile_id: profile_id,
                status: "searching".to_string(),
                source: None,
                source_ref: None,
                added_at: jiff::Timestamp::now().to_string(),
                fulfilled_at: None,
            },
        )
        .await
        .expect("seed want inserts");
        want_id
    }

    #[tokio::test]
    async fn enqueue_download_rejects_non_http_schemes() {
        let (state, auth) = test_state().await;
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let app = crate::build_router(state);

        for url in ["ftp://203.0.113.10/f.torrent", "file:///etc/passwd"] {
            let resp = post_enqueue(&app, &token, &uuid::Uuid::now_v7().to_string(), url).await;
            assert_eq!(
                resp.status(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "expected 422 for {url}"
            );
        }
    }

    /// Recording queue manager that also persists, mirroring the real
    /// service's contract (`enqueue` owns the `download_queue` write).
    fn persisting_queue(pool: &sqlx::SqlitePool) -> std::sync::Arc<RecordingQueueManager> {
        std::sync::Arc::new(RecordingQueueManager {
            persist_pool: Some(pool.clone()),
            ..Default::default()
        })
    }

    #[tokio::test]
    async fn enqueue_download_accepts_public_url_for_admin() {
        let (mut state, auth) = test_state().await;
        let pool = app_pool(&auth);
        state.queue = persisting_queue(&pool);
        let want_id = seed_want(&pool).await.to_string();
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let app = crate::build_router(state);
        let resp = post_enqueue(&app, &token, &want_id, "http://203.0.113.10/f.torrent").await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body["data"]["download_url"],
            "http://203.0.113.10/f.torrent"
        );
        assert_eq!(body["data"]["status"], "queued");
    }

    #[tokio::test]
    async fn enqueue_download_accepts_magnet_uri_for_admin() {
        let (mut state, auth) = test_state().await;
        let pool = app_pool(&auth);
        state.queue = persisting_queue(&pool);
        let want_id = seed_want(&pool).await.to_string();
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let app = crate::build_router(state);
        let resp = post_enqueue(&app, &token, &want_id, "magnet:?xt=urn:btih:abc123def456").await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // ── #651: releases/wants persisted before enqueue (HTTP surface) ───────

    #[tokio::test]
    async fn enqueue_download_rejects_unknown_want_id() {
        // WHY: #651 — a well-formed but non-existent want_id must be
        // rejected, not accepted and left to fail import later. Mirrors the
        // MCP bridge's own `enqueue_rejects_unknown_want_id`.
        let (mut state, auth) = test_state().await;
        let pool = app_pool(&auth);
        state.queue = persisting_queue(&pool);
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let app = crate::build_router(state);

        let resp = post_enqueue(
            &app,
            &token,
            &uuid::Uuid::now_v7().to_string(),
            "magnet:?xt=urn:btih:unknownwant",
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM download_queue")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "an unresolvable want must never reach the queue");
    }

    #[tokio::test]
    async fn enqueue_download_magnet_arm_persists_release_row_from_dn_and_xl() {
        // WHY: #651's release-row gap on the HTTP surface's self-supplied-URL
        // arm — a magnet posted directly (no by-reference resolve) still
        // needs a durable `releases` row, derived from the magnet URI's own
        // dn/xl metadata params, mirroring the MCP bridge's magnet arm.
        let (mut state, auth) = test_state().await;
        let pool = app_pool(&auth);
        state.queue = persisting_queue(&pool);
        let want_id = seed_want(&pool).await.to_string();
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let app = crate::build_router(state);

        let resp = post_enqueue(
            &app,
            &token,
            &want_id,
            "magnet:?xt=urn:btih:aabbccddeeff00112233445566778899aabbccdd&dn=My.Release&xl=123456",
        )
        .await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let row: (String, i64, i64) =
            sqlx::query_as("SELECT title, size_bytes, indexer_id FROM releases WHERE want_id = ?")
                .bind(
                    uuid::Uuid::parse_str(&want_id)
                        .unwrap()
                        .as_bytes()
                        .as_slice(),
                )
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "My.Release");
        assert_eq!(row.1, 123_456);
        assert_eq!(row.2, MANUAL_RELEASE_INDEXER_ID);
    }

    #[tokio::test]
    async fn enqueue_download_plain_url_persists_release_row_with_placeholder_title() {
        // WHY: #651 — a plain (non-magnet) direct file URL carries no dn/xl
        // metadata at all; it still gets a durable `releases` row, using the
        // neutral placeholder title rather than leaving the row unwritten.
        let (mut state, auth) = test_state().await;
        let pool = app_pool(&auth);
        state.queue = persisting_queue(&pool);
        let want_id = seed_want(&pool).await.to_string();
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let app = crate::build_router(state);

        let resp = post_enqueue(&app, &token, &want_id, "http://203.0.113.10/f.torrent").await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let row: (String, i64) =
            sqlx::query_as("SELECT title, indexer_id FROM releases WHERE want_id = ?")
                .bind(
                    uuid::Uuid::parse_str(&want_id)
                        .unwrap()
                        .as_bytes()
                        .as_slice(),
                )
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "manual download");
        assert_eq!(row.1, MANUAL_RELEASE_INDEXER_ID);
    }

    // ── #499: enqueue reaches the live queue manager ────────────────────────

    #[tokio::test]
    async fn enqueue_download_reaches_queue_manager_not_raw_db() {
        let (mut state, auth) = test_state().await;
        let pool = app_pool(&auth);
        let queue = persisting_queue(&pool);
        state.queue = queue.clone();
        let want_id = seed_want(&pool).await.to_string();
        let token = token_for(&auth, "admin", UserRole::Admin).await;

        let app = crate::build_router(state);
        let resp = post_enqueue(&app, &token, &want_id, "magnet:?xt=urn:btih:live499").await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        let enqueued = queue.enqueued.lock().unwrap().clone();
        assert_eq!(
            enqueued.len(),
            1,
            "the enqueue must be delegated to the queue manager"
        );
        let item = &enqueued[0];
        assert_eq!(item.download_url, "magnet:?xt=urn:btih:live499");
        assert_eq!(item.protocol, "torrent");
        assert_eq!(
            item.priority, 4,
            "the interactive default must reach the queue"
        );
        assert_eq!(
            body["data"]["id"],
            item.queue_id.to_string(),
            "the response must reference the row the queue manager persisted"
        );
        assert_eq!(body["data"]["status"], "queued");

        // The queue manager owns the DB write; a raw route INSERT would add a
        // second row (or violate the primary key and fail the request).
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM download_queue")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            count, 1,
            "the route must not raw-INSERT alongside the service"
        );
    }

    #[tokio::test]
    async fn enqueue_download_persists_the_release_row_before_calling_queue_enqueue() {
        // WHY: #651's core ordering/durability assertion, mirrored from the
        // MCP bridge's own `enqueue_persists_the_release_row_before_calling_queue_enqueue`
        // test — an end-state check alone ("does a releases row exist once
        // the request returns") would still pass if `enqueue_download`
        // persisted the release AFTER calling `queue.enqueue()`.
        // `RecordingQueueManager` records whether the row already existed
        // the INSTANT it ran, so this fails under that reordering even
        // though the final state would look identical.
        let (mut state, auth) = test_state().await;
        let pool = app_pool(&auth);
        let queue = persisting_queue(&pool);
        state.queue = queue.clone();
        let want_id = seed_want(&pool).await.to_string();
        let token = token_for(&auth, "admin", UserRole::Admin).await;

        let app = crate::build_router(state);
        let resp = post_enqueue(&app, &token, &want_id, "magnet:?xt=urn:btih:orderingprobe").await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let ordering_flags = queue.release_existed_at_enqueue.lock().unwrap().clone();
        assert_eq!(
            ordering_flags,
            vec![true],
            "the releases row must be written before queue.enqueue() is called, not after"
        );

        let release_id = queue.enqueued.lock().unwrap()[0].release_id;
        let persisted_want_id: Vec<u8> =
            sqlx::query_scalar("SELECT want_id FROM releases WHERE id = ?")
                .bind(release_id.as_bytes().as_slice())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            persisted_want_id,
            uuid::Uuid::parse_str(&want_id).unwrap().as_bytes().to_vec()
        );
    }

    #[tokio::test]
    async fn enqueue_download_unavailable_when_queue_not_wired() {
        let (state, auth) = test_state().await;
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let pool = app_pool(&auth);
        let want_id = seed_want(&pool).await.to_string();

        let app = crate::build_router(state);
        let resp = post_enqueue(&app, &token, &want_id, "magnet:?xt=urn:btih:nowire").await;
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

        // A download the live queue never accepted must not be persisted — a
        // DB-only row is exactly the #499 defect (invisible until restart).
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM download_queue")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "no row may exist without the live queue knowing");
    }

    #[tokio::test]
    async fn enqueue_download_rejects_magnet_with_private_tracker() {
        let (state, auth) = test_state().await;
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let app = crate::build_router(state);
        let resp = post_enqueue(
            &app,
            &token,
            &uuid::Uuid::now_v7().to_string(),
            "magnet:?xt=urn:btih:abc&tr=http%3A%2F%2F127.0.0.1%3A8080%2Fannounce",
        )
        .await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn enqueue_download_rejects_whitespace_only_download_url() {
        let (state, auth) = test_state().await;
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let app = crate::build_router(state);
        let resp = post_enqueue(&app, &token, &uuid::Uuid::now_v7().to_string(), "   ").await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    // ── #608: enqueue-by-reference — resolve_release server-side join ───────

    /// Search stub whose `resolve_release` answers with a fixed outcome — a
    /// stand-in for eksetasis's results cache.
    enum ResolveStub {
        Found {
            download_url: String,
            protocol: String,
            info_hash: Option<String>,
        },
        Miss,
    }

    // WHY: this route's own resolve/enqueue flow under test never reads
    // title/size_bytes/indexer_id — they exist on ResolvedRelease only for
    // the acquisition-bridge persistence path (#651), so the stub fills
    // them with fixed placeholders rather than threading real values
    // through every `ResolveStub::Found` call site.
    const STUB_TITLE: &str = "Stub.Release.Title";
    const STUB_SIZE_BYTES: Option<u64> = Some(1_000_000);
    const STUB_INDEXER_ID: i64 = 1;

    struct StubResolveSearch(ResolveStub);

    impl DynSearchService for StubResolveSearch {
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
            match &self.0 {
                ResolveStub::Found {
                    download_url,
                    protocol,
                    info_hash,
                } => {
                    let download_url = download_url.clone();
                    let protocol = protocol.clone();
                    let info_hash = info_hash.clone();
                    Box::pin(async move {
                        Ok(ResolvedRelease {
                            download_url,
                            protocol,
                            info_hash,
                            indexer_id: STUB_INDEXER_ID,
                            title: STUB_TITLE.to_string(),
                            size_bytes: STUB_SIZE_BYTES,
                        })
                    })
                }
                ResolveStub::Miss => Box::pin(async { Err(ServiceError::NotFound) }),
            }
        }
    }

    fn resolve_body(want_id: &str, protocol: Option<&str>) -> String {
        let mut body = serde_json::json!({
            "want_id": want_id,
            "release_id": uuid::Uuid::now_v7().to_string(),
        });
        if let Some(protocol) = protocol {
            body["protocol"] = serde_json::Value::String(protocol.to_string());
        }
        body.to_string()
    }

    async fn post_enqueue_body(
        app: &axum::Router,
        token: &str,
        body: String,
    ) -> axum::response::Response {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/downloads")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn enqueue_download_resolves_release_when_download_url_absent() {
        let (mut state, auth) = test_state().await;
        let pool = app_pool(&auth);
        let queue = persisting_queue(&pool);
        state.queue = queue.clone();
        state.search = std::sync::Arc::new(StubResolveSearch(ResolveStub::Found {
            download_url: "http://203.0.113.10/dl/42?apikey=SECRET".to_string(),
            protocol: "torrent".to_string(),
            info_hash: None,
        }));
        let want_id = seed_want(&pool).await.to_string();
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let app = crate::build_router(state);

        let resp = post_enqueue_body(&app, &token, resolve_body(&want_id, None)).await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let enqueued = queue.enqueued.lock().unwrap().clone();
        assert_eq!(
            enqueued.len(),
            1,
            "the resolved release must reach the queue manager"
        );
        assert_eq!(
            enqueued[0].download_url, "http://203.0.113.10/dl/42?apikey=SECRET",
            "the queue must receive the UNREDACTED resolved URL"
        );

        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body["data"]["download_url"], "http://203.0.113.10/dl/42?apikey=REDACTED",
            "the HTTP response must carry the REDACTED URL"
        );
        assert!(
            !String::from_utf8_lossy(&bytes).contains("SECRET"),
            "the raw indexer credential must never reach the response body"
        );
    }

    #[tokio::test]
    async fn enqueue_download_resolve_miss_enqueues_nothing_and_returns_404() {
        let (mut state, auth) = test_state().await;
        let queue = persisting_queue(&app_pool(&auth));
        state.queue = queue.clone();
        state.search = std::sync::Arc::new(StubResolveSearch(ResolveStub::Miss));
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let pool = app_pool(&auth);
        let app = crate::build_router(state);

        let resp = post_enqueue_body(
            &app,
            &token,
            resolve_body(&uuid::Uuid::now_v7().to_string(), None),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert!(queue.enqueued.lock().unwrap().is_empty());

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM download_queue")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "an unresolved release must not reach the queue");
    }

    #[tokio::test]
    async fn enqueue_download_rejects_resolved_url_to_private_space() {
        let (mut state, auth) = test_state().await;
        let queue = persisting_queue(&app_pool(&auth));
        state.queue = queue.clone();
        state.search = std::sync::Arc::new(StubResolveSearch(ResolveStub::Found {
            download_url: "http://127.0.0.1/dl/42?apikey=SECRET".to_string(),
            protocol: "torrent".to_string(),
            info_hash: None,
        }));
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let pool = app_pool(&auth);
        let app = crate::build_router(state);

        let resp = post_enqueue_body(
            &app,
            &token,
            resolve_body(&uuid::Uuid::now_v7().to_string(), None),
        )
        .await;
        assert_eq!(
            resp.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "SSRF validation must run on the RESOLVED url too — by-reference \
             must not become the SSRF bypass"
        );
        assert!(queue.enqueued.lock().unwrap().is_empty());

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM download_queue")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn enqueue_download_resolved_protocol_wins_over_body_default() {
        let (mut state, auth) = test_state().await;
        let pool = app_pool(&auth);
        let queue = persisting_queue(&pool);
        state.queue = queue.clone();
        state.search = std::sync::Arc::new(StubResolveSearch(ResolveStub::Found {
            download_url: "http://203.0.113.10/dl/42.nzb?apikey=SECRET".to_string(),
            protocol: "nzb".to_string(),
            info_hash: None,
        }));
        let want_id = seed_want(&pool).await.to_string();
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let app = crate::build_router(state);

        // Body carries the interactive default ("torrent"); the resolved
        // release is nzb and must win.
        let resp = post_enqueue_body(&app, &token, resolve_body(&want_id, Some("torrent"))).await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let enqueued = queue.enqueued.lock().unwrap().clone();
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].protocol, "nzb");
    }

    // ── #469/#499: enqueue/cancel/reprioritize reach the live queue manager ─

    #[derive(Default)]
    struct RecordingQueueManager {
        enqueued: std::sync::Mutex<Vec<crate::state::EnqueueItem>>,
        cancelled: std::sync::Mutex<Vec<uuid::Uuid>>,
        reprioritized: std::sync::Mutex<Vec<(uuid::Uuid, u8)>>,
        fail_with: std::sync::Mutex<Option<fn() -> crate::state::ServiceError>>,
        /// When set, `enqueue` persists the row like the real syntaxis service
        /// does, so the handler's response SELECT has a row to read.
        persist_pool: Option<sqlx::SqlitePool>,
        /// ORDERING PROBE (#651): for each `enqueue()` call, whether a
        /// `releases` row for that item's `release_id` already existed AT
        /// THE MOMENT this stub ran. An end-state check alone ("does the row
        /// exist once the request returns") would still pass if
        /// `enqueue_download` persisted the release AFTER calling
        /// `queue.enqueue()` — this catches exactly that reordering, mirroring
        /// the MCP bridge's own `StubQueue` probe. Arc-wrapped (not a bare
        /// `Mutex`) because `ServiceFut` requires `'static`, so the future
        /// below can only capture an owned clone, never `&self`.
        release_existed_at_enqueue: std::sync::Arc<std::sync::Mutex<Vec<bool>>>,
    }

    impl crate::state::DynQueueManager for RecordingQueueManager {
        fn enqueue(&self, item: crate::state::EnqueueItem) -> crate::state::ServiceFut<()> {
            if let Some(make_error) = *self.fail_with.lock().unwrap() {
                return Box::pin(async move { Err(make_error()) });
            }
            self.enqueued.lock().unwrap().push(item.clone());
            let pool = self.persist_pool.clone();
            let release_existed = std::sync::Arc::clone(&self.release_existed_at_enqueue);
            Box::pin(async move {
                if let Some(pool) = pool {
                    let existed: i64 =
                        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM releases WHERE id = ?)")
                            .bind(item.release_id.as_bytes().as_slice())
                            .fetch_one(&pool)
                            .await
                            .unwrap_or(0);
                    release_existed.lock().unwrap().push(existed != 0);
                    sqlx::query(
                        "INSERT INTO download_queue \
                         (id, want_id, release_id, download_url, protocol, priority, info_hash) \
                         VALUES (?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind(item.queue_id.as_bytes().as_slice())
                    .bind(item.want_id.as_bytes().as_slice())
                    .bind(item.release_id.as_bytes().as_slice())
                    .bind(&item.download_url)
                    .bind(&item.protocol)
                    .bind(i64::from(item.priority))
                    .bind(&item.info_hash)
                    .execute(&pool)
                    .await
                    .map_err(|e| crate::state::ServiceError::Internal(e.to_string()))?;
                }
                Ok(())
            })
        }

        fn cancel(&self, queue_id: uuid::Uuid) -> crate::state::ServiceFut<()> {
            if let Some(make_error) = *self.fail_with.lock().unwrap() {
                return Box::pin(async move { Err(make_error()) });
            }
            self.cancelled.lock().unwrap().push(queue_id);
            Box::pin(async { Ok(()) })
        }

        fn reprioritize(&self, queue_id: uuid::Uuid, priority: u8) -> crate::state::ServiceFut<()> {
            if let Some(make_error) = *self.fail_with.lock().unwrap() {
                return Box::pin(async move { Err(make_error()) });
            }
            self.reprioritized
                .lock()
                .unwrap()
                .push((queue_id, priority));
            Box::pin(async { Ok(()) })
        }
    }

    async fn insert_download_row(pool: &sqlx::SqlitePool, id: uuid::Uuid, priority: i64) {
        sqlx::query(
            "INSERT INTO download_queue \
             (id, want_id, release_id, download_url, protocol, priority, \
              status, added_at, retry_count) \
             VALUES (?, ?, ?, 'magnet:?xt=urn:btih:row', 'torrent', ?, \
                     'queued', '2026-01-01T00:00:00Z', 0)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(uuid::Uuid::now_v7().as_bytes().as_slice())
        .bind(uuid::Uuid::now_v7().as_bytes().as_slice())
        .bind(priority)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn cancel_download_reaches_queue_manager_not_raw_db() {
        let (mut state, auth) = test_state().await;
        let queue = std::sync::Arc::new(RecordingQueueManager::default());
        state.queue = queue.clone();
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let pool = app_pool(&auth);

        let id = uuid::Uuid::now_v7();
        insert_download_row(&pool, id, 2).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/downloads/{id}"))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            *queue.cancelled.lock().unwrap(),
            vec![id],
            "the cancel must be delegated to the queue manager"
        );
        // The queue manager owns the DB write; the route must not raw-DELETE.
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM download_queue WHERE id = ?")
            .bind(id.as_bytes().as_slice())
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            count, 1,
            "the route must not bypass the service with raw SQL"
        );
    }

    #[tokio::test]
    async fn cancel_download_maps_service_not_found() {
        let (mut state, auth) = test_state().await;
        let queue = std::sync::Arc::new(RecordingQueueManager::default());
        *queue.fail_with.lock().unwrap() = Some(|| crate::state::ServiceError::NotFound);
        state.queue = queue;
        let token = token_for(&auth, "admin", UserRole::Admin).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/downloads/{}", uuid::Uuid::now_v7()))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn cancel_download_unavailable_when_queue_not_wired() {
        let (state, auth) = test_state().await;
        let token = token_for(&auth, "admin", UserRole::Admin).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/downloads/{}", uuid::Uuid::now_v7()))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn cancel_download_rejects_unauthenticated() {
        let (state, _auth) = test_state().await;
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/downloads/{}", uuid::Uuid::now_v7()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn cancel_download_rejects_member() {
        let (state, auth) = test_state().await;
        let token = token_for(&auth, "member", UserRole::Member).await;
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/downloads/{}", uuid::Uuid::now_v7()))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn reprioritize_download_reaches_queue_manager() {
        let (mut state, auth) = test_state().await;
        let queue = std::sync::Arc::new(RecordingQueueManager::default());
        state.queue = queue.clone();
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let pool = app_pool(&auth);

        let id = uuid::Uuid::now_v7();
        insert_download_row(&pool, id, 2).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/v1/downloads/{id}/priority"))
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"priority": 4}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            *queue.reprioritized.lock().unwrap(),
            vec![(id, 4)],
            "the re-prioritization must be delegated to the queue manager"
        );
    }

    // ── #539: member-visible responses must not leak indexer credentials ────

    #[tokio::test]
    async fn queue_snapshot_redacts_indexer_credentials() {
        let (state, auth) = test_state().await;
        let token = token_for(&auth, "member", UserRole::Member).await;
        let pool = app_pool(&auth);

        let id = uuid::Uuid::now_v7();
        sqlx::query(
            "INSERT INTO download_queue \
             (id, want_id, release_id, download_url, protocol, priority, \
              status, added_at, retry_count) \
             VALUES (?, ?, ?, ?, 'torrent', 2, 'queued', \
                     '2026-01-01T00:00:00Z', 0)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(uuid::Uuid::now_v7().as_bytes().as_slice())
        .bind(uuid::Uuid::now_v7().as_bytes().as_slice())
        .bind("https://indexer.example/api?t=get&id=42&apikey=SECRET")
        .execute(&pool)
        .await
        .unwrap();

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/downloads")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body["data"]["queued"][0]["download_url"],
            "https://indexer.example/api?t=get&id=42&apikey=REDACTED"
        );
        assert!(
            !String::from_utf8_lossy(&bytes).contains("SECRET"),
            "the raw indexer credential must never reach the response body"
        );
    }

    #[tokio::test]
    async fn reprioritize_download_rejects_invalid_id() {
        let (state, auth) = test_state().await;
        let token = token_for(&auth, "admin", UserRole::Admin).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/downloads/not-a-uuid/priority")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"priority": 2}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
