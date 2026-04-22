use snafu::ResultExt;
use sqlx::SqlitePool;

use crate::error::{DbError, QuerySnafu};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct KOSyncUser {
    pub id: Vec<u8>,
    pub username: String,
    pub password_hash: String,
    pub created_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct KOSyncPosition {
    pub id: Vec<u8>,
    pub username: String,
    pub document: String,
    pub progress: Option<String>,
    pub percentage: f64,
    pub device: Option<String>,
    pub device_id: Option<String>,
    pub updated_at: String,
}

pub async fn create_kosync_user(
    pool: &SqlitePool,
    id: &[u8],
    username: &str,
    password_hash: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO kosync_users (id, username, password_hash)
         VALUES (?, ?, ?)",
    )
    .bind(id)
    .bind(username)
    .bind(password_hash)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "kosync_users",
    })?;
    Ok(())
}

pub async fn get_kosync_user_by_username(
    pool: &SqlitePool,
    username: &str,
) -> Result<Option<KOSyncUser>, DbError> {
    sqlx::query_as::<_, KOSyncUser>(
        "SELECT id, username, password_hash, created_at FROM kosync_users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "kosync_users",
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "KOSync protocol requires 8 fields per PUT /syncs/progress; refactoring to a struct would hide the wire format"
)]
pub async fn put_kosync_position(
    pool: &SqlitePool,
    id: &[u8],
    username: &str,
    document: &str,
    progress: Option<&str>,
    percentage: f64,
    device: Option<&str>,
    device_id: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO kosync_positions
         (id, username, document, progress, percentage, device, device_id)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(username, document) DO UPDATE SET
            progress = excluded.progress,
            percentage = excluded.percentage,
            device = excluded.device,
            device_id = excluded.device_id,
            updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
    )
    .bind(id)
    .bind(username)
    .bind(document)
    .bind(progress)
    .bind(percentage)
    .bind(device)
    .bind(device_id)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "kosync_positions",
    })?;
    Ok(())
}

pub async fn get_kosync_position(
    pool: &SqlitePool,
    username: &str,
    document: &str,
) -> Result<Option<KOSyncPosition>, DbError> {
    sqlx::query_as::<_, KOSyncPosition>(
        "SELECT id, username, document, progress, percentage, device, device_id, updated_at
         FROM kosync_positions WHERE username = ? AND document = ?",
    )
    .bind(username)
    .bind(document)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "kosync_positions",
    })
}

pub async fn get_all_kosync_positions_for_user(
    pool: &SqlitePool,
    username: &str,
) -> Result<Vec<KOSyncPosition>, DbError> {
    sqlx::query_as::<_, KOSyncPosition>(
        "SELECT id, username, document, progress, percentage, device, device_id, updated_at
         FROM kosync_positions WHERE username = ? ORDER BY updated_at DESC",
    )
    .bind(username)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "kosync_positions",
    })
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

    #[tokio::test]
    async fn create_user_and_retrieve() {
        let pool = setup().await;
        let user_id = make_id();
        let hash = "356a192b7913b04c54574d18c28d46e6395428ab"; // SHA1("1")

        create_kosync_user(&pool, &user_id, "testuser", hash)
            .await
            .unwrap();

        let fetched = get_kosync_user_by_username(&pool, "testuser")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.username, "testuser");
        assert_eq!(fetched.password_hash, hash);
    }

    #[tokio::test]
    async fn put_and_get_position() {
        let pool = setup().await;
        let user_id = make_id();
        let hash = "356a192b7913b04c54574d18c28d46e6395428ab";

        create_kosync_user(&pool, &user_id, "reader", hash)
            .await
            .unwrap();

        let pos_id = make_id();
        let doc_hash = "5d41402abc4b2a76b9719d911017c592"; // MD5 example

        put_kosync_position(
            &pool,
            &pos_id,
            "reader",
            doc_hash,
            Some("/body/DocFragment[5]/body/p[3]"),
            0.25,
            Some("Kindle"),
            Some("device-001"),
        )
        .await
        .unwrap();

        let fetched = get_kosync_position(&pool, "reader", doc_hash)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.username, "reader");
        assert_eq!(fetched.document, doc_hash);
        assert_eq!(fetched.percentage, 0.25);
        assert_eq!(fetched.device, Some("Kindle".to_string()));
    }

    #[tokio::test]
    async fn last_write_wins_updates_position() {
        let pool = setup().await;
        let user_id = make_id();
        let hash = "356a192b7913b04c54574d18c28d46e6395428ab";

        create_kosync_user(&pool, &user_id, "reader", hash)
            .await
            .unwrap();

        let doc_hash = "5d41402abc4b2a76b9719d911017c592";

        // First write
        put_kosync_position(
            &pool,
            &make_id(),
            "reader",
            doc_hash,
            Some("/body/p[10]"),
            0.2,
            Some("Device1"),
            Some("dev1"),
        )
        .await
        .unwrap();

        // Second write (same user, same document) — should overwrite
        put_kosync_position(
            &pool,
            &make_id(),
            "reader",
            doc_hash,
            Some("/body/p[20]"),
            0.4,
            Some("Device2"),
            Some("dev2"),
        )
        .await
        .unwrap();

        let final_pos = get_kosync_position(&pool, "reader", doc_hash)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(final_pos.percentage, 0.4);
        assert_eq!(final_pos.device, Some("Device2".to_string()));
    }
}
