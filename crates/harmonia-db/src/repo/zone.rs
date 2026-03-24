/// Zone and renderer management for multi-room synchronized playback.
use snafu::ResultExt;
use sqlx::SqlitePool;

use crate::error::{DbError, NotFoundSnafu, QuerySnafu};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Zone {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ZoneMemberRow {
    pub zone_id: String,
    pub renderer_id: String,
    pub joined_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Renderer {
    pub id: String,
    pub name: String,
    pub address: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ZoneWithMembers {
    pub zone: Zone,
    pub members: Vec<Renderer>,
}

pub async fn create_zone(pool: &SqlitePool, id: &str, name: &str) -> Result<Zone, DbError> {
    sqlx::query("INSERT INTO zones (id, name) VALUES (?, ?)")
        .bind(id)
        .bind(name)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "zones" })?;

    get_zone_row(pool, id).await?.ok_or_else(|| {
        NotFoundSnafu {
            table: "zones",
            id: id.to_string(),
        }
        .build()
    })
}

pub async fn delete_zone(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    let result = sqlx::query("DELETE FROM zones WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "zones" })?;

    if result.rows_affected() == 0 {
        return Err(NotFoundSnafu {
            table: "zones",
            id: id.to_string(),
        }
        .build());
    }
    Ok(())
}

pub async fn add_member(
    pool: &SqlitePool,
    zone_id: &str,
    renderer_id: &str,
) -> Result<(), DbError> {
    sqlx::query("INSERT INTO zone_members (zone_id, renderer_id) VALUES (?, ?)")
        .bind(zone_id)
        .bind(renderer_id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "zone_members",
        })?;
    Ok(())
}

pub async fn remove_member(
    pool: &SqlitePool,
    zone_id: &str,
    renderer_id: &str,
) -> Result<(), DbError> {
    let result = sqlx::query("DELETE FROM zone_members WHERE zone_id = ? AND renderer_id = ?")
        .bind(zone_id)
        .bind(renderer_id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "zone_members",
        })?;

    if result.rows_affected() == 0 {
        return Err(NotFoundSnafu {
            table: "zone_members",
            id: format!("{zone_id}/{renderer_id}"),
        }
        .build());
    }
    Ok(())
}

pub async fn list_zones(pool: &SqlitePool) -> Result<Vec<ZoneWithMembers>, DbError> {
    let zones: Vec<Zone> =
        sqlx::query_as::<_, Zone>("SELECT id, name, created_at FROM zones ORDER BY name")
            .fetch_all(pool)
            .await
            .context(QuerySnafu { table: "zones" })?;

    let mut result = Vec::with_capacity(zones.len());
    for zone in zones {
        let members = members_for_zone(pool, &zone.id).await?;
        result.push(ZoneWithMembers { zone, members });
    }
    Ok(result)
}

pub async fn get_zone(pool: &SqlitePool, id: &str) -> Result<ZoneWithMembers, DbError> {
    let zone = get_zone_row(pool, id).await?.ok_or_else(|| {
        NotFoundSnafu {
            table: "zones",
            id: id.to_string(),
        }
        .build()
    })?;

    let members = members_for_zone(pool, id).await?;
    Ok(ZoneWithMembers { zone, members })
}

pub async fn get_renderer_zone(
    pool: &SqlitePool,
    renderer_id: &str,
) -> Result<Option<Zone>, DbError> {
    sqlx::query_as::<_, Zone>(
        "SELECT z.id, z.name, z.created_at \
         FROM zones z \
         JOIN zone_members zm ON zm.zone_id = z.id \
         WHERE zm.renderer_id = ?",
    )
    .bind(renderer_id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu { table: "zones" })
}

// -- Renderer CRUD --------------------------------------------------------

pub async fn upsert_renderer(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    address: &str,
) -> Result<Renderer, DbError> {
    sqlx::query(
        "INSERT INTO renderers (id, name, address) VALUES (?, ?, ?) \
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, address = excluded.address",
    )
    .bind(id)
    .bind(name)
    .bind(address)
    .execute(pool)
    .await
    .context(QuerySnafu { table: "renderers" })?;

    sqlx::query_as::<_, Renderer>(
        "SELECT id, name, address, created_at FROM renderers WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .context(QuerySnafu { table: "renderers" })
}

pub async fn list_renderers(pool: &SqlitePool) -> Result<Vec<Renderer>, DbError> {
    sqlx::query_as::<_, Renderer>(
        "SELECT id, name, address, created_at FROM renderers ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .context(QuerySnafu { table: "renderers" })
}

// -- helpers --------------------------------------------------------------

async fn get_zone_row(pool: &SqlitePool, id: &str) -> Result<Option<Zone>, DbError> {
    sqlx::query_as::<_, Zone>("SELECT id, name, created_at FROM zones WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context(QuerySnafu { table: "zones" })
}

async fn members_for_zone(pool: &SqlitePool, zone_id: &str) -> Result<Vec<Renderer>, DbError> {
    sqlx::query_as::<_, Renderer>(
        "SELECT r.id, r.name, r.address, r.created_at \
         FROM renderers r \
         JOIN zone_members zm ON zm.renderer_id = r.id \
         WHERE zm.zone_id = ? \
         ORDER BY r.name",
    )
    .bind(zone_id)
    .fetch_all(pool)
    .await
    .context(QuerySnafu { table: "renderers" })
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

    async fn seed_renderer(pool: &SqlitePool, id: &str, name: &str) {
        upsert_renderer(pool, id, name, "127.0.0.1:5000")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn create_and_get_zone() {
        let pool = setup().await;
        let zone = create_zone(&pool, "z1", "Living Room").await.unwrap();
        assert_eq!(zone.name, "Living Room");

        let fetched = get_zone(&pool, "z1").await.unwrap();
        assert_eq!(fetched.zone.name, "Living Room");
        assert!(fetched.members.is_empty());
    }

    #[tokio::test]
    async fn add_remove_members() {
        let pool = setup().await;
        create_zone(&pool, "z1", "Kitchen").await.unwrap();
        seed_renderer(&pool, "r1", "Speaker A").await;
        seed_renderer(&pool, "r2", "Speaker B").await;

        add_member(&pool, "z1", "r1").await.unwrap();
        add_member(&pool, "z1", "r2").await.unwrap();

        let zone = get_zone(&pool, "z1").await.unwrap();
        assert_eq!(zone.members.len(), 2);

        remove_member(&pool, "z1", "r1").await.unwrap();
        let zone = get_zone(&pool, "z1").await.unwrap();
        assert_eq!(zone.members.len(), 1);
        assert_eq!(zone.members[0].id, "r2");
    }

    #[tokio::test]
    async fn delete_zone_cascades_members() {
        let pool = setup().await;
        create_zone(&pool, "z1", "Bedroom").await.unwrap();
        seed_renderer(&pool, "r1", "Speaker").await;
        add_member(&pool, "z1", "r1").await.unwrap();

        delete_zone(&pool, "z1").await.unwrap();

        let zones = list_zones(&pool).await.unwrap();
        assert!(zones.is_empty());

        // Renderer still exists after zone deletion
        let renderers = list_renderers(&pool).await.unwrap();
        assert_eq!(renderers.len(), 1);
    }

    #[tokio::test]
    async fn list_zones_with_members() {
        let pool = setup().await;
        create_zone(&pool, "z1", "A-Zone").await.unwrap();
        create_zone(&pool, "z2", "B-Zone").await.unwrap();
        seed_renderer(&pool, "r1", "Speaker 1").await;
        add_member(&pool, "z1", "r1").await.unwrap();

        let zones = list_zones(&pool).await.unwrap();
        assert_eq!(zones.len(), 2);
        assert_eq!(zones[0].zone.name, "A-Zone");
        assert_eq!(zones[0].members.len(), 1);
        assert_eq!(zones[1].zone.name, "B-Zone");
        assert!(zones[1].members.is_empty());
    }

    #[tokio::test]
    async fn get_renderer_zone_returns_zone() {
        let pool = setup().await;
        create_zone(&pool, "z1", "Office").await.unwrap();
        seed_renderer(&pool, "r1", "Desk Speaker").await;
        add_member(&pool, "z1", "r1").await.unwrap();

        let zone = get_renderer_zone(&pool, "r1").await.unwrap();
        assert!(zone.is_some());
        assert_eq!(zone.unwrap().name, "Office");

        let none = get_renderer_zone(&pool, "r-unknown").await.unwrap();
        assert!(none.is_none());
    }

    #[tokio::test]
    async fn delete_nonexistent_zone_returns_not_found() {
        let pool = setup().await;
        let result = delete_zone(&pool, "nope").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn remove_nonexistent_member_returns_not_found() {
        let pool = setup().await;
        create_zone(&pool, "z1", "Test").await.unwrap();
        let result = remove_member(&pool, "z1", "r-nope").await;
        assert!(result.is_err());
    }
}
