/// Download queue endpoints.
use axum::{
    Json,
    extract::{Path, State},
};
use exousia::AuthenticatedUser;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::ParocheError,
    response::{ApiResponse, deleted},
    state::AppState,
};

// ---------------------------------------------------------------------------
// Row / response types
// ---------------------------------------------------------------------------

fn bytes_to_uuid_str(bytes: &[u8]) -> String {
    Uuid::from_slice(bytes)
        .map(|u| u.to_string())
        .unwrap_or_default()
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
            download_url: r.download_url,
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
// Request types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct EnqueueRequest {
    pub want_id: String,
    pub release_id: String,
    pub download_url: String,
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

pub async fn enqueue_download(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<EnqueueRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    if body.download_url.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "download_url is required".to_string(),
        });
    }

    let id = Uuid::now_v7().as_bytes().to_vec();
    let want_id = Uuid::parse_str(&body.want_id)
        .map_err(|_| ParocheError::InvalidId)?
        .as_bytes()
        .to_vec();
    let release_id = Uuid::parse_str(&body.release_id)
        .map_err(|_| ParocheError::InvalidId)?
        .as_bytes()
        .to_vec();
    let now = crate::routes::music::chrono_now_pub();
    let priority = body.priority.clamp(1, 4) as i64;

    sqlx::query(
        "INSERT INTO download_queue \
         (id, want_id, release_id, download_url, protocol, priority, info_hash, \
          status, added_at, retry_count) \
         VALUES (?, ?, ?, ?, ?, ?, ?, 'queued', ?, 0)",
    )
    .bind(&id)
    .bind(&want_id)
    .bind(&release_id)
    .bind(&body.download_url)
    .bind(&body.protocol)
    .bind(priority)
    .bind(&body.info_hash)
    .bind(&now)
    .execute(&state.db.write)
    .await
    .map_err(|_| ParocheError::Internal)?;

    let q = format!("{SELECT_DOWNLOAD} WHERE id = ?");
    let row = sqlx::query_as::<_, DownloadRow>(&q)
        .bind(&id)
        .fetch_one(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?;

    Ok(ApiResponse::created(DownloadResponse::from(row)))
}

pub async fn cancel_download(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let affected = sqlx::query("DELETE FROM download_queue WHERE id = ?")
        .bind(&id_bytes)
        .execute(&state.db.write)
        .await
        .map_err(|_| ParocheError::Internal)?
        .rows_affected();

    if affected == 0 {
        return Err(ParocheError::NotFound);
    }

    Ok(deleted())
}

pub async fn reprioritize_download(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
    Json(body): Json<ReprioritizeRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();
    let priority = body.priority.clamp(1, 4) as i64;

    let affected = sqlx::query("UPDATE download_queue SET priority = ? WHERE id = ?")
        .bind(priority)
        .bind(&id_bytes)
        .execute(&state.db.write)
        .await
        .map_err(|_| ParocheError::Internal)?
        .rows_affected();

    if affected == 0 {
        return Err(ParocheError::NotFound);
    }

    let q = format!("{SELECT_DOWNLOAD} WHERE id = ?");
    let row = sqlx::query_as::<_, DownloadRow>(&q)
        .bind(&id_bytes)
        .fetch_one(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?;

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
