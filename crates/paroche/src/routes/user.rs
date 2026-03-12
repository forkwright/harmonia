use axum::{Json, extract::State, http::StatusCode};
use exousia::{
    AuthService, RequireAdmin, TokenPair,
    user::{CreateUserRequest, UserRole},
};
use serde::{Deserialize, Serialize};

use crate::{error::ParocheError, response::ApiResponse, state::AppState};

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

pub async fn list_users(
    State(state): State<AppState>,
    _admin: RequireAdmin,
) -> Result<impl axum::response::IntoResponse, ParocheError> {
    let users = harmonia_db::repo::user::list_users(&state.db.read, 100, 0)
        .await
        .map_err(ParocheError::from)?;

    let data: Vec<UserResponse> = users
        .into_iter()
        .filter_map(|u| {
            let id_bytes = &u.id;
            let uuid = uuid::Uuid::from_slice(id_bytes).ok()?;
            let user_id = harmonia_common::UserId::from_uuid(uuid);
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

    Ok(ApiResponse::ok(data))
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
            message: "could not create user".to_string(),
        })?;

    Ok(ApiResponse::created(UserResponse::from(user)))
}

pub async fn delete_user(
    State(_state): State<AppState>,
    _admin: RequireAdmin,
) -> impl axum::response::IntoResponse {
    StatusCode::NO_CONTENT
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
    use super::*;
    use crate::test_helpers::test_state;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use exousia::{AuthService, user::CreateUserRequest};
    use tower::ServiceExt;

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
