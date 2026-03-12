use axum::{Json, extract::State, http::StatusCode};
use exousia::RequireAdmin;
use serde_json::json;

use crate::{error::ParocheError, state::AppState};

pub async fn health() -> impl axum::response::IntoResponse {
    (
        StatusCode::OK,
        Json(json!({"status": "ok", "version": "0.1.0"})),
    )
}

pub async fn config(
    State(state): State<AppState>,
    _admin: RequireAdmin,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let cfg = &state.config;
    Ok((
        StatusCode::OK,
        Json(json!({
            "paroche": {
                "port": cfg.paroche.port,
                "listen_addr": cfg.paroche.listen_addr,
                "stream_buffer_kb": cfg.paroche.stream_buffer_kb,
                "opds_page_size": cfg.paroche.opds_page_size,
            }
        })),
    ))
}

pub fn system_routes() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/health", get(health))
        .route("/config", get(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::test_state;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use exousia::{AuthService, user::CreateUserRequest};
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_returns_ok() {
        let (state, _auth) = test_state().await;
        let app = system_routes().with_state(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn config_without_auth_returns_401() {
        let (state, _auth) = test_state().await;
        let app = system_routes().with_state(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/config")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn config_with_admin_returns_200() {
        let (state, auth) = test_state().await;
        auth.create_user(CreateUserRequest {
            username: "admin".to_string(),
            display_name: "Admin".to_string(),
            password: "password123".to_string(),
            role: exousia::user::UserRole::Admin,
        })
        .await
        .unwrap();
        let token = auth
            .login("admin", "password123")
            .await
            .unwrap()
            .access_token;

        let app = system_routes().with_state(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/config")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
