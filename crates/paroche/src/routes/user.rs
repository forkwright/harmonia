use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use exousia::user::{CreateUserRequest, UserRole};
use exousia::{AuthService, RequireAdmin, TokenPair};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use crate::error::{DatabaseSnafu, ParocheError};
use crate::response::ApiResponse;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct TokenPairResponse {
    pub access_token: String,
    pub refresh_token: String,
}

impl From<TokenPair> for TokenPairResponse {
    fn from(p: TokenPair) -> Self {
        Self {
            access_token: p.access_token,
            refresh_token: p.refresh_token,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateUserBody {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub role: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: String,
}

impl From<exousia::user::User> for UserResponse {
    fn from(u: exousia::user::User) -> Self {
        Self {
            id: u.id.into_uuid().to_string(),
            username: u.username,
            display_name: u.display_name,
            role: u.role.as_str().to_string(),
            is_active: u.is_active,
            created_at: u.created_at,
        }
    }
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let pair = state
        .auth
        .login(&body.username, &body.password)
        .await
        .map_err(|_| ParocheError::Unauthorized)?;

    Ok(ApiResponse::ok(TokenPairResponse::from(pair)))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let pair = state
        .auth
        .refresh(&body.refresh_token)
        .await
        .map_err(|_| ParocheError::Unauthorized)?;

    Ok(ApiResponse::ok(TokenPairResponse::from(pair)))
}

pub async fn logout(
    State(state): State<AppState>,
    Json(body): Json<LogoutRequest>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    state
        .auth
        .logout(&body.refresh_token)
        .await
        .map_err(|_| ParocheError::Unauthorized)?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct UserListQuery {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_per_page")]
    pub per_page: u64,
}
fn default_page() -> u64 {
    1
}
fn default_per_page() -> u64 {
    100
}

pub async fn list_users(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Query(query): Query<UserListQuery>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let per_page = query.per_page.clamp(1, 100);
    let page = query.page.max(1);
    let offset = (page - 1) * per_page;

    // INVARIANT: per_page <= 100 and page comes from a u64 query param;
    // the i64 conversions cannot overflow for any page the DB can hold.
    let users = apotheke::repo::user::list_active_users(
        &state.db.read,
        per_page as i64,
        i64::try_from(offset).unwrap_or(i64::MAX),
    )
    .await
    .map_err(ParocheError::from)?;
    let total = apotheke::repo::user::count_active_users(&state.db.read)
        .await
        .map_err(ParocheError::from)?;

    // WHY: deactivated users are soft-deleted — they must not reappear in
    // the roster (DELETE /users/{id} contract); the repo query filters them.
    let data: Vec<UserResponse> = users
        .into_iter()
        .filter_map(|u| {
            let id_bytes = &u.id;
            let uuid = uuid::Uuid::from_slice(id_bytes).ok()?;
            let user_id = aggelmata::UserId::from_uuid(uuid);
            let role = exousia::user::UserRole::parse(&u.role).unwrap_or(UserRole::Member);
            Some(exousia::user::User {
                id: user_id,
                username: u.username,
                display_name: u.display_name,
                password_hash: u.password_hash,
                role,
                is_active: u.is_active != 0,
                created_at: u.created_at,
                last_login_at: u.last_login_at,
            })
        })
        .map(UserResponse::from)
        .collect();

    Ok(ApiResponse::paginated(
        data,
        page,
        per_page,
        u64::try_from(total).unwrap_or(0),
    ))
}

pub async fn create_user(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Json(body): Json<CreateUserBody>,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let role = match body.role.as_str() {
        "admin" => UserRole::Admin,
        _ => UserRole::Member,
    };

    let user = state
        .auth
        .create_user(CreateUserRequest {
            username: body.username,
            display_name: body.display_name,
            password: body.password,
            role,
        })
        .await
        .map_err(|_| ParocheError::Validation {
            message: "could not CREATE user".to_string(),
        })?;

    Ok(ApiResponse::created(UserResponse::from(user)))
}

pub async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _admin: RequireAdmin,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    // WHY: an unparseable id addresses a resource that cannot exist — 404,
    // matching the unknown-id case rather than leaking format details.
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| ParocheError::NotFound)?;
    let id_bytes = uuid.as_bytes().to_vec();

    apotheke::repo::user::get_user(&state.db.read, &id_bytes)
        .await
        .context(DatabaseSnafu)?
        .ok_or(ParocheError::NotFound)?;

    // WHY: soft-delete — deactivation preserves FK integrity (playlists,
    // downloads, history) while credential revocation severs every access
    // path that consults the database (refresh tokens, API keys, new logins).
    apotheke::repo::user::deactivate_user(&state.db.write, &id_bytes)
        .await
        .context(DatabaseSnafu)?;
    apotheke::repo::user::delete_refresh_tokens_for_user(&state.db.write, &id_bytes)
        .await
        .context(DatabaseSnafu)?;
    apotheke::repo::user::revoke_api_keys_for_user(&state.db.write, &id_bytes)
        .await
        .context(DatabaseSnafu)?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn auth_routes() -> axum::Router<AppState> {
    use axum::routing::post;
    axum::Router::new()
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
}

pub fn user_routes() -> axum::Router<AppState> {
    use axum::routing::{delete, get};
    axum::Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/{id}", delete(delete_user))
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

    fn make_app(state: crate::state::AppState) -> axum::Router {
        axum::Router::new()
            .nest("/auth", auth_routes())
            .nest("/users", user_routes())
            .with_state(state)
    }

    #[tokio::test]
    async fn login_returns_token_pair() {
        let (state, auth) = test_state().await;
        auth.create_user(CreateUserRequest {
            username: "alice".to_string(),
            display_name: "Alice".to_string(),
            password: "secret123".to_string(),
            role: exousia::user::UserRole::Member,
        })
        .await
        .unwrap();

        let app = make_app(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"username":"alice","password":"secret123"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["data"]["access_token"].is_string());
        assert!(json["data"]["refresh_token"].is_string());
    }

    #[tokio::test]
    async fn list_users_unauthenticated_returns_401() {
        let (state, _auth) = test_state().await;
        let app = make_app(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/users")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn member_cannot_create_user() {
        let (state, auth) = test_state().await;
        auth.create_user(CreateUserRequest {
            username: "member".to_string(),
            display_name: "Member".to_string(),
            password: "password123".to_string(),
            role: exousia::user::UserRole::Member,
        })
        .await
        .unwrap();
        let token = auth
            .login("member", "password123")
            .await
            .unwrap()
            .access_token;

        let app = make_app(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/users")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"username":"new","display_name":"New","password":"pass","role":"member"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    async fn admin_setup(auth: &std::sync::Arc<exousia::ExousiaServiceImpl>) -> String {
        auth.create_user(CreateUserRequest {
            username: "root".to_string(),
            display_name: "Root".to_string(),
            password: "password123".to_string(),
            role: exousia::user::UserRole::Admin,
        })
        .await
        .unwrap();
        auth.login("root", "password123")
            .await
            .unwrap()
            .access_token
    }

    #[tokio::test]
    async fn delete_user_unknown_id_returns_404() {
        let (state, auth) = test_state().await;
        let admin = admin_setup(&auth).await;
        let app = make_app(state);

        for id in [uuid::Uuid::now_v7().to_string(), "not-a-uuid".to_string()] {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("DELETE")
                        .uri(format!("/users/{id}"))
                        .header("Authorization", format!("Bearer {admin}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                resp.status(),
                StatusCode::NOT_FOUND,
                "expected 404 for {id}"
            );
        }
    }

    #[tokio::test]
    async fn delete_user_requires_admin() {
        let (state, auth) = test_state().await;
        auth.create_user(CreateUserRequest {
            username: "member".to_string(),
            display_name: "Member".to_string(),
            password: "password123".to_string(),
            role: exousia::user::UserRole::Member,
        })
        .await
        .unwrap();
        let token = auth
            .login("member", "password123")
            .await
            .unwrap()
            .access_token;
        let app = make_app(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/users/{}", uuid::Uuid::now_v7()))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn delete_user_deactivates_and_revokes() {
        let (state, auth) = test_state().await;
        let admin = admin_setup(&auth).await;

        let victim = auth
            .create_user(CreateUserRequest {
                username: "victim".to_string(),
                display_name: "Victim".to_string(),
                password: "password123".to_string(),
                role: exousia::user::UserRole::Member,
            })
            .await
            .unwrap();
        let victim_id = victim.id.into_uuid().to_string();
        let victim_pair = auth.login("victim", "password123").await.unwrap();

        let app = make_app(state);

        // Delete (deactivate) as admin
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/users/{victim_id}"))
                    .header("Authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Fresh login is rejected
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"username":"victim","password":"password123"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // Prior refresh token is rejected
        let refresh_body =
            serde_json::json!({ "refresh_token": victim_pair.refresh_token }).to_string();
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/refresh")
                    .header("Content-Type", "application/json")
                    .body(Body::from(refresh_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // No longer listed
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/users")
                    .header("Authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let listed: Vec<&str> = json["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|u| u["id"].as_str().unwrap())
            .collect();
        assert!(!listed.contains(&victim_id.as_str()));

        // Repeat delete of an already-deactivated (but existing) user is idempotent
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/users/{victim_id}"))
                    .header("Authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn list_users_pages_beyond_100_users() {
        let (state, auth) = test_state().await;
        let admin = admin_setup(&auth).await;

        // Seed 104 members directly at the repo layer (bypassing Argon2)
        for i in 0..104 {
            let user = apotheke::repo::user::User {
                id: uuid::Uuid::now_v7().as_bytes().to_vec(),
                username: format!("user{i:03}"),
                display_name: format!("User {i:03}"),
                password_hash: "x".to_string(),
                role: "member".to_string(),
                is_active: 1,
                created_at: "2026-01-01T00:00:00Z".to_string(),
                last_login_at: None,
            };
            apotheke::repo::user::insert_user(&state.db.write, &user)
                .await
                .unwrap();
        }

        let app = make_app(state);

        // Default page caps at 100
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/users")
                    .header("Authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["data"].as_array().unwrap().len(), 100);
        // 104 seeded members + the admin
        assert_eq!(json["meta"]["total"], 105);

        // Second page returns the remainder
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/users?page=2")
                    .header("Authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["data"].as_array().unwrap().len(), 5);
    }

    #[tokio::test]
    async fn admin_can_list_users() {
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

        let app = make_app(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/users")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
