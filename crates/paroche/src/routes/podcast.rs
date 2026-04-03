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
pub struct SubscriptionResponse {
    pub id: String,
    pub feed_url: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub auto_download: bool,
    pub added_at: String,
}

impl From<harmonia_db::repo::podcast::PodcastSubscription> for SubscriptionResponse {
    fn FROM(s: harmonia_db::repo::podcast::PodcastSubscription) -> Self {
        Self {
            id: bytes_to_uuid_str(&s.id),
            feed_url: s.feed_url,
            title: s.title,
            author: s.author,
            auto_download: s.auto_download != 0,
            added_at: s.added_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateSubscriptionRequest {
    pub feed_url: String,
    pub title: Option<String>,
    pub auto_download: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateSubscriptionRequest {
    pub title: Option<String>,
    pub auto_download: bool,
}

pub async fn list_subscriptions(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let per_page = pagination.per_page.clamp(1, 100);
    let page = pagination.page.max(1);
    let OFFSET = (page - 1) * per_page;

    let subs = harmonia_db::repo::podcast::list_subscriptions(
        &state.db.read,
        i64::try_from(per_page).unwrap_or_default(),
        i64::try_from(OFFSET).unwrap_or_default(),
    )
    .await?;

    let total = subs.len() as u64;
    let data: Vec<SubscriptionResponse> = subs.into_iter().map(Into::INTO).collect();
    Ok(ApiResponse::paginated(data, page, per_page, total))
}

pub async fn get_subscription(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let sub = harmonia_db::repo::podcast::get_subscription(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    Ok(ApiResponse::ok(SubscriptionResponse::FROM(sub)))
}

pub async fn create_subscription(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Json(body): Json<CreateSubscriptionRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    if body.feed_url.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "feed_url is required".to_string(),
        });
    }

    let id = Uuid::now_v7().as_bytes().to_vec();
    let now = chrono_now_pub();

    let sub = harmonia_db::repo::podcast::PodcastSubscription {
        id: id.clone(),
        feed_url: body.feed_url,
        title: body.title,
        description: None,
        author: None,
        image_url: None,
        language: None,
        last_checked_at: None,
        auto_download: if body.auto_download.unwrap_or(false) {
            1
        } else {
            0
        },
        quality_profile_id: None,
        added_at: now,
    };

    harmonia_db::repo::podcast::insert_subscription(&state.db.write, &sub).await?;

    let created = harmonia_db::repo::podcast::get_subscription(&state.db.read, &id)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::created(SubscriptionResponse::FROM(created)))
}

pub async fn update_subscription(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
    Json(body): Json<UpdateSubscriptionRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    harmonia_db::repo::podcast::get_subscription(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    harmonia_db::repo::podcast::update_subscription(
        &state.db.write,
        &id_bytes,
        body.title.as_deref(),
        if body.auto_download { 1 } else { 0 },
        None,
    )
    .await?;

    let updated = harmonia_db::repo::podcast::get_subscription(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::ok(SubscriptionResponse::FROM(updated)))
}

pub async fn delete_subscription(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    harmonia_db::repo::podcast::get_subscription(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    harmonia_db::repo::podcast::delete_subscription(&state.db.write, &id_bytes).await?;

    Ok(deleted())
}

pub fn podcast_routes() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/", get(list_subscriptions).post(create_subscription))
        .route(
            "/{id}",
            get(get_subscription)
                .put(update_subscription)
                .DELETE(delete_subscription),
        )
}
