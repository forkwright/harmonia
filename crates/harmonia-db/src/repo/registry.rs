use sqlx::SqlitePool;

use crate::error::{DbError, QuerySnafu};
use snafu::ResultExt;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RegistryEntry {
    pub id: Vec<u8>,
    pub entity_type: String,
    pub display_name: String,
    pub sort_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RegistryExternalId {
    pub registry_id: Vec<u8>,
    pub provider: String,
    pub external_id: String,
}

pub async fn insert_registry_entry(
    pool: &SqlitePool,
    entry: &RegistryEntry,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO media_registry (id, entity_type, display_name, sort_name, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&entry.id)
    .bind(&entry.entity_type)
    .bind(&entry.display_name)
    .bind(&entry.sort_name)
    .bind(&entry.created_at)
    .bind(&entry.updated_at)
    .execute(pool)
    .await
    .context(QuerySnafu { table: "media_registry" })?;
    Ok(())
}

pub async fn get_registry_entry(
    pool: &SqlitePool,
    id: &[u8],
) -> Result<Option<RegistryEntry>, DbError> {
    sqlx::query_as::<_, RegistryEntry>(
        "SELECT id, entity_type, display_name, sort_name, created_at, updated_at
         FROM media_registry WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "media_registry",
    })
}

pub async fn list_registry_entries(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<RegistryEntry>, DbError> {
    sqlx::query_as::<_, RegistryEntry>(
        "SELECT id, entity_type, display_name, sort_name, created_at, updated_at
         FROM media_registry ORDER BY display_name LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "media_registry",
    })
}

pub async fn update_registry_entry(
    pool: &SqlitePool,
    id: &[u8],
    display_name: &str,
    sort_name: Option<&str>,
    updated_at: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE media_registry SET display_name = ?, sort_name = ?, updated_at = ? WHERE id = ?",
    )
    .bind(display_name)
    .bind(sort_name)
    .bind(updated_at)
    .bind(id)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "media_registry",
    })?;
    Ok(())
}

pub async fn delete_registry_entry(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM media_registry WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "media_registry",
        })?;
    Ok(())
}

pub async fn insert_external_id(
    pool: &SqlitePool,
    ext: &RegistryExternalId,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO registry_external_ids (registry_id, provider, external_id)
         VALUES (?, ?, ?)",
    )
    .bind(&ext.registry_id)
    .bind(&ext.provider)
    .bind(&ext.external_id)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "registry_external_ids",
    })?;
    Ok(())
}

pub async fn list_external_ids_for_entry(
    pool: &SqlitePool,
    registry_id: &[u8],
) -> Result<Vec<RegistryExternalId>, DbError> {
    sqlx::query_as::<_, RegistryExternalId>(
        "SELECT registry_id, provider, external_id
         FROM registry_external_ids WHERE registry_id = ?",
    )
    .bind(registry_id)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "registry_external_ids",
    })
}

pub async fn find_by_external_id(
    pool: &SqlitePool,
    provider: &str,
    external_id: &str,
) -> Result<Option<RegistryEntry>, DbError> {
    sqlx::query_as::<_, RegistryEntry>(
        "SELECT r.id, r.entity_type, r.display_name, r.sort_name, r.created_at, r.updated_at
         FROM media_registry r
         JOIN registry_external_ids e ON e.registry_id = r.id
         WHERE e.provider = ? AND e.external_id = ?",
    )
    .bind(provider)
    .bind(external_id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "registry_external_ids",
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
    async fn registry_round_trip() {
        let pool = setup().await;
        let id = make_id();
        let now = "2026-01-01T00:00:00Z".to_string();
        let entry = RegistryEntry {
            id: id.clone(),
            entity_type: "person".to_string(),
            display_name: "Frank Herbert".to_string(),
            sort_name: Some("Herbert, Frank".to_string()),
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        insert_registry_entry(&pool, &entry).await.unwrap();
        let fetched = get_registry_entry(&pool, &id).await.unwrap().unwrap();
        assert_eq!(fetched.display_name, "Frank Herbert");
        assert_eq!(fetched.sort_name, Some("Herbert, Frank".to_string()));
    }

    #[tokio::test]
    async fn list_empty_returns_empty() {
        let pool = setup().await;
        let results = list_registry_entries(&pool, 10, 0).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn external_id_round_trip() {
        let pool = setup().await;
        let reg_id = make_id();
        let now = "2026-01-01T00:00:00Z".to_string();
        let entry = RegistryEntry {
            id: reg_id.clone(),
            entity_type: "person".to_string(),
            display_name: "Test Person".to_string(),
            sort_name: None,
            created_at: now.clone(),
            updated_at: now,
        };
        insert_registry_entry(&pool, &entry).await.unwrap();
        let ext = RegistryExternalId {
            registry_id: reg_id.clone(),
            provider: "musicbrainz".to_string(),
            external_id: "mb-uuid-123".to_string(),
        };
        insert_external_id(&pool, &ext).await.unwrap();
        let found = find_by_external_id(&pool, "musicbrainz", "mb-uuid-123")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.display_name, "Test Person");
    }
}
