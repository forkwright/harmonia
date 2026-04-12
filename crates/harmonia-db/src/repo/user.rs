use sqlx::SqlitePool;

use crate::error::{DbError, QuerySnafu};
use snafu::ResultExt;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Vec<u8>,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub role: String,
    pub is_active: i64,
    pub created_at: String,
    pub last_login_at: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RefreshToken {
    pub id: Vec<u8>,
    pub user_id: Vec<u8>,
    pub token_hash: String,
    pub created_at: String,
    pub expires_at: String,
    pub revoked: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiKey {
    pub id: Vec<u8>,
    pub user_id: Vec<u8>,
    pub short_token: String,
    pub long_token_hash: String,
    pub label: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub revoked: i64,
}

// --- users ---

pub async fn insert_user(pool: &SqlitePool, user: &User) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO users
         (id, username, display_name, password_hash, role, is_active, created_at, last_login_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&user.id)
    .bind(&user.username)
    .bind(&user.display_name)
    .bind(&user.password_hash)
    .bind(&user.role)
    .bind(user.is_active)
    .bind(&user.created_at)
    .bind(&user.last_login_at)
    .execute(pool)
    .await
    .context(QuerySnafu { table: "users" })?;
    Ok(())
}

pub async fn get_user(pool: &SqlitePool, id: &[u8]) -> Result<Option<User>, DbError> {
    sqlx::query_as::<_, User>(
        "SELECT id, username, display_name, password_hash, role, is_active,
                created_at, last_login_at
         FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu { table: "users" })
}

pub async fn get_user_by_username(
    pool: &SqlitePool,
    username: &str,
) -> Result<Option<User>, DbError> {
    sqlx::query_as::<_, User>(
        "SELECT id, username, display_name, password_hash, role, is_active,
                created_at, last_login_at
         FROM users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu { table: "users" })
}

pub async fn list_users(pool: &SqlitePool, limit: i64, offset: i64) -> Result<Vec<User>, DbError> {
    sqlx::query_as::<_, User>(
        "SELECT id, username, display_name, password_hash, role, is_active,
                created_at, last_login_at
         FROM users ORDER BY username LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu { table: "users" })
}

pub async fn update_user(
    pool: &SqlitePool,
    id: &[u8],
    display_name: &str,
    role: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE users SET display_name = ?, role = ? WHERE id = ?")
        .bind(display_name)
        .bind(role)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "users" })?;
    Ok(())
}

pub async fn deactivate_user(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("UPDATE users SET is_active = 0 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "users" })?;
    Ok(())
}

pub async fn record_login(
    pool: &SqlitePool,
    id: &[u8],
    last_login_at: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE users SET last_login_at = ? WHERE id = ?")
        .bind(last_login_at)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "users" })?;
    Ok(())
}

pub async fn delete_user(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "users" })?;
    Ok(())
}

// --- refresh tokens ---

pub async fn insert_refresh_token(pool: &SqlitePool, token: &RefreshToken) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, created_at, expires_at, revoked)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&token.id)
    .bind(&token.user_id)
    .bind(&token.token_hash)
    .bind(&token.created_at)
    .bind(&token.expires_at)
    .bind(token.revoked)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "refresh_tokens",
    })?;
    Ok(())
}

pub async fn get_refresh_token_by_hash(
    pool: &SqlitePool,
    token_hash: &str,
) -> Result<Option<RefreshToken>, DbError> {
    sqlx::query_as::<_, RefreshToken>(
        "SELECT id, user_id, token_hash, created_at, expires_at, revoked
         FROM refresh_tokens WHERE token_hash = ?",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "refresh_tokens",
    })
}

pub async fn revoke_refresh_token(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("UPDATE refresh_tokens SET revoked = 1 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "refresh_tokens",
        })?;
    Ok(())
}

pub async fn delete_refresh_tokens_for_user(
    pool: &SqlitePool,
    user_id: &[u8],
) -> Result<(), DbError> {
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "refresh_tokens",
        })?;
    Ok(())
}

// --- api keys ---

pub async fn insert_api_key(pool: &SqlitePool, key: &ApiKey) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO api_keys
         (id, user_id, short_token, long_token_hash, label, created_at, last_used_at, revoked)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&key.id)
    .bind(&key.user_id)
    .bind(&key.short_token)
    .bind(&key.long_token_hash)
    .bind(&key.label)
    .bind(&key.created_at)
    .bind(&key.last_used_at)
    .bind(key.revoked)
    .execute(pool)
    .await
    .context(QuerySnafu { table: "api_keys" })?;
    Ok(())
}

pub async fn get_api_key_by_short_token(
    pool: &SqlitePool,
    short_token: &str,
) -> Result<Option<ApiKey>, DbError> {
    sqlx::query_as::<_, ApiKey>(
        "SELECT id, user_id, short_token, long_token_hash, label, created_at,
                last_used_at, revoked
         FROM api_keys WHERE short_token = ?",
    )
    .bind(short_token)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu { table: "api_keys" })
}

pub async fn list_api_keys_for_user(
    pool: &SqlitePool,
    user_id: &[u8],
) -> Result<Vec<ApiKey>, DbError> {
    sqlx::query_as::<_, ApiKey>(
        "SELECT id, user_id, short_token, long_token_hash, label, created_at,
                last_used_at, revoked
         FROM api_keys WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context(QuerySnafu { table: "api_keys" })
}

pub async fn revoke_api_key(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("UPDATE api_keys SET revoked = 1 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "api_keys" })?;
    Ok(())
}

pub async fn update_api_key_last_used(
    pool: &SqlitePool,
    id: &[u8],
    last_used_at: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE api_keys SET last_used_at = ? WHERE id = ?")
        .bind(last_used_at)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "api_keys" })?;
    Ok(())
}

pub async fn delete_api_key(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM api_keys WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "api_keys" })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate::MIGRATOR;

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    fn make_id() -> Vec<u8> {
        uuid::Uuid::now_v7().as_bytes().to_vec()
    }

    fn now() -> String {
        "2026-01-01T00:00:00Z".to_string()
    }

    fn test_user(id: Vec<u8>) -> User {
        User {
            id,
            username: "testuser".to_string(),
            display_name: "Test User".to_string(),
            password_hash: "$argon2id$placeholder".to_string(),
            role: "member".to_string(),
            is_active: 1,
            created_at: now(),
            last_login_at: None,
        }
    }

    #[tokio::test]
    async fn user_crud() {
        let pool = setup().await;
        let id = make_id();
        let user = test_user(id.clone());
        insert_user(&pool, &user).await.unwrap();

        // read
        let fetched = get_user(&pool, &id).await.unwrap().unwrap();
        assert_eq!(fetched.username, "testuser");
        assert_eq!(fetched.role, "member");
        assert_eq!(fetched.is_active, 1);

        // authenticate by username
        let by_username = get_user_by_username(&pool, "testuser")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(by_username.id, id);

        // UPDATE
        update_user(&pool, &id, "Updated Name", "admin")
            .await
            .unwrap();
        let updated = get_user(&pool, &id).await.unwrap().unwrap();
        assert_eq!(updated.display_name, "Updated Name");
        assert_eq!(updated.role, "admin");

        // deactivate
        deactivate_user(&pool, &id).await.unwrap();
        let deactivated = get_user(&pool, &id).await.unwrap().unwrap();
        assert_eq!(deactivated.is_active, 0);

        // record login
        record_login(&pool, &id, "2026-06-01T12:00:00Z")
            .await
            .unwrap();
        let with_login = get_user(&pool, &id).await.unwrap().unwrap();
        assert_eq!(
            with_login.last_login_at,
            Some("2026-06-01T12:00:00Z".to_string())
        );
    }

    #[tokio::test]
    async fn refresh_token_round_trip() {
        let pool = setup().await;
        let user_id = make_id();
        let user = test_user(user_id.clone());
        insert_user(&pool, &user).await.unwrap();

        let token_id = make_id();
        let token = RefreshToken {
            id: token_id.clone(),
            user_id: user_id.clone(),
            token_hash: "sha256hashoftoken".to_string(),
            created_at: now(),
            expires_at: "2026-02-01T00:00:00Z".to_string(),
            revoked: 0,
        };
        insert_refresh_token(&pool, &token).await.unwrap();

        let fetched = get_refresh_token_by_hash(&pool, "sha256hashoftoken")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.revoked, 0);

        revoke_refresh_token(&pool, &token_id).await.unwrap();
        let revoked = get_refresh_token_by_hash(&pool, "sha256hashoftoken")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(revoked.revoked, 1);
    }

    #[tokio::test]
    async fn api_key_round_trip() {
        let pool = setup().await;
        let user_id = make_id();
        let user = test_user(user_id.clone());
        insert_user(&pool, &user).await.unwrap();

        let key_id = make_id();
        let key = ApiKey {
            id: key_id.clone(),
            user_id: user_id.clone(),
            short_token: "abc12345".to_string(),
            long_token_hash: "sha256oflong".to_string(),
            label: "Home NAS".to_string(),
            created_at: now(),
            last_used_at: None,
            revoked: 0,
        };
        insert_api_key(&pool, &key).await.unwrap();

        let fetched = get_api_key_by_short_token(&pool, "abc12345")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.label, "Home NAS");
        assert_eq!(fetched.revoked, 0);

        revoke_api_key(&pool, &key_id).await.unwrap();
        let revoked = get_api_key_by_short_token(&pool, "abc12345")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(revoked.revoked, 1);
    }

    #[tokio::test]
    async fn list_users_empty_returns_empty() {
        let pool = setup().await;
        let results = list_users(&pool, 10, 0).await.unwrap();
        assert!(results.is_empty());
    }
}
