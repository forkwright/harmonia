use axum::Json;
use axum::extract::{Path, Query, Request, State};
use axum::http::HeaderValue;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use exousia::{AuthenticatedUser, RequireAdmin};
use serde::{Deserialize, Serialize};
use tracing;
use uuid::Uuid;

use crate::error::ParocheError;
use crate::opds::acquisition::effective_mime;
use crate::opds::auth::OpdsUser;
use crate::opds::content::{attachment_disposition, serve_media_file};
use crate::opds::cover;
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
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, len = bytes.len(), "malformed UUID bytes in db row");
            String::new()
        })
}

#[derive(Serialize)]
pub struct BookResponse {
    pub id: String,
    pub title: String,
    pub isbn: Option<String>,
    pub publisher: Option<String>,
    pub page_count: Option<i64>,
    pub added_at: String,
}

impl From<apotheke::repo::book::Book> for BookResponse {
    fn from(b: apotheke::repo::book::Book) -> Self {
        Self {
            id: bytes_to_uuid_str(&b.id),
            title: b.title,
            isbn: b.isbn,
            publisher: b.publisher,
            page_count: b.page_count,
            added_at: b.added_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateBookRequest {
    pub title: String,
    pub isbn: Option<String>,
    pub publisher: Option<String>,
    pub page_count: Option<i64>,
}

#[derive(Deserialize)]
pub struct UpdateBookRequest {
    pub title: String,
    pub quality_score: Option<i64>,
    pub file_path: Option<String>,
}

pub async fn list_books(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let per_page = pagination.per_page.clamp(1, 100);
    let page = pagination.page.max(1);
    let offset = (page - 1) * per_page;

    let books = apotheke::repo::book::list_books(
        &state.db.read,
        per_page as i64, // INVARIANT: per_page is clamped to [1,100]; i64 overflow impossible
        offset as i64, // INVARIANT: offset = (page-1)*per_page, bounded by u64 range; i64 overflow impossible
    )
    .await?;

    let total = apotheke::repo::book::count_books(&state.db.read).await? as u64;
    let data: Vec<BookResponse> = books.into_iter().map(Into::into).collect();
    Ok(ApiResponse::paginated(data, page, per_page, total))
}

pub async fn get_book(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let book = apotheke::repo::book::get_book(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    Ok(ApiResponse::ok(BookResponse::from(book)))
}

pub async fn create_book(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Json(body): Json<CreateBookRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    if body.title.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "title is required".to_string(),
        });
    }

    let id = Uuid::now_v7().as_bytes().to_vec();
    let now = chrono_now_pub();

    let book = apotheke::repo::book::Book {
        id: id.clone(),
        registry_id: None,
        title: body.title,
        subtitle: None,
        isbn: body.isbn,
        isbn13: None,
        openlibrary_id: None,
        goodreads_id: None,
        publisher: body.publisher,
        publish_date: None,
        language: None,
        page_count: body.page_count,
        description: None,
        file_path: None,
        file_format: None,
        file_size_bytes: None,
        quality_score: None,
        quality_profile_id: None,
        source_type: "manual".to_string(),
        added_at: now,
    };

    apotheke::repo::book::insert_book(&state.db.write, &book).await?;

    let created = apotheke::repo::book::get_book(&state.db.read, &id)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::created(BookResponse::from(created)))
}

pub async fn update_book(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
    Json(body): Json<UpdateBookRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    apotheke::repo::book::get_book(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    apotheke::repo::book::update_book(
        &state.db.write,
        &id_bytes,
        &body.title,
        body.quality_score,
        body.file_path.as_deref(),
        None,
    )
    .await?;

    let updated = apotheke::repo::book::get_book(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::ok(BookResponse::from(updated)))
}

pub async fn delete_book(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    apotheke::repo::book::get_book(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    apotheke::repo::book::delete_book(&state.db.write, &id_bytes).await?;

    Ok(deleted())
}

/// Serves the raw book file for OPDS acquisition. `?size=thumbnail` and any
/// other query params are accepted and ignored — no `Query` extractor here,
/// since a rejecting one would break query-bearing acquisition hrefs.
pub async fn download_book(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: OpdsUser,
    request: Request,
) -> Result<Response, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let book = apotheke::repo::book::get_book(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;
    let file_path = book.file_path.ok_or(ParocheError::NotFound)?;
    let mime = effective_mime(book.file_format.as_deref(), Some(&file_path));

    let mut response = serve_media_file(&file_path, mime, request).await;
    if response.status().is_success()
        && let Some(disposition) = attachment_disposition(&file_path)
    {
        response
            .headers_mut()
            .insert(CONTENT_DISPOSITION, disposition);
    }
    Ok(response)
}

/// Serves the book's cover: embedded epub cover first, sidecar `cover.*`
/// fallback, content type sniffed from the actual image bytes.
pub async fn book_cover(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: OpdsUser,
) -> Result<Response, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let book = apotheke::repo::book::get_book(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;
    let file_path = book.file_path.ok_or(ParocheError::NotFound)?;

    serve_cover(&file_path, book.file_format.as_deref()).await
}

pub(crate) async fn serve_cover(
    file_path: &str,
    file_format: Option<&str>,
) -> Result<Response, ParocheError> {
    let location = cover::locate_cover(file_path, file_format)
        .await
        .ok_or(ParocheError::NotFound)?;
    let (bytes, mime) = cover::read_cover(location)
        .await
        .ok_or(ParocheError::NotFound)?;
    Ok(([(CONTENT_TYPE, HeaderValue::from_static(mime))], bytes).into_response())
}

// NOTE: `/{id}/download` and `/{id}/cover` are registered by the streaming
// route group in `crate::build_router` — byte-serving routes are exempt from
// the API response timeout.
pub fn book_routes() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/", get(list_books).post(create_book))
        .route("/{id}", get(get_book).put(update_book).delete(delete_book))
}
