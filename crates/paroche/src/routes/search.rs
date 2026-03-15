/// Search endpoints — delegates to zetesis via DynSearchService.
use axum::{
    Json,
    extract::{Path, State},
};
use exousia::AuthenticatedUser;
use serde::{Deserialize, Serialize};

use crate::{error::ParocheError, response::ApiResponse, state::AppState};

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

    let results = state
        .search
        .search(query)
        .await
        .map_err(|_| ParocheError::Unavailable)?;

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

    let results = state
        .search
        .search(query)
        .await
        .map_err(|_| ParocheError::Unavailable)?;

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
