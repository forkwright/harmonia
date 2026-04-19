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
pub struct MovieResponse {
    pub id: String,
    pub title: String,
    pub year: Option<i64>,
    pub runtime_min: Option<i64>,
    pub overview: Option<String>,
    pub added_at: String,
}

impl From<apotheke::repo::movie::Movie> for MovieResponse {
    fn from(m: apotheke::repo::movie::Movie) -> Self {
        Self {
            id: bytes_to_uuid_str(&m.id),
            title: m.title,
            year: m.year,
            runtime_min: m.runtime_min,
            overview: m.overview,
            added_at: m.added_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateMovieRequest {
    pub title: String,
    pub year: Option<i64>,
    pub runtime_min: Option<i64>,
    pub overview: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateMovieRequest {
    pub title: String,
    pub quality_score: Option<i64>,
    pub file_path: Option<String>,
}

pub async fn list_movies(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let per_page = pagination.per_page.clamp(1, 100);
    let page = pagination.page.max(1);
    let offset = (page - 1) * per_page;

    let movies =
        apotheke::repo::movie::list_movies(&state.db.read, per_page as i64, offset as i64).await?;

    let total = movies.len() as u64;
    let data: Vec<MovieResponse> = movies.into_iter().map(Into::into).collect();
    Ok(ApiResponse::paginated(data, page, per_page, total))
}

pub async fn get_movie(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let movie = apotheke::repo::movie::get_movie(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    Ok(ApiResponse::ok(MovieResponse::from(movie)))
}

pub async fn create_movie(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Json(body): Json<CreateMovieRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    if body.title.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "title is required".to_string(),
        });
    }

    let id = Uuid::now_v7().as_bytes().to_vec();
    let now = chrono_now_pub();

    let movie = apotheke::repo::movie::Movie {
        id: id.clone(),
        registry_id: None,
        title: body.title,
        original_title: None,
        year: body.year,
        tmdb_id: None,
        imdb_id: None,
        runtime_min: body.runtime_min,
        overview: body.overview,
        certification: None,
        file_path: None,
        file_format: None,
        file_size_bytes: None,
        resolution: None,
        codec: None,
        hdr_type: None,
        quality_score: None,
        quality_profile_id: None,
        source_type: "manual".to_string(),
        added_at: now,
    };

    apotheke::repo::movie::insert_movie(&state.db.write, &movie).await?;

    let created = apotheke::repo::movie::get_movie(&state.db.read, &id)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::created(MovieResponse::from(created)))
}

pub async fn update_movie(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
    Json(body): Json<UpdateMovieRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    apotheke::repo::movie::get_movie(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    apotheke::repo::movie::update_movie(
        &state.db.write,
        &id_bytes,
        &body.title,
        body.quality_score,
        body.file_path.as_deref(),
    )
    .await?;

    let updated = apotheke::repo::movie::get_movie(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::ok(MovieResponse::from(updated)))
}

pub async fn delete_movie(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    apotheke::repo::movie::get_movie(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    apotheke::repo::movie::delete_movie(&state.db.write, &id_bytes).await?;

    Ok(deleted())
}

pub fn movie_routes() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/", get(list_movies).post(create_movie))
        .route(
            "/{id}",
            get(get_movie).put(update_movie).delete(delete_movie),
        )
}
