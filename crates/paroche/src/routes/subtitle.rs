/// Subtitle management endpoints.
use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use exousia::AuthenticatedUser;
use serde::Serialize;
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
struct SubtitleRow {
    id: Vec<u8>,
    media_id: Vec<u8>,
    language: String,
    format: String,
    file_path: String,
    provider: String,
    #[sqlx(rename = "provider_id")]
    _provider_id: String,
    hearing_impaired: bool,
    forced: bool,
    score: f64,
    acquired_at: String,
}

const SELECT_SUBTITLE: &str = "\
    SELECT id, media_id, language, format, file_path, provider, provider_id, \
           hearing_impaired, forced, score, acquired_at \
    FROM subtitles";

#[derive(Serialize)]
pub struct SubtitleResponse {
    pub id: String,
    pub media_id: String,
    pub language: String,
    pub format: String,
    pub file_path: String,
    pub provider: String,
    pub hearing_impaired: bool,
    pub forced: bool,
    pub score: f64,
    pub acquired_at: String,
}

impl From<SubtitleRow> for SubtitleResponse {
    fn from(r: SubtitleRow) -> Self {
        Self {
            id: bytes_to_uuid_str(&r.id),
            media_id: bytes_to_uuid_str(&r.media_id),
            language: r.language,
            format: r.format,
            file_path: r.file_path,
            provider: r.provider,
            hearing_impaired: r.hearing_impaired,
            forced: r.forced,
            score: r.score,
            acquired_at: r.acquired_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn list_subtitles_for_media(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let media_id = uuid.as_bytes().to_vec();

    let q = format!("{SELECT_SUBTITLE} WHERE media_id = ? ORDER BY score DESC");
    let rows = sqlx::query_as::<_, SubtitleRow>(&q)
        .bind(&media_id)
        .fetch_all(&state.db.read)
        .await
        .map_err(|_| ParocheError::Internal)?;

    let data: Vec<SubtitleResponse> = rows.into_iter().map(Into::into).collect();
    Ok(ApiResponse::ok(data))
}

pub async fn search_subtitles(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let media_id = uuid.as_bytes().to_vec();

    state
        .subtitles
        .search_for_media(media_id)
        .await
        .map_err(|_| ParocheError::Unavailable)?;

    Ok(StatusCode::ACCEPTED)
}

pub async fn remove_subtitle(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let affected = sqlx::query("DELETE FROM subtitles WHERE id = ?")
        .bind(&id_bytes)
        .execute(&state.db.write)
        .await
        .map_err(|_| ParocheError::Internal)?
        .rows_affected();

    if affected == 0 {
        return Err(ParocheError::NotFound);
    }

    Ok(deleted())
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn subtitle_routes() -> axum::Router<AppState> {
    use axum::routing::{delete, get, post};
    axum::Router::new()
        .route("/media/{id}/subtitles", get(list_subtitles_for_media))
        .route("/media/{id}/subtitles/search", post(search_subtitles))
        .route("/subtitles/{id}", delete(remove_subtitle))
}
