pub mod api_key;
pub mod error;
pub mod jwt;
pub mod middleware;
pub mod password;
pub mod service;
pub mod user;

use themelion::ids::{ApiKeyId, UserId};

pub use error::ExousiaError;
pub use middleware::{AuthMethod, AuthenticatedUser, RequireAdmin};
pub use service::ExousiaServiceImpl;
pub use user::{CreateUserRequest, User, UserRole};

#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

#[expect(
    async_fn_in_trait,
    reason = "async fn in trait is stable since Rust 1.75; suppressed until Send bound concern is resolved"
)]
pub trait AuthService: Send + Sync {
    async fn login(&self, username: &str, password: &str) -> Result<TokenPair, ExousiaError>;
    async fn refresh(&self, refresh_token: &str) -> Result<TokenPair, ExousiaError>;
    async fn logout(&self, refresh_token: &str) -> Result<(), ExousiaError>;
    async fn validate_bearer(&self, token: &str) -> Result<AuthenticatedUser, ExousiaError>;
    async fn validate_api_key(&self, key: &str) -> Result<AuthenticatedUser, ExousiaError>;
    async fn create_user(&self, req: CreateUserRequest) -> Result<User, ExousiaError>;
    async fn create_api_key(&self, user_id: UserId, label: &str) -> Result<String, ExousiaError>;
    async fn revoke_api_key(&self, key_id: ApiKeyId) -> Result<(), ExousiaError>;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use apotheke::{DbPools, migrate::MIGRATOR};
    use horismos::ExousiaConfig;
    use jsonwebtoken::{Algorithm, EncodingKey, Header};
    use rand::Rng;
    use sqlx::SqlitePool;

    use super::*;
    use crate::{
        AuthService,
        jwt::Claims,
        service::ExousiaServiceImpl,
        user::{CreateUserRequest, UserRole},
    };

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

    async fn create_test_user(service: &Arc<ExousiaServiceImpl>) -> User {
        service
            .create_user(CreateUserRequest {
                username: "testuser".to_string(),
                display_name: "Test User".to_string(),
                password: "password123".to_string(),
                role: UserRole::Member,
            })
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn login_flow_returns_valid_tokens() {
        let service = setup().await;
        create_test_user(&service).await;
        let pair = service.login("testuser", "password123").await.unwrap();
        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
        let authenticated = service.validate_bearer(&pair.access_token).await.unwrap();
        assert_eq!(authenticated.role, UserRole::Member);
    }

    #[tokio::test]
    async fn login_wrong_password_fails() {
        let service = setup().await;
        create_test_user(&service).await;
        let err = service.login("testuser", "wrong").await.unwrap_err();
        assert!(matches!(err, ExousiaError::InvalidCredentials { .. }));
    }

    #[tokio::test]
    async fn login_unknown_user_fails() {
        let service = setup().await;
        let err = service.login("nobody", "password").await.unwrap_err();
        assert!(matches!(err, ExousiaError::InvalidCredentials { .. }));
    }

    #[tokio::test]
    async fn refresh_flow_rotates_token() {
        let service = setup().await;
        create_test_user(&service).await;
        let pair1 = service.login("testuser", "password123").await.unwrap();
        let pair2 = service.refresh(&pair1.refresh_token).await.unwrap();
        assert!(!pair2.access_token.is_empty());
        assert_ne!(pair1.refresh_token, pair2.refresh_token);
        let err = service.refresh(&pair1.refresh_token).await.unwrap_err();
        assert!(matches!(err, ExousiaError::TokenInvalid { .. }));
    }

    #[tokio::test]
    async fn logout_revokes_refresh_token() {
        let service = setup().await;
        create_test_user(&service).await;
        let pair = service.login("testuser", "password123").await.unwrap();
        service.logout(&pair.refresh_token).await.unwrap();
        let err = service.refresh(&pair.refresh_token).await.unwrap_err();
        assert!(matches!(err, ExousiaError::TokenInvalid { .. }));
    }

    #[tokio::test]
    async fn expired_token_rejected() {
        let service = setup().await;
        let user = create_test_user(&service).await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let claims = Claims {
            sub: user.id.into_uuid().to_string(),
            iss: "harmonia".to_string(),
            aud: "harmonia-clients".to_string(),
            exp: now - 100,
            iat: now - 1000,
            jti: {
                let mut rng = rand::rng();
                let mut bytes = [0u8; 16];
                rng.fill_bytes(&mut bytes);
                bytes.iter().fold(String::new(), |mut s, b| {
                    use std::fmt::Write;
                    write!(s, "{b:02x}").unwrap();
                    s
                })
            },
            role: "member".to_string(),
            display_name: "Test User".to_string(),
        };
        let expired_token = jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"test-secret-that-is-long-enough-for-hs256"),
        )
        .unwrap();
        let err = service.validate_bearer(&expired_token).await.unwrap_err();
        assert!(matches!(err, ExousiaError::TokenExpired { .. }));
    }

    #[tokio::test]
    async fn api_key_generate_validate_revoke() {
        let service = setup().await;
        let user = create_test_user(&service).await;
        let full_key = service.create_api_key(user.id, "test key").await.unwrap();
        let authenticated = service.validate_api_key(&full_key).await.unwrap();
        assert_eq!(authenticated.role, UserRole::Member);
        let parts: Vec<&str> = full_key.split('_').collect();
        let short_token = parts.get(1).copied().unwrap_or_default();
        let db_key =
            apotheke::repo::user::get_api_key_by_short_token(&service.pools().read, short_token)
                .await
                .unwrap()
                .unwrap();
        let key_id = ApiKeyId::from_uuid(uuid::Uuid::from_slice(&db_key.id).unwrap());
        service.revoke_api_key(key_id).await.unwrap();
        let err = service.validate_api_key(&full_key).await.unwrap_err();
        assert!(matches!(err, ExousiaError::ApiKeyRevoked { .. }));
    }

    #[tokio::test]
    async fn create_user_persists() {
        let service = setup().await;
        let user = create_test_user(&service).await;
        assert_eq!(user.username, "testuser");
        assert_eq!(user.role, UserRole::Member);
        assert!(user.is_active);
    }
}
