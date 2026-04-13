use sqlx::SqlitePool;

use snafu::ResultExt;

use crate::error::{DbError, QuerySnafu};
use themelion::ids::{MediaId, SessionId, UserId};
use themelion::media::MediaType;

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaySource {
    Local,
    Subsonic,
    Stream,
}

impl PlaySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Subsonic => "subsonic",
            Self::Stream => "stream",
        }
    }
}

pub struct NewPlaySession {
    pub media_id: MediaId,
    pub user_id: UserId,
    pub media_type: MediaType,
    pub source: PlaySource,
    pub device_name: Option<String>,
    pub quality_score: Option<i32>,
    pub dsp_active: bool,
    pub total_ms: Option<i64>,
}

pub struct SessionOutcome {
    pub duration_ms: i64,
    pub completed: bool,
    pub percent_heard: Option<i32>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PlaySession {
    pub id: Vec<u8>,
    pub media_id: Vec<u8>,
    pub user_id: Vec<u8>,
    pub media_type: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_ms: i64,
    pub total_ms: Option<i64>,
    pub completed: i64,
    pub percent_heard: Option<i64>,
    pub source: String,
    pub scrobble_eligible: i64,
    pub scrobbled_at: Option<String>,
    pub scrobble_service: Option<String>,
    pub device_name: Option<String>,
    pub quality_score: Option<i64>,
    pub dsp_active: i64,
}

#[derive(Debug, Clone)]
pub struct ItemStats {
    pub media_id: MediaId,
    pub play_count: i32,
    pub total_ms: i64,
    pub skip_count: i32,
    pub last_played_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DailyStats {
    pub date: String,
    pub media_type: MediaType,
    pub sessions: i32,
    pub total_ms: i64,
    pub unique_items: i32,
}

#[derive(Debug, Clone)]
pub struct ListeningTimeSummary {
    pub total_ms: i64,
    pub by_media_type: Vec<(MediaType, i64)>,
    pub session_count: i32,
}

#[derive(Debug, Clone)]
pub struct Streak {
    pub start: String,
    pub end: String,
    pub days: i32,
}

#[derive(Debug, Clone)]
pub struct DateRange {
    pub start: String,
    pub end: String,
}

// ---------------------------------------------------------------------------
// Internal row types for sqlx::FromRow
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct ItemStatsRow {
    media_id: Vec<u8>,
    play_count: i32,
    total_ms: i64,
    skip_count: i32,
    last_played_at: Option<String>,
}

#[derive(sqlx::FromRow)]
struct DailyStatsRow {
    date: String,
    media_type: String,
    sessions: i32,
    total_ms: i64,
    unique_items: i32,
}

#[derive(sqlx::FromRow)]
struct StreakRow {
    streak_start: String,
    streak_end: String,
    days: i32,
}

#[derive(sqlx::FromRow)]
struct MediaTypeAggRow {
    media_type: String,
    total_ms: i64,
    session_count: i32,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn bytes_to_media_id(bytes: Vec<u8>) -> Option<MediaId> {
    let arr: [u8; 16] = bytes.try_into().ok()?;
    Some(MediaId::from_uuid(uuid::Uuid::from_bytes(arr)))
}

fn parse_media_type(s: &str) -> MediaType {
    match s {
        "music" => MediaType::Music,
        "audiobook" => MediaType::Audiobook,
        "book" => MediaType::Book,
        "comic" => MediaType::Comic,
        "podcast" => MediaType::Podcast,
        "news" => MediaType::News,
        "movie" => MediaType::Movie,
        "tv" => MediaType::Tv,
        _ => MediaType::Music,
    }
}

// ---------------------------------------------------------------------------
// Session lifecycle
// ---------------------------------------------------------------------------

pub async fn start_session(
    pool: &SqlitePool,
    session: &NewPlaySession,
) -> Result<SessionId, DbError> {
    let id = SessionId::new();
    sqlx::query(
        "INSERT INTO play_sessions
         (id, media_id, user_id, media_type, started_at, source,
          device_name, quality_score, dsp_active, total_ms)
         VALUES (?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                 ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().as_ref())
    .bind(session.media_id.as_bytes().as_ref())
    .bind(session.user_id.as_bytes().as_ref())
    .bind(session.media_type.to_string())
    .bind(session.source.as_str())
    .bind(&session.device_name)
    .bind(session.quality_score)
    .bind(i64::try_from(session.dsp_active).unwrap_or_default())
    .bind(session.total_ms)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "play_sessions",
    })?;
    Ok(id)
}

pub async fn end_session(
    pool: &SqlitePool,
    id: SessionId,
    outcome: &SessionOutcome,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE play_sessions
         SET ended_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
             duration_ms = ?,
             completed = ?,
             percent_heard = ?
         WHERE id = ?",
    )
    .bind(outcome.duration_ms)
    .bind(i64::try_from(outcome.completed).unwrap_or_default())
    .bind(outcome.percent_heard)
    .bind(id.as_bytes().as_ref())
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "play_sessions",
    })?;
    Ok(())
}

pub async fn get_active_sessions(
    pool: &SqlitePool,
    user_id: UserId,
) -> Result<Vec<PlaySession>, DbError> {
    sqlx::query_as::<_, PlaySession>(
        "SELECT id, media_id, user_id, media_type, started_at, ended_at,
                duration_ms, total_ms, completed, percent_heard, source,
                scrobble_eligible, scrobbled_at, scrobble_service,
                device_name, quality_score, dsp_active
         FROM play_sessions
         WHERE user_id = ? AND ended_at IS NULL
         ORDER BY started_at DESC",
    )
    .bind(user_id.as_bytes().as_ref())
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "play_sessions",
    })
}

// ---------------------------------------------------------------------------
// Scrobble queue
// ---------------------------------------------------------------------------

pub async fn mark_scrobble_eligible(
    pool: &SqlitePool,
    session_id: SessionId,
) -> Result<(), DbError> {
    sqlx::query("UPDATE play_sessions SET scrobble_eligible = 1 WHERE id = ?")
        .bind(session_id.as_bytes().as_ref())
        .execute(pool)
        .await
        .context(QuerySnafu {
            table: "play_sessions",
        })?;
    Ok(())
}

pub async fn get_pending_scrobbles(
    pool: &SqlitePool,
    user_id: UserId,
) -> Result<Vec<PlaySession>, DbError> {
    sqlx::query_as::<_, PlaySession>(
        "SELECT id, media_id, user_id, media_type, started_at, ended_at,
                duration_ms, total_ms, completed, percent_heard, source,
                scrobble_eligible, scrobbled_at, scrobble_service,
                device_name, quality_score, dsp_active
         FROM play_sessions
         WHERE user_id = ? AND scrobble_eligible = 1 AND scrobbled_at IS NULL
         ORDER BY started_at ASC",
    )
    .bind(user_id.as_bytes().as_ref())
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "play_sessions",
    })
}

pub async fn mark_scrobbled(
    pool: &SqlitePool,
    session_id: SessionId,
    service: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE play_sessions
         SET scrobbled_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
             scrobble_service = ?
         WHERE id = ?",
    )
    .bind(service)
    .bind(session_id.as_bytes().as_ref())
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "play_sessions",
    })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Stats UPDATE
// ---------------------------------------------------------------------------

pub async fn update_item_stats(
    pool: &SqlitePool,
    media_id: MediaId,
    user_id: UserId,
    session: &PlaySession,
) -> Result<(), DbError> {
    let skip = session.percent_heard.map(|p| p < 50).unwrap_or(false) as i64;
    sqlx::query(
        "INSERT INTO play_stats_item
             (media_id, user_id, play_count, total_ms, skip_count,
              first_played_at, last_played_at)
         VALUES (?, ?, 1, ?, ?, ?, ?)
         ON CONFLICT(media_id, user_id) DO UPDATE SET
             play_count     = play_count + 1,
             total_ms       = total_ms + excluded.total_ms,
             skip_count     = skip_count + excluded.skip_count,
             first_played_at = COALESCE(first_played_at, excluded.first_played_at),
             last_played_at = excluded.last_played_at",
    )
    .bind(media_id.as_bytes().as_ref())
    .bind(user_id.as_bytes().as_ref())
    .bind(session.duration_ms)
    .bind(skip)
    .bind(&session.started_at)
    .bind(&session.started_at)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "play_stats_item",
    })?;
    Ok(())
}

pub async fn update_daily_stats(
    pool: &SqlitePool,
    user_id: UserId,
    date: &str,
    media_type: MediaType,
    media_id: MediaId,
    duration_ms: i64,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO play_stats_daily
             (user_id, date, media_type, sessions, total_ms, unique_items)
         VALUES (?, ?, ?, 1, ?, 1)
         ON CONFLICT(user_id, date, media_type) DO UPDATE SET
             sessions   = sessions + 1,
             total_ms   = total_ms + excluded.total_ms",
    )
    .bind(user_id.as_bytes().as_ref())
    .bind(date)
    .bind(media_type.to_string())
    .bind(duration_ms)
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "play_stats_daily",
    })?;

    // Recompute unique_items for this (user, date, media_type) bucket.
    sqlx::query(
        "UPDATE play_stats_daily
         SET unique_items = (
             SELECT COUNT(DISTINCT media_id)
             FROM play_sessions
             WHERE user_id = ?
               AND date(started_at) = ?
               AND media_type = ?
         )
         WHERE user_id = ? AND date = ? AND media_type = ?",
    )
    .bind(user_id.as_bytes().as_ref())
    .bind(date)
    .bind(media_type.to_string())
    .bind(user_id.as_bytes().as_ref())
    .bind(date)
    .bind(media_type.to_string())
    .execute(pool)
    .await
    .context(QuerySnafu {
        table: "play_stats_daily",
    })?;

    let _ = media_id;
    Ok(())
}

/// Update (or CREATE) the current streak for `user_id`.
/// `today` must be an ISO date string in "YYYY-MM-DD" format.
pub async fn update_streak(pool: &SqlitePool, user_id: UserId, today: &str) -> Result<(), DbError> {
    // Compute yesterday using SQLite so we stay free of date-math crates here.
    let (yesterday,): (String,) = sqlx::query_as("SELECT date(?, '-1 day')")
        .bind(today)
        .fetch_one(pool)
        .await
        .context(QuerySnafu {
            table: "play_streaks",
        })?;

    let current = sqlx::query_as::<_, StreakRow>(
        "SELECT streak_start, streak_end, days
         FROM play_streaks
         WHERE user_id = ? AND is_current = 1",
    )
    .bind(user_id.as_bytes().as_ref())
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "play_streaks",
    })?;

    match current {
        None => {
            sqlx::query(
                "INSERT INTO play_streaks
                 (user_id, streak_start, streak_end, days, is_current)
                 VALUES (?, ?, ?, 1, 1)",
            )
            .bind(user_id.as_bytes().as_ref())
            .bind(today)
            .bind(today)
            .execute(pool)
            .await
            .context(QuerySnafu {
                table: "play_streaks",
            })?;
        }
        Some(ref row) if row.streak_end == today => {
            // Already counted today  -  no-op.
        }
        Some(ref row) if row.streak_end == yesterday => {
            sqlx::query(
                "UPDATE play_streaks
                 SET streak_end = ?, days = days + 1
                 WHERE user_id = ? AND is_current = 1",
            )
            .bind(today)
            .bind(user_id.as_bytes().as_ref())
            .execute(pool)
            .await
            .context(QuerySnafu {
                table: "play_streaks",
            })?;
        }
        Some(_) => {
            sqlx::query(
                "UPDATE play_streaks SET is_current = 0 WHERE user_id = ? AND is_current = 1",
            )
            .bind(user_id.as_bytes().as_ref())
            .execute(pool)
            .await
            .context(QuerySnafu {
                table: "play_streaks",
            })?;

            sqlx::query(
                "INSERT INTO play_streaks
                 (user_id, streak_start, streak_end, days, is_current)
                 VALUES (?, ?, ?, 1, 1)",
            )
            .bind(user_id.as_bytes().as_ref())
            .bind(today)
            .bind(today)
            .execute(pool)
            .await
            .context(QuerySnafu {
                table: "play_streaks",
            })?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Query  -  recent history
// ---------------------------------------------------------------------------

pub async fn recent_sessions(
    pool: &SqlitePool,
    user_id: UserId,
    limit: u32,
) -> Result<Vec<PlaySession>, DbError> {
    sqlx::query_as::<_, PlaySession>(
        "SELECT id, media_id, user_id, media_type, started_at, ended_at,
                duration_ms, total_ms, completed, percent_heard, source,
                scrobble_eligible, scrobbled_at, scrobble_service,
                device_name, quality_score, dsp_active
         FROM play_sessions
         WHERE user_id = ?
         ORDER BY started_at DESC
         LIMIT ?",
    )
    .bind(user_id.as_bytes().as_ref())
    .bind(i64::from(limit))
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "play_sessions",
    })
}

pub async fn recent_by_media_type(
    pool: &SqlitePool,
    user_id: UserId,
    media_type: MediaType,
    limit: u32,
) -> Result<Vec<PlaySession>, DbError> {
    sqlx::query_as::<_, PlaySession>(
        "SELECT id, media_id, user_id, media_type, started_at, ended_at,
                duration_ms, total_ms, completed, percent_heard, source,
                scrobble_eligible, scrobbled_at, scrobble_service,
                device_name, quality_score, dsp_active
         FROM play_sessions
         WHERE user_id = ? AND media_type = ?
         ORDER BY started_at DESC
         LIMIT ?",
    )
    .bind(user_id.as_bytes().as_ref())
    .bind(media_type.to_string())
    .bind(i64::from(limit))
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "play_sessions",
    })
}

// ---------------------------------------------------------------------------
// Query  -  analytics
// ---------------------------------------------------------------------------

pub async fn top_items(
    pool: &SqlitePool,
    user_id: UserId,
    media_type: MediaType,
    period: &DateRange,
    limit: u32,
) -> Result<Vec<ItemStats>, DbError> {
    let rows = sqlx::query_as::<_, ItemStatsRow>(
        "SELECT psi.media_id, psi.play_count, psi.total_ms,
                psi.skip_count, psi.last_played_at
         FROM play_stats_item psi
         WHERE psi.user_id = ?
           AND psi.media_id IN (
               SELECT DISTINCT ps.media_id
               FROM play_sessions ps
               WHERE ps.user_id = ?
                 AND ps.media_type = ?
                 AND date(ps.started_at) >= ?
                 AND date(ps.started_at) <= ?
           )
         ORDER BY psi.play_count DESC
         LIMIT ?",
    )
    .bind(user_id.as_bytes().as_ref())
    .bind(user_id.as_bytes().as_ref())
    .bind(media_type.to_string())
    .bind(&period.start)
    .bind(&period.end)
    .bind(i64::from(limit))
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "play_stats_item",
    })?;

    Ok(rows
        .into_iter()
        .filter_map(|r| {
            Some(ItemStats {
                media_id: bytes_to_media_id(r.media_id)?,
                play_count: r.play_count,
                total_ms: r.total_ms,
                skip_count: r.skip_count,
                last_played_at: r.last_played_at,
            })
        })
        .collect())
}

pub async fn listening_time(
    pool: &SqlitePool,
    user_id: UserId,
    period: &DateRange,
) -> Result<ListeningTimeSummary, DbError> {
    let rows = sqlx::query_as::<_, MediaTypeAggRow>(
        "SELECT media_type,
                SUM(total_ms) AS total_ms,
                CAST(SUM(sessions) AS INTEGER) AS session_count
         FROM play_stats_daily
         WHERE user_id = ? AND date >= ? AND date <= ?
         GROUP BY media_type",
    )
    .bind(user_id.as_bytes().as_ref())
    .bind(&period.start)
    .bind(&period.end)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "play_stats_daily",
    })?;

    let mut total_ms: i64 = 0;
    let mut session_count: i32 = 0;
    let mut by_media_type = Vec::with_capacity(rows.len());

    for row in rows {
        total_ms += row.total_ms;
        session_count += row.session_count;
        by_media_type.push((parse_media_type(&row.media_type), row.total_ms));
    }

    Ok(ListeningTimeSummary {
        total_ms,
        by_media_type,
        session_count,
    })
}

pub async fn daily_activity(
    pool: &SqlitePool,
    user_id: UserId,
    period: &DateRange,
) -> Result<Vec<DailyStats>, DbError> {
    let rows = sqlx::query_as::<_, DailyStatsRow>(
        "SELECT date, media_type, sessions, total_ms, unique_items
         FROM play_stats_daily
         WHERE user_id = ? AND date >= ? AND date <= ?
         ORDER BY date ASC, media_type ASC",
    )
    .bind(user_id.as_bytes().as_ref())
    .bind(&period.start)
    .bind(&period.end)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "play_stats_daily",
    })?;

    Ok(rows
        .into_iter()
        .map(|r| DailyStats {
            date: r.date,
            media_type: parse_media_type(&r.media_type),
            sessions: r.sessions,
            total_ms: r.total_ms,
            unique_items: r.unique_items,
        })
        .collect())
}

pub async fn current_streak(pool: &SqlitePool, user_id: UserId) -> Result<Option<Streak>, DbError> {
    let row = sqlx::query_as::<_, StreakRow>(
        "SELECT streak_start, streak_end, days
         FROM play_streaks
         WHERE user_id = ? AND is_current = 1",
    )
    .bind(user_id.as_bytes().as_ref())
    .fetch_optional(pool)
    .await
    .context(QuerySnafu {
        table: "play_streaks",
    })?;

    Ok(row.map(|r| Streak {
        start: r.streak_start,
        end: r.streak_end,
        days: r.days,
    }))
}

// ---------------------------------------------------------------------------
// Query  -  discovery support
// ---------------------------------------------------------------------------

pub async fn never_played(
    pool: &SqlitePool,
    user_id: UserId,
    media_type: MediaType,
    limit: u32,
) -> Result<Vec<MediaId>, DbError> {
    let table = match media_type {
        MediaType::Music => "music_tracks",
        MediaType::Audiobook => "audiobooks",
        MediaType::Book => "books",
        MediaType::Comic => "comics",
        MediaType::Podcast => "podcast_episodes",
        MediaType::News => "news_articles",
        MediaType::Movie => "movies",
        MediaType::Tv => "tv_episodes",
        _ => return Ok(vec![]),
    };

    let sql = format!(
        "SELECT id FROM {table}
         WHERE id NOT IN (
             SELECT media_id FROM play_stats_item WHERE user_id = ?
         )
         LIMIT ?"
    );

    let rows: Vec<(Vec<u8>,)> = sqlx::query_as(&sql)
        .bind(user_id.as_bytes().as_ref())
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await
        .context(QuerySnafu { table })?;

    Ok(rows
        .into_iter()
        .filter_map(|(bytes,)| bytes_to_media_id(bytes))
        .collect())
}

pub async fn not_played_since(
    pool: &SqlitePool,
    user_id: UserId,
    before: &str,
    limit: u32,
) -> Result<Vec<MediaId>, DbError> {
    let rows: Vec<(Vec<u8>,)> = sqlx::query_as(
        "SELECT media_id FROM play_stats_item
         WHERE user_id = ? AND last_played_at < ?
         ORDER BY last_played_at ASC
         LIMIT ?",
    )
    .bind(user_id.as_bytes().as_ref())
    .bind(before)
    .bind(i64::from(limit))
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "play_stats_item",
    })?;

    Ok(rows
        .into_iter()
        .filter_map(|(bytes,)| bytes_to_media_id(bytes))
        .collect())
}

pub async fn on_this_day(
    pool: &SqlitePool,
    user_id: UserId,
    month: u8,
    day: u8,
) -> Result<Vec<PlaySession>, DbError> {
    let month_day = format!("{month:02}-{day:02}");
    sqlx::query_as::<_, PlaySession>(
        "SELECT id, media_id, user_id, media_type, started_at, ended_at,
                duration_ms, total_ms, completed, percent_heard, source,
                scrobble_eligible, scrobbled_at, scrobble_service,
                device_name, quality_score, dsp_active
         FROM play_sessions
         WHERE user_id = ?
           AND strftime('%m-%d', started_at) = ?
         ORDER BY started_at DESC",
    )
    .bind(user_id.as_bytes().as_ref())
    .bind(month_day)
    .fetch_all(pool)
    .await
    .context(QuerySnafu {
        table: "play_sessions",
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
