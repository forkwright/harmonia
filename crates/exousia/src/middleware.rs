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
        // WHY: fmt::Write on String is infallible; ok() avoids unused-result warning
        write!(s, "{b:02x}").ok();
        s
    })
}

// WARNING: no query-parameter credential path. Tokens in URLs leak through
// access logs, referrer headers, and browser history; only header-delivered
// credentials (Authorization, X-Api-Key) are accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthMethod {
    Bearer,
    ApiKey,
    Basic,
}

/// Decodes an RFC 7617 `Authorization: Basic <base64(user:pass)>` header
/// value into `(username, password)`. The scheme match is case-insensitive
/// per RFC 7235; the password may itself contain colons.
pub fn decode_basic_credentials(header_value: &str) -> Option<(String, String)> {
    use base64::Engine;
    let (scheme, encoded) = header_value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("basic") {
        return None;
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .ok()?;
    let text = String::from_utf8(decoded).ok()?;
    let (username, password) = text.split_once(':')?;
    Some((username.to_string(), password.to_string()))
}

#[derive(Clone)]
pub struct AuthenticatedUser {
    pub user_id: UserId,
    pub role: UserRole,
    pub auth_method: AuthMethod,
}
impl std::fmt::Debug for AuthenticatedUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthenticatedUser")
            .field("user_id", &self.user_id)
            .field("role", &self.role)
            .field("auth_method", &self.auth_method)
            .finish()
    }
}

pub struct RequireAdmin(pub AuthenticatedUser);

fn unauthorized(message: &str) -> Response {
    unauthorized_with_code(message, "UNAUTHORIZED")
}

fn unauthorized_with_code(message: &str, code: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": message,
            "code": code,
            "correlation_id": correlation_id()
        })),
    )
        .into_response()
}

// NOTE: this is the handled site for auth-path errors — expiry is surfaced as
// a distinct code so clients can auto-refresh, infrastructure failures are
// logged here, and everything else collapses to an opaque 401.
fn auth_error_response(err: &crate::error::ExousiaError, credential: &str) -> Response {
    match err {
        crate::error::ExousiaError::TokenExpired { .. } => {
            unauthorized_with_code(&format!("expired {credential}"), "TOKEN_EXPIRED")
        }
        crate::error::ExousiaError::Database { .. } => {
            // WHY: a DB outage is not a credential problem — log the detail
            // server-side, keep the client body opaque.
            tracing::warn!(error = %err, "auth validation failed on infrastructure error");
            unauthorized_with_code(&format!("invalid {credential}"), "UNAUTHORIZED")
        }
        _ => unauthorized_with_code(&format!("invalid {credential}"), "UNAUTHORIZED"),
    }
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
                .map_err(|e| auth_error_response(&e, "bearer token"));
        }

        if let Some(key) = extract_api_key_header(parts) {
            return service
                .validate_api_key(&key)
                .await
                .map_err(|e| auth_error_response(&e, "API key"));
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
    use horismos::{Config, ConfigManager, ConfigOverrides, ExousiaConfig, Section};
    use http::{Request, StatusCode};
    use sqlx::SqlitePool;
    use tower::ServiceExt;

    use super::*;
    use crate::AuthService;
    use crate::service::ExousiaServiceImpl;
    use crate::user::CreateUserRequest;

    const TEST_JWT_SECRET: &str = "test-secret-that-is-long-enough-for-hs256";

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
            jwt_secret: TEST_JWT_SECRET.to_string(),
        };
        Arc::new(ExousiaServiceImpl::new(pools, Section::fixed(config)))
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

    async fn handler_auth_method(user: AuthenticatedUser) -> String {
        format!("{:?}", user.auth_method)
    }

    fn app(service: Arc<ExousiaServiceImpl>) -> Router {
        Router::new()
            .route("/auth", get(handler_ok))
            .route("/admin", get(handler_admin))
            .route("/auth-method", get(handler_auth_method))
            .with_state(service)
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    async fn body_string(response: axum::response::Response) -> String {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(body.to_vec()).unwrap()
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
    async fn query_param_token_is_rejected() {
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
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
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
        let json = body_json(response).await;
        assert!(json.get("error").is_some());
        assert_eq!(json["code"], "UNAUTHORIZED");
        assert!(json.get("correlation_id").is_some());
    }

    fn make_expired_token(user: &crate::user::User) -> String {
        use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // WHY: an hour in the past clears jsonwebtoken's default 60s leeway.
        let claims = crate::jwt::Claims {
            sub: user.id.into_uuid().to_string(),
            iss: "harmonia".to_string(),
            aud: "harmonia-clients".to_string(),
            exp: now - 3600,
            iat: now - 7200,
            jti: "test-jti".to_string(),
            role: "member".to_string(),
            display_name: user.display_name.clone(),
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn expired_bearer_returns_token_expired_code() {
        let service = setup().await;
        let (user, _) = make_user_and_token(&service, "grace", UserRole::Member).await;
        let expired = make_expired_token(&user);
        let response = app(service)
            .oneshot(
                Request::builder()
                    .uri("/auth")
                    .header("Authorization", format!("Bearer {expired}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(response).await;
        assert_eq!(json["code"], "TOKEN_EXPIRED");
    }

    #[tokio::test]
    async fn invalid_signature_bearer_returns_unauthorized_code() {
        let service = setup().await;
        make_user_and_token(&service, "heidi", UserRole::Member).await;
        let response = app(service)
            .oneshot(
                Request::builder()
                    .uri("/auth")
                    .header("Authorization", "Bearer not.a.jwt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(response).await;
        assert_eq!(json["code"], "UNAUTHORIZED");
    }

    // WHY: proves #529's immediate-rotation contract through the actual
    // middleware path — a bearer minted before a `jwt_secret` rotation must
    // 401 with `UNAUTHORIZED` (not `TOKEN_EXPIRED`) on the very next request,
    // with no dual-secret grace window.
    #[tokio::test]
    async fn rotated_secret_returns_401_unauthorized_not_expired() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let pools = Arc::new(DbPools {
            read: pool.clone(),
            write: pool,
        });
        let mut config = Config::default();
        config.exousia.jwt_secret = TEST_JWT_SECRET.to_string();
        let (manager, handle) = ConfigManager::new(
            config,
            std::path::PathBuf::from("unused.toml"),
            ConfigOverrides::default(),
        );
        let service = Arc::new(ExousiaServiceImpl::new(
            pools,
            handle.section(|c| &c.exousia),
        ));

        let (_, token) = make_user_and_token(&service, "mallory", UserRole::Member).await;

        let mut rotated = Config::default();
        rotated.exousia.jwt_secret = "rotated-secret-that-is-long-enough-for-hs256!!".to_string();
        manager.replace(rotated).unwrap();

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
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(response).await;
        assert_eq!(
            json["code"], "UNAUTHORIZED",
            "rotated-out token must fail as invalid, not surface as TOKEN_EXPIRED"
        );
    }

    #[tokio::test]
    async fn bearer_sets_auth_method_bearer() {
        let service = setup().await;
        let (_, token) = make_user_and_token(&service, "ivan", UserRole::Member).await;
        let response = app(service)
            .oneshot(
                Request::builder()
                    .uri("/auth-method")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_string(response).await, "Bearer");
    }

    fn basic_header(user: &str, pass: &str) -> String {
        use base64::Engine;
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("{user}:{pass}"))
        )
    }

    #[test]
    fn decode_basic_credentials_roundtrips() {
        let (user, pass) = decode_basic_credentials(&basic_header("alice", "s3cret")).unwrap();
        assert_eq!(user, "alice");
        assert_eq!(pass, "s3cret");
    }

    #[test]
    fn decode_basic_credentials_scheme_is_case_insensitive() {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode("alice:s3cret");
        let (user, _) = decode_basic_credentials(&format!("basic {encoded}")).unwrap();
        assert_eq!(user, "alice");
    }

    #[test]
    fn decode_basic_credentials_keeps_colons_in_password() {
        let (user, pass) = decode_basic_credentials(&basic_header("alice", "a:b:c")).unwrap();
        assert_eq!(user, "alice");
        assert_eq!(pass, "a:b:c");
    }

    #[test]
    fn decode_basic_credentials_rejects_malformed() {
        for bad in ["Basic", "Basic !!!not-base64!!!", "Bearer abc", &{
            use base64::Engine;
            // WHY: valid base64 but no colon separator inside.
            format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD.encode("no-colon")
            )
        }] {
            assert!(
                decode_basic_credentials(bad).is_none(),
                "should reject {bad:?}"
            );
        }
    }

    #[tokio::test]
    async fn api_key_sets_auth_method_api_key() {
        let service = setup().await;
        let (user, _) = make_user_and_token(&service, "judy", UserRole::Member).await;
        let key = service
            .create_api_key(user.id, "method test")
            .await
            .unwrap();
        let response = app(service)
            .oneshot(
                Request::builder()
                    .uri("/auth-method")
                    .header("X-Api-Key", key)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_string(response).await, "ApiKey");
    }
}
