//! OPDS-facing authentication: Basic + Bearer + API key, with the OPDS
//! Authentication Document served on every 401.

use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use exousia::{AuthService, AuthenticatedUser, ExousiaServiceImpl, decode_basic_credentials};
use serde_json::json;

pub const MIME_OPDS_AUTH: &str = "application/opds-authentication+json";
pub const AUTH_DOCUMENT_PATH: &str = "/opds/auth";

// NOTE: charset advertises UTF-8 credential encoding per RFC 7617 §2.1.
const WWW_AUTHENTICATE_BASIC: &str = "Basic realm=\"Harmonia\", charset=\"UTF-8\"";

// WHY: relative id/href — the server never knows its externally visible
// origin (no hardcoded hosts); OPDS clients resolve against the request base.
fn authentication_document() -> serde_json::Value {
    json!({
        "id": AUTH_DOCUMENT_PATH,
        "title": "Harmonia",
        "description": "Sign in with your Harmonia username and password.",
        "authentication": [{
            "type": "http://opds-spec.org/auth/basic",
            "labels": { "login": "Username", "password": "Password" }
        }]
    })
}

/// Serves the OPDS Authentication Document — deliberately unauthenticated so
/// clients can discover how to authenticate.
pub async fn auth_document() -> Response {
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static(MIME_OPDS_AUTH),
        )],
        authentication_document().to_string(),
    )
        .into_response()
}

// WHY: a 401 carrying the Authentication Document as its body plus a
// `WWW-Authenticate: Basic` challenge serves both client classes at once —
// conformant OPDS readers parse the document, plain HTTP clients (browsers,
// e-readers without OPDS-auth support) fall back to the Basic challenge.
fn opds_unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static(MIME_OPDS_AUTH),
            ),
            (
                header::WWW_AUTHENTICATE,
                HeaderValue::from_static(WWW_AUTHENTICATE_BASIC),
            ),
            (
                header::LINK,
                HeaderValue::from_static(
                    "</opds/auth>; rel=\"http://opds-spec.org/auth/document\"; \
                     type=\"application/opds-authentication+json\"",
                ),
            ),
        ],
        authentication_document().to_string(),
    )
        .into_response()
}

fn is_basic_scheme(header_value: &str) -> bool {
    header_value
        .split_once(' ')
        .is_some_and(|(scheme, _)| scheme.eq_ignore_ascii_case("basic"))
}

/// Authenticated identity for OPDS catalog/acquisition routes and the
/// browser reader: accepts HTTP Basic (validated against the exousia user
/// store) in addition to the Bearer/API-key paths, and answers every
/// credential failure with a 401 + OPDS Authentication Document.
pub struct OpdsUser(pub AuthenticatedUser);

impl std::fmt::Debug for OpdsUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OpdsUser").field(&self.0).finish()
    }
}

impl<S> FromRequestParts<S> for OpdsUser
where
    S: Send + Sync,
    Arc<ExousiaServiceImpl>: FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let authorization = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());

        if let Some(value) = authorization
            && is_basic_scheme(value)
        {
            let Some((username, password)) = decode_basic_credentials(value) else {
                return Err(opds_unauthorized());
            };
            let service = Arc::<ExousiaServiceImpl>::from_ref(state);
            return service
                .validate_basic(&username, &password)
                .await
                .map(OpdsUser)
                .map_err(|err| {
                    if matches!(err, exousia::ExousiaError::Database { .. }) {
                        // WHY: a DB outage is not a credential problem — log
                        // the detail server-side, keep the client body opaque.
                        tracing::warn!(error = %err, "opds basic auth failed on infrastructure error");
                    }
                    opds_unauthorized()
                });
        }

        // WHY: the standard extractor's JSON-error rejection is replaced
        // wholesale — on OPDS routes every 401 must carry the Authentication
        // Document, whatever credential form failed.
        AuthenticatedUser::from_request_parts(parts, state)
            .await
            .map(OpdsUser)
            .map_err(|_| opds_unauthorized())
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use base64::Engine;
    use tower::ServiceExt;

    use super::*;
    use crate::test_helpers::test_state;

    pub(crate) fn basic_header(user: &str, pass: &str) -> String {
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("{user}:{pass}"))
        )
    }

    async fn handler_ok(_user: OpdsUser) -> StatusCode {
        StatusCode::OK
    }

    async fn app() -> (Router, std::sync::Arc<exousia::ExousiaServiceImpl>) {
        let (state, auth) = test_state().await;
        let router = Router::new()
            .route("/protected", axum::routing::get(handler_ok))
            .with_state(state);
        (router, auth)
    }

    async fn create_member(auth: &exousia::ExousiaServiceImpl) {
        auth.create_user(exousia::CreateUserRequest {
            username: "alice".to_string(),
            display_name: "Alice".to_string(),
            password: "password123".to_string(),
            role: exousia::UserRole::Member,
        })
        .await
        .unwrap();
    }

    fn get(uri: &str, authorization: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder().uri(uri);
        if let Some(value) = authorization {
            builder = builder.header("Authorization", value);
        }
        builder.body(Body::empty()).unwrap()
    }

    #[tokio::test]
    async fn valid_basic_credentials_authenticate() {
        let (router, auth) = app().await;
        create_member(&auth).await;
        let resp = router
            .oneshot(get(
                "/protected",
                Some(&basic_header("alice", "password123")),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn wrong_basic_password_gets_401_with_auth_document() {
        let (router, auth) = app().await;
        create_member(&auth).await;
        let resp = router
            .oneshot(get(
                "/protected",
                Some(&basic_header("alice", "wrong-pass")),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(resp.headers().get("content-type").unwrap(), MIME_OPDS_AUTH);
        let www = resp
            .headers()
            .get("www-authenticate")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(www.starts_with("Basic"));
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            doc["authentication"][0]["type"],
            "http://opds-spec.org/auth/basic"
        );
    }

    #[tokio::test]
    async fn absent_credentials_get_401_with_challenge() {
        let (router, _auth) = app().await;
        let resp = router.oneshot(get("/protected", None)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert!(resp.headers().get("www-authenticate").is_some());
        assert!(resp.headers().get("link").is_some());
        assert_eq!(resp.headers().get("content-type").unwrap(), MIME_OPDS_AUTH);
    }

    #[tokio::test]
    async fn malformed_basic_value_gets_401() {
        let (router, _auth) = app().await;
        let resp = router
            .oneshot(get("/protected", Some("Basic !!!not-base64!!!")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn bearer_still_authenticates() {
        let (router, auth) = app().await;
        create_member(&auth).await;
        let token = auth
            .login("alice", "password123")
            .await
            .unwrap()
            .access_token;
        let resp = router
            .oneshot(get("/protected", Some(&format!("Bearer {token}"))))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn lowercase_basic_scheme_authenticates() {
        let (router, auth) = app().await;
        create_member(&auth).await;
        let header = basic_header("alice", "password123").replacen("Basic", "basic", 1);
        let resp = router
            .oneshot(get("/protected", Some(&header)))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
