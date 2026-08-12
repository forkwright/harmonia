/// Metadata resolution endpoints — delegates to epignosis via DynMetadataResolver.
use std::path::PathBuf;

use aggelmata::{MediaId, MediaType};
use axum::Json;
use axum::extract::State;
use exousia::{AuthenticatedUser, RequireAdmin};
use serde::Deserialize;

use crate::error::ParocheError;
use crate::response::ApiResponse;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct IdentifyRequest {
    pub media_id: MediaId,
    pub media_type: MediaType,
    pub file_path: String,
    pub filename_hint: Option<String>,
}

#[derive(Deserialize)]
pub struct FingerprintRequest {
    pub file_path: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn identify_media(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<IdentifyRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let file_path = PathBuf::from(&body.file_path);
    // WHY: without a hint the resolver's provider query is an empty title and
    // can never match; the file stem is the best available fallback.
    let filename_hint = body.filename_hint.or_else(|| {
        file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(ToOwned::to_owned)
    });

    let identity = state
        .metadata
        .resolve_identity(epignosis::UnidentifiedItem {
            media_id: body.media_id,
            media_type: body.media_type,
            file_path,
            filename_hint,
            tags: None,
        })
        .await?;

    Ok(ApiResponse::ok(identity))
}

pub async fn enrich_media(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(identity): Json<epignosis::MediaIdentity>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let enriched = state.metadata.enrich(identity).await?;

    Ok(ApiResponse::ok(enriched))
}

// WHY: admin-only — fingerprinting reads an arbitrary server-side file path.
pub async fn fingerprint_media(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Json(body): Json<FingerprintRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let result = state
        .metadata
        .fingerprint_audio(PathBuf::from(body.file_path))
        .await?;

    Ok(ApiResponse::ok(result))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn metadata_routes() -> axum::Router<AppState> {
    use axum::routing::post;
    axum::Router::new()
        .route("/identify", post(identify_media))
        .route("/enrich", post(enrich_media))
        .route("/fingerprint", post(fingerprint_media))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use exousia::AuthService;
    use exousia::user::{CreateUserRequest, UserRole};
    use tower::ServiceExt;

    #[expect(
        unused_imports,
        reason = "kanon: test-missing-use-super; parent items accessed via explicit super:: prefix in test bodies"
    )]
    use super::*;
    use crate::state::{DynMetadataResolver, ServiceError, ServiceFut};
    use crate::test_helpers::test_state;

    #[derive(Default)]
    struct RecordingResolver {
        identify_calls: AtomicUsize,
        enrich_calls: AtomicUsize,
    }

    impl DynMetadataResolver for RecordingResolver {
        fn resolve_identity(
            &self,
            item: epignosis::UnidentifiedItem,
        ) -> ServiceFut<epignosis::MediaIdentity> {
            self.identify_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                Ok(epignosis::MediaIdentity {
                    media_id: item.media_id,
                    media_type: item.media_type,
                    provider: "test-provider".to_string(),
                    provider_id: epignosis::identity::MetadataProviderId("prov-1".to_string()),
                    canonical_title: item.filename_hint.unwrap_or_default(),
                    canonical_artist: None,
                    year: Some(1997),
                    extra: serde_json::Value::Null,
                })
            })
        }

        fn enrich(
            &self,
            identity: epignosis::MediaIdentity,
        ) -> ServiceFut<epignosis::EnrichedMetadata> {
            self.enrich_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                Ok(epignosis::EnrichedMetadata {
                    identity,
                    enrichments: vec![],
                })
            })
        }

        fn fingerprint_audio(
            &self,
            _file_path: std::path::PathBuf,
        ) -> ServiceFut<epignosis::FingerprintResult> {
            Box::pin(async { Err(ServiceError::Internal("no fpcalc".to_string())) })
        }
    }

    async fn admin_token(auth: &Arc<exousia::ExousiaServiceImpl>) -> String {
        auth.create_user(CreateUserRequest {
            username: "admin".to_string(),
            display_name: "admin".to_string(),
            password: "password123".to_string(),
            role: UserRole::Admin,
        })
        .await
        .unwrap();
        auth.login("admin", "password123")
            .await
            .unwrap()
            .access_token
    }

    fn identify_body() -> String {
        serde_json::json!({
            "media_id": uuid::Uuid::now_v7().to_string(),
            "media_type": "music",
            "file_path": "/library/music/Radiohead/OK Computer/01 - Airbag.flac",
        })
        .to_string()
    }

    #[tokio::test]
    async fn identify_invokes_resolver_and_returns_identity() {
        let (mut state, auth) = test_state().await;
        let resolver = Arc::new(RecordingResolver::default());
        state.metadata = resolver.clone();
        let token = admin_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/metadata/identify")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(identify_body()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resolver.identify_calls.load(Ordering::SeqCst), 1);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["data"]["provider"], "test-provider");
        assert_eq!(
            body["data"]["canonical_title"], "01 - Airbag",
            "a missing filename_hint must default to the file stem"
        );
    }

    #[tokio::test]
    async fn enrich_invokes_resolver() {
        let (mut state, auth) = test_state().await;
        let resolver = Arc::new(RecordingResolver::default());
        state.metadata = resolver.clone();
        let token = admin_token(&auth).await;

        let identity = serde_json::json!({
            "media_id": uuid::Uuid::now_v7().to_string(),
            "media_type": "music",
            "provider": "musicbrainz",
            "provider_id": "mbid-1",
            "canonical_title": "OK Computer",
            "canonical_artist": "Radiohead",
            "year": 1997,
            "extra": null,
        });

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/metadata/enrich")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(identity.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resolver.enrich_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn identify_unavailable_when_resolver_not_wired() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/metadata/identify")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(identify_body()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn identify_rejects_unauthenticated() {
        let (state, _auth) = test_state().await;
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/metadata/identify")
                    .header("content-type", "application/json")
                    .body(Body::from(identify_body()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn fingerprint_requires_admin() {
        let (state, auth) = test_state().await;
        auth.create_user(CreateUserRequest {
            username: "member".to_string(),
            display_name: "member".to_string(),
            password: "password123".to_string(),
            role: UserRole::Member,
        })
        .await
        .unwrap();
        let token = auth
            .login("member", "password123")
            .await
            .unwrap()
            .access_token;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/metadata/fingerprint")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"file_path": "/library/a.flac"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
