/// Search endpoints — delegates to zetesis via DynSearchService.
use axum::{
    Json,
    extract::{Path, State},
};
use exousia::AuthenticatedUser;
use serde::{Deserialize, Serialize};

use crate::error::ParocheError;
use crate::response::ApiResponse;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Deserialize, Serialize)]
pub struct SearchRequest {
    pub query_text: Option<String>,
    pub media_type: Option<String>,
    #[serde(default)]
    pub category_ids: Vec<u32>,
    pub imdb_id: Option<String>,
    pub tvdb_id: Option<u32>,
    pub tmdb_id: Option<u32>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub author: Option<String>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

fn default_limit() -> u32 {
    100
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn search(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<SearchRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let query = serde_json::to_value(&body).map_err(|_| ParocheError::Internal)?;

    let mut results = state.search.search(query).await?;
    // WHY: result download_urls embed indexer apikey/passkey credentials
    // (Torznab/Newznab convention); this endpoint is member-visible, so raw
    // URLs hand out the operator's private-tracker credentials.
    crate::redact::redact_download_urls_in_json(&mut results);

    Ok(ApiResponse::ok(results))
}

pub async fn get_search_results(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(query_id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    // Retrieve cached results for a prior search. The query_id is produced by
    // the search service and stored server-side; when the search service is not
    // wired this returns 503.
    let query = serde_json::json!({ "query_id": query_id });

    let mut results = state.search.search(query).await?;
    // WHY: same credential exposure as `search` — cached results carry the
    // same raw download_urls.
    crate::redact::redact_download_urls_in_json(&mut results);

    Ok(ApiResponse::ok(results))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn search_routes() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/", post(search))
        .route("/{query_id}/results", get(get_search_results))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

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
    use crate::state::{DynSearchService, ServiceError, ServiceFut};
    use crate::test_helpers::test_state;

    /// Search stub returning a fixed result payload, standing in for zetesis.
    struct FixedSearch(serde_json::Value);

    impl DynSearchService for FixedSearch {
        fn search(&self, _query: serde_json::Value) -> ServiceFut<serde_json::Value> {
            let results = self.0.clone();
            Box::pin(async move { Ok(results) })
        }
        fn test_indexer(&self, _indexer_id: i64) -> ServiceFut<serde_json::Value> {
            Box::pin(async { Err(ServiceError::NotAvailable) })
        }
        fn refresh_caps(&self, _indexer_id: i64) -> ServiceFut<serde_json::Value> {
            Box::pin(async { Err(ServiceError::NotAvailable) })
        }
    }

    async fn member_token(auth: &Arc<exousia::ExousiaServiceImpl>) -> String {
        auth.create_user(CreateUserRequest {
            username: "member".to_string(),
            display_name: "Member".to_string(),
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

    #[tokio::test]
    async fn search_redacts_indexer_credentials_in_results() {
        let (mut state, auth) = test_state().await;
        state.search = Arc::new(FixedSearch(serde_json::json!({
            "results": [{
                "title": "Kind of Blue",
                "download_url":
                    "https://indexer.example/dl/42?apikey=SECRET&file=x.torrent",
            }]
        })));
        let token = member_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/search")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body["data"]["results"][0]["download_url"],
            "https://indexer.example/dl/42?apikey=REDACTED&file=x.torrent"
        );
        assert!(
            !String::from_utf8_lossy(&bytes).contains("SECRET"),
            "the raw indexer credential must never reach the response body"
        );
    }

    #[tokio::test]
    async fn cached_search_results_redact_indexer_credentials() {
        let (mut state, auth) = test_state().await;
        state.search = Arc::new(FixedSearch(serde_json::json!({
            "results": [{
                "title": "Kind of Blue",
                "download_url": "https://indexer.example/dl/42?passkey=SECRET",
            }]
        })));
        let token = member_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/search/q-123/results")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body["data"]["results"][0]["download_url"],
            "https://indexer.example/dl/42?passkey=REDACTED"
        );
        assert!(!String::from_utf8_lossy(&bytes).contains("SECRET"));
    }
}
