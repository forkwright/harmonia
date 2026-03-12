use axum::{
    extract::{Query, State},
    response::Response,
};
use serde_json::json;

use super::{
    auth::authenticate,
    types::{SubsonicCommon, respond_ok},
};
use crate::state::AppState;

pub async fn ping(State(state): State<AppState>, Query(common): Query<SubsonicCommon>) -> Response {
    let user = match authenticate(&common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };
    respond_ok(user.format, "<ping />", Some(("ping", json!({}))))
}

pub async fn get_license(
    State(state): State<AppState>,
    Query(common): Query<SubsonicCommon>,
) -> Response {
    let user = match authenticate(&common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };
    let xml = r#"<license valid="true" email="" licenseExpires="2099-12-31T00:00:00Z" />"#;
    let json_val = json!({
        "valid": true,
        "email": "",
        "licenseExpires": "2099-12-31T00:00:00Z"
    });
    respond_ok(user.format, xml, Some(("license", json_val)))
}

pub async fn get_open_subsonic_extensions(
    State(state): State<AppState>,
    Query(common): Query<SubsonicCommon>,
) -> Response {
    let user = match authenticate(&common, &state).await {
        Ok(u) => u,
        Err(r) => return r,
    };
    let xml = r#"<openSubsonicExtensions><openSubsonicExtension name="formPost" versions="1" /><openSubsonicExtension name="apiKeyAuthentication" versions="1" /></openSubsonicExtensions>"#;
    let json_val = json!([
        { "name": "formPost", "versions": [1] },
        { "name": "apiKeyAuthentication", "versions": [1] }
    ]);
    respond_ok(user.format, xml, Some(("openSubsonicExtensions", json_val)))
}

#[cfg(test)]
mod tests {

    use crate::subsonic::test_helpers::{make_api_key, subsonic_app};
    use axum::{body::Body, body::to_bytes, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn ping_returns_ok_xml_with_correct_version() {
        let (app, _state, key) = subsonic_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/ping.view?apiKey={key}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));
        assert!(body.contains("version=\"1.16.1\""));
        assert!(body.contains("openSubsonic=\"true\""));
    }

    #[tokio::test]
    async fn ping_returns_json_when_requested() {
        let (app, _state, key) = subsonic_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/ping.view?apiKey={key}&f=json"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["subsonic-response"]["status"], "ok");
        assert_eq!(json["subsonic-response"]["version"], "1.16.1");
        assert_eq!(json["subsonic-response"]["openSubsonic"], true);
    }

    #[tokio::test]
    async fn ping_wrong_api_key_returns_error_40() {
        let (app, _state, _key) = subsonic_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/rest/ping.view?apiKey=hmn_bad_key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"failed\""));
        assert!(body.contains("code=\"40\""));
    }

    #[tokio::test]
    async fn ping_legacy_token_valid_accepted() {
        let (app, state, _key) = subsonic_app().await;
        let (u, t, s) = make_api_key::legacy_params(&state, "testuser", "secret123").await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/rest/ping.view?u={u}&t={t}&s={s}&v=1.16.1&c=test"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"ok\""));
    }

    #[tokio::test]
    async fn ping_legacy_token_wrong_rejected_with_40() {
        let (app, state, _key) = subsonic_app().await;
        let (u, _t, s) = make_api_key::legacy_params(&state, "testuser2", "correct_pass").await;
        // send wrong token
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/rest/ping.view?u={u}&t=wronghash&s={s}&v=1.16.1&c=test"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert!(body.contains("status=\"failed\""));
        assert!(body.contains("code=\"40\""));
    }
}
