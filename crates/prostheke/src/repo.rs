//! Database operations for the `subtitles` table.

use apotheke::error::QuerySnafu as DbQuerySnafu;
use snafu::ResultExt;
use sqlx::SqlitePool;
use themelion::MediaId;
use uuid::Uuid;

use crate::error::{CorruptSubtitleRowSnafu, DatabaseSnafu, ProsthekeError};
use crate::types::{SubtitleFormat, SubtitleProviderId, SubtitleTrack};

// ── Row type ─────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct SubtitleRow {
    id: Vec<u8>,
    media_id: Vec<u8>,
    language: String,
    format: String,
    file_path: String,
    provider: String,
    provider_id: String,
    hearing_impaired: bool,
    forced: bool,
    score: f64,
    acquired_at: String,
}

impl SubtitleRow {
    // WHY: fallible by design — a corrupt row must decode to a diagnosable
    // error, not silently vanish FROM query results.
    fn into_domain(self) -> Result<SubtitleTrack, ProsthekeError> {
        let id = Uuid::from_slice(&self.id).map_err(|e| {
            CorruptSubtitleRowSnafu {
                detail: format!("invalid id bytes: {e}"),
            }
            .build()
        })?;
        let media_id_uuid = Uuid::from_slice(&self.media_id).map_err(|e| {
            CorruptSubtitleRowSnafu {
                detail: format!("invalid media_id bytes: {e}"),
            }
            .build()
        })?;
        let format = parse_format(&self.format).ok_or_else(|| {
            CorruptSubtitleRowSnafu {
                detail: format!("unknown subtitle format: {}", self.format),
            }
            .build()
        })?;
        let acquired_at = self.acquired_at.parse::<jiff::Timestamp>().map_err(|e| {
            CorruptSubtitleRowSnafu {
                detail: format!("invalid acquired_at timestamp: {e}"),
            }
            .build()
        })?;

        Ok(SubtitleTrack {
            id,
            media_id: MediaId::from_uuid(media_id_uuid),
            language: self.language,
            format,
            file_path: self.file_path.into(),
            provider: self.provider,
            provider_id: SubtitleProviderId(self.provider_id),
            hearing_impaired: self.hearing_impaired,
            forced: self.forced,
            score: self.score,
            acquired_at,
        })
    }
}

fn parse_format(s: &str) -> Option<SubtitleFormat> {
    match s {
        "srt" => Some(SubtitleFormat::Srt),
        "ass" => Some(SubtitleFormat::Ass),
        "sub" => Some(SubtitleFormat::Sub),
        "vtt" => Some(SubtitleFormat::Vtt),
        _ => None,
    }
}

// ── Write operations ──────────────────────────────────────────────────────────

/// Insert a subtitle track record.
///
/// The unique index on `(media_id, language, forced)` prevents duplicates.
/// A conflict returns a database error the caller can inspect.
pub async fn insert_subtitle(
    pool: &SqlitePool,
    track: &SubtitleTrack,
) -> Result<(), ProsthekeError> {
    sqlx::query(
        "INSERT INTO subtitles
         (id, media_id, language, format, file_path, provider, provider_id,
          hearing_impaired, forced, score, acquired_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(track.id.as_bytes().as_slice())
    .bind(track.media_id.as_bytes().as_slice())
    .bind(&track.language)
    .bind(track.format.as_str())
    .bind(track.file_path.to_string_lossy().as_ref())
    .bind(&track.provider)
    .bind(&track.provider_id.0)
    .bind(track.hearing_impaired)
    .bind(track.forced)
    .bind(track.score)
    .bind(track.acquired_at.to_string())
    .execute(pool)
    .await
    .context(DbQuerySnafu { table: "subtitles" })
    .context(DatabaseSnafu)?;
    Ok(())
}

/// Delete a subtitle track by its UUID.
pub async fn delete_subtitle(pool: &SqlitePool, id: &Uuid) -> Result<(), ProsthekeError> {
    sqlx::query("DELETE FROM subtitles WHERE id = ?")
        .bind(id.as_bytes().as_slice())
        .execute(pool)
        .await
        .context(DbQuerySnafu { table: "subtitles" })
        .context(DatabaseSnafu)?;
    Ok(())
}

// ── Read operations ───────────────────────────────────────────────────────────

/// Return all subtitle tracks for a media item.
pub async fn get_subtitles_for_media(
    pool: &SqlitePool,
    media_id: &MediaId,
) -> Result<Vec<SubtitleTrack>, ProsthekeError> {
    let rows = sqlx::query_as::<_, SubtitleRow>(
        "SELECT id, media_id, language, format, file_path, provider, provider_id,
                hearing_impaired, forced, score, acquired_at
         FROM subtitles WHERE media_id = ? ORDER BY language",
    )
    .bind(media_id.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .context(DbQuerySnafu { table: "subtitles" })
    .context(DatabaseSnafu)?;

    let mut tracks = Vec::with_capacity(rows.len());
    for row in rows {
        match row.into_domain() {
            Ok(track) => tracks.push(track),
            // WHY: this is the handled site — one corrupt row is skipped
            // (observable), the rest of the query still succeeds.
            Err(e) => tracing::warn!(error = %e, "skipping corrupt subtitle row"),
        }
    }
    Ok(tracks)
}

/// Return media IDs that have subtitle records but are missing at least one of
/// the requested languages.
///
/// Used for batch re-search operations to identify media that needs subtitle
/// acquisition for additional languages.
pub async fn list_media_missing_subtitles(
    pool: &SqlitePool,
    languages: &[String],
) -> Result<Vec<MediaId>, ProsthekeError> {
    if languages.is_empty() {
        return Ok(vec![]);
    }

    // WHY: group in SQL (one row per media_id, static statement) instead of
    // materializing every subtitle row in memory and grouping in Rust.
    let rows = sqlx::query_as::<_, (Vec<u8>, String)>(
        "SELECT media_id, GROUP_CONCAT(DISTINCT language)
         FROM subtitles GROUP BY media_id",
    )
    .fetch_all(pool)
    .await
    .context(DbQuerySnafu { table: "subtitles" })
    .context(DatabaseSnafu)?;

    // Return media_ids missing at least one requested language.
    let mut missing: Vec<MediaId> = Vec::new();
    for (raw_id, langs) in rows {
        let acquired: std::collections::HashSet<&str> = langs.split(',').collect();
        let has_all = languages.iter().all(|l| acquired.contains(l.as_str()));
        if !has_all && let Ok(uuid) = Uuid::from_slice(&raw_id) {
            missing.push(MediaId::from_uuid(uuid));
        }
    }

    Ok(missing)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use apotheke::migrate::MIGRATOR;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    use super::*;

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    fn make_track(media_id: MediaId, language: &str, forced: bool) -> SubtitleTrack {
        SubtitleTrack {
            id: Uuid::now_v7(),
            media_id,
            language: language.to_string(),
            format: SubtitleFormat::Srt,
            file_path: format!("/library/movie.{language}.srt").into(),
            provider: "opensubtitles".to_string(),
            provider_id: SubtitleProviderId("12345".to_string()),
            hearing_impaired: false,
            forced,
            score: 0.95,
            acquired_at: jiff::Timestamp::now(),
        }
    }

    #[tokio::test]
    async fn insert_and_get_subtitles_for_media() {
        let pool = setup().await;
        let media_id = MediaId::new();
        let track = make_track(media_id, "en", false);
        let track_id = track.id;

        insert_subtitle(&pool, &track).await.unwrap();

        let results = get_subtitles_for_media(&pool, &media_id).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, track_id);
        assert_eq!(results[0].language, "en");
    }

    #[tokio::test]
    async fn delete_subtitle_removes_row() {
        let pool = setup().await;
        let media_id = MediaId::new();
        let track = make_track(media_id, "en", false);
        let track_id = track.id;

        insert_subtitle(&pool, &track).await.unwrap();
        delete_subtitle(&pool, &track_id).await.unwrap();

        let results = get_subtitles_for_media(&pool, &media_id).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn get_subtitles_returns_empty_for_unknown_media() {
        let pool = setup().await;
        let media_id = MediaId::new();
        let results = get_subtitles_for_media(&pool, &media_id).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn unique_constraint_prevents_duplicate_language_forced_combo() {
        let pool = setup().await;
        let media_id = MediaId::new();

        let track1 = make_track(media_id, "en", false);
        insert_subtitle(&pool, &track1).await.unwrap();

        // Same (media_id, language, forced) triplet — different id.
        let track2 = SubtitleTrack {
            id: Uuid::now_v7(),
            provider_id: SubtitleProviderId("99999".to_string()),
            ..make_track(media_id, "en", false)
        };
        let result = insert_subtitle(&pool, &track2).await;
        assert!(result.is_err(), "duplicate should fail");
    }

    #[tokio::test]
    async fn forced_and_non_forced_same_language_both_allowed() {
        // (media_id, language, forced=false) and (media_id, language, forced=true) are distinct.
        let pool = setup().await;
        let media_id = MediaId::new();

        let normal = make_track(media_id, "en", false);
        let forced = make_track(media_id, "en", true);

        insert_subtitle(&pool, &normal).await.unwrap();
        insert_subtitle(&pool, &forced).await.unwrap();

        let results = get_subtitles_for_media(&pool, &media_id).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn list_media_missing_subtitles_returns_media_missing_requested_lang() {
        let pool = setup().await;
        let media_en = MediaId::new();
        let media_both = MediaId::new();

        // media_en has only "en".
        insert_subtitle(&pool, &make_track(media_en, "en", false))
            .await
            .unwrap();

        // media_both has "en" and "fr".
        insert_subtitle(&pool, &make_track(media_both, "en", false))
            .await
            .unwrap();
        insert_subtitle(&pool, &make_track(media_both, "fr", false))
            .await
            .unwrap();

        // Request both "en" and "fr"; media_en should appear as missing "fr".
        let missing = list_media_missing_subtitles(&pool, &["en".to_string(), "fr".to_string()])
            .await
            .unwrap();

        assert!(missing.contains(&media_en));
        assert!(!missing.contains(&media_both));
    }

    #[tokio::test]
    async fn list_media_missing_subtitles_empty_languages_returns_empty() {
        let pool = setup().await;
        let result = list_media_missing_subtitles(&pool, &[]).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn list_media_missing_subtitles_large_library() {
        let pool = setup().await;
        let requested = vec!["en".to_string(), "fr".to_string(), "de".to_string()];

        // WHY: 50 complete + 50 incomplete media items exercise the grouped
        // SQL path with many rows per media_id.
        let mut expect_missing: Vec<MediaId> = Vec::new();
        for i in 0..100 {
            let media_id = MediaId::new();
            let langs: &[&str] = if i % 2 == 0 {
                &["en", "fr", "de"]
            } else {
                expect_missing.push(media_id);
                &["en"]
            };
            for lang in langs {
                insert_subtitle(&pool, &make_track(media_id, lang, false))
                    .await
                    .unwrap();
            }
        }

        let missing: std::collections::HashSet<MediaId> =
            list_media_missing_subtitles(&pool, &requested)
                .await
                .unwrap()
                .into_iter()
                .collect();
        let expected: std::collections::HashSet<MediaId> = expect_missing.into_iter().collect();
        assert_eq!(missing, expected);
    }

    fn valid_row() -> SubtitleRow {
        SubtitleRow {
            id: Uuid::now_v7().as_bytes().to_vec(),
            media_id: MediaId::new().as_bytes().to_vec(),
            language: "en".to_string(),
            format: "srt".to_string(),
            file_path: "/library/movie.en.srt".to_string(),
            provider: "opensubtitles".to_string(),
            provider_id: "12345".to_string(),
            hearing_impaired: false,
            forced: false,
            score: 0.95,
            acquired_at: jiff::Timestamp::now().to_string(),
        }
    }

    #[test]
    fn into_domain_success() {
        let row = valid_row();
        let track = row.into_domain().unwrap();
        assert_eq!(track.language, "en");
        assert_eq!(track.format, SubtitleFormat::Srt);
    }

    #[test]
    fn into_domain_bad_id_bytes() {
        let row = SubtitleRow {
            id: vec![1, 2, 3],
            ..valid_row()
        };
        let err = row.into_domain().unwrap_err();
        assert!(matches!(err, ProsthekeError::CorruptSubtitleRow { .. }));
        assert!(err.to_string().contains("invalid id bytes"));
    }

    #[test]
    fn into_domain_bad_media_id_bytes() {
        let row = SubtitleRow {
            media_id: vec![9],
            ..valid_row()
        };
        let err = row.into_domain().unwrap_err();
        assert!(err.to_string().contains("invalid media_id bytes"));
    }

    #[test]
    fn into_domain_bad_format() {
        let row = SubtitleRow {
            format: "docx".to_string(),
            ..valid_row()
        };
        let err = row.into_domain().unwrap_err();
        assert!(err.to_string().contains("unknown subtitle format: docx"));
    }

    #[test]
    fn into_domain_bad_timestamp() {
        let row = SubtitleRow {
            acquired_at: "not-a-timestamp".to_string(),
            ..valid_row()
        };
        let err = row.into_domain().unwrap_err();
        assert!(err.to_string().contains("invalid acquired_at timestamp"));
    }

    #[tokio::test]
    async fn corrupt_row_is_skipped_not_fatal() {
        let pool = setup().await;
        let media_id = MediaId::new();
        insert_subtitle(&pool, &make_track(media_id, "en", false))
            .await
            .unwrap();

        // Corrupt the stored timestamp directly, bypassing the typed insert.
        // NOTE: format has a schema CHECK constraint, so acquired_at is the
        // corruptible column here.
        sqlx::query("UPDATE subtitles SET acquired_at = 'not-a-timestamp' WHERE media_id = ?")
            .bind(media_id.as_bytes().as_slice())
            .execute(&pool)
            .await
            .unwrap();
        insert_subtitle(&pool, &make_track(media_id, "fr", false))
            .await
            .unwrap();

        let results = get_subtitles_for_media(&pool, &media_id).await.unwrap();
        assert_eq!(results.len(), 1, "only the intact row survives");
        assert_eq!(results[0].language, "fr");
    }
}
