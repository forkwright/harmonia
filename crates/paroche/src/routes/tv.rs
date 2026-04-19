use axum::Json;
use axum::extract::{Path, Query, State};
use exousia::{AuthenticatedUser, RequireAdmin};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ParocheError;
use crate::response::{ApiResponse, deleted};
use crate::routes::music::chrono_now_pub;
use crate::state::AppState;

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

fn bytes_to_uuid_str(bytes: &[u8]) -> String {
    Uuid::from_slice(bytes)
        .map(|u| u.to_string())
        .unwrap_or_default()
}

#[derive(Serialize)]
pub struct TvSeriesResponse {
    pub id: String,
    pub title: String,
    pub status: String,
    pub overview: Option<String>,
    pub network: Option<String>,
    pub added_at: String,
}

impl From<apotheke::repo::tv::TvSeries> for TvSeriesResponse {
    fn from(s: apotheke::repo::tv::TvSeries) -> Self {
        Self {
            id: bytes_to_uuid_str(&s.id),
            title: s.title,
            status: s.status,
            overview: s.overview,
            network: s.network,
            added_at: s.added_at,
        }
    }
}

#[derive(Serialize)]
pub struct TvEpisodeResponse {
    pub id: String,
    pub episode_number: i64,
    pub title: Option<String>,
    pub air_date: Option<String>,
    pub added_at: String,
}

impl From<apotheke::repo::tv::TvEpisode> for TvEpisodeResponse {
    fn from(e: apotheke::repo::tv::TvEpisode) -> Self {
        Self {
            id: bytes_to_uuid_str(&e.id),
            episode_number: e.episode_number,
            title: e.title,
            air_date: e.air_date,
            added_at: e.added_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateSeriesRequest {
    pub title: String,
    pub status: Option<String>,
    pub overview: Option<String>,
    pub network: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateSeriesRequest {
    pub title: String,
    pub status: String,
    pub quality_profile_id: Option<i64>,
}

pub async fn list_series(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let per_page = pagination.per_page.clamp(1, 100);
    let page = pagination.page.max(1);
    let offset = (page - 1) * per_page;

    let series =
        apotheke::repo::tv::list_series(&state.db.read, per_page as i64, offset as i64).await?;

    let total = series.len() as u64;
    let data: Vec<TvSeriesResponse> = series.into_iter().map(Into::into).collect();
    Ok(ApiResponse::paginated(data, page, per_page, total))
}

pub async fn get_series(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let series = apotheke::repo::tv::get_series(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    Ok(ApiResponse::ok(TvSeriesResponse::from(series)))
}

pub async fn create_series(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Json(body): Json<CreateSeriesRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    if body.title.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "title is required".to_string(),
        });
    }

    let id = Uuid::now_v7().as_bytes().to_vec();
    let now = chrono_now_pub();

    let series = apotheke::repo::tv::TvSeries {
        id: id.clone(),
        registry_id: None,
        title: body.title,
        tmdb_id: None,
        tvdb_id: None,
        imdb_id: None,
        status: body.status.unwrap_or_else(|| "unknown".to_string()),
        overview: body.overview,
        network: body.network,
        quality_profile_id: None,
        added_at: now,
    };

    apotheke::repo::tv::insert_series(&state.db.write, &series).await?;

    let created = apotheke::repo::tv::get_series(&state.db.read, &id)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::created(TvSeriesResponse::from(created)))
}

pub async fn update_series(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
    Json(body): Json<UpdateSeriesRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    apotheke::repo::tv::get_series(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    apotheke::repo::tv::update_series(
        &state.db.write,
        &id_bytes,
        &body.title,
        &body.status,
        body.quality_profile_id,
    )
    .await?;

    let updated = apotheke::repo::tv::get_series(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::ok(TvSeriesResponse::from(updated)))
}

pub async fn delete_series(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    apotheke::repo::tv::get_series(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    apotheke::repo::tv::delete_series(&state.db.write, &id_bytes).await?;

    Ok(deleted())
}

pub fn tv_routes() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/", get(list_series).post(create_series))
        .route(
            "/{id}",
            get(get_series).put(update_series).delete(delete_series),
        )
}
