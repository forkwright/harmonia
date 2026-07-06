pub mod api_key;
pub mod error;
pub mod jwt;
pub mod middleware;
pub mod password;
pub mod service;
pub mod user;

pub use error::ExousiaError;
pub use middleware::{AuthMethod, AuthenticatedUser, RequireAdmin};
pub use service::ExousiaServiceImpl;
use themelion::ids::{ApiKeyId, UserId};
pub use user::{CreateUserRequest, User, UserRole};

#[derive(Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}
impl std::fmt::Debug for TokenPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenPair")
            .field("access_token", &"[redacted]")
            .field("refresh_token", &"[redacted]")
            .finish()
    }
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
    /// Revokes `key_id` only when it belongs to `user_id` — a cross-user pair
    /// matches nothing and errors, so a key cannot be revoked by id alone.
    async fn revoke_api_key(&self, user_id: UserId, key_id: ApiKeyId) -> Result<(), ExousiaError>;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use apotheke::DbPools;
    use apotheke::migrate::MIGRATOR;
    use horismos::{Config, ConfigManager, ConfigOverrides, ExousiaConfig, Section};
    use jsonwebtoken::{Algorithm, EncodingKey, Header};
    use rand::Rng;
    use sqlx::SqlitePool;

    use super::*;
    use crate::AuthService;
    use crate::jwt::Claims;
    use crate::service::ExousiaServiceImpl;
    use crate::user::{CreateUserRequest, UserRole};

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
        Arc::new(ExousiaServiceImpl::new(pools, Section::fixed(config)))
    }

    /// A `Config` with a valid `jwt_secret` and everything else defaulted —
    /// mirrors horismos's own `valid_config()` test helper (handle.rs).
    fn config_with_secret(secret: &str) -> Config {
        let mut config = Config::default();
        config.exousia.jwt_secret = secret.to_string();
        config
    }

    /// A service backed by a LIVE `Section` (not `Section::fixed`) plus the
    /// `ConfigManager` that drives it — the lever the rotation tests use to
    /// mirror step 2's live-handle test pattern (`ConfigManager::replace`).
    async fn setup_live(secret: &str) -> (Arc<ExousiaServiceImpl>, ConfigManager) {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let pools = Arc::new(DbPools {
            read: pool.clone(),
            write: pool,
        });
        let (manager, handle) = ConfigManager::new(
            config_with_secret(secret),
            std::path::PathBuf::from("unused.toml"),
            ConfigOverrides::default(),
        );
        let service = Arc::new(ExousiaServiceImpl::new(
            pools,
            handle.section(|c| &c.exousia),
        ));
        (service, manager)
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

    // ── #529 step 3: live JWT secret / TTL, immediate rotation ───────────────

    const OLD_SECRET: &str = "old-secret-that-is-long-enough-for-hs256!!!!";
    const NEW_SECRET: &str = "new-secret-that-is-long-enough-for-hs256!!!!";

    #[tokio::test]
    async fn rotation_invalidates_old_access_token_immediately() {
        let (service, manager) = setup_live(OLD_SECRET).await;
        create_test_user(&service).await;
        let pair = service.login("testuser", "password123").await.unwrap();
        // Sanity: valid before rotation.
        service.validate_bearer(&pair.access_token).await.unwrap();

        manager.replace(config_with_secret(NEW_SECRET)).unwrap();

        let err = service
            .validate_bearer(&pair.access_token)
            .await
            .unwrap_err();
        assert!(
            matches!(err, ExousiaError::TokenInvalid { .. }),
            "rotated-out token must be TokenInvalid (signature mismatch), not TokenExpired: {err:?}"
        );

        // A fresh login mints and verifies under the new secret with no
        // dual-secret grace window.
        let new_pair = service.login("testuser", "password123").await.unwrap();
        let authenticated = service
            .validate_bearer(&new_pair.access_token)
            .await
            .unwrap();
        assert_eq!(authenticated.role, UserRole::Member);
    }

    #[tokio::test]
    async fn refresh_across_rotation_mints_under_new_secret_and_keeps_session() {
        let (service, manager) = setup_live(OLD_SECRET).await;
        create_test_user(&service).await;
        let pair = service.login("testuser", "password123").await.unwrap();

        manager.replace(config_with_secret(NEW_SECRET)).unwrap();

        // WHY: refresh tokens are opaque sha256-hashed DB rows, not signed
        // with jwt_secret — the session survives rotation intact.
        let refreshed = service.refresh(&pair.refresh_token).await.unwrap();
        crate::jwt::validate_token(&refreshed.access_token, NEW_SECRET.as_bytes())
            .expect("post-rotation mint must verify under the new secret");
        let under_old_secret =
            crate::jwt::validate_token(&refreshed.access_token, OLD_SECRET.as_bytes());
        assert!(
            under_old_secret.is_err(),
            "post-rotation mint must NOT verify under the retired secret"
        );
    }

    #[tokio::test]
    async fn ttl_change_affects_only_the_next_mint() {
        let (service, manager) = setup_live(OLD_SECRET).await;
        create_test_user(&service).await;

        let pair = service.login("testuser", "password123").await.unwrap();
        let exp_before = crate::jwt::validate_token(&pair.access_token, OLD_SECRET.as_bytes())
            .unwrap()
            .exp;

        let mut shorter_ttl = config_with_secret(OLD_SECRET);
        shorter_ttl.exousia.access_token_ttl_secs = 60;
        manager.replace(shorter_ttl).unwrap();

        // The already-minted token's baked-in exp is untouched by the reload
        // — no retroactive expiry.
        let exp_after_reload =
            crate::jwt::validate_token(&pair.access_token, OLD_SECRET.as_bytes())
                .unwrap()
                .exp;
        assert_eq!(
            exp_after_reload, exp_before,
            "a TTL reload must not retroactively change an already-minted exp"
        );

        let pair2 = service.login("testuser", "password123").await.unwrap();
        let exp2 = crate::jwt::validate_token(&pair2.access_token, OLD_SECRET.as_bytes())
            .unwrap()
            .exp;
        assert!(
            exp2 < exp_before,
            "a mint after the reload must reflect the shortened TTL"
        );
    }

    #[tokio::test]
    async fn refresh_concurrent_double_use_returns_one_success() {
        // WHY: max_connections(1) mirrors the production write pool and keeps
        // sqlite::memory: on a single database across both concurrent tasks.
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let pools = Arc::new(DbPools {
            read: pool.clone(),
            write: pool.clone(),
        });
        let config = ExousiaConfig {
            access_token_ttl_secs: 900,
            refresh_token_ttl_days: 30,
            jwt_secret: "test-secret-that-is-long-enough-for-hs256".to_string(),
        };
        let service = Arc::new(ExousiaServiceImpl::new(pools, Section::fixed(config)));
        create_test_user(&service).await;
        let pair = service.login("testuser", "password123").await.unwrap();
        let (r1, r2) = tokio::join!(
            service.refresh(&pair.refresh_token),
            service.refresh(&pair.refresh_token)
        );
        let ok_count = usize::from(r1.is_ok()) + usize::from(r2.is_ok());
        assert_eq!(ok_count, 1, "exactly one concurrent refresh must succeed");
        let failure = if r1.is_err() { r1.err() } else { r2.err() };
        assert!(matches!(failure, Some(ExousiaError::TokenInvalid { .. })));
        let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM refresh_tokens")
            .fetch_one(&pool)
            .await
            .unwrap();
        let (active,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM refresh_tokens WHERE revoked = 0")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(total, 2, "original token plus exactly one rotation");
        assert_eq!(active, 1, "only the rotated token remains active");
    }

    #[tokio::test]
    async fn refresh_rejects_deactivated_user() {
        let service = setup().await;
        let user = create_test_user(&service).await;
        let pair = service.login("testuser", "password123").await.unwrap();
        apotheke::repo::user::deactivate_user(
            &service.pools().write,
            user.id.as_bytes().as_slice(),
        )
        .await
        .unwrap();
        let err = service.refresh(&pair.refresh_token).await.unwrap_err();
        assert!(matches!(err, ExousiaError::UserInactive { .. }));
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
    async fn bearer_reflects_role_demotion() {
        let service = setup().await;
        let user = service
            .create_user(CreateUserRequest {
                username: "admin-user".to_string(),
                display_name: "Admin User".to_string(),
                password: "password123".to_string(),
                role: UserRole::Admin,
            })
            .await
            .unwrap();
        let pair = service.login("admin-user", "password123").await.unwrap();
        let before = service.validate_bearer(&pair.access_token).await.unwrap();
        assert_eq!(before.role, UserRole::Admin);
        apotheke::repo::user::update_user(
            &service.pools().write,
            user.id.as_bytes().as_slice(),
            "Admin User",
            "member",
        )
        .await
        .unwrap();
        let after = service.validate_bearer(&pair.access_token).await.unwrap();
        assert_eq!(
            after.role,
            UserRole::Member,
            "role must come from the live DB row, not the stale JWT claim"
        );
    }

    #[tokio::test]
    async fn bearer_rejects_deactivated_user() {
        let service = setup().await;
        let user = create_test_user(&service).await;
        let pair = service.login("testuser", "password123").await.unwrap();
        service.validate_bearer(&pair.access_token).await.unwrap();
        apotheke::repo::user::deactivate_user(
            &service.pools().write,
            user.id.as_bytes().as_slice(),
        )
        .await
        .unwrap();
        let err = service
            .validate_bearer(&pair.access_token)
            .await
            .unwrap_err();
        assert!(matches!(err, ExousiaError::UserInactive { .. }));
    }

    #[tokio::test]
    async fn bearer_rejects_deleted_user() {
        let service = setup().await;
        let user = create_test_user(&service).await;
        let pair = service.login("testuser", "password123").await.unwrap();
        apotheke::repo::user::delete_user(&service.pools().write, user.id.as_bytes().as_slice())
            .await
            .unwrap();
        let err = service
            .validate_bearer(&pair.access_token)
            .await
            .unwrap_err();
        assert!(matches!(err, ExousiaError::TokenInvalid { .. }));
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
        service.revoke_api_key(user.id, key_id).await.unwrap();
        let err = service.validate_api_key(&full_key).await.unwrap_err();
        assert!(matches!(err, ExousiaError::ApiKeyRevoked { .. }));
    }

    #[tokio::test]
    async fn revoke_api_key_rejects_cross_user() {
        let service = setup().await;
        let owner = create_test_user(&service).await;
        let other = service
            .create_user(CreateUserRequest {
                username: "otheruser".to_string(),
                display_name: "Other User".to_string(),
                password: "password123".to_string(),
                role: UserRole::Member,
            })
            .await
            .unwrap();

        let full_key = service.create_api_key(owner.id, "owner key").await.unwrap();
        let parts: Vec<&str> = full_key.split('_').collect();
        let short_token = parts.get(1).copied().unwrap_or_default();
        let db_key =
            apotheke::repo::user::get_api_key_by_short_token(&service.pools().read, short_token)
                .await
                .unwrap()
                .unwrap();
        let key_id = ApiKeyId::from_uuid(uuid::Uuid::from_slice(&db_key.id).unwrap());

        let err = service.revoke_api_key(other.id, key_id).await.unwrap_err();
        assert!(matches!(err, ExousiaError::Database { .. }));

        // Owner's key still works — the cross-user attempt revoked nothing.
        service.validate_api_key(&full_key).await.unwrap();
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
