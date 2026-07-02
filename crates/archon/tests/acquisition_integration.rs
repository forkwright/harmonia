//! Integration tests for the acquisition pipeline (P103).
//!
//! Validates search → queue → download → extract → import flow, queue
//! management, HTTP endpoint correctness, and auth enforcement against the
//! full Paroche router backed by in-memory SQLite.

use std::pin::Pin;
use std::sync::Arc;

use aitesis::{IdentityValidator, MonitorService, RequestService, UserRoleProvider};
use apotheke::DbPools;
use apotheke::migrate::MIGRATOR;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use ergasia::{DownloadProgress, DownloadState, ErgasiaError, ExtractionResult};
use exousia::{AuthService, CreateUserRequest, ExousiaServiceImpl, UserRole};
use horismos::{AitesisConfig, Config, ExousiaConfig};
use paroche::state::{
    AppState, DynQueueManager, DynRequestService, DynSearchService, RequestServiceFut, ServiceFut,
};
use serde_json::{Value, json};
use snafu::ResultExt;
use sqlx::SqlitePool;
use syntaxis::{CompletedDownload, ImportService, QueueManager};
use themelion::create_event_bus;
use themelion::ids::DownloadId;
use tokio::sync::mpsc;
use tower::ServiceExt;
use uuid::Uuid;

// ── Mock search service ──────────────────────────────────────────────────────

struct MockSearchService;

impl DynSearchService for MockSearchService {
    fn search(&self, _query: Value) -> ServiceFut<Value> {
        Box::pin(async {
            Ok(json!({
                "results": [{
                    "title": "Test Album - FLAC",
                    "download_url": "magnet:?xt=urn:btih:abc123def456",
                    "size_bytes": 500_000_000,
                    "seeders": 42,
                    "protocol": "torrent"
                }]
            }))
        })
    }
    fn test_indexer(&self, _indexer_id: i64) -> ServiceFut<Value> {
        Box::pin(async { Ok(json!({"healthy": true})) })
    }
    fn refresh_caps(&self, _indexer_id: i64) -> ServiceFut<Value> {
        Box::pin(async { Ok(json!({"caps": []})) })
    }
}

// ── Mock download engine ─────────────────────────────────────────────────────

struct MockEngine {
    started_tx: mpsc::UnboundedSender<DownloadId>,
}

impl ergasia::DownloadEngine for MockEngine {
    async fn start_download(
        &self,
        request: ergasia::DownloadRequest,
    ) -> Result<DownloadId, ErgasiaError> {
        let _ = self.started_tx.send(request.download_id);
        Ok(request.download_id)
    }

    async fn cancel_download(&self, _download_id: DownloadId) -> Result<(), ErgasiaError> {
        Ok(())
    }

    async fn get_progress(
        &self,
        download_id: DownloadId,
    ) -> Result<DownloadProgress, ErgasiaError> {
        Ok(DownloadProgress {
            download_id,
            state: DownloadState::Downloading,
            percent_complete: 50,
            download_speed_bps: 1_000_000,
            upload_speed_bps: 100_000,
            peers_connected: 5,
            seeders: 10,
            eta_seconds: Some(300),
        })
    }

    async fn extract(
        &self,
        _download_path: &std::path::Path,
        _output_dir: &std::path::Path,
    ) -> Result<Option<ExtractionResult>, ErgasiaError> {
        Ok(None)
    }
}

// ── Queue-manager adapter over the real syntaxis service ────────────────────

struct QueueAdapter(Arc<syntaxis::DownloadQueue<MockEngine>>);

impl DynQueueManager for QueueAdapter {
    fn enqueue(&self, item: paroche::state::EnqueueItem) -> ServiceFut<()> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            let protocol = syntaxis::DownloadProtocol::parse(&item.protocol).ok_or_else(|| {
                paroche::state::ServiceError::InvalidInput(format!(
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
                .map_err(queue_service_error)
        })
    }

    fn cancel(&self, queue_id: Uuid) -> ServiceFut<()> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .cancel_by_queue_id(queue_id)
                .await
                .map_err(queue_service_error)
        })
    }

    fn reprioritize(&self, queue_id: Uuid, priority: u8) -> ServiceFut<()> {
        let service = Arc::clone(&self.0);
        Box::pin(async move {
            service
                .reprioritize_by_queue_id(queue_id, priority)
                .await
                .map_err(queue_service_error)
        })
    }
}

fn queue_service_error(error: syntaxis::SyntaxisError) -> paroche::state::ServiceError {
    match error {
        syntaxis::SyntaxisError::ItemNotFound { .. } => paroche::state::ServiceError::NotFound,
        other => paroche::state::ServiceError::Internal(other.to_string()),
    }
}

// ── Mock import service ──────────────────────────────────────────────────────

struct MockImportService {
    imported_tx: mpsc::UnboundedSender<DownloadId>,
}

impl ImportService for MockImportService {
    fn import(
        &self,
        completed: CompletedDownload,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        let tx = self.imported_tx.clone();
        Box::pin(async move {
            let _ = tx.send(completed.download_id);
            Ok(())
        })
    }
}

// ── Mock request service boundary ────────────────────────────────────────────

type MockRequestService =
    aitesis::AitesisServiceImpl<MockRequestRoles, MockRequestIdentity, MockRequestMonitor>;

struct MockRequestAdapter(Arc<MockRequestService>);

impl DynRequestService for MockRequestAdapter {
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

struct MockRequestRoles {
    pool: SqlitePool,
}

impl UserRoleProvider for MockRequestRoles {
    async fn role_of(
        &self,
        user_id: themelion::UserId,
    ) -> Result<aitesis::UserRole, aitesis::AitesisError> {
        let user = apotheke::repo::user::get_user(&self.pool, user_id.as_bytes().as_slice())
            .await
            .context(aitesis::error::DatabaseSnafu)?;
        let Some(user) = user else {
            return aitesis::error::InsufficientPermissionSnafu.fail();
        };
        match exousia::UserRole::parse(&user.role) {
            Some(exousia::UserRole::Admin) => Ok(aitesis::UserRole::Admin),
            Some(exousia::UserRole::Member) => Ok(aitesis::UserRole::Member),
            Some(_) | None => aitesis::error::InsufficientPermissionSnafu.fail(),
        }
    }
}

struct MockRequestIdentity;

impl IdentityValidator for MockRequestIdentity {
    async fn validate(
        &self,
        _media_type: themelion::MediaType,
        _title: &str,
        _external_id: Option<&str>,
    ) -> Result<(), aitesis::AitesisError> {
        Ok(())
    }
}

struct MockRequestMonitor;

impl MonitorService for MockRequestMonitor {
    async fn create_want(
        &self,
        _request: &aitesis::MediaRequest,
    ) -> Result<themelion::WantId, aitesis::AitesisError> {
        Ok(themelion::WantId::new())
    }
}

// ── Test helpers ─────────────────────────────────────────────────────────────

type TestError = Box<dyn std::error::Error + Send + Sync>; // kanon:ignore RUST/box-dyn-error -- integration test helper, surfaces any error source without requiring conversion impls

async fn test_db() -> Result<SqlitePool, TestError> {
    let pool = SqlitePool::connect("sqlite::memory:").await?;
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

async fn test_state_with_queue(
    max_concurrent_downloads: usize,
) -> Result<
    (
        AppState,
        Arc<ExousiaServiceImpl>,
        SqlitePool,
        mpsc::UnboundedReceiver<DownloadId>,
    ),
    TestError,
> {
    let pool = test_db().await?;
    let pools = Arc::new(DbPools {
        read: pool.clone(),
        write: pool.clone(),
    });
    let config = Arc::new(Config {
        aitesis: AitesisConfig {
            auto_approve_admins: false,
            ..AitesisConfig::default()
        },
        ..Config::default()
    });
    let (event_tx, _) = create_event_bus(64);
    let exousia_config = ExousiaConfig {
        access_token_ttl_secs: 900,
        refresh_token_ttl_days: 30,
        jwt_secret: "test-secret-that-is-long-enough-for-hs256".to_string(),
    };
    let auth = Arc::new(ExousiaServiceImpl::new(pools.clone(), exousia_config));
    let import = paroche::state::make_import_service(|| async { Ok(vec![]) });
    let mut state = AppState::with_stubs(pools, config, event_tx, auth.clone(), import);
    state.search = Arc::new(MockSearchService);
    // WHY: enqueue/cancel/reprioritize routes delegate to the live syntaxis
    // service; wiring the real DownloadQueue keeps these tests end-to-end.
    let (started_tx, started_rx) = mpsc::unbounded_channel();
    let (imported_tx, _imported_rx) = mpsc::unbounded_channel();
    let queue_svc = Arc::new(
        syntaxis::DownloadQueue::new(
            pool.clone(),
            Arc::new(MockEngine { started_tx }),
            Arc::new(MockImportService { imported_tx }) as Arc<dyn ImportService>,
            horismos::SyntaxisConfig {
                max_concurrent_downloads,
                max_per_tracker: 3,
                retry_count: 2,
                retry_backoff_base_seconds: 0,
                stalled_download_timeout_hours: 24,
            },
        )
        .await?,
    );
    state.queue = Arc::new(QueueAdapter(queue_svc));
    state.requests = Arc::new(MockRequestAdapter(Arc::new(
        aitesis::AitesisServiceImpl::new(
            pool.clone(),
            pool.clone(),
            state.config.aitesis.clone(),
            MockRequestRoles { pool: pool.clone() },
            MockRequestIdentity,
            MockRequestMonitor,
        ),
    )));
    Ok((state, auth, pool, started_rx))
}

async fn test_state() -> Result<(AppState, Arc<ExousiaServiceImpl>, SqlitePool), TestError> {
    // WHY: zero slots keeps API-enqueued items deterministically 'queued' for
    // the snapshot/cancel/reprioritize contract tests; live dispatch is
    // covered by enqueue_download_dispatches_to_live_engine.
    let (state, auth, pool, _started_rx) = test_state_with_queue(0).await?;
    Ok((state, auth, pool))
}

async fn admin_token(auth: &ExousiaServiceImpl) -> Result<String, TestError> {
    auth.create_user(CreateUserRequest {
        username: "alice".to_string(),
        display_name: "Alice".to_string(),
        password: "password123".to_string(),
        role: UserRole::Admin,
    })
    .await?;
    let pair = auth.login("alice", "password123").await?;
    Ok(pair.access_token)
}

async fn member_token(auth: &ExousiaServiceImpl) -> Result<String, TestError> {
    auth.create_user(CreateUserRequest {
        username: "bob".to_string(),
        display_name: "Bob".to_string(),
        password: "password123".to_string(),
        role: UserRole::Member,
    })
    .await?;
    let pair = auth.login("bob", "password123").await?;
    Ok(pair.access_token)
}

fn auth_header(token: &str) -> String {
    format!("Bearer {token}")
}

async fn body_json(resp: axum::http::Response<Body>) -> Result<Value, TestError> {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await?;
    let val: Value = serde_json::from_slice(&bytes)?;
    Ok(val)
}

fn build_app(state: AppState) -> axum::Router {
    paroche::build_router(state)
}

async fn enqueue_via_api(
    app: &axum::Router,
    token: &str,
    priority: u8,
) -> Result<(StatusCode, Value), TestError> {
    let want_id = Uuid::now_v7().to_string();
    let release_id = Uuid::now_v7().to_string();
    let body = json!({
        "want_id": want_id,
        "release_id": release_id,
        "download_url": format!("magnet:?xt=urn:btih:{}", Uuid::now_v7()),
        "protocol": "torrent",
        "priority": priority,
    });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/downloads")
                .header("Content-Type", "application/json")
                .header("Authorization", auth_header(token))
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;
    let status = resp.status();
    let json = body_json(resp).await?;
    Ok((status, json))
}

async fn get_queue_snapshot(
    app: &axum::Router,
    token: &str,
) -> Result<(StatusCode, Value), TestError> {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/downloads")
                .header("Authorization", auth_header(token))
                .body(Body::empty())?,
        )
        .await?;
    let status = resp.status();
    let json = body_json(resp).await?;
    Ok((status, json))
}

// ── Search endpoint tests ────────────────────────────────────────────────────

#[tokio::test]
async fn search_returns_results_from_mock() -> Result<(), TestError> {
    let (state, auth, _pool) = test_state().await?;
    let token = admin_token(&auth).await?;
    let app = build_app(state);

    let body = json!({"query_text": "test album", "media_type": "music"});
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/search")
                .header("Content-Type", "application/json")
                .header("Authorization", auth_header(&token))
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await?;
    let results = &json["data"]["results"];
    assert!(results.is_array());
    assert_eq!(results[0]["title"], "Test Album - FLAC");
    assert_eq!(results[0]["seeders"], 42);
    Ok(())
}

#[tokio::test]
async fn search_requires_authentication() -> Result<(), TestError> {
    let (state, _, _pool) = test_state().await?;
    let app = build_app(state);

    let body = json!({"query_text": "test"});
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/search")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

// ── Download queue API tests ─────────────────────────────────────────────────

#[tokio::test]
async fn queue_snapshot_empty_initially() -> Result<(), TestError> {
    let (state, auth, _pool) = test_state().await?;
    let token = admin_token(&auth).await?;
    let app = build_app(state);

    let (status, json) = get_queue_snapshot(&app, &token).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["data"]["active"].as_array().unwrap().len(), 0);
    assert_eq!(json["data"]["queued"].as_array().unwrap().len(), 0);
    assert_eq!(json["data"]["completed_count"], 0);
    assert_eq!(json["data"]["failed_count"], 0);
    Ok(())
}

#[tokio::test]
async fn enqueue_download_returns_created() -> Result<(), TestError> {
    let (state, auth, _pool) = test_state().await?;
    let token = admin_token(&auth).await?;
    let app = build_app(state);

    let (status, json) = enqueue_via_api(&app, &token, 4).await?;
    assert_eq!(status, StatusCode::CREATED);
    assert!(!json["data"]["id"].as_str().unwrap().is_empty());
    assert_eq!(json["data"]["status"], "queued");
    assert_eq!(json["data"]["priority"], 4);
    assert_eq!(json["data"]["protocol"], "torrent");
    Ok(())
}

// WHY: the #499 regression — a raw route INSERT persisted the row but the
// running queue never learned of it, so nothing dispatched until restart.
#[tokio::test]
async fn enqueue_download_dispatches_to_live_engine() -> Result<(), TestError> {
    let (state, auth, _pool, mut started_rx) = test_state_with_queue(5).await?;
    let token = admin_token(&auth).await?;
    let app = build_app(state);

    let (status, _json) = enqueue_via_api(&app, &token, 4).await?;
    assert_eq!(status, StatusCode::CREATED);

    let dispatched =
        tokio::time::timeout(std::time::Duration::from_secs(5), started_rx.recv()).await;
    assert!(
        dispatched.is_ok_and(|d| d.is_some()),
        "an API enqueue must reach the live engine without a process restart"
    );
    Ok(())
}

#[tokio::test]
async fn enqueue_download_appears_in_queue_snapshot() -> Result<(), TestError> {
    let (state, auth, _pool) = test_state().await?;
    let token = admin_token(&auth).await?;
    let app = build_app(state);

    enqueue_via_api(&app, &token, 3).await?;

    let (status, json) = get_queue_snapshot(&app, &token).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["data"]["queued"].as_array().unwrap().len(), 1);
    assert_eq!(json["data"]["queued"][0]["priority"], 3);
    Ok(())
}

#[tokio::test]
async fn priority_ordering_highest_first_in_snapshot() -> Result<(), TestError> {
    let (state, auth, _pool) = test_state().await?;
    let token = admin_token(&auth).await?;
    let app = build_app(state);

    enqueue_via_api(&app, &token, 1).await?;
    enqueue_via_api(&app, &token, 3).await?;
    enqueue_via_api(&app, &token, 2).await?;
    enqueue_via_api(&app, &token, 4).await?;

    let (_, json) = get_queue_snapshot(&app, &token).await?;
    let queued = json["data"]["queued"].as_array().unwrap();
    assert_eq!(queued.len(), 4);
    // Snapshot is ordered by priority DESC, added_at ASC
    assert_eq!(queued[0]["priority"], 4);
    assert_eq!(queued[1]["priority"], 3);
    assert_eq!(queued[2]["priority"], 2);
    assert_eq!(queued[3]["priority"], 1);
    Ok(())
}

#[tokio::test]
async fn cancel_download_removes_from_snapshot() -> Result<(), TestError> {
    let (state, auth, _pool) = test_state().await?;
    let token = admin_token(&auth).await?;
    let app = build_app(state);

    let (_, created) = enqueue_via_api(&app, &token, 3).await?;
    let dl_id = created["data"]["id"].as_str().unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/downloads/{dl_id}"))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let (_, json) = get_queue_snapshot(&app, &token).await?;
    assert_eq!(json["data"]["queued"].as_array().unwrap().len(), 0);
    Ok(())
}

#[tokio::test]
async fn cancel_nonexistent_download_returns_not_found() -> Result<(), TestError> {
    let (state, auth, _pool) = test_state().await?;
    let token = admin_token(&auth).await?;
    let app = build_app(state);

    let fake_id = Uuid::now_v7();
    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/downloads/{fake_id}"))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
async fn reprioritize_download_updates_priority() -> Result<(), TestError> {
    let (state, auth, _pool) = test_state().await?;
    let token = admin_token(&auth).await?;
    let app = build_app(state);

    let (_, created) = enqueue_via_api(&app, &token, 1).await?;
    let dl_id = created["data"]["id"].as_str().unwrap();

    let body = json!({"priority": 3});
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/downloads/{dl_id}/priority"))
                .header("Content-Type", "application/json")
                .header("Authorization", auth_header(&token))
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await?;
    assert_eq!(json["data"]["priority"], 3);
    Ok(())
}

// ── Request workflow tests ───────────────────────────────────────────────────

#[tokio::test]
async fn submit_request_returns_created() -> Result<(), TestError> {
    let (state, auth, _pool) = test_state().await?;
    let token = admin_token(&auth).await?;
    let app = build_app(state);

    let body = json!({"media_type": "music_album", "title": "Requested Album"});
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/requests")
                .header("Content-Type", "application/json")
                .header("Authorization", auth_header(&token))
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await?;
    assert_eq!(json["data"]["title"], "Requested Album");
    assert_eq!(json["data"]["status"], "submitted");
    Ok(())
}

#[tokio::test]
async fn approve_request_requires_admin() -> Result<(), TestError> {
    let (state, auth, _pool) = test_state().await?;
    let admin = admin_token(&auth).await?;
    let member = member_token(&auth).await?;
    let app = build_app(state);

    // Submit as admin
    let body = json!({"media_type": "music_album", "title": "Album"});
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/requests")
                .header("Content-Type", "application/json")
                .header("Authorization", auth_header(&admin))
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;
    let created = body_json(resp).await?;
    let req_id = created["data"]["id"].as_str().unwrap();

    // Member tries to approve -> 403
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/requests/{req_id}/approve"))
                .header("Authorization", auth_header(&member))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Admin approves -> 200
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/requests/{req_id}/approve"))
                .header("Authorization", auth_header(&admin))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await?;
    assert_eq!(json["data"]["status"], "monitoring");
    Ok(())
}

#[tokio::test]
async fn deny_request_requires_admin() -> Result<(), TestError> {
    let (state, auth, _pool) = test_state().await?;
    let admin = admin_token(&auth).await?;
    let member = member_token(&auth).await?;
    let app = build_app(state);

    let body = json!({"media_type": "movie", "title": "Some Movie"});
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/requests")
                .header("Content-Type", "application/json")
                .header("Authorization", auth_header(&admin))
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;
    let created = body_json(resp).await?;
    let req_id = created["data"]["id"].as_str().unwrap();

    // Member tries to deny -> 403
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/requests/{req_id}/deny"))
                .header("Content-Type", "application/json")
                .header("Authorization", auth_header(&member))
                .body(Body::from(serde_json::to_vec(&json!({"reason": "no"}))?))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Admin denies -> 200
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/requests/{req_id}/deny"))
                .header("Content-Type", "application/json")
                .header("Authorization", auth_header(&admin))
                .body(Body::from(serde_json::to_vec(
                    &json!({"reason": "out of scope"}),
                )?))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await?;
    assert_eq!(json["data"]["status"], "denied");
    Ok(())
}

#[tokio::test]
async fn list_requests_scopes_members_to_own_requests() -> Result<(), TestError> {
    let (state, auth, _pool) = test_state().await?;
    let admin = admin_token(&auth).await?;
    let member = member_token(&auth).await?;
    let app = build_app(state);

    // One request from each user.
    let submit = |token: String, title: &str| {
        let app = app.clone();
        let body = json!({"media_type": "music_album", "title": title});
        async move {
            let resp = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/requests")
                        .header("Content-Type", "application/json")
                        .header("Authorization", auth_header(&token))
                        .body(Body::from(serde_json::to_vec(&body)?))?,
                )
                .await?;
            assert_eq!(resp.status(), StatusCode::CREATED);
            body_json(resp).await
        }
    };
    let admin_req = submit(admin.clone(), "Admin Album").await?;
    let member_req = submit(member.clone(), "Member Album").await?;
    let admin_user_id = admin_req["data"]["user_id"].as_str().unwrap().to_string();
    let member_user_id = member_req["data"]["user_id"].as_str().unwrap().to_string();

    // Member with no filter sees only their own requests.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/requests")
                .header("Authorization", auth_header(&member))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await?;
    let data = json["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["user_id"], Value::String(member_user_id));

    // Member naming another user's requests is rejected.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/requests?user_id={admin_user_id}"))
                .header("Authorization", auth_header(&member))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Admin with no filter sees everything.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/requests")
                .header("Authorization", auth_header(&admin))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await?;
    assert_eq!(json["data"].as_array().unwrap().len(), 2);
    Ok(())
}

// ── Wanted list tests ────────────────────────────────────────────────────────

#[tokio::test]
async fn add_wanted_returns_created() -> Result<(), TestError> {
    let (state, auth, _pool) = test_state().await?;
    let token = admin_token(&auth).await?;
    let app = build_app(state);

    let body = json!({
        "media_type": "music_album",
        "title": "Wanted Album",
        "quality_profile_id": 1
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/wanted")
                .header("Content-Type", "application/json")
                .header("Authorization", auth_header(&token))
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await?;
    assert_eq!(json["data"]["title"], "Wanted Album");
    assert_eq!(json["data"]["status"], "searching");
    Ok(())
}

#[tokio::test]
async fn remove_wanted_returns_no_content() -> Result<(), TestError> {
    let (state, auth, _pool) = test_state().await?;
    let token = admin_token(&auth).await?;
    let app = build_app(state);

    let body = json!({
        "media_type": "music_album",
        "title": "To Remove",
        "quality_profile_id": 1
    });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/wanted")
                .header("Content-Type", "application/json")
                .header("Authorization", auth_header(&token))
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;
    let created = body_json(resp).await?;
    let want_id = created["data"]["id"].as_str().unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/wanted/{want_id}"))
                .header("Authorization", auth_header(&token))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    Ok(())
}

// ── Auth enforcement tests ───────────────────────────────────────────────────

#[tokio::test]
async fn unauthenticated_requests_return_401() -> Result<(), TestError> {
    let (state, _, _pool) = test_state().await?;
    let app = build_app(state);

    let endpoints = [
        ("GET", "/api/v1/downloads"),
        ("POST", "/api/v1/downloads"),
        ("GET", "/api/v1/wanted"),
        ("GET", "/api/v1/requests"),
        ("GET", "/api/v1/indexers"),
    ];

    for (method, uri) in endpoints {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri} should return 401 without auth"
        );
    }
    Ok(())
}

#[tokio::test]
async fn member_on_admin_routes_returns_403() -> Result<(), TestError> {
    let (state, auth, _pool) = test_state().await?;
    // Need admin first so member can be created (admin is user #1)
    let _admin = admin_token(&auth).await?;
    let member = member_token(&auth).await?;
    let app = build_app(state);

    // POST /api/v1/indexers requires admin
    let body = json!({"name": "test", "url": "https://example.com"});
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/indexers")
                .header("Content-Type", "application/json")
                .header("Authorization", auth_header(&member))
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // PUT /api/v1/indexers/1 requires admin
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/indexers/1")
                .header("Content-Type", "application/json")
                .header("Authorization", auth_header(&member))
                .body(Body::from(serde_json::to_vec(&json!({"name": "x"}))?))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // DELETE /api/v1/indexers/1 requires admin
    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/indexers/1")
                .header("Authorization", auth_header(&member))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    Ok(())
}

#[path = "acquisition_integration/pipeline_tests.rs"]
mod pipeline_tests;
#[path = "acquisition_integration/recovery_tests.rs"]
mod recovery_tests;
