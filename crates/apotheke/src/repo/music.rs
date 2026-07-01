use snafu::ResultExt;
use sqlx::SqlitePool;

use crate::error::{DbError, QuerySnafu};

// WHY: wire DTO — SQLx row from the music_release_groups table.
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

// WHY: wire DTO — SQLx row from the music_releases table.
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

// WHY: wire DTO — SQLx row from the music_media table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MusicMedium {
    pub id: Vec<u8>,
    pub release_id: Vec<u8>,
    pub position: i64,
    pub format: String,
    pub title: Option<String>,
}

// WHY: wire DTO — SQLx row from the music_tracks table.
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

// --- track artists ---

pub async fn insert_track_artist(
    pool: &SqlitePool,
    track_id: &[u8],
    artist_id: &[u8],
    role: &str,
) -> Result<(), DbError> {
    sqlx::query("INSERT INTO music_track_artists (track_id, artist_id, role) VALUES (?, ?, ?)")
        .bind(track_id)
        .bind(artist_id)
        .bind(role)
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "music_track_artists",
        })?;
    Ok(())
}

pub async fn insert_release_group_artist(
    pool: &SqlitePool,
    release_group_id: &[u8],
    artist_id: &[u8],
    role: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO music_release_group_artists (release_group_id, artist_id, role)
         VALUES (?, ?, ?)",
    )
    .bind(release_group_id)
    .bind(artist_id)
    .bind(role)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "music_release_group_artists",
    })?;
    Ok(())
}

// WHY: wire DTO — scrobble metadata joined across tracks, releases, and artists.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TrackScrobbleMetadata {
    pub track_title: String,
    pub album_title: String,
    pub artist_name: Option<String>,
}

/// Resolves the metadata a scrobbler needs for a track: title, album title,
/// and primary artist display name.
///
/// The artist is the track-level primary artist, falling back to the
/// release-group-level primary artist; `artist_name` is `None` when neither
/// link exists.
pub async fn get_track_scrobble_metadata(
    pool: &SqlitePool,
    track_id: &[u8],
) -> Result<Option<TrackScrobbleMetadata>, DbError> {
    sqlx::query_as::<_, TrackScrobbleMetadata>(
        "SELECT t.title AS track_title,
                mr.title AS album_title,
                COALESCE(
                    (SELECT reg.display_name
                     FROM music_track_artists mta
                     JOIN media_registry reg ON reg.id = mta.artist_id
                     WHERE mta.track_id = t.id AND mta.role = 'primary'
                     LIMIT 1),
                    (SELECT reg.display_name
                     FROM music_release_group_artists rga
                     JOIN media_registry reg ON reg.id = rga.artist_id
                     WHERE rga.release_group_id = mr.release_group_id
                       AND rga.role = 'primary'
                     LIMIT 1)
                ) AS artist_name
         FROM music_tracks t
         JOIN music_media mm ON mm.id = t.medium_id
         JOIN music_releases mr ON mr.id = mm.release_id
         WHERE t.id = ?",
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "music_tracks",
    })
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

    async fn seed_track_chain(pool: &SqlitePool) -> (Vec<u8>, Vec<u8>) {
        let group_id = make_id();
        insert_release_group(
            pool,
            &MusicReleaseGroup {
                id: group_id.clone(),
                registry_id: None,
                title: "Music Has the Right to Children".to_string(),
                rg_type: "album".to_string(),
                mb_release_group_id: None,
                year: Some(1998),
                quality_profile_id: None,
                added_at: now(),
            },
        )
        .await
        .unwrap();

        let release_id = make_id();
        insert_release(
            pool,
            &MusicRelease {
                id: release_id.clone(),
                release_group_id: group_id.clone(),
                title: "Music Has the Right to Children".to_string(),
                release_date: None,
                country: None,
                label: None,
                catalog_number: None,
                mb_release_id: None,
                added_at: now(),
            },
        )
        .await
        .unwrap();

        let medium_id = make_id();
        insert_medium(
            pool,
            &MusicMedium {
                id: medium_id.clone(),
                release_id,
                position: 1,
                format: "Digital".to_string(),
                title: None,
            },
        )
        .await
        .unwrap();

        let track_id = make_id();
        insert_track(
            pool,
            &MusicTrack {
                id: track_id.clone(),
                medium_id,
                position: 1,
                title: "Roygbiv".to_string(),
                duration_ms: Some(150_000),
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
            },
        )
        .await
        .unwrap();

        (group_id, track_id)
    }

    async fn seed_artist(pool: &SqlitePool, name: &str) -> Vec<u8> {
        let artist_id = make_id();
        crate::repo::registry::insert_registry_entry(
            pool,
            &crate::repo::registry::RegistryEntry {
                id: artist_id.clone(),
                entity_type: "person".to_string(),
                display_name: name.to_string(),
                sort_name: None,
                created_at: now(),
                updated_at: now(),
            },
        )
        .await
        .unwrap();
        artist_id
    }

    #[tokio::test]
    async fn track_scrobble_metadata_resolves_track_artist() {
        let pool = setup().await;
        let (_, track_id) = seed_track_chain(&pool).await;
        let artist_id = seed_artist(&pool, "Boards of Canada").await;
        insert_track_artist(&pool, &track_id, &artist_id, "primary")
            .await
            .unwrap();

        let meta = get_track_scrobble_metadata(&pool, &track_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(meta.track_title, "Roygbiv");
        assert_eq!(meta.album_title, "Music Has the Right to Children");
        assert_eq!(meta.artist_name.as_deref(), Some("Boards of Canada"));
    }

    #[tokio::test]
    async fn track_scrobble_metadata_falls_back_to_release_group_artist() {
        let pool = setup().await;
        let (group_id, track_id) = seed_track_chain(&pool).await;
        let artist_id = seed_artist(&pool, "Boards of Canada").await;
        insert_release_group_artist(&pool, &group_id, &artist_id, "primary")
            .await
            .unwrap();

        let meta = get_track_scrobble_metadata(&pool, &track_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(meta.artist_name.as_deref(), Some("Boards of Canada"));
    }

    #[tokio::test]
    async fn track_scrobble_metadata_prefers_track_artist_over_group_artist() {
        let pool = setup().await;
        let (group_id, track_id) = seed_track_chain(&pool).await;
        let track_artist = seed_artist(&pool, "Track Artist").await;
        let group_artist = seed_artist(&pool, "Group Artist").await;
        insert_track_artist(&pool, &track_id, &track_artist, "primary")
            .await
            .unwrap();
        insert_release_group_artist(&pool, &group_id, &group_artist, "primary")
            .await
            .unwrap();

        let meta = get_track_scrobble_metadata(&pool, &track_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(meta.artist_name.as_deref(), Some("Track Artist"));
    }

    #[tokio::test]
    async fn track_scrobble_metadata_artist_none_when_unlinked() {
        let pool = setup().await;
        let (_, track_id) = seed_track_chain(&pool).await;

        let meta = get_track_scrobble_metadata(&pool, &track_id)
            .await
            .unwrap()
            .unwrap();
        assert!(meta.artist_name.is_none());
    }

    #[tokio::test]
    async fn track_scrobble_metadata_missing_track_returns_none() {
        let pool = setup().await;
        let meta = get_track_scrobble_metadata(&pool, &make_id())
            .await
            .unwrap();
        assert!(meta.is_none());
    }
}
