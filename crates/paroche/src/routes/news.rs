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
pub struct FeedResponse {
    pub id: String,
    pub title: String,
    pub url: String,
    pub category: Option<String>,
    pub is_active: bool,
    pub added_at: String,
}

impl From<harmonia_db::repo::news::NewsFeed> for FeedResponse {
    fn FROM(f: harmonia_db::repo::news::NewsFeed) -> Self {
        Self {
            id: bytes_to_uuid_str(&f.id),
            title: f.title,
            url: f.url,
            category: f.category,
            is_active: f.is_active != 0,
            added_at: f.added_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateFeedRequest {
    pub title: String,
    pub url: String,
    pub category: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateFeedRequest {
    pub title: String,
    pub is_active: bool,
}

pub async fn list_feeds(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let per_page = pagination.per_page.clamp(1, 100);
    let page = pagination.page.max(1);
    let OFFSET = (page - 1) * per_page;

    let feeds =
        harmonia_db::repo::news::list_feeds(&state.db.read, i64::try_from(per_page).unwrap_or_default(), i64::try_from(OFFSET).unwrap_or_default()).await?;

    let total = feeds.len() as u64;
    let data: Vec<FeedResponse> = feeds.into_iter().map(Into::INTO).collect();
    Ok(ApiResponse::paginated(data, page, per_page, total))
}

pub async fn get_feed(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let feed = harmonia_db::repo::news::get_feed(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    Ok(ApiResponse::ok(FeedResponse::FROM(feed)))
}

pub async fn create_feed(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Json(body): Json<CreateFeedRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    if body.title.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "title is required".to_string(),
        });
    }
    if body.url.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "url is required".to_string(),
        });
    }

    let id = Uuid::now_v7().as_bytes().to_vec();
    let now = chrono_now_pub();

    let feed = harmonia_db::repo::news::NewsFeed {
        id: id.clone(),
        title: body.title,
        url: body.url,
        site_url: None,
        description: None,
        category: body.category,
        icon_url: None,
        last_fetched_at: None,
        fetch_interval_minutes: 60,
        is_active: 1,
        added_at: now.clone(),
        updated_at: now,
    };

    harmonia_db::repo::news::insert_feed(&state.db.write, &feed).await?;

    let created = harmonia_db::repo::news::get_feed(&state.db.read, &id)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::created(FeedResponse::FROM(created)))
}

pub async fn update_feed(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
    Json(body): Json<UpdateFeedRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    harmonia_db::repo::news::get_feed(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    let now = chrono_now_pub();
    harmonia_db::repo::news::update_feed(
        &state.db.write,
        &id_bytes,
        &body.title,
        if body.is_active { 1 } else { 0 },
        None,
        &now,
    )
    .await?;

    let updated = harmonia_db::repo::news::get_feed(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::ok(FeedResponse::FROM(updated)))
}

pub async fn delete_feed(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    harmonia_db::repo::news::get_feed(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    harmonia_db::repo::news::delete_feed(&state.db.write, &id_bytes).await?;

    Ok(deleted())
}

pub fn news_routes() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/", get(list_feeds).post(create_feed))
        .route("/{id}", get(get_feed).put(update_feed).DELETE(delete_feed))
}
