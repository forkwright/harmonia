//! Plex integration endpoints — collection sync (the Kometa replacement) and
//! viewing-history stats (the Wrapperr replacement), delegated to syndesmos
//! via `DynExternalIntegration`.
use axum::Json;
use axum::extract::{Query, State};
use exousia::AuthenticatedUser;
use serde::{Deserialize, Serialize};
use themelion::MediaType;

use crate::error::ParocheError;
use crate::response::ApiResponse;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request/response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct SyncCollectionRequest {
    pub name: String,
    pub media_type: MediaType,
    pub rating_keys: Vec<String>,
}

#[derive(Serialize)]
pub struct SyncCollectionResponse {
    pub name: String,
    pub items: usize,
}

#[derive(Deserialize)]
pub struct WatchHistoryQuery {
    pub account_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn sync_collection(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<SyncCollectionRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(ParocheError::Validation {
            message: "name must not be empty".to_string(),
        });
    }
    if body.rating_keys.is_empty() {
        return Err(ParocheError::Validation {
            message: "rating_keys must not be empty".to_string(),
        });
    }
    let items = body.rating_keys.len();
    state
        .external
        .sync_plex_collection(name.clone(), body.media_type, body.rating_keys)
        .await?;

    Ok(ApiResponse::ok(SyncCollectionResponse { name, items }))
}

pub async fn watch_history(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(query): Query<WatchHistoryQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let records = state.external.plex_watch_history(query.account_id).await?;

    Ok(ApiResponse::ok(records))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn plex_routes() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/collections/sync", post(sync_collection))
        .route("/stats/history", get(watch_history))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use exousia::AuthService;
    use exousia::user::{CreateUserRequest, UserRole};
    use tower::ServiceExt;

    #[expect(
        unused_imports,
        reason = "kanon: test-missing-use-super; parent items accessed via explicit super:: prefix in test bodies"
    )]
    use super::*;
    use crate::state::{DynExternalIntegration, ServiceFut};
    use crate::test_helpers::test_state;

    #[derive(Default)]
    struct RecordingExternal {
        sync_calls: Mutex<Vec<(String, themelion::MediaType, Vec<String>)>>,
        history_calls: Mutex<Vec<Option<String>>>,
        history_records: Vec<themelion::WatchRecord>,
    }

    impl RecordingExternal {
        fn with_records(records: Vec<themelion::WatchRecord>) -> Self {
            Self {
                sync_calls: Mutex::new(Vec::new()),
                history_calls: Mutex::new(Vec::new()),
                history_records: records,
            }
        }

        fn sync_calls(&self) -> Vec<(String, themelion::MediaType, Vec<String>)> {
            self.sync_calls.lock().unwrap().clone()
        }

        fn history_calls(&self) -> Vec<Option<String>> {
            self.history_calls.lock().unwrap().clone()
        }
    }

    impl DynExternalIntegration for RecordingExternal {
        fn sync_plex_collection(
            &self,
            name: String,
            media_type: themelion::MediaType,
            rating_keys: Vec<String>,
        ) -> ServiceFut<()> {
            self.sync_calls
                .lock()
                .unwrap()
                .push((name, media_type, rating_keys));
            Box::pin(async { Ok(()) })
        }

        fn plex_watch_history(
            &self,
            account_id: Option<String>,
        ) -> ServiceFut<Vec<themelion::WatchRecord>> {
            self.history_calls.lock().unwrap().push(account_id);
            let records = self.history_records.clone();
            Box::pin(async move { Ok(records) })
        }
    }

    async fn user_token(auth: &Arc<exousia::ExousiaServiceImpl>) -> String {
        auth.create_user(CreateUserRequest {
            username: "member".to_string(),
            display_name: "member".to_string(),
            password: "password123".to_string(),
            role: UserRole::Member,
        })
        .await
        .unwrap();
        auth.login("member", "password123")
            .await
            .unwrap()
            .access_token
    }

    fn sample_record() -> themelion::WatchRecord {
        themelion::WatchRecord {
            source_ref: "101".to_string(),
            title: "Gantz Graf".to_string(),
            grandparent_title: Some("Autechre".to_string()),
            media_kind: "track".to_string(),
            account_id: Some(42),
            viewed_at: Some(1_700_000_000),
        }
    }

    #[tokio::test]
    async fn sync_collection_invokes_external_service() {
        let (mut state, auth) = test_state().await;
        let external = Arc::new(RecordingExternal::default());
        state.external = external.clone();
        let token = user_token(&auth).await;

        let body = serde_json::json!({
            "name": "Jazz",
            "media_type": "music",
            "rating_keys": ["101", "102"],
        });

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/plex/collections/sync")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let calls = external.sync_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "Jazz");
        assert_eq!(calls[0].1, themelion::MediaType::Music);
        assert_eq!(calls[0].2, vec!["101".to_string(), "102".to_string()]);

        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(payload["data"]["name"], "Jazz");
        assert_eq!(payload["data"]["items"], 2);
    }

    #[tokio::test]
    async fn sync_collection_rejects_empty_name() {
        let (mut state, auth) = test_state().await;
        let external = Arc::new(RecordingExternal::default());
        state.external = external.clone();
        let token = user_token(&auth).await;

        let body = serde_json::json!({
            "name": "   ",
            "media_type": "music",
            "rating_keys": ["101"],
        });

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/plex/collections/sync")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(external.sync_calls().is_empty());
    }

    #[tokio::test]
    async fn sync_collection_rejects_empty_rating_keys() {
        let (mut state, auth) = test_state().await;
        let external = Arc::new(RecordingExternal::default());
        state.external = external.clone();
        let token = user_token(&auth).await;

        let body = serde_json::json!({
            "name": "Jazz",
            "media_type": "music",
            "rating_keys": [],
        });

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/plex/collections/sync")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(external.sync_calls().is_empty());
    }

    #[tokio::test]
    async fn watch_history_returns_records_and_forwards_account_filter() {
        let (mut state, auth) = test_state().await;
        let external = Arc::new(RecordingExternal::with_records(vec![sample_record()]));
        state.external = external.clone();
        let token = user_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/plex/stats/history?account_id=42")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(external.history_calls(), vec![Some("42".to_string())]);

        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(payload["data"][0]["source_ref"], "101");
        assert_eq!(payload["data"][0]["title"], "Gantz Graf");
        assert_eq!(payload["data"][0]["grandparent_title"], "Autechre");
        assert_eq!(payload["data"][0]["media_kind"], "track");
        assert_eq!(payload["data"][0]["account_id"], 42);
        assert_eq!(payload["data"][0]["viewed_at"], 1_700_000_000i64);
    }

    #[tokio::test]
    async fn watch_history_without_filter_passes_none() {
        let (mut state, auth) = test_state().await;
        let external = Arc::new(RecordingExternal::default());
        state.external = external.clone();
        let token = user_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/plex/stats/history")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(external.history_calls(), vec![None]);
    }

    #[tokio::test]
    async fn sync_collection_unavailable_when_external_not_wired() {
        let (state, auth) = test_state().await;
        let token = user_token(&auth).await;

        let body = serde_json::json!({
            "name": "Jazz",
            "media_type": "music",
            "rating_keys": ["101"],
        });

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/plex/collections/sync")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn watch_history_rejects_unauthenticated() {
        let (state, _auth) = test_state().await;
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/plex/stats/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
