use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use super::auth::authenticate;
use super::types::{ERR_NOT_FOUND, SubsonicCommon, respond_error, uuid_bytes};
use crate::state::AppState;

#[derive(Deserialize, Default)]
pub struct StreamQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    pub id: Option<String>,
    pub format: Option<String>,
    #[serde(rename = "maxBitRate")]
    pub max_bit_rate: Option<u32>,
}

#[derive(Deserialize, Default)]
pub struct CoverArtQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    pub id: Option<String>,
    pub size: Option<u32>,
}

#[derive(Deserialize, Default)]
pub struct AvatarQuery {
    #[serde(flatten)]
    pub common: SubsonicCommon,
    pub username: Option<String>,
}

// ---------------------------------------------------------------------------
// stream.view — proxy to the track file
// ---------------------------------------------------------------------------

pub async fn stream(
    State(state): State<AppState>,
    Query(q): Query<StreamQuery>,
    request: axum::extract::Request,
) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let id = match &q.id {
        Some(id) => id.clone(),
        None => return respond_error(user.format, 10, "required parameter is missing: id"),
    };

    let id_bytes = match uuid_bytes(&id) {
        Some(b) => b,
        None => return respond_error(user.format, ERR_NOT_FOUND, "not found"),
    };

    let track = match apotheke::repo::music::get_track(&state.db.read, &id_bytes).await {
        Ok(Some(t)) => t,
        _ => return respond_error(user.format, ERR_NOT_FOUND, "not found"),
    };

    let file_path = match track.file_path {
        Some(p) => p,
        None => return respond_error(user.format, ERR_NOT_FOUND, "media file not available"),
    };

    ServeFile::new(&file_path)
        .oneshot(request)
        .await
        .into_response()
}

// ---------------------------------------------------------------------------
// getCoverArt.view
// ---------------------------------------------------------------------------

pub async fn get_cover_art(
    State(state): State<AppState>,
    Query(q): Query<CoverArtQuery>,
) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    // No cover art storage yet — return not found
    respond_error(user.format, ERR_NOT_FOUND, "cover art not found")
}

// ---------------------------------------------------------------------------
// getAvatar.view
// ---------------------------------------------------------------------------

pub async fn get_avatar(State(state): State<AppState>, Query(q): Query<AvatarQuery>) -> Response {
    let user = match authenticate(&q.common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    // No avatar support — return not found
    respond_error(user.format, ERR_NOT_FOUND, "avatar not found")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::subsonic::test_helpers::subsonic_app;

    #[tokio::test]
    async fn stream_missing_id_returns_error() {
        let (app, _state, key) = subsonic_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/stream.view?apiKey={key}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"failed\""));
    }

    #[tokio::test]
    async fn stream_nonexistent_track_returns_error_70() {
        let (app, _state, key) = subsonic_app().await;
        let fake_id = Uuid::now_v7();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/stream.view?apiKey={key}&id={fake_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("code=\"70\""));
    }
}
