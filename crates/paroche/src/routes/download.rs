/// Download queue endpoints.
use axum::{
    Json,
    extract::{Path, State},
};
use exousia::{AuthenticatedUser, RequireAdmin};
use serde::{Deserialize, Serialize};
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
    if body.download_url.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "download_url is required".to_string(),
        });
    }

    crate::net_validate::validate_download_url(&body.download_url).await?;

    let queue_id = Uuid::now_v7();
    let want_id = Uuid::parse_str(&body.want_id).map_err(|_| ParocheError::InvalidId)?;
    let release_id = Uuid::parse_str(&body.release_id).map_err(|_| ParocheError::InvalidId)?;

    state
        .queue
        .enqueue(crate::state::EnqueueItem {
            queue_id,
            want_id,
            release_id,
            download_url: body.download_url,
            protocol: body.protocol,
            priority: body.priority.clamp(1, 4),
            info_hash: body.info_hash,
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
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use exousia::AuthService;
    use exousia::user::{CreateUserRequest, UserRole};
    use tower::ServiceExt;

    #[expect(
        unused_imports,
        reason = "kanon: test-missing-use-super; parent items accessed via explicit super:: prefix in test bodies"
    )]
    use super::*;
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

    fn enqueue_body(download_url: &str) -> String {
        serde_json::json!({
            "want_id": uuid::Uuid::now_v7().to_string(),
            "release_id": uuid::Uuid::now_v7().to_string(),
            "download_url": download_url,
        })
        .to_string()
    }

    async fn post_enqueue(
        app: &axum::Router,
        token: &str,
        download_url: &str,
    ) -> axum::response::Response {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/downloads")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(enqueue_body(download_url)))
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
                    .body(Body::from(enqueue_body("http://203.0.113.10/f.torrent")))
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
        let resp = post_enqueue(&app, &token, "http://203.0.113.10/f.torrent").await;
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
            let resp = post_enqueue(&app, &token, url).await;
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

    #[tokio::test]
    async fn enqueue_download_rejects_non_http_schemes() {
        let (state, auth) = test_state().await;
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let app = crate::build_router(state);

        for url in ["ftp://203.0.113.10/f.torrent", "file:///etc/passwd"] {
            let resp = post_enqueue(&app, &token, url).await;
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
        state.queue = persisting_queue(&app_pool(&auth));
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let app = crate::build_router(state);
        let resp = post_enqueue(&app, &token, "http://203.0.113.10/f.torrent").await;
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
        state.queue = persisting_queue(&app_pool(&auth));
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let app = crate::build_router(state);
        let resp = post_enqueue(&app, &token, "magnet:?xt=urn:btih:abc123def456").await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // ── #499: enqueue reaches the live queue manager ────────────────────────

    #[tokio::test]
    async fn enqueue_download_reaches_queue_manager_not_raw_db() {
        let (mut state, auth) = test_state().await;
        let pool = app_pool(&auth);
        let queue = persisting_queue(&pool);
        state.queue = queue.clone();
        let token = token_for(&auth, "admin", UserRole::Admin).await;

        let app = crate::build_router(state);
        let resp = post_enqueue(&app, &token, "magnet:?xt=urn:btih:live499").await;
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
    async fn enqueue_download_unavailable_when_queue_not_wired() {
        let (state, auth) = test_state().await;
        let token = token_for(&auth, "admin", UserRole::Admin).await;
        let pool = app_pool(&auth);

        let app = crate::build_router(state);
        let resp = post_enqueue(&app, &token, "magnet:?xt=urn:btih:nowire").await;
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
            "magnet:?xt=urn:btih:abc&tr=http%3A%2F%2F127.0.0.1%3A8080%2Fannounce",
        )
        .await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
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
    }

    impl crate::state::DynQueueManager for RecordingQueueManager {
        fn enqueue(&self, item: crate::state::EnqueueItem) -> crate::state::ServiceFut<()> {
            if let Some(make_error) = *self.fail_with.lock().unwrap() {
                return Box::pin(async move { Err(make_error()) });
            }
            self.enqueued.lock().unwrap().push(item.clone());
            let pool = self.persist_pool.clone();
            Box::pin(async move {
                if let Some(pool) = pool {
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
