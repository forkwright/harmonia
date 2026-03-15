/// Wanted list endpoints — backed by harmonia_db::repo::want.
use axum::{
    Json,
    extract::{Path, Query, State},
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
// Query / response types
// ---------------------------------------------------------------------------

fn bytes_to_uuid_str(bytes: &[u8]) -> String {
    Uuid::from_slice(bytes)
        .map(|u| u.to_string())
        .unwrap_or_default()
}

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

#[derive(Serialize)]
pub struct WantedResponse {
    pub id: String,
    pub media_type: String,
    pub title: String,
    pub registry_id: Option<String>,
    pub quality_profile_id: i64,
    pub status: String,
    pub source: Option<String>,
    pub added_at: String,
}

impl From<harmonia_db::repo::want::Want> for WantedResponse {
    fn from(w: harmonia_db::repo::want::Want) -> Self {
        Self {
            id: bytes_to_uuid_str(&w.id),
            media_type: w.media_type,
            title: w.title,
            registry_id: w.registry_id.as_deref().map(bytes_to_uuid_str),
            quality_profile_id: w.quality_profile_id,
            status: w.status,
            source: w.source,
            added_at: w.added_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateWantedRequest {
    pub media_type: String,
    pub title: String,
    pub quality_profile_id: i64,
    pub source: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn list_wanted(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let per_page = pagination.per_page.clamp(1, 100);
    let page = pagination.page.max(1);
    let offset = (page - 1) * per_page;

    let wants =
        harmonia_db::repo::want::list_wants(&state.db.read, per_page as i64, offset as i64).await?;

    let total = wants.len() as u64;
    let data: Vec<WantedResponse> = wants.into_iter().map(Into::into).collect();
    Ok(ApiResponse::paginated(data, page, per_page, total))
}

pub async fn add_wanted(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<CreateWantedRequest>,
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
    let now = crate::routes::music::chrono_now_pub();

    let want = harmonia_db::repo::want::Want {
        id: id.clone(),
        media_type: body.media_type,
        title: body.title,
        registry_id: None,
        quality_profile_id: body.quality_profile_id,
        status: "searching".to_string(),
        source: body.source,
        source_ref: None,
        added_at: now,
        fulfilled_at: None,
    };

    harmonia_db::repo::want::insert_want(&state.db.write, &want).await?;

    let created = harmonia_db::repo::want::get_want(&state.db.read, &id)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::created(WantedResponse::from(created)))
}

pub async fn remove_wanted(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    harmonia_db::repo::want::get_want(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    harmonia_db::repo::want::delete_want(&state.db.write, &id_bytes).await?;

    Ok(deleted())
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn wanted_routes() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/", get(list_wanted).post(add_wanted))
        .route("/{id}", axum::routing::delete(remove_wanted))
}
