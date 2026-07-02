// Renderer registry: CRUD for paired playback renderers
use snafu::ResultExt;
use sqlx::SqlitePool;

use crate::error::{DbError, NotFoundSnafu, QuerySnafu};

#[derive(Clone, sqlx::FromRow)]
pub struct Renderer {
    pub id: String,
    pub name: String,
    pub api_key_hash: String,
    pub cert_fingerprint: String,
    pub last_seen: Option<String>,
    pub paired_at: String,
    pub enabled: i64,
}
impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("api_key_hash", &"[redacted]")
            .field("cert_fingerprint", &self.cert_fingerprint)
            .field("last_seen", &self.last_seen)
            .field("paired_at", &self.paired_at)
            .field("enabled", &self.enabled)
            .finish()
    }
}

pub async fn create_renderer(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    api_key_hash: &str,
    cert_fingerprint: &str,
) -> Result<Renderer, DbError> {
    let now = jiff::Zoned::now()
        .strftime("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    sqlx::query(
        "INSERT INTO renderers (id, name, api_key_hash, cert_fingerprint, last_seen, paired_at, enabled)
         VALUES (?, ?, ?, ?, NULL, ?, 1)",
    )
    .bind(id)
    .bind(name)
    .bind(api_key_hash)
    .bind(cert_fingerprint)
    .bind(&now)
    .execute(pool)
    .await
    .context(QuerySnafu { table: "renderers" })?;

    get_renderer(pool, id)
        .await?
        .ok_or_else(|| DbError::NotFound {
            table: "renderers".to_string(),
            id: id.to_string(),
            location: snafu::location!(),
        })
}

pub async fn get_renderer(pool: &SqlitePool, id: &str) -> Result<Option<Renderer>, DbError> {
    sqlx::query_as::<_, Renderer>(
        "SELECT id, name, api_key_hash, cert_fingerprint, last_seen, paired_at, enabled
         FROM renderers WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu { table: "renderers" })
}

pub async fn list_renderers(pool: &SqlitePool) -> Result<Vec<Renderer>, DbError> {
    sqlx::query_as::<_, Renderer>(
        "SELECT id, name, api_key_hash, cert_fingerprint, last_seen, paired_at, enabled
         FROM renderers ORDER BY paired_at DESC",
    )
    .fetch_all(pool)
    .await
    .context(QuerySnafu { table: "renderers" })
}

pub async fn update_last_seen(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    let now = jiff::Zoned::now()
        .strftime("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let result = sqlx::query("UPDATE renderers SET last_seen = ? WHERE id = ?")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "renderers" })?;

    if result.rows_affected() == 0 {
        return Err(NotFoundSnafu {
            table: "renderers",
            id: id.to_string(),
        }
        .build());
    }
    Ok(())
}

pub async fn set_enabled(pool: &SqlitePool, id: &str, enabled: bool) -> Result<(), DbError> {
    let result = sqlx::query("UPDATE renderers SET enabled = ? WHERE id = ?")
        .bind(if enabled { 1i64 } else { 0i64 })
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "renderers" })?;

    if result.rows_affected() == 0 {
        return Err(NotFoundSnafu {
            table: "renderers",
            id: id.to_string(),
        }
        .build());
    }
    Ok(())
}

pub async fn rename_renderer(pool: &SqlitePool, id: &str, name: &str) -> Result<(), DbError> {
    let result = sqlx::query("UPDATE renderers SET name = ? WHERE id = ?")
        .bind(name)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "renderers" })?;

    if result.rows_affected() == 0 {
        return Err(NotFoundSnafu {
            table: "renderers",
            id: id.to_string(),
        }
        .build());
    }
    Ok(())
}

pub async fn delete_renderer(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    let result = sqlx::query("DELETE FROM renderers WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "renderers" })?;

    if result.rows_affected() == 0 {
        return Err(NotFoundSnafu {
            table: "renderers",
            id: id.to_string(),
        }
        .build());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    use super::*;
    use crate::migrate::MIGRATOR;

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    fn renderer_id() -> String {
        uuid::Uuid::now_v7().to_string()
    }

    #[tokio::test]
    async fn renderer_crud() {
        let pool = setup().await;
        let id = renderer_id();

        // CREATE
        let r = create_renderer(&pool, &id, "Living Room", "hash1", "fp1")
            .await
            .unwrap();
        assert_eq!(r.name, "Living Room");
        assert_eq!(r.enabled, 1);
        assert!(r.last_seen.is_none());

        // get
        let fetched = get_renderer(&pool, &id).await.unwrap().unwrap();
        assert_eq!(fetched.api_key_hash, "hash1");
        assert_eq!(fetched.cert_fingerprint, "fp1");

        // update_last_seen
        update_last_seen(&pool, &id).await.unwrap();
        let updated = get_renderer(&pool, &id).await.unwrap().unwrap();
        assert!(updated.last_seen.is_some());

        // rename
        rename_renderer(&pool, &id, "Kitchen").await.unwrap();
        let renamed = get_renderer(&pool, &id).await.unwrap().unwrap();
        assert_eq!(renamed.name, "Kitchen");

        // disable
        set_enabled(&pool, &id, false).await.unwrap();
        let disabled = get_renderer(&pool, &id).await.unwrap().unwrap();
        assert_eq!(disabled.enabled, 0);

        // re-enable
        set_enabled(&pool, &id, true).await.unwrap();
        let enabled = get_renderer(&pool, &id).await.unwrap().unwrap();
        assert_eq!(enabled.enabled, 1);

        // DELETE
        delete_renderer(&pool, &id).await.unwrap();
        assert!(get_renderer(&pool, &id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_renderers_empty() {
        let pool = setup().await;
        let result = list_renderers(&pool).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn list_renderers_multiple() {
        let pool = setup().await;
        let id1 = renderer_id();
        let id2 = renderer_id();

        create_renderer(&pool, &id1, "A", "h1", "f1").await.unwrap();
        create_renderer(&pool, &id2, "B", "h2", "f2").await.unwrap();

        let list = list_renderers(&pool).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    // -- zero-row writes surface NotFound (issue: silent no-op writes) --

    #[tokio::test]
    async fn update_last_seen_nonexistent_returns_not_found() {
        let pool = setup().await;
        let result = update_last_seen(&pool, &renderer_id()).await;
        assert!(matches!(result, Err(DbError::NotFound { .. })));
    }

    #[tokio::test]
    async fn set_enabled_nonexistent_returns_not_found() {
        let pool = setup().await;
        let result = set_enabled(&pool, &renderer_id(), false).await;
        assert!(matches!(result, Err(DbError::NotFound { .. })));
    }

    #[tokio::test]
    async fn rename_renderer_nonexistent_returns_not_found() {
        let pool = setup().await;
        let result = rename_renderer(&pool, &renderer_id(), "Ghost").await;
        assert!(matches!(result, Err(DbError::NotFound { .. })));
    }

    #[tokio::test]
    async fn delete_renderer_nonexistent_returns_not_found() {
        let pool = setup().await;
        let result = delete_renderer(&pool, &renderer_id()).await;
        assert!(matches!(result, Err(DbError::NotFound { .. })));
    }
}
