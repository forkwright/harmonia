use snafu::ResultExt;
use sqlx::SqlitePool;

use crate::error::{DbError, QuerySnafu};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MusicReleaseGroup {
    pub id: Vec<u8>,
    pub registry_id: Option<Vec<u8>>,
    pub title: String,
    pub rg_type: String,
    pub mb_release_group_id: Option<String>,
    pub year: Option<i64>,
    pub quality_profile_id: Option<i64>,
    pub added_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MusicRelease {
    pub id: Vec<u8>,
    pub release_group_id: Vec<u8>,
    pub title: String,
    pub release_date: Option<String>,
    pub country: Option<String>,
    pub label: Option<String>,
    pub catalog_number: Option<String>,
    pub mb_release_id: Option<String>,
    pub added_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MusicMedium {
    pub id: Vec<u8>,
    pub release_id: Vec<u8>,
    pub position: i64,
    pub format: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MusicTrack {
    pub id: Vec<u8>,
    pub medium_id: Vec<u8>,
    pub position: i64,
    pub title: String,
    pub duration_ms: Option<i64>,
    pub mb_recording_id: Option<String>,
    pub acoustid_fingerprint: Option<String>,
    pub acoustid_id: Option<String>,
    pub file_path: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub bit_depth: Option<i64>,
    pub sample_rate: Option<i64>,
    pub codec: Option<String>,
    pub quality_score: Option<i64>,
    pub replay_gain_track_db: Option<f64>,
    pub replay_gain_album_db: Option<f64>,
    pub source_type: String,
    pub added_at: String,
}

// --- release groups ---

pub async fn insert_release_group(
    pool: &SqlitePool,
    group: &MusicReleaseGroup,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO music_release_groups
         (id, registry_id, title, rg_type, mb_release_group_id, year, quality_profile_id, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&group.id)
    .bind(&group.registry_id)
    .bind(&group.title)
    .bind(&group.rg_type)
    .bind(&group.mb_release_group_id)
    .bind(group.year)
    .bind(group.quality_profile_id)
    .bind(&group.added_at)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "music_release_groups",
    })?;
    Ok(())
}

pub async fn get_release_group(
    pool: &SqlitePool,
    id: &[u8],
) -> Result<Option<MusicReleaseGroup>, DbError> {
    sqlx::query_as::<_, MusicReleaseGroup>(
        "SELECT id, registry_id, title, rg_type, mb_release_group_id, year,
                quality_profile_id, added_at
         FROM music_release_groups WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "music_release_groups",
    })
}

pub async fn list_release_groups(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<MusicReleaseGroup>, DbError> {
    sqlx::query_as::<_, MusicReleaseGroup>(
        "SELECT id, registry_id, title, rg_type, mb_release_group_id, year,
                quality_profile_id, added_at
         FROM music_release_groups ORDER BY title LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "music_release_groups",
    })
}

pub async fn update_release_group(
    pool: &SqlitePool,
    id: &[u8],
    title: &str,
    rg_type: &str,
    quality_profile_id: Option<i64>,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE music_release_groups SET title = ?, rg_type = ?, quality_profile_id = ?
         WHERE id = ?",
    )
    .bind(title)
    .bind(rg_type)
    .bind(quality_profile_id)
    .bind(id)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "music_release_groups",
    })?;
    Ok(())
}

pub async fn delete_release_group(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM music_release_groups WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "music_release_groups",
        })?;
    Ok(())
}

// --- releases ---

pub async fn insert_release(pool: &SqlitePool, release: &MusicRelease) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO music_releases
         (id, release_group_id, title, release_date, country, label, catalog_number, mb_release_id, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&release.id)
    .bind(&release.release_group_id)
    .bind(&release.title)
    .bind(&release.release_date)
    .bind(&release.country)
    .bind(&release.label)
    .bind(&release.catalog_number)
    .bind(&release.mb_release_id)
    .bind(&release.added_at)
    .execute(pool)
    .await
    .context(QuerySnafu { table: "music_releases" })?;
    Ok(())
}

pub async fn get_release(pool: &SqlitePool, id: &[u8]) -> Result<Option<MusicRelease>, DbError> {
    sqlx::query_as::<_, MusicRelease>(
        "SELECT id, release_group_id, title, release_date, country, label,
                catalog_number, mb_release_id, added_at
         FROM music_releases WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "music_releases",
    })
}

pub async fn list_releases(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<MusicRelease>, DbError> {
    sqlx::query_as::<_, MusicRelease>(
        "SELECT id, release_group_id, title, release_date, country, label,
                catalog_number, mb_release_id, added_at
         FROM music_releases ORDER BY title LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "music_releases",
    })
}

pub async fn update_release(
    pool: &SqlitePool,
    id: &[u8],
    title: &str,
    release_date: Option<&str>,
    label: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE music_releases SET title = ?, release_date = ?, label = ? WHERE id = ?")
        .bind(title)
        .bind(release_date)
        .bind(label)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "music_releases",
        })?;
    Ok(())
}

pub async fn delete_release(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM music_releases WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "music_releases",
        })?;
    Ok(())
}

// --- media ---

pub async fn insert_medium(pool: &SqlitePool, medium: &MusicMedium) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO music_media (id, release_id, position, format, title)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&medium.id)
    .bind(&medium.release_id)
    .bind(medium.position)
    .bind(&medium.format)
    .bind(&medium.title)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "music_media",
    })?;
    Ok(())
}

pub async fn get_medium(pool: &SqlitePool, id: &[u8]) -> Result<Option<MusicMedium>, DbError> {
    sqlx::query_as::<_, MusicMedium>(
        "SELECT id, release_id, position, format, title FROM music_media WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "music_media",
    })
}

pub async fn list_media_for_release(
    pool: &SqlitePool,
    release_id: &[u8],
) -> Result<Vec<MusicMedium>, DbError> {
    sqlx::query_as::<_, MusicMedium>(
        "SELECT id, release_id, position, format, title
         FROM music_media WHERE release_id = ? ORDER BY position",
    )
    .bind(release_id)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "music_media",
    })
}

pub async fn update_medium(
    pool: &SqlitePool,
    id: &[u8],
    format: &str,
    title: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE music_media SET format = ?, title = ? WHERE id = ?")
        .bind(format)
        .bind(title)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "music_media",
        })?;
    Ok(())
}

pub async fn delete_medium(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM music_media WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "music_media",
        })?;
    Ok(())
}

// --- tracks ---

pub async fn insert_track(pool: &SqlitePool, track: &MusicTrack) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO music_tracks
         (id, medium_id, position, title, duration_ms, mb_recording_id,
          acoustid_fingerprint, acoustid_id, file_path, file_size_bytes,
          bit_depth, sample_rate, codec, quality_score,
          replay_gain_track_db, replay_gain_album_db, source_type, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&track.id)
    .bind(&track.medium_id)
    .bind(track.position)
    .bind(&track.title)
    .bind(track.duration_ms)
    .bind(&track.mb_recording_id)
    .bind(&track.acoustid_fingerprint)
    .bind(&track.acoustid_id)
    .bind(&track.file_path)
    .bind(track.file_size_bytes)
    .bind(track.bit_depth)
    .bind(track.sample_rate)
    .bind(&track.codec)
    .bind(track.quality_score)
    .bind(track.replay_gain_track_db)
    .bind(track.replay_gain_album_db)
    .bind(&track.source_type)
    .bind(&track.added_at)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "music_tracks",
    })?;
    Ok(())
}

pub async fn get_track(pool: &SqlitePool, id: &[u8]) -> Result<Option<MusicTrack>, DbError> {
    sqlx::query_as::<_, MusicTrack>(
        "SELECT id, medium_id, position, title, duration_ms, mb_recording_id,
                acoustid_fingerprint, acoustid_id, file_path, file_size_bytes,
                bit_depth, sample_rate, codec, quality_score,
                replay_gain_track_db, replay_gain_album_db, source_type, added_at
         FROM music_tracks WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "music_tracks",
    })
}

pub async fn list_tracks_for_medium(
    pool: &SqlitePool,
    medium_id: &[u8],
) -> Result<Vec<MusicTrack>, DbError> {
    sqlx::query_as::<_, MusicTrack>(
        "SELECT id, medium_id, position, title, duration_ms, mb_recording_id,
                acoustid_fingerprint, acoustid_id, file_path, file_size_bytes,
                bit_depth, sample_rate, codec, quality_score,
                replay_gain_track_db, replay_gain_album_db, source_type, added_at
         FROM music_tracks WHERE medium_id = ? ORDER BY position",
    )
    .bind(medium_id)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "music_tracks",
    })
}

pub async fn update_track(
    pool: &SqlitePool,
    id: &[u8],
    title: &str,
    quality_score: Option<i64>,
    file_path: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE music_tracks SET title = ?, quality_score = ?, file_path = ? WHERE id = ?")
        .bind(title)
        .bind(quality_score)
        .bind(file_path)
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "music_tracks",
        })?;
    Ok(())
}

pub async fn delete_track(pool: &SqlitePool, id: &[u8]) -> Result<(), DbError> {
    sqlx::query("DELETE FROM music_tracks WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "music_tracks",
        })?;
    Ok(())
}

// --- hierarchy queries ---

pub async fn get_release_group_with_releases(
    pool: &SqlitePool,
    group_id: &[u8],
) -> Result<(Option<MusicReleaseGroup>, Vec<MusicRelease>), DbError> {
    let group = get_release_group(pool, group_id).await?;
    let releases = sqlx::query_as::<_, MusicRelease>(
        "SELECT id, release_group_id, title, release_date, country, label,
                catalog_number, mb_release_id, added_at
         FROM music_releases WHERE release_group_id = ? ORDER BY release_date",
    )
    .bind(group_id)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "music_releases",
    })?;
    Ok((group, releases))
}

pub async fn list_tracks_by_release_group(
    pool: &SqlitePool,
    group_id: &[u8],
) -> Result<Vec<MusicTrack>, DbError> {
    sqlx::query_as::<_, MusicTrack>(
        "SELECT t.id, t.medium_id, t.position, t.title, t.duration_ms, t.mb_recording_id,
                t.acoustid_fingerprint, t.acoustid_id, t.file_path, t.file_size_bytes,
                t.bit_depth, t.sample_rate, t.codec, t.quality_score,
                t.replay_gain_track_db, t.replay_gain_album_db, t.source_type, t.added_at
         FROM music_tracks t
         JOIN music_media mm ON mm.id = t.medium_id
         JOIN music_releases mr ON mr.id = mm.release_id
         WHERE mr.release_group_id = ?
         ORDER BY mr.release_date, mm.position, t.position",
    )
    .bind(group_id)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "music_tracks",
    })
}

pub async fn search_tracks(
    pool: &SqlitePool,
    query: &str,
    limit: i64,
) -> Result<Vec<MusicTrack>, DbError> {
    let pattern = format!("%{query}%");
    sqlx::query_as::<_, MusicTrack>(
        "SELECT id, medium_id, position, title, duration_ms, mb_recording_id,
                acoustid_fingerprint, acoustid_id, file_path, file_size_bytes,
                bit_depth, sample_rate, codec, quality_score,
                replay_gain_track_db, replay_gain_album_db, source_type, added_at
         FROM music_tracks WHERE title LIKE ? LIMIT ?",
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "music_tracks",
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

    fn now() -> String {
        "2026-01-01T00:00:00Z".to_string()
    }

    #[tokio::test]
    async fn release_group_round_trip() {
        let pool = setup().await;
        let id = make_id();
        let group = MusicReleaseGroup {
            id: id.clone(),
            registry_id: None,
            title: "Led Zeppelin IV".to_string(),
            rg_type: "album".to_string(),
            mb_release_group_id: None,
            year: Some(1971),
            quality_profile_id: None,
            added_at: now(),
        };
        insert_release_group(&pool, &group).await.unwrap();
        let fetched = get_release_group(&pool, &id).await.unwrap().unwrap();
        assert_eq!(fetched.title, "Led Zeppelin IV");
        assert_eq!(fetched.year, Some(1971));
    }

    #[tokio::test]
    async fn four_level_hierarchy_round_trip() {
        let pool = setup().await;

        let group_id = make_id();
        let group = MusicReleaseGroup {
            id: group_id.clone(),
            registry_id: None,
            title: "Test Album".to_string(),
            rg_type: "album".to_string(),
            mb_release_group_id: None,
            year: Some(2024),
            quality_profile_id: None,
            added_at: now(),
        };
        insert_release_group(&pool, &group).await.unwrap();

        let release_id = make_id();
        let release = MusicRelease {
            id: release_id.clone(),
            release_group_id: group_id.clone(),
            title: "Test Album (US Edition)".to_string(),
            release_date: Some("2024-01-01".to_string()),
            country: Some("US".to_string()),
            label: None,
            catalog_number: None,
            mb_release_id: None,
            added_at: now(),
        };
        insert_release(&pool, &release).await.unwrap();

        let medium_id = make_id();
        let medium = MusicMedium {
            id: medium_id.clone(),
            release_id: release_id.clone(),
            position: 1,
            format: "Digital".to_string(),
            title: None,
        };
        insert_medium(&pool, &medium).await.unwrap();

        let track_id = make_id();
        let track = MusicTrack {
            id: track_id.clone(),
            medium_id: medium_id.clone(),
            position: 1,
            title: "Track One".to_string(),
            duration_ms: Some(240000),
            mb_recording_id: None,
            acoustid_fingerprint: None,
            acoustid_id: None,
            file_path: None,
            file_size_bytes: None,
            bit_depth: None,
            sample_rate: None,
            codec: None,
            quality_score: None,
            replay_gain_track_db: None,
            replay_gain_album_db: None,
            source_type: "local".to_string(),
            added_at: now(),
        };
        insert_track(&pool, &track).await.unwrap();

        let (fetched_group, releases) = get_release_group_with_releases(&pool, &group_id)
            .await
            .unwrap();
        assert!(fetched_group.is_some());
        assert_eq!(releases.len(), 1);

        let media = list_media_for_release(&pool, &release_id).await.unwrap();
        assert_eq!(media.len(), 1);

        let tracks = list_tracks_for_medium(&pool, &medium_id).await.unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].title, "Track One");

        let flat = list_tracks_by_release_group(&pool, &group_id)
            .await
            .unwrap();
        assert_eq!(flat.len(), 1);
    }

    #[tokio::test]
    async fn list_empty_returns_empty() {
        let pool = setup().await;
        let results = list_release_groups(&pool, 10, 0).await.unwrap();
        assert!(results.is_empty());
    }
}
