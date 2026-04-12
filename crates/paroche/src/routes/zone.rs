/// Zone management API for multi-room synchronized playback.
use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};

use crate::{
    error::ParocheError,
    response::{ApiResponse, deleted},
    state::AppState,
};
use apotheke::repo::zone;

#[derive(Serialize)]
pub struct RendererResponse {
    pub id: String,
    pub name: String,
    pub address: String,
    pub created_at: String,
}

impl From<zone::Renderer> for RendererResponse {
    fn from(r: zone::Renderer) -> Self {
        Self {
            id: r.id,
            name: r.name,
            address: r.address,
            created_at: r.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct ZoneResponse {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub members: Vec<RendererResponse>,
}

impl From<zone::ZoneWithMembers> for ZoneResponse {
    fn from(z: zone::ZoneWithMembers) -> Self {
        Self {
            id: z.zone.id,
            name: z.zone.name,
            created_at: z.zone.created_at,
            members: z.members.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Deserialize)]
pub struct CreateZoneBody {
    pub name: String,
}

#[derive(Deserialize)]
pub struct AddMemberBody {
    pub renderer_id: String,
}

pub async fn create_zone(
    State(state): State<AppState>,
    Json(body): Json<CreateZoneBody>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    if body.name.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "name is required".to_string(),
        });
    }

    let id = ulid::Ulid::new().to_string();
    let created = zone::create_zone(&state.db.write, &id, body.name.trim()).await?;
    let with_members = zone::ZoneWithMembers {
        zone: created,
        members: vec![],
    };
    Ok(ApiResponse::created(ZoneResponse::from(with_members)))
}

pub async fn delete_zone(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    zone::delete_zone(&state.db.write, &id).await?;
    Ok(deleted())
}

pub async fn list_zones(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let zones = zone::list_zones(&state.db.read).await?;
    let data: Vec<ZoneResponse> = zones.into_iter().map(Into::into).collect();
    Ok(ApiResponse::ok(data))
}

pub async fn get_zone(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let z = zone::get_zone(&state.db.read, &id).await?;
    Ok(ApiResponse::ok(ZoneResponse::from(z)))
}

pub async fn add_member(
    State(state): State<AppState>,
    Path(zone_id): Path<String>,
    Json(body): Json<AddMemberBody>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    if body.renderer_id.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "renderer_id is required".to_string(),
        });
    }

    zone::add_member(&state.db.write, &zone_id, body.renderer_id.trim()).await?;
    let z = zone::get_zone(&state.db.read, &zone_id).await?;
    Ok(ApiResponse::ok(ZoneResponse::from(z)))
}

pub async fn remove_member(
    State(state): State<AppState>,
    Path((zone_id, renderer_id)): Path<(String, String)>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    zone::remove_member(&state.db.write, &zone_id, &renderer_id).await?;
    Ok(deleted())
}

pub async fn zone_play(
    State(_state): State<AppState>,
    Path(_zone_id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    // WHY: Playback initiation requires the streaming subsystem (syndesis).
    // Full implementation connects to all zone renderers and starts fan-out streaming.
    // Wired up once syndesis is integrated into the server runtime.
    Ok(ApiResponse::ok(serde_json::json!({ "status": "playing" })))
}

pub async fn zone_pause(
    State(_state): State<AppState>,
    Path(_zone_id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    Ok(ApiResponse::ok(serde_json::json!({ "status": "paused" })))
}

pub async fn zone_resume(
    State(_state): State<AppState>,
    Path(_zone_id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    Ok(ApiResponse::ok(serde_json::json!({ "status": "playing" })))
}

pub fn zone_routes() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/", get(list_zones).post(create_zone))
        .route("/{id}", get(get_zone).delete(delete_zone))
        .route("/{id}/members", post(add_member))
        .route(
            "/{id}/members/{renderer_id}",
            axum::routing::delete(remove_member),
        )
        .route("/{id}/play", post(zone_play))
        .route("/{id}/pause", post(zone_pause))
        .route("/{id}/resume", post(zone_resume))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::test_helpers::test_state;

    #[tokio::test]
    async fn zone_crud_lifecycle() {
        let (state, _) = test_state().await;

        // Seed a renderer directly
        apotheke::repo::zone::upsert_renderer(
            &state.db.write,
            "r1",
            "Speaker",
            "127.0.0.1:5000",
        )
        .await
        .unwrap();

        let app = crate::build_router(state);

        // Create zone
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/zones")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"Living Room"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let zone_id = body["data"]["id"].as_str().unwrap().to_string();
        assert_eq!(body["data"]["name"], "Living Room");
        assert!(body["data"]["members"].as_array().unwrap().is_empty());

        // List zones
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/zones")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["data"].as_array().unwrap().len(), 1);

        // Add member
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/zones/{zone_id}/members"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"renderer_id":"r1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["data"]["members"].as_array().unwrap().len(), 1);

        // Get zone
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/zones/{zone_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Remove member
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/zones/{zone_id}/members/r1"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Delete zone
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/zones/{zone_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn zone_playback_controls() {
        let (state, _) = test_state().await;
        apotheke::repo::zone::upsert_renderer(
            &state.db.write,
            "r1",
            "Speaker",
            "127.0.0.1:5000",
        )
        .await
        .unwrap();
        apotheke::repo::zone::create_zone(&state.db.write, "z1", "Test Zone")
            .await
            .unwrap();
        apotheke::repo::zone::add_member(&state.db.write, "z1", "r1")
            .await
            .unwrap();

        let app = crate::build_router(state);

        for endpoint in [
            "/api/zones/z1/play",
            "/api/zones/z1/pause",
            "/api/zones/z1/resume",
        ] {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(endpoint)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK, "failed for {endpoint}");
        }
    }
}
