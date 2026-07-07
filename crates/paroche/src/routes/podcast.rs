use axum::Json;
use axum::extract::{Path, Query, State};
use exousia::{AuthenticatedUser, RequireAdmin};
use serde::{Deserialize, Serialize};
use tracing;
use uuid::Uuid;

use crate::error::ParocheError;
use crate::response::{ApiResponse, deleted};
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
pub struct SubscriptionResponse {
    pub id: String,
    pub feed_url: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub auto_download: bool,
    pub added_at: String,
}

impl From<apotheke::repo::podcast::PodcastSubscription> for SubscriptionResponse {
    fn from(s: apotheke::repo::podcast::PodcastSubscription) -> Self {
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
    let offset = (page - 1) * per_page;

    let subs =
        apotheke::repo::podcast::list_subscriptions(&state.db.read, per_page as i64, offset as i64)
            .await?;

    let total = apotheke::repo::podcast::count_subscriptions(&state.db.read).await? as u64;
    let data: Vec<SubscriptionResponse> = subs.into_iter().map(Into::into).collect();
    Ok(ApiResponse::paginated(data, page, per_page, total))
}

pub async fn get_subscription(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let sub = apotheke::repo::podcast::get_subscription(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    Ok(ApiResponse::ok(SubscriptionResponse::from(sub)))
}

/// Delegates to komide's rich subscribe (fetch + parse + episode seeding +
/// `FeedSetChanged`) instead of a bare row insert — the bare insert left a
/// dead, never-polled row (#577). An unreachable or unparseable feed URL is
/// now an error instead of a silently-inserted dead subscription.
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

    let feed_id = state
        .feeds
        .subscribe_podcast(body.feed_url, body.title, body.auto_download)
        .await?;

    let created = apotheke::repo::podcast::get_subscription(&state.db.read, feed_id.as_bytes())
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::created(SubscriptionResponse::from(created)))
}

pub async fn update_subscription(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
    Json(body): Json<UpdateSubscriptionRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    apotheke::repo::podcast::get_subscription(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    // NOTE: the stored auto_download is komide's episode-download COUNT —
    // the API bool maps to the configured count or 0, never a raw 0/1.
    let auto_download_count = if body.auto_download {
        i64::try_from(state.config.current().komide.auto_download_latest_n).unwrap_or_default() // WHY: auto_download_latest_n is a small config value; bounded within i64
    } else {
        0
    };
    apotheke::repo::podcast::update_subscription(
        &state.db.write,
        &id_bytes,
        body.title.as_deref(),
        auto_download_count,
        None,
    )
    .await?;

    // WHY: a subscription flip changes what the poll set should do — tell
    // the feed supervisor to re-enumerate (fire-and-forget bus fact).
    let _ = state
        .event_tx
        .send(themelion::HarmoniaEvent::FeedSetChanged {
            feed_id: themelion::FeedId::from_uuid(uuid),
            media_type: themelion::MediaType::Podcast,
        });

    let updated = apotheke::repo::podcast::get_subscription(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::ok(SubscriptionResponse::from(updated)))
}

pub async fn delete_subscription(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    // WHY: the existence check keeps this endpoint scoped to podcast rows —
    // komide's unsubscribe also matches news feeds by id.
    apotheke::repo::podcast::get_subscription(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    state
        .feeds
        .unsubscribe(themelion::FeedId::from_uuid(uuid))
        .await?;

    Ok(deleted())
}

/// Starts a server-side download of one episode's audio enclosure — the
/// route the desktop client already posts to. Returns 202 immediately; the
/// transfer completes in the background and lands on the episode row.
pub async fn download_episode(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;

    state
        .feeds
        .download_episode(themelion::EpisodeId::from_uuid(uuid))
        .await?;

    Ok(ApiResponse::accepted(serde_json::json!({
        "episode_id": id,
        "status": "downloading",
    })))
}

pub fn podcast_routes() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/", get(list_subscriptions).post(create_subscription))
        .route(
            "/{id}",
            get(get_subscription)
                .put(update_subscription)
                .delete(delete_subscription),
        )
        .route("/episodes/{id}/download", post(download_episode))
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

    enum StubOutcome {
        Succeed,
        InvalidInput,
        NotFound,
    }

    /// Recording `DynFeedService` stub: every call is captured, and the
    /// configured outcome is returned.
    struct RecordingFeeds {
        feed_id: themelion::FeedId,
        outcome: StubOutcome,
        subscribe_podcast_calls: Mutex<Vec<(String, Option<String>, Option<bool>)>>,
        unsubscribe_calls: AtomicUsize,
        download_calls: Mutex<Vec<themelion::EpisodeId>>,
    }

    impl RecordingFeeds {
        fn with_outcome(feed_id: themelion::FeedId, outcome: StubOutcome) -> Arc<Self> {
            Arc::new(Self {
                feed_id,
                outcome,
                subscribe_podcast_calls: Mutex::new(Vec::new()),
                unsubscribe_calls: AtomicUsize::new(0),
                download_calls: Mutex::new(Vec::new()),
            })
        }

        fn succeeding(feed_id: themelion::FeedId) -> Arc<Self> {
            Self::with_outcome(feed_id, StubOutcome::Succeed)
        }

        fn error(&self) -> Option<ServiceError> {
            match self.outcome {
                StubOutcome::Succeed => None,
                StubOutcome::InvalidInput => {
                    Some(ServiceError::InvalidInput("invalid feed URL".to_string()))
                }
                StubOutcome::NotFound => Some(ServiceError::NotFound),
            }
        }
    }

    impl DynFeedService for RecordingFeeds {
        fn subscribe_podcast(
            &self,
            url: String,
            title: Option<String>,
            auto_download: Option<bool>,
        ) -> ServiceFut<themelion::FeedId> {
            self.subscribe_podcast_calls
                .lock()
                .unwrap()
                .push((url, title, auto_download));
            let result = match self.error() {
                Some(e) => Err(e),
                None => Ok(self.feed_id),
            };
            Box::pin(async move { result })
        }

        fn subscribe_news(
            &self,
            _url: String,
            _title: Option<String>,
            _category: Option<String>,
        ) -> ServiceFut<themelion::FeedId> {
            let result = match self.error() {
                Some(e) => Err(e),
                None => Ok(self.feed_id),
            };
            Box::pin(async move { result })
        }

        fn unsubscribe(&self, _feed_id: themelion::FeedId) -> ServiceFut<()> {
            self.unsubscribe_calls.fetch_add(1, Ordering::SeqCst);
            let result = match self.error() {
                Some(e) => Err(e),
                None => Ok(()),
            };
            Box::pin(async move { result })
        }

        fn download_episode(&self, episode_id: themelion::EpisodeId) -> ServiceFut<()> {
            self.download_calls.lock().unwrap().push(episode_id);
            let result = match self.error() {
                Some(e) => Err(e),
                None => Ok(()),
            };
            Box::pin(async move { result })
        }
    }

    async fn seed_subscription(state: &AppState, feed_id: themelion::FeedId, auto_download: i64) {
        let sub = apotheke::repo::podcast::PodcastSubscription {
            id: feed_id.as_bytes().to_vec(),
            feed_url: "https://example.com/feed.xml".to_string(),
            title: Some("Seeded".to_string()),
            description: None,
            author: None,
            image_url: None,
            language: None,
            last_checked_at: None,
            auto_download,
            quality_profile_id: None,
            added_at: "2026-01-01T00:00:00Z".to_string(),
        };
        apotheke::repo::podcast::insert_subscription(&state.db.write, &sub)
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

    fn empty_request(method: &str, uri: &str, token: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap()
    }

    async fn body_json(resp: axum::response::Response) -> serde_json::Value {
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn create_subscription_delegates_to_feed_service() {
        let (mut state, auth) = test_state().await;
        let feed_id = themelion::FeedId::new();
        // WHY: the stub returns the id; the ROW comes FROM the DB — komide
        // inserts it in production, so the test seeds it up front.
        seed_subscription(&state, feed_id, 3).await;
        let feeds = RecordingFeeds::succeeding(feed_id);
        state.feeds = feeds.clone();
        let token = admin_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_request(
                "POST",
                "/api/podcasts",
                &token,
                serde_json::json!({
                    "feed_url": "https://example.com/feed.xml",
                    "title": "My Show",
                    "auto_download": true,
                }),
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        // WHY the block: the guard must provably end before the next await —
        // clippy's await_holding_lock does not credit an explicit drop().
        {
            let calls = feeds.subscribe_podcast_calls.lock().unwrap();
            assert_eq!(
                calls.as_slice(),
                &[(
                    "https://example.com/feed.xml".to_string(),
                    Some("My Show".to_string()),
                    Some(true),
                )]
            );
        }
        let body = body_json(resp).await;
        assert_eq!(
            body["data"]["id"],
            feed_id.as_uuid().to_string(),
            "the response row is the one komide created"
        );
        assert_eq!(
            body["data"]["auto_download"], true,
            "a nonzero episode count reads back as auto_download=true"
        );
    }

    #[tokio::test]
    async fn create_subscription_maps_invalid_feed_to_validation_error() {
        let (mut state, auth) = test_state().await;
        state.feeds =
            RecordingFeeds::with_outcome(themelion::FeedId::new(), StubOutcome::InvalidInput);
        let token = admin_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_request(
                "POST",
                "/api/podcasts",
                &token,
                serde_json::json!({ "feed_url": "https://bad.example.com/feed.xml" }),
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = body_json(resp).await;
        assert_eq!(body["code"], "VALIDATION_ERROR");
    }

    #[tokio::test]
    async fn delete_subscription_delegates_unsubscribe() {
        let (mut state, auth) = test_state().await;
        let feed_id = themelion::FeedId::new();
        seed_subscription(&state, feed_id, 0).await;
        let feeds = RecordingFeeds::succeeding(feed_id);
        state.feeds = feeds.clone();
        let token = admin_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(empty_request(
                "DELETE",
                &format!("/api/podcasts/{}", feed_id.as_uuid()),
                &token,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert_eq!(feeds.unsubscribe_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn update_subscription_stores_count_and_emits_feed_set_changed() {
        let (state, auth) = test_state().await;
        let feed_id = themelion::FeedId::new();
        seed_subscription(&state, feed_id, 0).await;
        let token = admin_token(&auth).await;
        let mut event_rx = state.event_tx.subscribe();
        let db_read = state.db.read.clone();

        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_request(
                "PUT",
                &format!("/api/podcasts/{}", feed_id.as_uuid()),
                &token,
                serde_json::json!({ "title": "Renamed", "auto_download": true }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let row = apotheke::repo::podcast::get_subscription(&db_read, feed_id.as_bytes())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            row.auto_download,
            i64::try_from(horismos::Config::default().komide.auto_download_latest_n).unwrap(),
            "the API bool maps to komide's configured episode COUNT, not a raw 1"
        );

        let mut saw_feed_set_changed = false;
        while let Ok(event) = event_rx.try_recv() {
            if matches!(
                event,
                themelion::HarmoniaEvent::FeedSetChanged {
                    feed_id: id,
                    media_type: themelion::MediaType::Podcast,
                } if id == feed_id
            ) {
                saw_feed_set_changed = true;
            }
        }
        assert!(
            saw_feed_set_changed,
            "an auto_download flip must emit FeedSetChanged"
        );
    }

    #[tokio::test]
    async fn download_episode_route_accepts_and_delegates() {
        let (mut state, auth) = test_state().await;
        let feeds = RecordingFeeds::succeeding(themelion::FeedId::new());
        state.feeds = feeds.clone();
        let token = admin_token(&auth).await;
        let episode_id = themelion::EpisodeId::new();

        let app = crate::build_router(state);
        let resp = app
            .oneshot(empty_request(
                "POST",
                &format!("/api/podcasts/episodes/{}/download", episode_id.as_uuid()),
                &token,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        assert_eq!(
            feeds.download_calls.lock().unwrap().as_slice(),
            &[episode_id]
        );
        let body = body_json(resp).await;
        assert_eq!(body["data"]["status"], "downloading");
    }

    #[tokio::test]
    async fn download_episode_unknown_maps_to_not_found() {
        let (mut state, auth) = test_state().await;
        state.feeds = RecordingFeeds::with_outcome(themelion::FeedId::new(), StubOutcome::NotFound);
        let token = admin_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(empty_request(
                "POST",
                &format!("/api/podcasts/episodes/{}/download", uuid::Uuid::now_v7()),
                &token,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn download_episode_invalid_id_is_bad_request() {
        let (mut state, auth) = test_state().await;
        let feeds = RecordingFeeds::succeeding(themelion::FeedId::new());
        state.feeds = feeds.clone();
        let token = admin_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(empty_request(
                "POST",
                "/api/podcasts/episodes/not-a-uuid/download",
                &token,
            ))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert!(
            feeds.download_calls.lock().unwrap().is_empty(),
            "an unparseable id must never reach the service"
        );
    }
}
