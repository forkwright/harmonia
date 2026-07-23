/// Indexer management endpoints.
use std::collections::BTreeMap;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use exousia::{AuthenticatedUser, RequireAdmin};
use serde::{Deserialize, Serialize};

use crate::error::ParocheError;
use crate::response::{ApiResponse, deleted};
use crate::state::{AppState, ServiceError};

// ---------------------------------------------------------------------------
// Query / request / response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_per_page")]
    pub per_page: u64,
}
fn default_page() -> u64 {
    1
}
fn default_per_page() -> u64 {
    20
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct IndexerRow {
    id: i64,
    name: String,
    url: String,
    protocol: String,
    api_key: Option<String>,
    enabled: bool,
    cf_bypass: bool,
    status: String,
    last_tested: Option<String>,
    #[sqlx(rename = "caps_json")]
    _caps_json: Option<String>,
    priority: i64,
    added_at: String,
    settings_json: Option<String>,
}

#[derive(Serialize)]
pub struct IndexerResponse {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub protocol: String,
    pub has_api_key: bool,
    pub enabled: bool,
    pub cf_bypass: bool,
    pub status: String,
    pub last_tested: Option<String>,
    pub priority: i64,
    pub added_at: String,
    /// Configured setting names only — never values (may carry passwords).
    pub settings_keys: Vec<String>,
}

impl From<IndexerRow> for IndexerResponse {
    fn from(r: IndexerRow) -> Self {
        let settings_keys = r
            .settings_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<BTreeMap<String, String>>(json).ok())
            .map(|settings| settings.into_keys().collect())
            .unwrap_or_default();
        Self {
            id: r.id,
            name: r.name,
            url: r.url,
            protocol: r.protocol,
            has_api_key: r.api_key.is_some(),
            enabled: r.enabled,
            cf_bypass: r.cf_bypass,
            status: r.status,
            last_tested: r.last_tested,
            priority: r.priority,
            added_at: r.added_at,
            settings_keys,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateIndexerRequest {
    pub name: String,
    pub url: String,
    #[serde(default = "default_protocol")]
    pub protocol: String,
    pub api_key: Option<String>,
    #[serde(default)]
    pub cf_bypass: bool,
    #[serde(default = "default_priority")]
    pub priority: i64,
    #[serde(default)]
    pub settings: Option<BTreeMap<String, String>>,
}
fn default_protocol() -> String {
    "torznab".to_string()
}
fn default_priority() -> i64 {
    50
}

#[derive(Deserialize)]
pub struct UpdateIndexerRequest {
    pub name: Option<String>,
    pub url: Option<String>,
    pub protocol: Option<String>,
    pub api_key: Option<String>,
    pub cf_bypass: Option<bool>,
    pub enabled: Option<bool>,
    pub priority: Option<i64>,
    #[serde(default)]
    pub settings: Option<BTreeMap<String, String>>,
}

// ---------------------------------------------------------------------------
// SQL helpers
// ---------------------------------------------------------------------------

const SELECT_INDEXER: &str = "\
    SELECT id, name, url, protocol, api_key, enabled, cf_bypass, \
           status, last_tested, caps_json, priority, added_at, settings_json \
    FROM indexers";

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn list_indexers(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let per_page = pagination.per_page.clamp(1, 100);
    let page = pagination.page.max(1);
    let offset = (page - 1) * per_page;

    let q = format!("{SELECT_INDEXER} ORDER BY priority DESC, name ASC LIMIT ? OFFSET ?");
    let rows = sqlx::query_as::<_, IndexerRow>(&q)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM indexers")
        .fetch_one(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?;

    let data: Vec<IndexerResponse> = rows.into_iter().map(Into::into).collect();
    Ok(ApiResponse::paginated(data, page, per_page, total as u64))
}

pub async fn get_indexer(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let q = format!("{SELECT_INDEXER} WHERE id = ?");
    let row = sqlx::query_as::<_, IndexerRow>(&q)
        .bind(id)
        .fetch_optional(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?
        .ok_or(ParocheError::NotFound)?;

    Ok(ApiResponse::ok(IndexerResponse::from(row)))
}

pub async fn create_indexer(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Json(body): Json<CreateIndexerRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    if body.name.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "name is required".to_string(),
        });
    }
    if body.url.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "url is required".to_string(),
        });
    }

    let now = crate::routes::music::chrono_now_pub();
    let settings_json = body
        .settings
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|_| ParocheError::Internal)?;

    // NOTE: 'unknown' violates the indexers.status CHECK (active/degraded/
    // failed) — a newly created indexer starts 'active' until the next
    // caps/test probe says otherwise, same as eksetasis::repo::insert_indexer.
    let result = sqlx::query(
        "INSERT INTO indexers (name, url, protocol, api_key, cf_bypass, enabled, status, priority, added_at, settings_json) \
         VALUES (?, ?, ?, ?, ?, 1, 'active', ?, ?, ?)",
    )
    .bind(&body.name)
    .bind(&body.url)
    .bind(&body.protocol)
    .bind(&body.api_key)
    .bind(body.cf_bypass)
    .bind(body.priority)
    .bind(&now)
    .bind(&settings_json)
    .execute(&state.db.write)
    .await
    .map_err(|_| ParocheError::Internal)?;

    let id = result.last_insert_rowid();
    let q = format!("{SELECT_INDEXER} WHERE id = ?");
    let row = sqlx::query_as::<_, IndexerRow>(&q)
        .bind(id)
        .fetch_one(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?;

    Ok(ApiResponse::created(IndexerResponse::from(row)))
}

pub async fn update_indexer(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<i64>,
    Json(body): Json<UpdateIndexerRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let q = format!("{SELECT_INDEXER} WHERE id = ?");
    let existing = sqlx::query_as::<_, IndexerRow>(&q)
        .bind(id)
        .fetch_optional(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?
        .ok_or(ParocheError::NotFound)?;

    let name = body.name.unwrap_or(existing.name);
    let url = body.url.unwrap_or(existing.url);
    let protocol = body.protocol.unwrap_or(existing.protocol);
    let api_key = body.api_key.or(existing.api_key);
    let cf_bypass = body.cf_bypass.unwrap_or(existing.cf_bypass);
    let enabled = body.enabled.unwrap_or(existing.enabled);
    let priority = body.priority.unwrap_or(existing.priority);
    let settings_json = match body.settings {
        Some(settings) => {
            Some(serde_json::to_string(&settings).map_err(|_| ParocheError::Internal)?)
        }
        None => existing.settings_json,
    };

    sqlx::query(
        "UPDATE indexers SET name = ?, url = ?, protocol = ?, api_key = ?, \
         cf_bypass = ?, enabled = ?, priority = ?, settings_json = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(&url)
    .bind(&protocol)
    .bind(&api_key)
    .bind(cf_bypass)
    .bind(enabled)
    .bind(priority)
    .bind(&settings_json)
    .bind(id)
    .execute(&state.db.write)
    .await
    .map_err(|_| ParocheError::Internal)?;

    let q2 = format!("{SELECT_INDEXER} WHERE id = ?");
    let updated = sqlx::query_as::<_, IndexerRow>(&q2)
        .bind(id)
        .fetch_one(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?;

    Ok(ApiResponse::ok(IndexerResponse::from(updated)))
}

pub async fn delete_indexer(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let affected = sqlx::query("DELETE FROM indexers WHERE id = ?")
        .bind(id)
        .execute(&state.db.write)
        .await
        .map_err(|_| ParocheError::Internal)?
        .rows_affected();

    if affected == 0 {
        return Err(ParocheError::NotFound);
    }

    Ok(deleted())
}

pub async fn test_indexer(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    // Verify indexer exists.
    sqlx::query("SELECT 1 FROM indexers WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?
        .ok_or(ParocheError::NotFound)?;

    let result = state
        .search
        .test_indexer(id)
        .await
        .map_err(indexer_service_error)?;

    Ok(ApiResponse::ok(result))
}

pub async fn refresh_caps(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    sqlx::query("SELECT 1 FROM indexers WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?
        .ok_or(ParocheError::NotFound)?;

    let caps = state
        .search
        .refresh_caps(id)
        .await
        .map_err(indexer_service_error)?;

    Ok(ApiResponse::ok(caps))
}

fn indexer_service_error(error: ServiceError) -> ParocheError {
    match error {
        ServiceError::NotFound => ParocheError::NotFound,
        ServiceError::NotAvailable => ParocheError::Unavailable,
        ServiceError::InvalidInput(message) => ParocheError::Validation { message },
        ServiceError::Internal(_) => ParocheError::Internal,
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn indexer_routes() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/", get(list_indexers).post(create_indexer))
        .route(
            "/{id}",
            get(get_indexer).put(update_indexer).delete(delete_indexer),
        )
        .route("/{id}/test", post(test_indexer))
        .route("/{id}/caps", post(refresh_caps))
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use super::*;
    use crate::test_helpers::{admin_token, test_state};

    async fn body_json(resp: axum::response::Response) -> serde_json::Value {
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn create_indexer_persists_settings_and_response_carries_keys_only() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let pool = state.db.read.clone();
        let app = indexer_routes().with_state(state);

        let request_body = serde_json::json!({
            "name": "Cardigann Sample",
            "url": "sample-tracker",
            "protocol": "cardigann",
            "settings": {"sort": "hunter2secret"},
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let bytes_resp = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let raw = String::from_utf8(bytes_resp.to_vec()).unwrap();
        assert!(
            !raw.contains("hunter2secret"),
            "settings value leaked into response: {raw}"
        );
        let payload: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            payload["data"]["settings_keys"],
            serde_json::json!(["sort"])
        );
        assert!(payload["data"].get("settings").is_none());

        let id = payload["data"]["id"].as_i64().unwrap();
        let settings_json: Option<String> =
            sqlx::query_scalar("SELECT settings_json FROM indexers WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            settings_json.as_deref(),
            Some(r#"{"sort":"hunter2secret"}"#)
        );
    }

    #[tokio::test]
    async fn update_indexer_persists_settings_and_response_carries_keys_only() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let pool = state.db.read.clone();
        let app = indexer_routes().with_state(state);

        let create_body = serde_json::json!({
            "name": "Plain",
            "url": "https://example.com/api",
            "protocol": "torznab",
        });
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(create_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let created = body_json(resp).await;
        let id = created["data"]["id"].as_i64().unwrap();
        assert_eq!(created["data"]["settings_keys"], serde_json::json!([]));

        let update_body = serde_json::json!({
            "settings": {"password": "supersecretvalue"},
        });
        let resp = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/{id}"))
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(update_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes_resp = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let raw = String::from_utf8(bytes_resp.to_vec()).unwrap();
        assert!(
            !raw.contains("supersecretvalue"),
            "settings value leaked into response: {raw}"
        );
        let payload: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            payload["data"]["settings_keys"],
            serde_json::json!(["password"])
        );

        let settings_json: Option<String> =
            sqlx::query_scalar("SELECT settings_json FROM indexers WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            settings_json.as_deref(),
            Some(r#"{"password":"supersecretvalue"}"#)
        );
    }
}
