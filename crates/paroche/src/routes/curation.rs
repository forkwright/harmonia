use aggelmata::{HaveId, MediaType};
/// Library curation endpoints — delegates to kritike via DynCurationService.
use axum::Json;
use axum::extract::{Path, Query, State};
use exousia::AuthenticatedUser;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::ParocheError;
use crate::response::ApiResponse;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct AssessRequest {
    pub media_type: MediaType,
    pub metadata: kritike::QualityMetadata,
}

#[derive(Deserialize)]
pub struct UpgradeEligibilityQuery {
    pub candidate_score: i32,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn health_report(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let report = state.curation.health_report().await?;

    Ok(ApiResponse::ok(report))
}

pub async fn assess_quality(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<AssessRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let assessment = state
        .curation
        .assess_quality(body.media_type, body.metadata)
        .await?;

    Ok(ApiResponse::ok(assessment))
}

pub async fn check_upgrade_eligibility(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(have_id): Path<String>,
    Query(query): Query<UpgradeEligibilityQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let have_id = Uuid::parse_str(&have_id)
        .map(HaveId::from_uuid)
        .map_err(|_| ParocheError::InvalidId)?;

    let decision = state
        .curation
        .check_upgrade_eligibility(have_id, query.candidate_score)
        .await?;

    Ok(ApiResponse::ok(decision))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn curation_routes() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/health", get(health_report))
        .route("/assess", post(assess_quality))
        .route(
            "/upgrade-eligibility/{have_id}",
            get(check_upgrade_eligibility),
        )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
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
    use crate::state::{DynCurationService, ServiceFut};
    use crate::test_helpers::test_state;

    #[derive(Default)]
    struct RecordingCuration {
        health_calls: AtomicUsize,
        assess_calls: AtomicUsize,
        upgrade_calls: AtomicUsize,
    }

    impl DynCurationService for RecordingCuration {
        fn assess_quality(
            &self,
            _media_type: aggelmata::MediaType,
            item_metadata: kritike::QualityMetadata,
        ) -> ServiceFut<kritike::QualityAssessment> {
            self.assess_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                Ok(kritike::QualityAssessment {
                    score: 80 + item_metadata.custom_format_score,
                    format: item_metadata.format,
                    meets_minimum: true,
                    meets_ceiling: false,
                })
            })
        }

        fn check_upgrade_eligibility(
            &self,
            _have_id: aggelmata::HaveId,
            _candidate_score: i32,
        ) -> ServiceFut<kritike::UpgradeDecision> {
            self.upgrade_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(kritike::UpgradeDecision::Upgrade) })
        }

        fn health_report(&self) -> ServiceFut<kritike::HealthReport> {
            self.health_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {
                Ok(kritike::HealthReport {
                    total_items: 3,
                    per_type: HashMap::new(),
                })
            })
        }
    }

    async fn user_token(auth: &Arc<exousia::ExousiaServiceImpl>) -> String {
        auth.create_user(CreateUserRequest {
            username: "member".to_string(),
            display_name: "member".to_string(),
            password: "password123".to_string(),
            role: UserRole::Member,
        })
        .await
        .unwrap();
        auth.login("member", "password123")
            .await
            .unwrap()
            .access_token
    }

    #[tokio::test]
    async fn health_report_invokes_curation_service() {
        let (mut state, auth) = test_state().await;
        let curation = Arc::new(RecordingCuration::default());
        state.curation = curation.clone();
        let token = user_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/curation/health")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(curation.health_calls.load(Ordering::SeqCst), 1);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["data"]["total_items"], 3);
    }

    #[tokio::test]
    async fn assess_quality_invokes_curation_service() {
        let (mut state, auth) = test_state().await;
        let curation = Arc::new(RecordingCuration::default());
        state.curation = curation.clone();
        let token = user_token(&auth).await;

        let body = serde_json::json!({
            "media_type": "music",
            "metadata": {
                "format": "FLAC_24BIT",
                "custom_format_score": 5,
                "profile_id": 1,
                "codec": null,
                "bit_depth": 24,
                "sample_rate": 96000,
                "file_size": null,
                "channels": 2,
            }
        });

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/curation/assess")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(curation.assess_calls.load(Ordering::SeqCst), 1);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(payload["data"]["score"], 85);
        assert_eq!(payload["data"]["format"], "FLAC_24BIT");
    }

    #[tokio::test]
    async fn upgrade_eligibility_invokes_curation_service() {
        let (mut state, auth) = test_state().await;
        let curation = Arc::new(RecordingCuration::default());
        state.curation = curation.clone();
        let token = user_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/curation/upgrade-eligibility/{}?candidate_score=90",
                        uuid::Uuid::now_v7()
                    ))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(curation.upgrade_calls.load(Ordering::SeqCst), 1);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(payload["data"], "Upgrade");
    }

    #[tokio::test]
    async fn upgrade_eligibility_rejects_invalid_have_id() {
        let (state, auth) = test_state().await;
        let token = user_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/curation/upgrade-eligibility/not-a-uuid?candidate_score=90")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn health_report_unavailable_when_curation_not_wired() {
        let (state, auth) = test_state().await;
        let token = user_token(&auth).await;

        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/curation/health")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn health_report_rejects_unauthenticated() {
        let (state, _auth) = test_state().await;
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/curation/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
