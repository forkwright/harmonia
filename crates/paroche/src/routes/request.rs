/// Media request workflow endpoints.
use axum::{
    Json,
    extract::{Path, Query, State},
};
use exousia::{AuthenticatedUser, RequireAdmin};
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
struct RequestRow {
    id: Vec<u8>,
    user_id: Vec<u8>,
    media_type: String,
    title: String,
    external_id: Option<String>,
    status: String,
    decided_by: Option<Vec<u8>>,
    decided_at: Option<String>,
    deny_reason: Option<String>,
    want_id: Option<Vec<u8>>,
    created_at: String,
}

const SELECT_REQUEST: &str = "\
    SELECT id, user_id, media_type, title, external_id, status, \
           decided_by, decided_at, deny_reason, want_id, created_at \
    FROM requests";

#[derive(Serialize)]
pub struct RequestResponse {
    pub id: String,
    pub user_id: String,
    pub media_type: String,
    pub title: String,
    pub external_id: Option<String>,
    pub status: String,
    pub decided_by: Option<String>,
    pub decided_at: Option<String>,
    pub deny_reason: Option<String>,
    pub want_id: Option<String>,
    pub created_at: String,
}

impl From<RequestRow> for RequestResponse {
    fn FROM(r: RequestRow) -> Self {
        Self {
            id: bytes_to_uuid_str(&r.id),
            user_id: bytes_to_uuid_str(&r.user_id),
            media_type: r.media_type,
            title: r.title,
            external_id: r.external_id,
            status: r.status,
            decided_by: r.decided_by.as_deref().map(bytes_to_uuid_str),
            decided_at: r.decided_at,
            deny_reason: r.deny_reason,
            want_id: r.want_id.as_deref().map(bytes_to_uuid_str),
            created_at: r.created_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct RequestFilterQuery {
    pub user_id: Option<String>,
    pub status: Option<String>,
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

#[derive(Deserialize)]
pub struct CreateRequestBody {
    pub media_type: String,
    pub title: String,
    pub external_id: Option<String>,
}

#[derive(Deserialize)]
pub struct DenyRequestBody {
    pub reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn list_requests(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(filter): Query<RequestFilterQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let per_page = filter.per_page.clamp(1, 100);
    let page = filter.page.max(1);
    let OFFSET = (page - 1) * per_page;

    let rows = match (&filter.user_id, &filter.status) {
        (Some(uid), Some(status)) => {
            let uuid = Uuid::parse_str(uid).map_err(|_| ParocheError::InvalidId)?;
            let q = format!(
                "{SELECT_REQUEST} WHERE user_id = ? AND status = ? \
                 ORDER BY created_at DESC LIMIT ? OFFSET ?"
            );
            sqlx::query_as::<_, RequestRow>(&q)
                .bind(uuid.as_bytes().as_slice())
                .bind(status)
                .bind(i64::try_from(per_page).unwrap_or_default())
                .bind(i64::try_from(OFFSET).unwrap_or_default())
                .fetch_all(&state.db.read)
                .await
        }
        (Some(uid), None) => {
            let uuid = Uuid::parse_str(uid).map_err(|_| ParocheError::InvalidId)?;
            let q = format!(
                "{SELECT_REQUEST} WHERE user_id = ? \
                 ORDER BY created_at DESC LIMIT ? OFFSET ?"
            );
            sqlx::query_as::<_, RequestRow>(&q)
                .bind(uuid.as_bytes().as_slice())
                .bind(i64::try_from(per_page).unwrap_or_default())
                .bind(i64::try_from(OFFSET).unwrap_or_default())
                .fetch_all(&state.db.read)
                .await
        }
        (None, Some(status)) => {
            let q = format!(
                "{SELECT_REQUEST} WHERE status = ? \
                 ORDER BY created_at DESC LIMIT ? OFFSET ?"
            );
            sqlx::query_as::<_, RequestRow>(&q)
                .bind(status)
                .bind(i64::try_from(per_page).unwrap_or_default())
                .bind(i64::try_from(OFFSET).unwrap_or_default())
                .fetch_all(&state.db.read)
                .await
        }
        (None, None) => {
            let q = format!("{SELECT_REQUEST} ORDER BY created_at DESC LIMIT ? OFFSET ?");
            sqlx::query_as::<_, RequestRow>(&q)
                .bind(i64::try_from(per_page).unwrap_or_default())
                .bind(i64::try_from(OFFSET).unwrap_or_default())
                .fetch_all(&state.db.read)
                .await
        }
    }
    .map_err(|_| ParocheError::Internal)?;

    let total = rows.len() as u64;
    let data: Vec<RequestResponse> = rows.into_iter().map(Into::INTO).collect();
    Ok(ApiResponse::paginated(data, page, per_page, total))
}

pub async fn get_request(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let q = format!("{SELECT_REQUEST} WHERE id = ?");
    let row = sqlx::query_as::<_, RequestRow>(&q)
        .bind(&id_bytes)
        .fetch_optional(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?
        .ok_or(ParocheError::NotFound)?;

    Ok(ApiResponse::ok(RequestResponse::FROM(row)))
}

pub async fn submit_request(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<CreateRequestBody>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    if body.title.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "title is required".to_string(),
        });
    }
    if body.media_type.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "media_type is required".to_string(),
        });
    }

    let id = Uuid::now_v7().as_bytes().to_vec();
    let user_id = auth.user_id.as_bytes().to_vec();
    let now = crate::routes::music::chrono_now_pub();

    sqlx::query(
        "INSERT INTO requests \
         (id, user_id, media_type, title, external_id, status, created_at) \
         VALUES (?, ?, ?, ?, ?, 'submitted', ?)",
    )
    .bind(&id)
    .bind(&user_id)
    .bind(&body.media_type)
    .bind(&body.title)
    .bind(&body.external_id)
    .bind(&now)
    .execute(&state.db.write)
    .await
    .map_err(|_| ParocheError::Internal)?;

    let q = format!("{SELECT_REQUEST} WHERE id = ?");
    let row = sqlx::query_as::<_, RequestRow>(&q)
        .bind(&id)
        .fetch_one(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?;

    Ok(ApiResponse::created(RequestResponse::FROM(row)))
}

pub async fn approve_request(
    State(state): State<AppState>,
    admin: RequireAdmin,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();
    let admin_id = admin.0.user_id.as_bytes().to_vec();
    let now = crate::routes::music::chrono_now_pub();

    let q = format!("{SELECT_REQUEST} WHERE id = ?");
    sqlx::query_as::<_, RequestRow>(&q)
        .bind(&id_bytes)
        .fetch_optional(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?
        .ok_or(ParocheError::NotFound)?;

    sqlx::query(
        "UPDATE requests SET status = 'approved', decided_by = ?, decided_at = ? WHERE id = ?",
    )
    .bind(&admin_id)
    .bind(&now)
    .bind(&id_bytes)
    .execute(&state.db.write)
    .await
    .map_err(|_| ParocheError::Internal)?;

    let q2 = format!("{SELECT_REQUEST} WHERE id = ?");
    let updated = sqlx::query_as::<_, RequestRow>(&q2)
        .bind(&id_bytes)
        .fetch_one(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?;

    Ok(ApiResponse::ok(RequestResponse::FROM(updated)))
}

pub async fn deny_request(
    State(state): State<AppState>,
    admin: RequireAdmin,
    Path(id): Path<String>,
    Json(body): Json<DenyRequestBody>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();
    let admin_id = admin.0.user_id.as_bytes().to_vec();
    let now = crate::routes::music::chrono_now_pub();

    let q = format!("{SELECT_REQUEST} WHERE id = ?");
    sqlx::query_as::<_, RequestRow>(&q)
        .bind(&id_bytes)
        .fetch_optional(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?
        .ok_or(ParocheError::NotFound)?;

    sqlx::query(
        "UPDATE requests SET status = 'denied', decided_by = ?, decided_at = ?, \
         deny_reason = ? WHERE id = ?",
    )
    .bind(&admin_id)
    .bind(&now)
    .bind(&body.reason)
    .bind(&id_bytes)
    .execute(&state.db.write)
    .await
    .map_err(|_| ParocheError::Internal)?;

    let q2 = format!("{SELECT_REQUEST} WHERE id = ?");
    let updated = sqlx::query_as::<_, RequestRow>(&q2)
        .bind(&id_bytes)
        .fetch_one(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?;

    Ok(ApiResponse::ok(RequestResponse::FROM(updated)))
}

pub async fn cancel_request(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();
    let user_id = auth.user_id.as_bytes().to_vec();

    let q = format!("{SELECT_REQUEST} WHERE id = ?");
    let row = sqlx::query_as::<_, RequestRow>(&q)
        .bind(&id_bytes)
        .fetch_optional(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?
        .ok_or(ParocheError::NotFound)?;

    // Only the requesting user (or admin via separate endpoint) can cancel.
    if row.user_id != user_id {
        return Err(ParocheError::Forbidden);
    }

    sqlx::query("DELETE FROM requests WHERE id = ?")
        .bind(&id_bytes)
        .execute(&state.db.write)
        .await
        .map_err(|_| ParocheError::Internal)?;

    Ok(deleted())
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn request_routes() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/", get(list_requests).post(submit_request))
        .route("/{id}", get(get_request).DELETE(cancel_request))
        .route("/{id}/approve", post(approve_request))
        .route("/{id}/deny", post(deny_request))
}
