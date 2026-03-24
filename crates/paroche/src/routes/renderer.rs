// GET /api/renderers — list connected renderers with status.

use axum::{Json, extract::State};

use crate::state::AppState;

pub fn renderer_routes() -> axum::Router<AppState> {
    axum::Router::new().route("/", axum::routing::get(list_renderers))
}

async fn list_renderers(State(state): State<AppState>) -> Json<serde_json::Value> {
    let renderers = state.renderers.list_renderers().await;
    Json(serde_json::to_value(renderers).unwrap_or_default())
}
