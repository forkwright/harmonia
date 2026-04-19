// Renderer registry API endpoints
use apotheke::repo::renderer;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, patch};
use exousia::RequireAdmin;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use crate::error::{DatabaseSnafu, ParocheError};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct RendererResponse {
    pub id: String,
    pub name: String,
    pub cert_fingerprint: String,
    pub last_seen: Option<String>,
    pub paired_at: String,
    pub enabled: bool,
}

impl From<renderer::Renderer> for RendererResponse {
    fn from(r: renderer::Renderer) -> Self {
        Self {
            id: r.id,
            name: r.name,
            cert_fingerprint: r.cert_fingerprint,
            last_seen: r.last_seen,
            paired_at: r.paired_at,
            enabled: r.enabled != 0,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PatchRendererRequest {
    pub name: Option<String>,
    pub enabled: Option<bool>,
}

pub async fn list_renderers(
    State(state): State<AppState>,
    _admin: RequireAdmin,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let renderers = renderer::list_renderers(&state.db.read)
        .await
        .context(DatabaseSnafu)?;
    let body: Vec<RendererResponse> = renderers.into_iter().map(Into::into).collect();
    Ok((StatusCode::OK, Json(body)))
}

pub async fn unpair_renderer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _admin: RequireAdmin,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let existing = renderer::get_renderer(&state.db.read, &id)
        .await
        .context(DatabaseSnafu)?;

    if existing.is_none() {
        return Err(ParocheError::NotFound);
    }

    renderer::delete_renderer(&state.db.write, &id)
        .await
        .context(DatabaseSnafu)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn patch_renderer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _admin: RequireAdmin,
    Json(req): Json<PatchRendererRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let existing = renderer::get_renderer(&state.db.read, &id)
        .await
        .context(DatabaseSnafu)?;

    if existing.is_none() {
        return Err(ParocheError::NotFound);
    }

    if let Some(name) = &req.name {
        renderer::rename_renderer(&state.db.write, &id, name)
            .await
            .context(DatabaseSnafu)?;
    }

    if let Some(enabled) = req.enabled {
        renderer::set_enabled(&state.db.write, &id, enabled)
            .await
            .context(DatabaseSnafu)?;
    }

    let updated = renderer::get_renderer(&state.db.read, &id)
        .await
        .context(DatabaseSnafu)?
        .ok_or(ParocheError::NotFound)?;

    Ok((StatusCode::OK, Json(RendererResponse::from(updated))))
}

pub fn renderer_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(list_renderers))
        .route("/{id}/unpair", delete(unpair_renderer))
        .route("/{id}", patch(patch_renderer))
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use exousia::AuthService;
    use exousia::user::CreateUserRequest;
    use tower::ServiceExt;

    use super::*;
    use crate::test_helpers::test_state;

    async fn admin_token(auth: &std::sync::Arc<exousia::ExousiaServiceImpl>) -> String {
        auth.create_user(CreateUserRequest {
            username: "admin".to_string(),
            display_name: "Admin".to_string(),
            password: "password123".to_string(),
            role: exousia::user::UserRole::Admin,
        })
        .await
        .unwrap();
        auth.login("admin", "password123")
            .await
            .unwrap()
            .access_token
    }

    #[tokio::test]
    async fn list_renderers_empty() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let app = renderer_routes().with_state(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn list_renderers_requires_auth() {
        let (state, _auth) = test_state().await;
        let app = renderer_routes().with_state(state);
        let resp = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn unpair_nonexistent_returns_404() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let app = renderer_routes().with_state(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/nonexistent-id/unpair")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn renderer_lifecycle() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;

        // Seed a renderer directly via DB
        let id = uuid::Uuid::now_v7().to_string();
        renderer::create_renderer(&state.db.write, &id, "Test", "hash", "fp")
            .await
            .unwrap();

        let app = renderer_routes().with_state(state);

        // list
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 1);

        // patch (rename + disable)
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/{id}"))
                    .header("Authorization", format!("Bearer {token}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"name":"Renamed","enabled":false}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["name"], "Renamed");
        assert_eq!(json["enabled"], false);

        // unpair
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/{id}/unpair"))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }
}
