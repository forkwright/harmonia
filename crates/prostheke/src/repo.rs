//! Database operations for the `subtitles` table.

use apotheke::error::QuerySnafu as DbQuerySnafu;
use snafu::ResultExt;
use sqlx::SqlitePool;
use themelion::MediaId;
use uuid::Uuid;

use crate::error::{DatabaseSnafu, ProsthekeError};
use crate::types::{SubtitleFormat, SubtitleTrack};

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
    fn into_domain(self) -> Option<SubtitleTrack> {
        let id = Uuid::from_slice(&self.id).ok()?;
        let media_id_uuid = Uuid::from_slice(&self.media_id).ok()?;
        let format = parse_format(&self.format)?;
        let acquired_at = self.acquired_at.parse::<jiff::Timestamp>().ok()?;

        Some(SubtitleTrack {
            id,
            media_id: MediaId::from_uuid(media_id_uuid),
            language: self.language,
            format,
            file_path: self.file_path.into(),
            provider: self.provider,
            provider_id: self.provider_id,
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
    .bind(&track.provider_id)
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

    Ok(rows
        .into_iter()
        .filter_map(SubtitleRow::into_domain)
        .collect())
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

    // Fetch all subtitle rows and filter in Rust to avoid dynamic SQL.
    let rows = sqlx::query_as::<_, SubtitleRow>(
        "SELECT id, media_id, language, format, file_path, provider, provider_id,
                hearing_impaired, forced, score, acquired_at
         FROM subtitles",
    )
    .fetch_all(pool)
    .await
    .context(DbQuerySnafu { table: "subtitles" })
    .context(DatabaseSnafu)?;

    // Group acquired languages by media_id.
    let mut by_media: std::collections::HashMap<Vec<u8>, std::collections::HashSet<String>> =
        std::collections::HashMap::new();

    for row in &rows {
        by_media
            .entry(row.media_id.clone())
            .or_default()
            .insert(row.language.clone());
    }

    // Return media_ids missing at least one requested language.
    let mut missing: Vec<MediaId> = Vec::new();
    for (raw_id, acquired_langs) in by_media {
        let has_all = languages.iter().all(|l| acquired_langs.contains(l));
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
            provider_id: "12345".to_string(),
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
            provider_id: "99999".to_string(),
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
}
