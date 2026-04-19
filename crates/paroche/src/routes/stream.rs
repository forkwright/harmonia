use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use exousia::AuthenticatedUser;
use serde::Deserialize;
use tower::ServiceExt;
use tower_http::services::ServeFile;
use uuid::Uuid;

use crate::error::ParocheError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct StreamQuery {
    pub quality: Option<String>,
}

pub async fn stream_media(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: AuthenticatedUser,
    Query(_query): Query<StreamQuery>,
    request: axum::extract::Request,
) -> Result<impl IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let track = apotheke::repo::music::get_track(&state.db.read, &id_bytes)
        .await
        .map_err(|e| ParocheError::Database { source: e })?
        .ok_or(ParocheError::NotFound)?;

    let file_path = track.file_path.ok_or(ParocheError::NotFound)?;

    let response = ServeFile::new(&file_path)
        .oneshot(request)
        .await
        .unwrap_or_else(|never| match never {});

    Ok(response)
}

pub fn stream_routes() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new().route("/stream/{id}", get(stream_media))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use super::*;
    use crate::test_helpers::test_state;

    #[tokio::test]
    async fn stream_nonexistent_track_returns_401_without_auth() {
        let (state, _auth) = test_state().await;
        let app = stream_routes().with_state(state);
        let id = Uuid::now_v7();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/stream/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
