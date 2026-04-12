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
    routes::music::chrono_now_pub,
    state::AppState,
};

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
pub struct ComicResponse {
    pub id: String,
    pub series_name: String,
    pub volume: Option<i64>,
    pub issue_number: Option<f64>,
    pub title: Option<String>,
    pub publisher: Option<String>,
    pub added_at: String,
}

impl From<harmonia_db::repo::comic::Comic> for ComicResponse {
    fn from(c: harmonia_db::repo::comic::Comic) -> Self {
        Self {
            id: bytes_to_uuid_str(&c.id),
            series_name: c.series_name,
            volume: c.volume,
            issue_number: c.issue_number,
            title: c.title,
            publisher: c.publisher,
            added_at: c.added_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateComicRequest {
    pub series_name: String,
    pub volume: Option<i64>,
    pub issue_number: Option<f64>,
    pub title: Option<String>,
    pub publisher: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateComicRequest {
    pub title: Option<String>,
    pub quality_score: Option<i64>,
    pub file_path: Option<String>,
}

pub async fn list_comics(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let per_page = pagination.per_page.clamp(1, 100);
    let page = pagination.page.max(1);
    let offset = (page - 1) * per_page;

    let comics = harmonia_db::repo::comic::list_comics(
        &state.db.read,
        i64::try_from(per_page).unwrap_or_default(),
        i64::try_from(offset).unwrap_or_default(),
    )
    .await?;

    let total = comics.len() as u64;
    let data: Vec<ComicResponse> = comics.into_iter().map(Into::into).collect();
    Ok(ApiResponse::paginated(data, page, per_page, total))
}

pub async fn get_comic(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let comic = harmonia_db::repo::comic::get_comic(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    Ok(ApiResponse::ok(ComicResponse::from(comic)))
}

pub async fn create_comic(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Json(body): Json<CreateComicRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    if body.series_name.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "series_name is required".to_string(),
        });
    }

    let id = Uuid::now_v7().as_bytes().to_vec();
    let now = chrono_now_pub();

    let comic = harmonia_db::repo::comic::Comic {
        id: id.clone(),
        registry_id: None,
        series_name: body.series_name,
        volume: body.volume,
        issue_number: body.issue_number,
        title: body.title,
        publisher: body.publisher,
        release_date: None,
        page_count: None,
        summary: None,
        language: None,
        comicinfo_writer: None,
        comicinfo_penciller: None,
        comicinfo_inker: None,
        comicinfo_colorist: None,
        file_path: None,
        file_format: None,
        file_size_bytes: None,
        quality_score: None,
        quality_profile_id: None,
        source_type: "manual".to_string(),
        added_at: now,
    };

    harmonia_db::repo::comic::insert_comic(&state.db.write, &comic).await?;

    let created = harmonia_db::repo::comic::get_comic(&state.db.read, &id)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::created(ComicResponse::from(created)))
}

pub async fn update_comic(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
    Json(body): Json<UpdateComicRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    harmonia_db::repo::comic::get_comic(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    harmonia_db::repo::comic::update_comic(
        &state.db.write,
        &id_bytes,
        body.title.as_deref(),
        body.quality_score,
        body.file_path.as_deref(),
    )
    .await?;

    let updated = harmonia_db::repo::comic::get_comic(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::ok(ComicResponse::from(updated)))
}

pub async fn delete_comic(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    harmonia_db::repo::comic::get_comic(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    harmonia_db::repo::comic::delete_comic(&state.db.write, &id_bytes).await?;

    Ok(deleted())
}

pub fn comic_routes() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/", get(list_comics).post(create_comic))
        .route(
            "/{id}",
            get(get_comic).put(update_comic).delete(delete_comic),
        )
}
