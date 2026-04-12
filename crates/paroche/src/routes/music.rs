use axum::{
    Json,
    extract::{Path, Query, State},
};
use exousia::{AuthenticatedUser, RequireAdmin};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::ParocheError,
    response::{ApiResponse, deleted},
    state::AppState,
};

#[derive(Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_per_page")]
    pub per_page: u64,
}

fn default_page() -> u64 {
    1
}

fn default_per_page() -> u64 {
    20
}

fn bytes_to_uuid_str(bytes: &[u8]) -> String {
    Uuid::from_slice(bytes)
        .map(|u| u.to_string())
        .unwrap_or_default()
}

#[derive(Serialize)]
pub struct ReleaseGroupResponse {
    pub id: String,
    pub title: String,
    pub rg_type: String,
    pub year: Option<i64>,
    pub added_at: String,
}

impl From<harmonia_db::repo::music::MusicReleaseGroup> for ReleaseGroupResponse {
    fn from(rg: harmonia_db::repo::music::MusicReleaseGroup) -> Self {
        Self {
            id: bytes_to_uuid_str(&rg.id),
            title: rg.title,
            rg_type: rg.rg_type,
            year: rg.year,
            added_at: rg.added_at,
        }
    }
}

#[derive(Serialize)]
pub struct TrackResponse {
    pub id: String,
    pub title: String,
    pub position: i64,
    pub duration_ms: Option<i64>,
    pub codec: Option<String>,
    pub added_at: String,
}

impl From<harmonia_db::repo::music::MusicTrack> for TrackResponse {
    fn from(t: harmonia_db::repo::music::MusicTrack) -> Self {
        Self {
            id: bytes_to_uuid_str(&t.id),
            title: t.title,
            position: t.position,
            duration_ms: t.duration_ms,
            codec: t.codec,
            added_at: t.added_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateReleaseGroupRequest {
    pub title: String,
    pub rg_type: String,
    pub year: Option<i64>,
}

#[derive(Deserialize)]
pub struct UpdateReleaseGroupRequest {
    pub title: String,
    pub rg_type: String,
    pub quality_profile_id: Option<i64>,
}

pub async fn list_release_groups(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let per_page = pagination.per_page.clamp(1, 100);
    let page = pagination.page.max(1);
    let offset = (page - 1) * per_page;

    let groups = harmonia_db::repo::music::list_release_groups(
        &state.db.read,
        per_page as i64,
        offset as i64,
    )
    .await?;

    let total = groups.len() as u64;
    let data: Vec<ReleaseGroupResponse> = groups.into_iter().map(Into::into).collect();
    Ok(ApiResponse::paginated(data, page, per_page, total))
}

pub async fn get_release_group(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let group = harmonia_db::repo::music::get_release_group(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    Ok(ApiResponse::ok(ReleaseGroupResponse::from(group)))
}

pub async fn create_release_group(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Json(body): Json<CreateReleaseGroupRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    if body.title.trim().is_empty() {
        return Err(ParocheError::Validation {
            message: "title is required".to_string(),
        });
    }

    let id = Uuid::now_v7().as_bytes().to_vec();
    let now = chrono_now();

    let group = harmonia_db::repo::music::MusicReleaseGroup {
        id: id.clone(),
        registry_id: None,
        title: body.title,
        rg_type: body.rg_type,
        mb_release_group_id: None,
        year: body.year,
        quality_profile_id: None,
        added_at: now,
    };

    harmonia_db::repo::music::insert_release_group(&state.db.write, &group).await?;

    let created = harmonia_db::repo::music::get_release_group(&state.db.read, &id)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::created(ReleaseGroupResponse::from(created)))
}

pub async fn update_release_group(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
    Json(body): Json<UpdateReleaseGroupRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    harmonia_db::repo::music::get_release_group(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    harmonia_db::repo::music::update_release_group(
        &state.db.write,
        &id_bytes,
        &body.title,
        &body.rg_type,
        body.quality_profile_id,
    )
    .await?;

    let updated = harmonia_db::repo::music::get_release_group(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::Internal)?;

    Ok(ApiResponse::ok(ReleaseGroupResponse::from(updated)))
}

pub async fn delete_release_group(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    harmonia_db::repo::music::get_release_group(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    harmonia_db::repo::music::delete_release_group(&state.db.write, &id_bytes).await?;

    Ok(deleted())
}

pub async fn list_tracks(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let per_page = pagination.per_page.clamp(1, 100);
    let page = pagination.page.max(1);
    let offset = (page - 1) * per_page;

    let tracks =
        harmonia_db::repo::music::search_tracks(&state.db.read, "", per_page as i64).await?;

    let _ = offset;
    let total = tracks.len() as u64;
    let data: Vec<TrackResponse> = tracks.into_iter().map(Into::into).collect();
    Ok(ApiResponse::paginated(data, page, per_page, total))
}

pub async fn get_track(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ParocheError::InvalidId)?;
    let id_bytes = uuid.as_bytes().to_vec();

    let track = harmonia_db::repo::music::get_track(&state.db.read, &id_bytes)
        .await?
        .ok_or(ParocheError::NotFound)?;

    Ok(ApiResponse::ok(TrackResponse::from(track)))
}

pub fn chrono_now_pub() -> String {
    chrono_now()
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format_unix_ts(secs)
}

fn format_unix_ts(secs: u64) -> String {
    let s = i64::try_from(secs).unwrap_or_default();
    let (year, month, day, hour, min, sec) = unix_to_parts(s);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

fn unix_to_parts(secs: i64) -> (i64, i64, i64, i64, i64, i64) {
    let sec = secs % 60;
    let mins = secs / 60;
    let min = mins % 60;
    let hours = mins / 60;
    let hour = hours % 24;
    let days = hours / 24;

    let mut year = 1970i64;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }

    let month_days = [
        31i64,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1i64;
    for &md in &month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        month += 1;
    }
    let day = remaining + 1;

    (year, month, day, hour, min, sec)
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn music_routes() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route(
            "/release-groups",
            get(list_release_groups).post(create_release_group),
        )
        .route(
            "/release-groups/{id}",
            get(get_release_group)
                .put(update_release_group)
                .delete(delete_release_group),
        )
        .route("/tracks", get(list_tracks))
        .route("/tracks/{id}", get(get_track))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::test_state;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use exousia::{
        AuthService,
        user::{CreateUserRequest, UserRole},
    };
    use tower::ServiceExt;

    async fn admin_token(auth: &std::sync::Arc<exousia::ExousiaServiceImpl>) -> String {
        auth.create_user(CreateUserRequest {
            username: "admin".to_string(),
            display_name: "Admin".to_string(),
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

    async fn member_token(auth: &std::sync::Arc<exousia::ExousiaServiceImpl>) -> String {
        auth.create_user(CreateUserRequest {
            username: "member".to_string(),
            display_name: "Member".to_string(),
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
    async fn list_release_groups_unauthenticated_returns_401() {
        let (state, _auth) = test_state().await;
        let app = music_routes().with_state(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/release-groups")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn create_release_group_requires_admin() {
        let (state, auth) = test_state().await;
        let token = member_token(&auth).await;
        let app = music_routes().with_state(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/release-groups")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"title":"Test","rg_type":"album"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn crud_release_group_happy_path() {
        let (state, auth) = test_state().await;
        let token = admin_token(&auth).await;
        let app = music_routes().with_state(state);

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/release-groups")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"title":"Led Zeppelin IV","rg_type":"album","year":1971}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let id = json["data"]["id"].as_str().unwrap().to_string();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/release-groups/{id}"))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/release-groups")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json.get("meta").is_some());

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/release-groups/{id}"))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn get_nonexistent_release_group_returns_404() {
        let (state, auth) = test_state().await;
        let token = member_token(&auth).await;
        let app = music_routes().with_state(state);
        let id = Uuid::now_v7();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/release-groups/{id}"))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
