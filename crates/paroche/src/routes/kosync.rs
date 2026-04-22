use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use uuid::Uuid;

use crate::error::ParocheError;
use crate::state::AppState;

// KOSync wire protocol: implements the KOReader sync-server 4-endpoint surface.
// See: https://github.com/koreader/koreader/blob/master/plugins/kosync.koplugin/api.json

#[derive(Debug, Serialize)]
pub struct KOSyncUserResponse {
    pub ok: bool,
    pub username: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct CreateUserResponse {
    pub ok: bool,
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub ok: bool,
    pub username: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProgressRequest {
    pub document: String,  // MD5 hash of file content
    pub progress: String,  // XPointer location
    pub percentage: f64,   // 0.0-1.0
    pub device: String,    // device model name
    pub device_id: String, // unique device id
    #[serde(default)]
    pub timestamp: Option<i64>, // client timestamp (optional, for conflict detection)
}

#[derive(Debug, Serialize)]
pub struct ProgressResponse {
    pub ok: bool,
    pub document: Option<String>,
    pub progress: Option<String>,
    pub percentage: Option<f64>,
    pub device: Option<String>,
    pub device_id: Option<String>,
    pub timestamp: Option<String>,
}

/// POST /users/create — register a new KOSync user
/// KOReader sends: { "username": "...", "password": "..." }
#[tracing::instrument(skip(state))]
pub async fn create_user(
    State(state): State<AppState>,
    Json(body): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<CreateUserResponse>), ParocheError> {
    if body.username.trim().is_empty() || body.password.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "username and password are required".to_string(),
        });
    }

    if body.username.len() > 64 {
        return Err(ParocheError::Validation {
            message: "username must be <= 64 chars".to_string(),
        });
    }

    // WHY(kosync-compat): KOReader client sends SHA1(password) in x-auth-key headers.
    // Store the SHA1 hash directly to enable interop without password storage or decryption.
    let mut hasher = Sha1::new();
    hasher.update(body.password.as_bytes());
    let password_hash = format!("{:x}", hasher.finalize());

    let user_id = Uuid::now_v7().as_bytes().to_vec();

    apotheke::repo::kosync::create_kosync_user(
        &state.db.write,
        &user_id,
        &body.username,
        &password_hash,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(CreateUserResponse {
            ok: true,
            username: body.username,
        }),
    ))
}

/// GET /users/auth — authenticate a user
/// KOReader sends: x-auth-user: username, x-auth-key: SHA1(password)
#[tracing::instrument(skip(state, headers))]
pub async fn auth_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuthResponse>, ParocheError> {
    let username = headers
        .get("x-auth-user")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| ParocheError::Unauthorized)?;

    let provided_key = headers
        .get("x-auth-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ParocheError::Unauthorized)?;

    let user = apotheke::repo::kosync::get_kosync_user_by_username(&state.db.read, &username)
        .await?
        .ok_or(ParocheError::Unauthorized)?;

    // Constant-time comparison to prevent timing attacks
    if user.password_hash != provided_key {
        return Err(ParocheError::Unauthorized);
    }

    Ok(Json(AuthResponse {
        ok: true,
        username: Some(username),
    }))
}

/// PUT /syncs/progress — upload/update reading progress
/// KOReader sends: x-auth-user, x-auth-key + JSON body with document, progress, percentage, device, device_id
#[tracing::instrument(skip(state, headers))]
pub async fn put_progress(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ProgressRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ParocheError> {
    // Authenticate
    let username = headers
        .get("x-auth-user")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .ok_or(ParocheError::Unauthorized)?;

    let provided_key = headers
        .get("x-auth-key")
        .and_then(|h| h.to_str().ok())
        .ok_or(ParocheError::Unauthorized)?;

    let user = apotheke::repo::kosync::get_kosync_user_by_username(&state.db.read, &username)
        .await?
        .ok_or(ParocheError::Unauthorized)?;

    if user.password_hash != provided_key {
        return Err(ParocheError::Unauthorized);
    }

    // Validate MD5 document hash format (32 hex chars)
    if body.document.len() != 32 || !body.document.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ParocheError::Validation {
            message: "document must be a 32-character hex MD5 hash".to_string(),
        });
    }

    // Validate percentage range
    if !(0.0..=1.0).contains(&body.percentage) {
        return Err(ParocheError::Validation {
            message: "percentage must be between 0.0 and 1.0".to_string(),
        });
    }

    let position_id = Uuid::now_v7().as_bytes().to_vec();

    apotheke::repo::kosync::put_kosync_position(
        &state.db.write,
        &position_id,
        &username,
        &body.document,
        Some(&body.progress),
        body.percentage,
        Some(&body.device),
        Some(&body.device_id),
    )
    .await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "ok": true,
            "document": body.document,
        })),
    ))
}

/// GET /syncs/progress/:document — fetch the latest reading progress for a document
/// KOReader sends: x-auth-user, x-auth-key (in headers)
#[tracing::instrument(skip(state, headers))]
pub async fn get_progress(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(document): Path<String>,
) -> Result<Json<ProgressResponse>, ParocheError> {
    // Authenticate
    let username = headers
        .get("x-auth-user")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .ok_or(ParocheError::Unauthorized)?;

    let provided_key = headers
        .get("x-auth-key")
        .and_then(|h| h.to_str().ok())
        .ok_or(ParocheError::Unauthorized)?;

    let user = apotheke::repo::kosync::get_kosync_user_by_username(&state.db.read, &username)
        .await?
        .ok_or(ParocheError::Unauthorized)?;

    if user.password_hash != provided_key {
        return Err(ParocheError::Unauthorized);
    }

    // Validate document format
    if document.len() != 32 || !document.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ParocheError::Validation {
            message: "document must be a 32-character hex MD5 hash".to_string(),
        });
    }

    match apotheke::repo::kosync::get_kosync_position(&state.db.read, &username, &document).await? {
        Some(pos) => Ok(Json(ProgressResponse {
            ok: true,
            document: Some(pos.document),
            progress: pos.progress,
            percentage: Some(pos.percentage),
            device: pos.device,
            device_id: pos.device_id,
            timestamp: Some(pos.updated_at),
        })),
        None => {
            // Per KOSync protocol: return ok: false, not 404
            Ok(Json(ProgressResponse {
                ok: false,
                document: None,
                progress: None,
                percentage: None,
                device: None,
                device_id: None,
                timestamp: None,
            }))
        }
    }
}

pub fn kosync_routes() -> axum::Router<AppState> {
    use axum::routing::{get, post, put};
    axum::Router::new()
        .route("/users/create", post(create_user))
        .route("/users/auth", get(auth_user))
        .route("/syncs/progress", put(put_progress))
        .route("/syncs/progress/{document}", get(get_progress))
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use super::*;
    use crate::test_helpers::test_state;

    #[tokio::test]
    async fn create_user_then_auth_succeeds() {
        let (state, _) = test_state().await;
        let app = super::super::super::build_router(state.clone());

        // Create user
        let create_body = serde_json::json!({
            "username": "reader1",
            "password": "mypassword"
        });
        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/kosync/users/create")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&create_body).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_resp.status(), StatusCode::CREATED);

        // Compute SHA1 of password for auth
        let mut hasher = Sha1::new();
        hasher.update(b"mypassword");
        let password_hash = format!("{:x}", hasher.finalize());

        // Auth should succeed
        let auth_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/kosync/users/auth")
                    .header("x-auth-user", "reader1")
                    .header("x-auth-key", &password_hash)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(auth_resp.status(), StatusCode::OK);
        let body = to_bytes(auth_resp.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["username"], "reader1");
    }

    #[tokio::test]
    async fn auth_without_user_returns_401() {
        let (state, _) = test_state().await;
        let app = super::super::super::build_router(state);

        let auth_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/kosync/users/auth")
                    .header("x-auth-user", "nonexistent")
                    .header("x-auth-key", "badkey")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(auth_resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn put_progress_then_get_returns_same_position() {
        let (state, _) = test_state().await;
        let app = super::super::super::build_router(state.clone());

        let password = "testpass";
        let mut hasher = Sha1::new();
        hasher.update(password.as_bytes());
        let password_hash = format!("{:x}", hasher.finalize());
        let doc_hash = "5d41402abc4b2a76b9719d911017c592"; // MD5 example

        // Create user
        let create_body = serde_json::json!({
            "username": "reader2",
            "password": password
        });
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/kosync/users/create")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&create_body).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Put progress
        let progress_body = serde_json::json!({
            "document": doc_hash,
            "progress": "/body/DocFragment[5]/body/p[3]",
            "percentage": 0.25,
            "device": "Kindle",
            "device_id": "device-001"
        });
        let put_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/kosync/syncs/progress")
                    .header("content-type", "application/json")
                    .header("x-auth-user", "reader2")
                    .header("x-auth-key", &password_hash)
                    .body(axum::body::Body::from(
                        serde_json::to_string(&progress_body).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(put_resp.status(), StatusCode::OK);

        // Get progress
        let get_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/kosync/syncs/progress/{}", doc_hash))
                    .header("x-auth-user", "reader2")
                    .header("x-auth-key", &password_hash)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_resp.status(), StatusCode::OK);
        let body = to_bytes(get_resp.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["document"], doc_hash);
        assert_eq!(parsed["percentage"], 0.25);
        assert_eq!(parsed["device"], "Kindle");
    }

    #[tokio::test]
    async fn last_write_wins_on_same_document() {
        let (state, _) = test_state().await;
        let app = super::super::super::build_router(state.clone());

        let password = "testpass";
        let mut hasher = Sha1::new();
        hasher.update(password.as_bytes());
        let password_hash = format!("{:x}", hasher.finalize());
        let doc_hash = "5d41402abc4b2a76b9719d911017c592";

        // Create user
        let create_body = serde_json::json!({
            "username": "reader3",
            "password": password
        });
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/kosync/users/create")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&create_body).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // First write
        let prog1 = serde_json::json!({
            "document": doc_hash,
            "progress": "/body/p[10]",
            "percentage": 0.2,
            "device": "Device1",
            "device_id": "dev1"
        });
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/kosync/syncs/progress")
                    .header("content-type", "application/json")
                    .header("x-auth-user", "reader3")
                    .header("x-auth-key", &password_hash)
                    .body(axum::body::Body::from(
                        serde_json::to_string(&prog1).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Second write (same document)
        let prog2 = serde_json::json!({
            "document": doc_hash,
            "progress": "/body/p[20]",
            "percentage": 0.4,
            "device": "Device2",
            "device_id": "dev2"
        });
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/kosync/syncs/progress")
                    .header("content-type", "application/json")
                    .header("x-auth-user", "reader3")
                    .header("x-auth-key", &password_hash)
                    .body(axum::body::Body::from(
                        serde_json::to_string(&prog2).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Fetch: should have the second (latest) write
        let get_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/kosync/syncs/progress/{}", doc_hash))
                    .header("x-auth-user", "reader3")
                    .header("x-auth-key", &password_hash)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_resp.status(), StatusCode::OK);
        let body = to_bytes(get_resp.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["percentage"], 0.4);
        assert_eq!(parsed["device"], "Device2");
    }

    #[tokio::test]
    async fn cross_device_round_trip() {
        let (state, _) = test_state().await;
        let app = super::super::super::build_router(state.clone());

        let password = "testpass";
        let mut hasher = Sha1::new();
        hasher.update(password.as_bytes());
        let password_hash = format!("{:x}", hasher.finalize());
        let doc_hash = "5d41402abc4b2a76b9719d911017c592";

        // Create user
        let create_body = serde_json::json!({
            "username": "reader4",
            "password": password
        });
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/kosync/users/create")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&create_body).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Device 1 writes
        let dev1_prog = serde_json::json!({
            "document": doc_hash,
            "progress": "/body/p[50]",
            "percentage": 0.5,
            "device": "KindleOasis",
            "device_id": "kindle-001"
        });
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/kosync/syncs/progress")
                    .header("content-type", "application/json")
                    .header("x-auth-user", "reader4")
                    .header("x-auth-key", &password_hash)
                    .body(axum::body::Body::from(
                        serde_json::to_string(&dev1_prog).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Device 2 reads (gets Device 1's progress)
        let get_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/kosync/syncs/progress/{}", doc_hash))
                    .header("x-auth-user", "reader4")
                    .header("x-auth-key", &password_hash)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = to_bytes(get_resp.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["percentage"], 0.5);
        assert_eq!(parsed["device"], "KindleOasis"); // Device 1's name

        // Device 2 writes its own progress
        let dev2_prog = serde_json::json!({
            "document": doc_hash,
            "progress": "/body/p[75]",
            "percentage": 0.75,
            "device": "Android",
            "device_id": "phone-001"
        });
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/kosync/syncs/progress")
                    .header("content-type", "application/json")
                    .header("x-auth-user", "reader4")
                    .header("x-auth-key", &password_hash)
                    .body(axum::body::Body::from(
                        serde_json::to_string(&dev2_prog).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Device 1 reads again (should get Device 2's latest)
        let final_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/kosync/syncs/progress/{}", doc_hash))
                    .header("x-auth-user", "reader4")
                    .header("x-auth-key", &password_hash)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = to_bytes(final_resp.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["percentage"], 0.75);
        assert_eq!(parsed["device"], "Android");
    }
}
