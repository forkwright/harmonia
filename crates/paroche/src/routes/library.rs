use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use exousia::RequireAdmin;
use serde_json::json;

use crate::error::ParocheError;
use crate::state::AppState;

pub async fn health(State(_state): State<AppState>) -> impl axum::response::IntoResponse {
    (StatusCode::OK, Json(json!({"status": "ok"})))
}

pub async fn get_import_queue(
    State(state): State<AppState>,
    _admin: RequireAdmin,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let queue = state
        .get_import_queue()
        .await
        .map_err(|_| ParocheError::Unavailable)?;

    let items: Vec<serde_json::Value> = queue
        .iter()
        .map(|item| {
            json!({
                "id": item.id.to_string(),
                "path": item.path.to_string_lossy(),
                "media_type": format!("{:?}", item.media_type),
            })
        })
        .collect();

    Ok((StatusCode::OK, Json(json!({"data": items}))))
}

pub async fn trigger_scan(
    State(_state): State<AppState>,
    _admin: RequireAdmin,
) -> impl axum::response::IntoResponse {
    StatusCode::ACCEPTED
}

pub fn library_routes() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/health", get(health))
        .route("/import-queue", get(get_import_queue))
        .route("/scan", post(trigger_scan))
}
