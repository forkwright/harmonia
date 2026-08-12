use axum::Json;
use axum::extract::{Path, Query, State};
use exousia::{AuthenticatedUser, RequireAdmin};
use serde::{Deserialize, Serialize};
use tracing;
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
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, len = bytes.len(), "malformed UUID bytes in db row");
            String::new()
        })
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

impl From<apotheke::repo::news::NewsFeed> for FeedResponse {
    fn from(f: apotheke::repo::news::NewsFeed) -> Self {
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
    let offset = (page - 1) * per_page;

    let feeds = apotheke::repo::news::list_feeds(
        &state.db.read,
        per_page as i64, // INVARIANT: per_page is clamped to [1,100]; i64 overflow impossible
        offset as i64,   // INVARIANT: offset = (page-1)*per_page, bounded; i64 overflow impossible
    )
    .await?;

    let total = apotheke::repo::news::count_feeds(&state.db.read).await? as u64;
    let data: Vec<FeedResponse> = feeds.into_iter().map(Into::into).collect();
    Ok(ApiResponse::paginated(data, page, per_page, total))
}

pub async fn get_feed(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let feed = apotheke::repo::news::get_feed(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    Ok(ApiResponse::ok(FeedResponse::from(feed)))
}

/// Delegates to komide's rich subscribe (fetch + parse + article seeding +
/// `FeedSetChanged`, poll interval FROM config) instead of a bare row insert
/// with a hardcoded 60-minute interval (#577). An unreachable or
/// unparseable feed URL is now an error instead of a silently-inserted dead
/// row.
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

    let feed_id = state
        .feeds
        .subscribe_news(body.url, Some(body.title), body.category)
        .await?;

    let created = apotheke::repo::news::get_feed(&state.db.read, feed_id.as_bytes())
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::created(FeedResponse::from(created)))
}

pub async fn update_feed(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
    Json(body): Json<UpdateFeedRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    apotheke::repo::news::get_feed(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    let now = chrono_now_pub();
    apotheke::repo::news::update_feed(
        &state.db.write,
        &id_bytes,
        &body.title,
        if body.is_active { 1 } else { 0 },
        None,
        &now,
    )
    .await?;

    // WHY: an is_active flip changes the poll set — tell the feed
    // supervisor to re-enumerate (fire-and-forget bus fact).
    let _ = state
        .event_tx
        .send(aggelmata::HarmoniaEvent::FeedSetChanged {
            feed_id: aggelmata::FeedId::from_uuid(uuid),
            media_type: aggelmata::MediaType::News,
        });

    let updated = apotheke::repo::news::get_feed(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::ok(FeedResponse::from(updated)))
}

pub async fn delete_feed(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    // WHY: the existence check keeps this endpoint scoped to news rows —
    // komide's unsubscribe also matches podcast subscriptions by id.
    apotheke::repo::news::get_feed(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    state
        .feeds
        .unsubscribe(aggelmata::FeedId::from_uuid(uuid))
        .await?;

    Ok(deleted())
}

pub fn news_routes() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/", get(list_feeds).post(create_feed))
        .route("/{id}", get(get_feed).put(update_feed).delete(delete_feed))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[expect(
        unused_imports,
        reason = "kanon: test-missing-use-super; parent items accessed via explicit super:: prefix in test bodies"
    )]
    use super::*;
    use crate::state::{AppState, DynFeedService, ServiceError, ServiceFut};
    use crate::test_helpers::{admin_token, test_state};

    /// Recording `DynFeedService` stub for the news routes.
    struct RecordingFeeds {
        feed_id: aggelmata::FeedId,
        subscribe_news_calls: Mutex<Vec<(String, Option<String>, Option<String>)>>,
        unsubscribe_calls: AtomicUsize,
    }

    impl RecordingFeeds {
        fn succeeding(feed_id: aggelmata::FeedId) -> Arc<Self> {
            Arc::new(Self {
                feed_id,
                subscribe_news_calls: Mutex::new(Vec::new()),
                unsubscribe_calls: AtomicUsize::new(0),
            })
        }
    }

    impl DynFeedService for RecordingFeeds {
        fn subscribe_podcast(
            &self,
            _url: String,
            _title: Option<String>,
            _auto_download: Option<bool>,
        ) -> ServiceFut<aggelmata::FeedId> {
            Box::pin(async { Err(ServiceError::NotAvailable) })
        }

        fn subscribe_news(
            &self,
            url: String,
            title: Option<String>,
            category: Option<String>,
        ) -> ServiceFut<aggelmata::FeedId> {
            self.subscribe_news_calls
                .lock()
                .unwrap()
                .push((url, title, category));
            let feed_id = self.feed_id;
            Box::pin(async move { Ok(feed_id) })
        }

        fn unsubscribe(&self, _feed_id: aggelmata::FeedId) -> ServiceFut<()> {
            self.unsubscribe_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(()) })
        }

        fn download_episode(&self, _episode_id: aggelmata::EpisodeId) -> ServiceFut<()> {
            Box::pin(async { Err(ServiceError::NotAvailable) })
        }
    }

    async fn seed_feed(state: &AppState, feed_id: aggelmata::FeedId) {
        let feed = apotheke::repo::news::NewsFeed {
            id: feed_id.as_bytes().to_vec(),
            title: "Seeded News".to_string(),
            url: "https://news.example.com/rss".to_string(),
            site_url: None,
            description: None,
            category: Some("tech".to_string()),
            icon_url: None,
            last_fetched_at: None,
            fetch_interval_minutes: 15,
            is_active: 1,
            added_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        apotheke::repo::news::insert_feed(&state.db.write, &feed)
            .await
            .unwrap();
    }

    fn json_request(
        method: &str,
        uri: &str,
        token: &str,
        body: serde_json::Value,
    ) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("Authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn create_feed_delegates_to_feed_service() {
        let (mut state, auth) = test_state().await;
        let feed_id = aggelmata::FeedId::new();
        // WHY: the stub returns the id; the ROW comes FROM the DB — komide
        // inserts it in production, so the test seeds it up front.
        seed_feed(&state, feed_id).await;
        let feeds = RecordingFeeds::succeeding(feed_id);
        state.feeds = feeds.clone();
        let token = admin_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_request(
                "POST",
                "/api/news",
                &token,
                serde_json::json!({
                    "title": "Seeded News",
                    "url": "https://news.example.com/rss",
                    "category": "tech",
                }),
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        // WHY the block: the guard must provably end before the next await —
        // clippy's await_holding_lock does not credit an explicit drop().
        {
            let calls = feeds.subscribe_news_calls.lock().unwrap();
            assert_eq!(
                calls.as_slice(),
                &[(
                    "https://news.example.com/rss".to_string(),
                    Some("Seeded News".to_string()),
                    Some("tech".to_string()),
                )]
            );
        }
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["data"]["id"], feed_id.as_uuid().to_string());
        assert_eq!(body["data"]["category"], "tech");
    }

    #[tokio::test]
    async fn delete_feed_delegates_unsubscribe() {
        let (mut state, auth) = test_state().await;
        let feed_id = aggelmata::FeedId::new();
        seed_feed(&state, feed_id).await;
        let feeds = RecordingFeeds::succeeding(feed_id);
        state.feeds = feeds.clone();
        let token = admin_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/news/{}", feed_id.as_uuid()))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert_eq!(feeds.unsubscribe_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn update_feed_emits_feed_set_changed() {
        let (state, auth) = test_state().await;
        let feed_id = aggelmata::FeedId::new();
        seed_feed(&state, feed_id).await;
        let token = admin_token(&auth).await;
        let mut event_rx = state.event_tx.subscribe();

        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_request(
                "PUT",
                &format!("/api/news/{}", feed_id.as_uuid()),
                &token,
                serde_json::json!({ "title": "Renamed", "is_active": false }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let mut saw_feed_set_changed = false;
        while let Ok(event) = event_rx.try_recv() {
            if matches!(
                event,
                aggelmata::HarmoniaEvent::FeedSetChanged {
                    feed_id: id,
                    media_type: aggelmata::MediaType::News,
                } if id == feed_id
            ) {
                saw_feed_set_changed = true;
            }
        }
        assert!(
            saw_feed_set_changed,
            "an is_active flip must emit FeedSetChanged"
        );
    }
}
