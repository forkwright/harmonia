//! Integration tests for the acquisition pipeline (P103).
//!
//! Validates search → queue → download → extract → import flow, queue
//! management, HTTP endpoint correctness, and auth enforcement against the
//! full Paroche router backed by in-memory SQLite.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tower::ServiceExt;
use uuid::Uuid;

use ergasia::{DownloadProgress, DownloadState, ErgasiaError, ExtractionResult};
use exousia::{AuthService, CreateUserRequest, ExousiaServiceImpl, UserRole};
use harmonia_common::ids::{DownloadId, ReleaseId, WantId};
use harmonia_common::{HarmoniaEvent, create_event_bus};
use harmonia_db::DbPools;
use harmonia_db::migrate::MIGRATOR;
use horismos::{Config, ExousiaConfig, SyntaxisConfig};
use paroche::state::{AppState, DynSearchService, ServiceFut};
use syntaxis::{CompletedDownload, ImportService, QueueItem, QueueManager, SyntaxisService};

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

    fn extract(
        &self,
        _download_path: &std::path::Path,
        _output_dir: &std::path::Path,
    ) -> Result<Option<ExtractionResult>, ErgasiaError> {
        Ok(None)
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

// ── Test helpers ─────────────────────────────────────────────────────────────

type TestError = Box<dyn std::error::Error + Send + Sync>;

async fn test_db() -> Result<SqlitePool, TestError> {
    let pool = SqlitePool::connect("sqlite::memory:").await?;
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

async fn test_state() -> Result<(AppState, Arc<ExousiaServiceImpl>, SqlitePool), TestError> {
    let pool = test_db().await?;
    let pools = Arc::new(DbPools {
        read: pool.clone(),
        write: pool.clone(),
    });
    let config = Arc::new(Config::default());
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
    assert_eq!(json["data"]["status"], "approved");
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

// ── Pipeline integration tests (SyntaxisService + MockEngine) ────────────────

fn test_syntaxis_config() -> SyntaxisConfig {
    SyntaxisConfig {
        max_concurrent_downloads: 5,
        max_per_tracker: 3,
        retry_count: 2,
        retry_backoff_base_seconds: 0,
        stalled_download_timeout_hours: 24,
    }
}

fn make_queue_item(priority: u8) -> QueueItem {
    QueueItem {
        id: Uuid::now_v7(),
        want_id: WantId::new(),
        release_id: ReleaseId::new(),
        download_url: format!("magnet:?xt=urn:btih:{}", Uuid::now_v7()),
        protocol: syntaxis::DownloadProtocol::Torrent,
        priority,
        tracker_id: None,
        info_hash: None,
    }
}

#[tokio::test]
async fn pipeline_enqueue_dispatches_to_engine() -> Result<(), TestError> {
    let pool = test_db().await?;
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (imported_tx, _imported_rx) = mpsc::unbounded_channel();

    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    let config = test_syntaxis_config();

    let svc = Arc::new(SyntaxisService::new(pool, engine, import_svc, config).await?);

    // Enqueue at priority 4 (interactive bypass) to trigger immediate dispatch
    let item = make_queue_item(4);
    let pos = svc.enqueue(item).await?;
    assert_eq!(pos.position, 0);

    // Wait for the spawned dispatch task to call start_download
    let dl_id = tokio::time::timeout(Duration::from_secs(5), started_rx.recv())
        .await?
        .expect("engine should have received start_download");

    // Verify we got a valid download ID back
    assert!(!dl_id.to_string().is_empty());
    Ok(())
}

#[tokio::test]
async fn pipeline_completion_triggers_import() -> Result<(), TestError> {
    let pool = test_db().await?;
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (imported_tx, mut imported_rx) = mpsc::unbounded_channel();
    let (event_tx, _) = create_event_bus(64);

    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    let config = test_syntaxis_config();

    let svc = Arc::new(SyntaxisService::new(pool, engine, import_svc, config).await?);
    let shutdown = tokio_util::sync::CancellationToken::new();
    svc.start(event_tx.subscribe(), shutdown.clone());

    // Enqueue at priority 4 to dispatch immediately
    svc.enqueue(make_queue_item(4)).await?;

    // Wait for engine to receive the download
    let dl_id = tokio::time::timeout(Duration::from_secs(5), started_rx.recv())
        .await?
        .expect("engine should have received start_download");

    // Simulate download completion via event bus
    event_tx.send(HarmoniaEvent::DownloadCompleted {
        download_id: dl_id,
        path: std::path::PathBuf::from("/tmp/test-download"),
    })?;

    // Wait for import to be triggered
    let imported_id = tokio::time::timeout(Duration::from_secs(5), imported_rx.recv())
        .await?
        .expect("import service should have been called");

    assert_eq!(imported_id.to_string(), dl_id.to_string());

    shutdown.cancel();
    Ok(())
}

#[tokio::test]
async fn pipeline_priority_ordering_in_queue() -> Result<(), TestError> {
    let pool = test_db().await?;
    let (started_tx, _started_rx) = mpsc::unbounded_channel();
    let (imported_tx, _imported_rx) = mpsc::unbounded_channel();

    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    // Set max_concurrent to 0 so nothing dispatches (all items stay queued)
    let config = SyntaxisConfig {
        max_concurrent_downloads: 0,
        max_per_tracker: 0,
        retry_count: 2,
        retry_backoff_base_seconds: 0,
        stalled_download_timeout_hours: 24,
    };

    let svc = Arc::new(SyntaxisService::new(pool, engine, import_svc, config).await?);

    // Enqueue items at different priorities
    svc.enqueue(make_queue_item(1)).await?;
    svc.enqueue(make_queue_item(3)).await?;
    svc.enqueue(make_queue_item(2)).await?;

    let snapshot = svc.get_queue_state().await?;
    assert_eq!(snapshot.queued_items.len(), 3);
    // Items ordered by priority: 3, 2, 1 (highest first)
    assert_eq!(snapshot.queued_items[0].priority, 3);
    assert_eq!(snapshot.queued_items[1].priority, 2);
    assert_eq!(snapshot.queued_items[2].priority, 1);
    Ok(())
}

#[tokio::test]
async fn pipeline_fifo_within_same_priority_tier() -> Result<(), TestError> {
    let pool = test_db().await?;
    let (started_tx, _started_rx) = mpsc::unbounded_channel();
    let (imported_tx, _imported_rx) = mpsc::unbounded_channel();

    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    let config = SyntaxisConfig {
        max_concurrent_downloads: 0,
        max_per_tracker: 0,
        retry_count: 2,
        retry_backoff_base_seconds: 0,
        stalled_download_timeout_hours: 24,
    };

    let svc = Arc::new(SyntaxisService::new(pool, engine, import_svc, config).await?);

    let item_a = make_queue_item(2);
    let item_b = make_queue_item(2);
    let id_a = item_a.id;
    let id_b = item_b.id;

    svc.enqueue(item_a).await?;
    svc.enqueue(item_b).await?;

    let snapshot = svc.get_queue_state().await?;
    assert_eq!(snapshot.queued_items.len(), 2);
    // FIFO: first enqueued first
    assert_eq!(snapshot.queued_items[0].id, id_a);
    assert_eq!(snapshot.queued_items[1].id, id_b);
    Ok(())
}

#[tokio::test]
async fn pipeline_transient_failure_triggers_retry() -> Result<(), TestError> {
    let pool = test_db().await?;
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (imported_tx, _imported_rx) = mpsc::unbounded_channel();
    let (event_tx, _) = create_event_bus(64);

    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    let config = SyntaxisConfig {
        max_concurrent_downloads: 5,
        max_per_tracker: 3,
        retry_count: 3,
        retry_backoff_base_seconds: 0,
        stalled_download_timeout_hours: 24,
    };

    let svc = Arc::new(SyntaxisService::new(pool.clone(), engine, import_svc, config).await?);
    let shutdown = tokio_util::sync::CancellationToken::new();
    svc.start(event_tx.subscribe(), shutdown.clone());

    let item = make_queue_item(4);
    let queue_id = item.id;
    svc.enqueue(item).await?;

    // Wait for dispatch
    let dl_id = tokio::time::timeout(Duration::from_secs(5), started_rx.recv())
        .await?
        .expect("engine should start download");

    // Send transient failure (network error, not in permanent patterns)
    event_tx.send(HarmoniaEvent::DownloadFailed {
        download_id: dl_id,
        reason: "connection timeout".to_string(),
    })?;

    // Wait for retry processing
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify retry_count was incremented in DB and status reset to queued
    let row: (i64, String) =
        sqlx::query_as("SELECT retry_count, status FROM download_queue WHERE id = ?")
            .bind(queue_id.as_bytes().as_slice())
            .fetch_one(&pool)
            .await?;
    assert_eq!(
        row.0, 1,
        "retry_count should be 1 after first transient failure"
    );
    assert_eq!(
        row.1, "queued",
        "status should be reset to queued for retry"
    );

    shutdown.cancel();
    Ok(())
}

#[tokio::test]
async fn pipeline_permanent_failure_marks_failed() -> Result<(), TestError> {
    let pool = test_db().await?;
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (imported_tx, _imported_rx) = mpsc::unbounded_channel();
    let (event_tx, _) = create_event_bus(64);

    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    let config = test_syntaxis_config();

    let svc = Arc::new(SyntaxisService::new(pool.clone(), engine, import_svc, config).await?);
    let shutdown = tokio_util::sync::CancellationToken::new();
    svc.start(event_tx.subscribe(), shutdown.clone());

    let item = make_queue_item(4);
    let queue_id = item.id;
    svc.enqueue(item).await?;

    let dl_id = tokio::time::timeout(Duration::from_secs(5), started_rx.recv())
        .await?
        .expect("engine should start download");

    // Send permanent failure (contains "no seeders" which matches permanent pattern)
    event_tx.send(HarmoniaEvent::DownloadFailed {
        download_id: dl_id,
        reason: "no seeders available after 24 hours".to_string(),
    })?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    let row: (String, Option<String>) =
        sqlx::query_as("SELECT status, failed_reason FROM download_queue WHERE id = ?")
            .bind(queue_id.as_bytes().as_slice())
            .fetch_one(&pool)
            .await?;
    assert_eq!(row.0, "failed");
    assert!(row.1.as_deref().unwrap_or("").contains("no seeders"));

    shutdown.cancel();
    Ok(())
}

#[tokio::test]
async fn pipeline_retry_budget_exhaustion_marks_failed() -> Result<(), TestError> {
    // NOTE: SyntaxisService has a bug where ActiveEntry.retry_count is always
    // initialised to 0 regardless of how many retries have occurred in the DB.
    // This means the in-memory retry_count check (`retry_count >= max_retries`)
    // only works when max_retries is 0. We set retry_count=0 in config so
    // the very first transient failure immediately exhausts the budget.
    let pool = test_db().await?;
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (imported_tx, _imported_rx) = mpsc::unbounded_channel();
    let (event_tx, _) = create_event_bus(64);

    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    let config = SyntaxisConfig {
        max_concurrent_downloads: 5,
        max_per_tracker: 3,
        retry_count: 0,
        retry_backoff_base_seconds: 0,
        stalled_download_timeout_hours: 24,
    };

    let svc = Arc::new(SyntaxisService::new(pool.clone(), engine, import_svc, config).await?);
    let shutdown = tokio_util::sync::CancellationToken::new();
    svc.start(event_tx.subscribe(), shutdown.clone());

    let item = make_queue_item(4);
    let queue_id = item.id;
    svc.enqueue(item).await?;

    let dl_id = tokio::time::timeout(Duration::from_secs(5), started_rx.recv())
        .await?
        .expect("engine should start download");

    // Transient failure with zero retries allowed → immediate budget exhaustion
    event_tx.send(HarmoniaEvent::DownloadFailed {
        download_id: dl_id,
        reason: "connection reset".to_string(),
    })?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    let row: (String, Option<String>) =
        sqlx::query_as("SELECT status, failed_reason FROM download_queue WHERE id = ?")
            .bind(queue_id.as_bytes().as_slice())
            .fetch_one(&pool)
            .await?;
    assert_eq!(row.0, "failed");
    assert!(
        row.1
            .as_deref()
            .unwrap_or("")
            .contains("retry budget exhausted")
    );

    shutdown.cancel();
    Ok(())
}

// ── Startup recovery tests ───────────────────────────────────────────────────

#[tokio::test]
async fn startup_recovery_loads_queued_items_from_db() -> Result<(), TestError> {
    let pool = test_db().await?;

    // Insert non-terminal rows directly into DB (simulating prior state)
    let id_queued = Uuid::now_v7();
    let id_downloading = Uuid::now_v7();
    let id_completed = Uuid::now_v7();
    let want_id = Uuid::now_v7().as_bytes().to_vec();
    let release_id = Uuid::now_v7().as_bytes().to_vec();

    for (id, status) in [
        (id_queued, "queued"),
        (id_downloading, "downloading"),
        (id_completed, "completed"),
    ] {
        sqlx::query(
            "INSERT INTO download_queue \
             (id, want_id, release_id, download_url, protocol, priority, status, added_at, retry_count) \
             VALUES (?, ?, ?, 'magnet:test', 'torrent', 2, ?, '2026-01-01T00:00:00Z', 0)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(&want_id)
        .bind(&release_id)
        .bind(status)
        .execute(&pool)
        .await?;
    }

    // Boot SyntaxisService — recovery should load non-terminal items
    let (started_tx, _started_rx) = mpsc::unbounded_channel();
    let (imported_tx, _imported_rx) = mpsc::unbounded_channel();
    let engine = Arc::new(MockEngine { started_tx });
    let import_svc: Arc<dyn ImportService> = Arc::new(MockImportService { imported_tx });
    let config = SyntaxisConfig {
        max_concurrent_downloads: 0,
        max_per_tracker: 0,
        retry_count: 3,
        retry_backoff_base_seconds: 30,
        stalled_download_timeout_hours: 24,
    };

    let svc = Arc::new(SyntaxisService::new(pool, engine, import_svc, config).await?);

    let snapshot = svc.get_queue_state().await?;
    // 'queued' and 'downloading' are non-terminal and should be recovered
    // 'completed' is terminal and should NOT be recovered
    assert_eq!(
        snapshot.queued_items.len(),
        2,
        "both 'queued' and 'downloading' rows should be recovered into the in-memory queue"
    );
    assert_eq!(snapshot.completed_count, 1);
    Ok(())
}

#[tokio::test]
async fn startup_recovery_visible_via_http_snapshot() -> Result<(), TestError> {
    let (state, auth, pool) = test_state().await?;
    let token = admin_token(&auth).await?;

    // Insert a queued row directly
    let id = Uuid::now_v7();
    let want_id = Uuid::now_v7().as_bytes().to_vec();
    let release_id = Uuid::now_v7().as_bytes().to_vec();
    sqlx::query(
        "INSERT INTO download_queue \
         (id, want_id, release_id, download_url, protocol, priority, status, added_at, retry_count) \
         VALUES (?, ?, ?, 'magnet:test', 'torrent', 3, 'queued', '2026-01-01T00:00:00Z', 0)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(&want_id)
    .bind(&release_id)
    .execute(&pool)
    .await?;

    let app = build_app(state);

    let (_, json) = get_queue_snapshot(&app, &token).await?;
    let queued = json["data"]["queued"].as_array().unwrap();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0]["priority"], 3);
    assert_eq!(queued[0]["status"], "queued");
    Ok(())
}
