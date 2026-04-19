use std::sync::Arc;

use axum::Json;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use rand::Rng;
use serde_json::json;
use themelion::ids::UserId;

use crate::AuthService;
use crate::service::ExousiaServiceImpl;
use crate::user::UserRole;

fn correlation_id() -> String {
    let mut rng = rand::rng();
    let mut bytes = [0u8; 16];
    rng.fill_bytes(&mut bytes);
    bytes.iter().fold(String::with_capacity(32), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthMethod {
    Bearer,
    ApiKey,
    QueryParam,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: UserId,
    pub role: UserRole,
    pub auth_method: AuthMethod,
}

pub struct RequireAdmin(pub AuthenticatedUser);

fn unauthorized(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": message,
            "code": "UNAUTHORIZED",
            "correlation_id": correlation_id()
        })),
    )
        .into_response()
}

fn forbidden(message: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "error": message,
            "code": "FORBIDDEN",
            "correlation_id": correlation_id()
        })),
    )
        .into_response()
}

fn extract_bearer(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

fn extract_api_key_header(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn extract_query_token(parts: &Parts) -> Option<String> {
    parts.uri.query().and_then(|q| {
        q.split('&')
            .find_map(|pair| pair.strip_prefix("token=").map(|v| v.to_string()))
    })
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    Arc<ExousiaServiceImpl>: FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let service = Arc::<ExousiaServiceImpl>::from_ref(state);

        if let Some(token) = extract_bearer(parts) {
            return service
                .validate_bearer(&token)
                .await
                .map_err(|_| unauthorized("invalid or expired bearer token"));
        }

        if let Some(key) = extract_api_key_header(parts) {
            return service
                .validate_api_key(&key)
                .await
                .map_err(|_| unauthorized("invalid or revoked API key"));
        }

        if let Some(token) = extract_query_token(parts) {
            return service
                .validate_bearer(&token)
                .await
                .map(|mut u| {
                    u.auth_method = AuthMethod::QueryParam;
                    u
                })
                .map_err(|_| unauthorized("invalid or expired token"));
        }

        Err(unauthorized("authentication required"))
    }
}

impl<S> FromRequestParts<S> for RequireAdmin
where
    S: Send + Sync,
    Arc<ExousiaServiceImpl>: FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::from_request_parts(parts, state).await?;
        if user.role != UserRole::Admin {
            return Err(forbidden("admin access required"));
        }
        Ok(RequireAdmin(user))
    }
}

#[cfg(test)]
mod tests {
    use apotheke::DbPools;
    use apotheke::migrate::MIGRATOR;
    use axum::Router;
    use axum::body::Body;
    use axum::routing::get;
    use horismos::ExousiaConfig;
    use http::{Request, StatusCode};
    use sqlx::SqlitePool;
    use tower::ServiceExt;

    use super::*;
    use crate::AuthService;
    use crate::service::ExousiaServiceImpl;
    use crate::user::CreateUserRequest;

    async fn setup() -> Arc<ExousiaServiceImpl> {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let pools = Arc::new(DbPools {
            read: pool.clone(),
            write: pool,
        });
        let config = ExousiaConfig {
            access_token_ttl_secs: 900,
            refresh_token_ttl_days: 30,
            jwt_secret: "test-secret-that-is-long-enough-for-hs256".to_string(),
        };
        Arc::new(ExousiaServiceImpl::new(pools, config))
    }

    async fn make_user_and_token(
        service: &Arc<ExousiaServiceImpl>,
        username: &str,
        role: crate::user::UserRole,
    ) -> (crate::user::User, String) {
        let user = service
            .create_user(CreateUserRequest {
                username: username.to_string(),
                display_name: username.to_string(),
                password: "password123".to_string(),
                role,
            })
            .await
            .unwrap();
        let pair = service.login(username, "password123").await.unwrap();
        (user, pair.access_token)
    }

    async fn handler_ok(_user: AuthenticatedUser) -> StatusCode {
        StatusCode::OK
    }

    async fn handler_admin(_admin: RequireAdmin) -> StatusCode {
        StatusCode::OK
    }

    fn app(service: Arc<ExousiaServiceImpl>) -> Router {
        Router::new()
            .route("/auth", get(handler_ok))
            .route("/admin", get(handler_admin))
            .with_state(service)
    }

    #[tokio::test]
    async fn bearer_token_produces_authenticated_user() {
        let service = setup().await;
        let (_, token) = make_user_and_token(&service, "alice", UserRole::Member).await;
        let response = app(service)
            .oneshot(
                Request::builder()
                    .uri("/auth")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn api_key_header_produces_authenticated_user() {
        let service = setup().await;
        let (user, _) = make_user_and_token(&service, "bob", UserRole::Member).await;
        let key = service.create_api_key(user.id, "test key").await.unwrap();
        let response = app(service)
            .oneshot(
                Request::builder()
                    .uri("/auth")
                    .header("X-Api-Key", key)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn query_param_token_produces_authenticated_user() {
        let service = setup().await;
        let (_, token) = make_user_and_token(&service, "carol", UserRole::Member).await;
        let response = app(service)
            .oneshot(
                Request::builder()
                    .uri(format!("/auth?token={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn no_auth_returns_401() {
        let service = setup().await;
        let response = app(service)
            .oneshot(Request::builder().uri("/auth").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn require_admin_passes_for_admin() {
        let service = setup().await;
        let (_, token) = make_user_and_token(&service, "dave", UserRole::Admin).await;
        let response = app(service)
            .oneshot(
                Request::builder()
                    .uri("/admin")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn require_admin_returns_403_for_member() {
        let service = setup().await;
        let (_, token) = make_user_and_token(&service, "eve", UserRole::Member).await;
        let response = app(service)
            .oneshot(
                Request::builder()
                    .uri("/admin")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn bearer_takes_priority_over_api_key() {
        let service = setup().await;
        let (user, token) = make_user_and_token(&service, "frank", UserRole::Member).await;
        let key = service
            .create_api_key(user.id, "priority test")
            .await
            .unwrap();
        let response = app(service)
            .oneshot(
                Request::builder()
                    .uri("/auth")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("X-Api-Key", key)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn no_auth_response_has_structured_body() {
        let service = setup().await;
        let response = app(service)
            .oneshot(Request::builder().uri("/auth").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("error").is_some());
        assert_eq!(json["code"], "UNAUTHORIZED");
        assert!(json.get("correlation_id").is_some());
    }
}
