use sqlx::SqlitePool;

use crate::error::{DbError, QuerySnafu};
use snafu::ResultExt;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Want {
    pub id: Vec<u8>,
    pub media_type: String,
    pub title: String,
    pub registry_id: Option<Vec<u8>>,
    pub quality_profile_id: i64,
    pub status: String,
    pub source: Option<String>,
    pub source_ref: Option<String>,
    pub added_at: String,
    pub fulfilled_at: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Release {
    pub id: Vec<u8>,
    pub want_id: Vec<u8>,
    pub indexer_id: i64,
    pub title: String,
    pub size_bytes: i64,
    pub quality_score: i64,
    pub custom_format_score: i64,
    pub download_url: String,
    pub protocol: String,
    pub info_hash: Option<String>,
    pub found_at: String,
    pub grabbed_at: Option<String>,
    pub rejected_reason: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Have {
    pub id: Vec<u8>,
    pub want_id: Vec<u8>,
    pub release_id: Option<Vec<u8>>,
    pub media_type: String,
    pub media_type_id: Vec<u8>,
    pub quality_score: i64,
    pub file_path: String,
    pub file_size_bytes: i64,
    pub status: String,
    pub imported_at: String,
    pub upgraded_from_id: Option<Vec<u8>>,
}

// --- wants ---

pub async fn insert_want(pool: &SqlitePool, want: &Want) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO wants
         (id, media_type, title, registry_id, quality_profile_id, status,
          source, source_ref, added_at, fulfilled_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&want.id)
    .bind(&want.media_type)
    .bind(&want.title)
    .bind(&want.registry_id)
    .bind(want.quality_profile_id)
    .bind(&want.status)
    .bind(&want.source)
    .bind(&want.source_ref)
    .bind(&want.added_at)
    .bind(&want.fulfilled_at)
    .execute(pool)
    .await
    .context(QuerySnafu { table: "wants" })?;
    Ok(())
}

pub async fn get_want(pool: &SqlitePool, id: &[u8]) -> Result<Option<Want>, DbError> {
    sqlx::query_as::<_, Want>(
        "SELECT id, media_type, title, registry_id, quality_profile_id, status,
                source, source_ref, added_at, fulfilled_at
         FROM wants WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu { table: "wants" })
}

pub async fn list_wants(pool: &SqlitePool, limit: i64, offset: i64) -> Result<Vec<Want>, DbError> {
    sqlx::query_as::<_, Want>(
        "SELECT id, media_type, title, registry_id, quality_profile_id, status,
                source, source_ref, added_at, fulfilled_at
         FROM wants ORDER BY added_at DESC LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu { table: "wants" })
}

pub async fn list_wants_by_type_and_status(
    pool: &SqlitePool,
    media_type: &str,
    status: &str,
) -> Result<Vec<Want>, DbError> {
    sqlx::query_as::<_, Want>(
        "SELECT id, media_type, title, registry_id, quality_profile_id, status,
                source, source_ref, added_at, fulfilled_at
         FROM wants WHERE media_type = ? AND status = ? ORDER BY added_at DESC",
    )
    .bind(media_type)
    .bind(status)
    .fetch_all(pool)
    .await
    .context(QuerySnafu { table: "wants" })
}

pub async fn update_want_status(
    pool: &SqlitePool,
    id: &[u8],
    status: &str,
    fulfilled_at: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE wants SET status = ?, fulfilled_at = ? WHERE id = ?")
        .bind(status)
        .bind(fulfilled_at)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "wants" })?;
    Ok(())
}

pub async fn delete_want(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM wants WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "wants" })?;
    Ok(())
}

// --- releases ---

pub async fn insert_release(pool: &SqlitePool, release: &Release) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO releases
         (id, want_id, indexer_id, title, size_bytes, quality_score,
          custom_format_score, download_url, protocol, info_hash,
          found_at, grabbed_at, rejected_reason)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&release.id)
    .bind(&release.want_id)
    .bind(release.indexer_id)
    .bind(&release.title)
    .bind(release.size_bytes)
    .bind(release.quality_score)
    .bind(release.custom_format_score)
    .bind(&release.download_url)
    .bind(&release.protocol)
    .bind(&release.info_hash)
    .bind(&release.found_at)
    .bind(&release.grabbed_at)
    .bind(&release.rejected_reason)
    .execute(pool)
    .await
    .context(QuerySnafu { table: "releases" })?;
    Ok(())
}

pub async fn get_release(pool: &SqlitePool, id: &[u8]) -> Result<Option<Release>, DbError> {
    sqlx::query_as::<_, Release>(
        "SELECT id, want_id, indexer_id, title, size_bytes, quality_score,
                custom_format_score, download_url, protocol, info_hash,
                found_at, grabbed_at, rejected_reason
         FROM releases WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu { table: "releases" })
}

pub async fn list_releases_for_want(
    pool: &SqlitePool,
    want_id: &[u8],
) -> Result<Vec<Release>, DbError> {
    sqlx::query_as::<_, Release>(
        "SELECT id, want_id, indexer_id, title, size_bytes, quality_score,
                custom_format_score, download_url, protocol, info_hash,
                found_at, grabbed_at, rejected_reason
         FROM releases WHERE want_id = ? ORDER BY quality_score DESC",
    )
    .bind(want_id)
    .fetch_all(pool)
    .await
    .context(QuerySnafu { table: "releases" })
}

pub async fn update_release(
    pool: &SqlitePool,
    id: &[u8],
    grabbed_at: Option<&str>,
    rejected_reason: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE releases SET grabbed_at = ?, rejected_reason = ? WHERE id = ?")
        .bind(grabbed_at)
        .bind(rejected_reason)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "releases" })?;
    Ok(())
}

pub async fn delete_release(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM releases WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "releases" })?;
    Ok(())
}

// --- haves ---

pub async fn insert_have(pool: &SqlitePool, have: &Have) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO haves
         (id, want_id, release_id, media_type, media_type_id, quality_score,
          file_path, file_size_bytes, status, imported_at, upgraded_from_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&have.id)
    .bind(&have.want_id)
    .bind(&have.release_id)
    .bind(&have.media_type)
    .bind(&have.media_type_id)
    .bind(have.quality_score)
    .bind(&have.file_path)
    .bind(have.file_size_bytes)
    .bind(&have.status)
    .bind(&have.imported_at)
    .bind(&have.upgraded_from_id)
    .execute(pool)
    .await
    .context(QuerySnafu { table: "haves" })?;
    Ok(())
}

pub async fn get_have(pool: &SqlitePool, id: &[u8]) -> Result<Option<Have>, DbError> {
    sqlx::query_as::<_, Have>(
        "SELECT id, want_id, release_id, media_type, media_type_id, quality_score,
                file_path, file_size_bytes, status, imported_at, upgraded_from_id
         FROM haves WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu { table: "haves" })
}

pub async fn list_haves_for_want(pool: &SqlitePool, want_id: &[u8]) -> Result<Vec<Have>, DbError> {
    sqlx::query_as::<_, Have>(
        "SELECT id, want_id, release_id, media_type, media_type_id, quality_score,
                file_path, file_size_bytes, status, imported_at, upgraded_from_id
         FROM haves WHERE want_id = ? ORDER BY quality_score DESC",
    )
    .bind(want_id)
    .fetch_all(pool)
    .await
    .context(QuerySnafu { table: "haves" })
}

pub async fn update_have_status(pool: &SqlitePool, id: &[u8], status: &str) -> Result<(), DbError> {
    sqlx::query("UPDATE haves SET status = ? WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "haves" })?;
    Ok(())
}

pub async fn delete_have(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM haves WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu { table: "haves" })?;
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

    async fn insert_test_profile(pool: &SqlitePool) -> i64 {
        use sqlx::Row;
        let row = sqlx::query("SELECT id FROM quality_profiles WHERE media_type = 'music' LIMIT 1")
            .fetch_one(pool)
            .await
            .unwrap();
        row.try_get::<i64, _>("id").unwrap()
    }

    #[tokio::test]
    async fn want_lifecycle() {
        let pool = setup().await;
        let profile_id = insert_test_profile(&pool).await;

        let want_id = make_id();
        let want = Want {
            id: want_id.clone(),
            media_type: "music_album".to_string(),
            title: "Led Zeppelin IV".to_string(),
            registry_id: None,
            quality_profile_id: profile_id,
            status: "searching".to_string(),
            source: Some("manual".to_string()),
            source_ref: None,
            added_at: now(),
            fulfilled_at: None,
        };
        insert_want(&pool, &want).await.unwrap();

        let release_id = make_id();
        let release = Release {
            id: release_id.clone(),
            want_id: want_id.clone(),
            indexer_id: 1,
            title: "Led Zeppelin IV FLAC".to_string(),
            size_bytes: 500_000_000,
            quality_score: 90,
            custom_format_score: 0,
            download_url: "https://example.com/release.torrent".to_string(),
            protocol: "torrent".to_string(),
            info_hash: Some("abc123".to_string()),
            found_at: now(),
            grabbed_at: None,
            rejected_reason: None,
        };
        insert_release(&pool, &release).await.unwrap();

        let media_id = make_id();
        let have_id = make_id();
        let have = Have {
            id: have_id.clone(),
            want_id: want_id.clone(),
            release_id: Some(release_id.clone()),
            media_type: "music_album".to_string(),
            media_type_id: media_id.clone(),
            quality_score: 90,
            file_path: "/music/Led Zeppelin IV/".to_string(),
            file_size_bytes: 490_000_000,
            status: "complete".to_string(),
            imported_at: now(),
            upgraded_from_id: None,
        };
        insert_have(&pool, &have).await.unwrap();

        update_want_status(&pool, &want_id, "fulfilled", Some(&now()))
            .await
            .unwrap();

        let fetched_want = get_want(&pool, &want_id).await.unwrap().unwrap();
        assert_eq!(fetched_want.status, "fulfilled");
        assert!(fetched_want.fulfilled_at.is_some());

        let haves = list_haves_for_want(&pool, &want_id).await.unwrap();
        assert_eq!(haves.len(), 1);
        assert_eq!(haves[0].quality_score, 90);

        let releases = list_releases_for_want(&pool, &want_id).await.unwrap();
        assert_eq!(releases.len(), 1);
    }

    #[tokio::test]
    async fn list_wants_empty_returns_empty() {
        let pool = setup().await;
        let results = list_wants(&pool, 10, 0).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn want_transaction_atomicity() {
        let pool = setup().await;
        let profile_id = insert_test_profile(&pool).await;

        let want_id = make_id();
        let media_id = make_id();
        let have_id = make_id();

        let mut tx = pool.begin().await.unwrap();

        sqlx::query(
            "INSERT INTO wants (id, media_type, title, quality_profile_id, status, added_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&want_id)
        .bind("movie")
        .bind("Atomic Test Movie")
        .bind(profile_id)
        .bind("searching")
        .bind(now())
        .execute(&mut *tx)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO haves (id, want_id, media_type, media_type_id, quality_score,
             file_path, file_size_bytes, status, imported_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&have_id)
        .bind(&want_id)
        .bind("movie")
        .bind(&media_id)
        .bind(70i64)
        .bind("/movies/atomic.mkv")
        .bind(10_000_000_000i64)
        .bind("complete")
        .bind(now())
        .execute(&mut *tx)
        .await
        .unwrap();

        tx.commit().await.unwrap();

        let fetched = get_want(&pool, &want_id).await.unwrap().unwrap();
        assert_eq!(fetched.title, "Atomic Test Movie");

        let haves = list_haves_for_want(&pool, &want_id).await.unwrap();
        assert_eq!(haves.len(), 1);
    }
}
