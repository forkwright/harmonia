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
pub struct BookResponse {
    pub id: String,
    pub title: String,
    pub isbn: Option<String>,
    pub publisher: Option<String>,
    pub page_count: Option<i64>,
    pub added_at: String,
}

impl From<harmonia_db::repo::book::Book> for BookResponse {
    fn from(b: harmonia_db::repo::book::Book) -> Self {
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

    let books =
        harmonia_db::repo::book::list_books(&state.db.read, i64::try_from(per_page).unwrap_or_default(), i64::try_from(offset).unwrap_or_default()).await?;

    let total = books.len() as u64;
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

    let book = harmonia_db::repo::book::get_book(&state.db.read, &id_bytes)
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

    let book = harmonia_db::repo::book::Book {
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

    harmonia_db::repo::book::insert_book(&state.db.write, &book).await?;

    let created = harmonia_db::repo::book::get_book(&state.db.read, &id)
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

    harmonia_db::repo::book::get_book(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    harmonia_db::repo::book::update_book(
        &state.db.write,
        &id_bytes,
        &body.title,
        body.quality_score,
        body.file_path.as_deref(),
        None,
    )
    .await?;

    let updated = harmonia_db::repo::book::get_book(&state.db.read, &id_bytes)
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

    harmonia_db::repo::book::get_book(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    harmonia_db::repo::book::delete_book(&state.db.write, &id_bytes).await?;

    Ok(deleted())
}

pub fn book_routes() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/", get(list_books).post(create_book))
        .route("/{id}", get(get_book).put(update_book).delete(delete_book))
}
