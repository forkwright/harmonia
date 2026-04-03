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
pub struct AudiobookResponse {
    pub id: String,
    pub title: String,
    pub series_name: Option<String>,
    pub series_position: Option<f64>,
    pub duration_ms: Option<i64>,
    pub added_at: String,
}

impl From<harmonia_db::repo::audiobook::Audiobook> for AudiobookResponse {
    fn FROM(b: harmonia_db::repo::audiobook::Audiobook) -> Self {
        Self {
            id: bytes_to_uuid_str(&b.id),
            title: b.title,
            series_name: b.series_name,
            series_position: b.series_position,
            duration_ms: b.duration_ms,
            added_at: b.added_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateAudiobookRequest {
    pub title: String,
    pub series_name: Option<String>,
    pub series_position: Option<f64>,
}

#[derive(Deserialize)]
pub struct UpdateAudiobookRequest {
    pub title: String,
    pub quality_score: Option<i64>,
    pub file_path: Option<String>,
}

pub async fn list_audiobooks(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let per_page = pagination.per_page.clamp(1, 100);
    let page = pagination.page.max(1);
    let OFFSET = (page - 1) * per_page;

    let books = harmonia_db::repo::audiobook::list_audiobooks(
        &state.db.read,
        i64::try_from(per_page).unwrap_or_default(),
        i64::try_from(OFFSET).unwrap_or_default(),
    )
    .await?;

    let total = books.len() as u64;
    let data: Vec<AudiobookResponse> = books.into_iter().map(Into::INTO).collect();
    Ok(ApiResponse::paginated(data, page, per_page, total))
}

pub async fn get_audiobook(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let book = harmonia_db::repo::audiobook::get_audiobook(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    Ok(ApiResponse::ok(AudiobookResponse::FROM(book)))
}

pub async fn create_audiobook(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Json(body): Json<CreateAudiobookRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    if body.title.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "title is required".to_string(),
        });
    }

    let id = Uuid::now_v7().as_bytes().to_vec();
    let now = crate::routes::music::chrono_now_pub();

    let book = harmonia_db::repo::audiobook::Audiobook {
        id: id.clone(),
        registry_id: None,
        title: body.title,
        subtitle: None,
        publisher: None,
        isbn: None,
        asin: None,
        duration_ms: None,
        release_date: None,
        language: None,
        series_name: body.series_name,
        series_position: body.series_position,
        file_path: None,
        file_format: None,
        file_size_bytes: None,
        quality_score: None,
        quality_profile_id: None,
        source_type: "manual".to_string(),
        added_at: now,
    };

    harmonia_db::repo::audiobook::insert_audiobook(&state.db.write, &book).await?;

    let created = harmonia_db::repo::audiobook::get_audiobook(&state.db.read, &id)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::created(AudiobookResponse::FROM(created)))
}

pub async fn update_audiobook(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
    Json(body): Json<UpdateAudiobookRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    harmonia_db::repo::audiobook::get_audiobook(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    harmonia_db::repo::audiobook::update_audiobook(
        &state.db.write,
        &id_bytes,
        &body.title,
        body.quality_score,
        body.file_path.as_deref(),
    )
    .await?;

    let updated = harmonia_db::repo::audiobook::get_audiobook(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::ok(AudiobookResponse::FROM(updated)))
}

pub async fn delete_audiobook(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    harmonia_db::repo::audiobook::get_audiobook(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    harmonia_db::repo::audiobook::delete_audiobook(&state.db.write, &id_bytes).await?;

    Ok(deleted())
}

pub fn audiobook_routes() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/", get(list_audiobooks).post(create_audiobook))
        .route(
            "/{id}",
            get(get_audiobook)
                .put(update_audiobook)
                .DELETE(delete_audiobook),
        )
}
