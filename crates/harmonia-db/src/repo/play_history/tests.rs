use super::*;
use crate::migrate::MIGRATOR;

async fn setup() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    pool
}

fn make_user_id() -> UserId {
    UserId::new()
}

fn make_media_id() -> MediaId {
    MediaId::new()
}

async fn insert_user(pool: &SqlitePool, user_id: UserId) {
    sqlx::query(
        "INSERT INTO users (id, username, display_name, password_hash, role)
             VALUES (?, ?, ?, ?, ?)",
    )
    .bind(user_id.as_bytes().as_ref())
    .bind(format!("user_{}", uuid::Uuid::now_v7()))
    .bind("Test User")
    .bind("$argon2id$placeholder")
    .bind("member")
    .execute(pool)
    .await
    .unwrap();
}

fn new_session(user_id: UserId, media_id: MediaId, media_type: MediaType) -> NewPlaySession {
    NewPlaySession {
        media_id,
        user_id,
        media_type,
        source: PlaySource::Local,
        device_name: None,
        quality_score: None,
        dsp_active: false,
        total_ms: Some(210_000),
    }
}

// -----------------------------------------------------------------------
// Session lifecycle
// -----------------------------------------------------------------------

#[tokio::test]
async fn start_session_creates_row_with_null_ended_at() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;
    let media_id = make_media_id();

    let session_id = start_session(&pool, &new_session(user_id, media_id, MediaType::Music))
        .await
        .unwrap();

    let row: (Option<String>,) = sqlx::query_as("SELECT ended_at FROM play_sessions WHERE id = ?")
        .bind(session_id.as_bytes().as_ref())
        .fetch_one(&pool)
        .await
        .unwrap();

    assert!(row.0.is_none());
}

#[tokio::test]
async fn end_session_populates_outcome_fields() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;

    let session_id = start_session(
        &pool,
        &new_session(user_id, make_media_id(), MediaType::Music),
    )
    .await
    .unwrap();

    end_session(
        &pool,
        session_id,
        &SessionOutcome {
            duration_ms: 180_000,
            completed: false,
            percent_heard: Some(85),
        },
    )
    .await
    .unwrap();

    let row: (Option<String>, i64, Option<i64>) = sqlx::query_as(
        "SELECT ended_at, duration_ms, percent_heard FROM play_sessions WHERE id = ?",
    )
    .bind(session_id.as_bytes().as_ref())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(row.0.is_some());
    assert_eq!(row.1, 180_000);
    assert_eq!(row.2, Some(85));
}

#[tokio::test]
async fn end_session_completed_flag() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;

    let session_id = start_session(
        &pool,
        &new_session(user_id, make_media_id(), MediaType::Music),
    )
    .await
    .unwrap();

    end_session(
        &pool,
        session_id,
        &SessionOutcome {
            duration_ms: 210_000,
            completed: true,
            percent_heard: Some(100),
        },
    )
    .await
    .unwrap();

    let (completed,): (i64,) = sqlx::query_as("SELECT completed FROM play_sessions WHERE id = ?")
        .bind(session_id.as_bytes().as_ref())
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(completed, 1);
}

#[tokio::test]
async fn get_active_sessions_excludes_ended() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;

    let active_id = start_session(
        &pool,
        &new_session(user_id, make_media_id(), MediaType::Music),
    )
    .await
    .unwrap();
    let ended_id = start_session(
        &pool,
        &new_session(user_id, make_media_id(), MediaType::Music),
    )
    .await
    .unwrap();

    end_session(
        &pool,
        ended_id,
        &SessionOutcome {
            duration_ms: 100,
            completed: false,
            percent_heard: None,
        },
    )
    .await
    .unwrap();

    let active = get_active_sessions(&pool, user_id).await.unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, active_id.as_bytes().to_vec());
}

// -----------------------------------------------------------------------
// Scrobble tracking
// -----------------------------------------------------------------------

#[tokio::test]
async fn mark_scrobble_eligible_sets_flag() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;

    let session_id = start_session(
        &pool,
        &new_session(user_id, make_media_id(), MediaType::Music),
    )
    .await
    .unwrap();

    mark_scrobble_eligible(&pool, session_id).await.unwrap();

    let (flag,): (i64,) =
        sqlx::query_as("SELECT scrobble_eligible FROM play_sessions WHERE id = ?")
            .bind(session_id.as_bytes().as_ref())
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(flag, 1);
}

#[tokio::test]
async fn get_pending_scrobbles_returns_eligible_unscrobbled() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;

    let s1 = start_session(
        &pool,
        &new_session(user_id, make_media_id(), MediaType::Music),
    )
    .await
    .unwrap();
    let s2 = start_session(
        &pool,
        &new_session(user_id, make_media_id(), MediaType::Music),
    )
    .await
    .unwrap();
    let _s3 = start_session(
        &pool,
        &new_session(user_id, make_media_id(), MediaType::Music),
    )
    .await
    .unwrap();

    mark_scrobble_eligible(&pool, s1).await.unwrap();
    mark_scrobble_eligible(&pool, s2).await.unwrap();
    mark_scrobbled(&pool, s2, "lastfm").await.unwrap();

    let pending = get_pending_scrobbles(&pool, user_id).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, s1.as_bytes().to_vec());
}

#[tokio::test]
async fn mark_scrobbled_sets_service_and_timestamp() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;

    let session_id = start_session(
        &pool,
        &new_session(user_id, make_media_id(), MediaType::Music),
    )
    .await
    .unwrap();

    mark_scrobble_eligible(&pool, session_id).await.unwrap();
    mark_scrobbled(&pool, session_id, "listenbrainz")
        .await
        .unwrap();

    let row: (Option<String>, Option<String>) =
        sqlx::query_as("SELECT scrobbled_at, scrobble_service FROM play_sessions WHERE id = ?")
            .bind(session_id.as_bytes().as_ref())
            .fetch_one(&pool)
            .await
            .unwrap();

    assert!(row.0.is_some());
    assert_eq!(row.1.as_deref(), Some("listenbrainz"));
}

// -----------------------------------------------------------------------
// Stats aggregation
// -----------------------------------------------------------------------

#[tokio::test]
async fn update_item_stats_increments_play_count() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;
    let media_id = make_media_id();

    let session_id = start_session(&pool, &new_session(user_id, media_id, MediaType::Music))
        .await
        .unwrap();
    end_session(
        &pool,
        session_id,
        &SessionOutcome {
            duration_ms: 180_000,
            completed: true,
            percent_heard: Some(100),
        },
    )
    .await
    .unwrap();

    let session = sqlx::query_as::<_, PlaySession>(
        "SELECT id, media_id, user_id, media_type, started_at, ended_at,
                    duration_ms, total_ms, completed, percent_heard, source,
                    scrobble_eligible, scrobbled_at, scrobble_service,
                    device_name, quality_score, dsp_active
             FROM play_sessions WHERE id = ?",
    )
    .bind(session_id.as_bytes().as_ref())
    .fetch_one(&pool)
    .await
    .unwrap();

    update_item_stats(&pool, media_id, user_id, &session)
        .await
        .unwrap();
    update_item_stats(&pool, media_id, user_id, &session)
        .await
        .unwrap();

    let (play_count, total_ms): (i32, i64) = sqlx::query_as(
        "SELECT play_count, total_ms FROM play_stats_item WHERE media_id = ? AND user_id = ?",
    )
    .bind(media_id.as_bytes().as_ref())
    .bind(user_id.as_bytes().as_ref())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(play_count, 2);
    assert_eq!(total_ms, 360_000);
}

#[tokio::test]
async fn update_item_stats_skip_count_when_percent_under_50() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;
    let media_id = make_media_id();

    let session_id = start_session(&pool, &new_session(user_id, media_id, MediaType::Music))
        .await
        .unwrap();
    end_session(
        &pool,
        session_id,
        &SessionOutcome {
            duration_ms: 30_000,
            completed: false,
            percent_heard: Some(14),
        },
    )
    .await
    .unwrap();

    let session = sqlx::query_as::<_, PlaySession>(
        "SELECT id, media_id, user_id, media_type, started_at, ended_at,
                    duration_ms, total_ms, completed, percent_heard, source,
                    scrobble_eligible, scrobbled_at, scrobble_service,
                    device_name, quality_score, dsp_active
             FROM play_sessions WHERE id = ?",
    )
    .bind(session_id.as_bytes().as_ref())
    .fetch_one(&pool)
    .await
    .unwrap();

    update_item_stats(&pool, media_id, user_id, &session)
        .await
        .unwrap();

    let (skip_count,): (i32,) =
        sqlx::query_as("SELECT skip_count FROM play_stats_item WHERE media_id = ? AND user_id = ?")
            .bind(media_id.as_bytes().as_ref())
            .bind(user_id.as_bytes().as_ref())
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(skip_count, 1);
}

#[tokio::test]
async fn update_item_stats_first_played_set_once() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;
    let media_id = make_media_id();

    // First play
    let s1 = start_session(&pool, &new_session(user_id, media_id, MediaType::Music))
        .await
        .unwrap();
    end_session(
        &pool,
        s1,
        &SessionOutcome {
            duration_ms: 100,
            completed: false,
            percent_heard: None,
        },
    )
    .await
    .unwrap();
    let sess1 = sqlx::query_as::<_, PlaySession>(
        "SELECT id, media_id, user_id, media_type, started_at, ended_at,
                    duration_ms, total_ms, completed, percent_heard, source,
                    scrobble_eligible, scrobbled_at, scrobble_service,
                    device_name, quality_score, dsp_active
             FROM play_sessions WHERE id = ?",
    )
    .bind(s1.as_bytes().as_ref())
    .fetch_one(&pool)
    .await
    .unwrap();
    update_item_stats(&pool, media_id, user_id, &sess1)
        .await
        .unwrap();

    let (first1,): (Option<String>,) = sqlx::query_as(
        "SELECT first_played_at FROM play_stats_item WHERE media_id = ? AND user_id = ?",
    )
    .bind(media_id.as_bytes().as_ref())
    .bind(user_id.as_bytes().as_ref())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(first1.is_some());

    // Second play
    let s2 = start_session(&pool, &new_session(user_id, media_id, MediaType::Music))
        .await
        .unwrap();
    end_session(
        &pool,
        s2,
        &SessionOutcome {
            duration_ms: 100,
            completed: false,
            percent_heard: None,
        },
    )
    .await
    .unwrap();
    let sess2 = sqlx::query_as::<_, PlaySession>(
        "SELECT id, media_id, user_id, media_type, started_at, ended_at,
                    duration_ms, total_ms, completed, percent_heard, source,
                    scrobble_eligible, scrobbled_at, scrobble_service,
                    device_name, quality_score, dsp_active
             FROM play_sessions WHERE id = ?",
    )
    .bind(s2.as_bytes().as_ref())
    .fetch_one(&pool)
    .await
    .unwrap();
    update_item_stats(&pool, media_id, user_id, &sess2)
        .await
        .unwrap();

    let (first2, last2): (Option<String>, Option<String>) = sqlx::query_as(
            "SELECT first_played_at, last_played_at FROM play_stats_item WHERE media_id = ? AND user_id = ?",
        )
        .bind(media_id.as_bytes().as_ref())
        .bind(user_id.as_bytes().as_ref())
        .fetch_one(&pool)
        .await
        .unwrap();

    // first_played_at unchanged, last_played_at updated
    assert_eq!(first2, first1);
    assert!(last2.is_some());
}

#[tokio::test]
async fn update_daily_stats_upsert() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;
    let media_id = make_media_id();

    update_daily_stats(
        &pool,
        user_id,
        "2026-03-12",
        MediaType::Music,
        media_id,
        180_000,
    )
    .await
    .unwrap();
    update_daily_stats(
        &pool,
        user_id,
        "2026-03-12",
        MediaType::Music,
        media_id,
        210_000,
    )
    .await
    .unwrap();

    let (sessions, total_ms): (i32, i64) = sqlx::query_as(
            "SELECT sessions, total_ms FROM play_stats_daily WHERE user_id = ? AND date = ? AND media_type = ?",
        )
        .bind(user_id.as_bytes().as_ref())
        .bind("2026-03-12")
        .bind("music")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(sessions, 2);
    assert_eq!(total_ms, 390_000);
}

// -----------------------------------------------------------------------
// Analytics queries
// -----------------------------------------------------------------------

#[tokio::test]
async fn top_items_ordered_by_play_count() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;

    let m1 = make_media_id();
    let m2 = make_media_id();

    // m2 played twice, m1 played once
    for media_id in [m1, m2, m2] {
        let s = start_session(&pool, &new_session(user_id, media_id, MediaType::Music))
            .await
            .unwrap();
        end_session(
            &pool,
            s,
            &SessionOutcome {
                duration_ms: 100,
                completed: true,
                percent_heard: Some(100),
            },
        )
        .await
        .unwrap();
        let session = sqlx::query_as::<_, PlaySession>(
            "SELECT id, media_id, user_id, media_type, started_at, ended_at,
                        duration_ms, total_ms, completed, percent_heard, source,
                        scrobble_eligible, scrobbled_at, scrobble_service,
                        device_name, quality_score, dsp_active
                 FROM play_sessions WHERE id = ?",
        )
        .bind(s.as_bytes().as_ref())
        .fetch_one(&pool)
        .await
        .unwrap();
        update_item_stats(&pool, media_id, user_id, &session)
            .await
            .unwrap();
    }

    let period = DateRange {
        start: "2000-01-01".to_string(),
        end: "2099-12-31".to_string(),
    };
    let items = top_items(&pool, user_id, MediaType::Music, &period, 10)
        .await
        .unwrap();

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].media_id, m2);
    assert_eq!(items[0].play_count, 2);
    assert_eq!(items[1].media_id, m1);
    assert_eq!(items[1].play_count, 1);
}

#[tokio::test]
async fn listening_time_aggregates_across_media_types() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;
    let media_id = make_media_id();

    update_daily_stats(
        &pool,
        user_id,
        "2026-03-10",
        MediaType::Music,
        media_id,
        100_000,
    )
    .await
    .unwrap();
    update_daily_stats(
        &pool,
        user_id,
        "2026-03-11",
        MediaType::Podcast,
        media_id,
        200_000,
    )
    .await
    .unwrap();
    update_daily_stats(
        &pool,
        user_id,
        "2026-03-12",
        MediaType::Music,
        media_id,
        50_000,
    )
    .await
    .unwrap();

    let period = DateRange {
        start: "2026-03-10".to_string(),
        end: "2026-03-12".to_string(),
    };
    let summary = listening_time(&pool, user_id, &period).await.unwrap();

    assert_eq!(summary.total_ms, 350_000);
    assert_eq!(summary.session_count, 3);
    assert_eq!(summary.by_media_type.len(), 2);
}

#[tokio::test]
async fn daily_activity_returns_one_row_per_date_media_type() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;
    let media_id = make_media_id();

    update_daily_stats(
        &pool,
        user_id,
        "2026-03-10",
        MediaType::Music,
        media_id,
        100_000,
    )
    .await
    .unwrap();
    update_daily_stats(
        &pool,
        user_id,
        "2026-03-11",
        MediaType::Music,
        media_id,
        200_000,
    )
    .await
    .unwrap();

    let period = DateRange {
        start: "2026-03-10".to_string(),
        end: "2026-03-11".to_string(),
    };
    let rows = daily_activity(&pool, user_id, &period).await.unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].date, "2026-03-10");
    assert_eq!(rows[1].date, "2026-03-11");
}

#[tokio::test]
async fn on_this_day_returns_same_month_day_sessions() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;
    let media_id = make_media_id();

    // Insert sessions with explicit started_at timestamps
    sqlx::query(
        "INSERT INTO play_sessions
             (id, media_id, user_id, media_type, started_at, source)
             VALUES (?, ?, ?, 'music', '2024-03-12T10:00:00Z', 'local'),
                    (?, ?, ?, 'music', '2025-03-12T11:00:00Z', 'local'),
                    (?, ?, ?, 'music', '2026-03-15T12:00:00Z', 'local')",
    )
    .bind(SessionId::new().as_bytes().as_ref())
    .bind(media_id.as_bytes().as_ref())
    .bind(user_id.as_bytes().as_ref())
    .bind(SessionId::new().as_bytes().as_ref())
    .bind(media_id.as_bytes().as_ref())
    .bind(user_id.as_bytes().as_ref())
    .bind(SessionId::new().as_bytes().as_ref())
    .bind(media_id.as_bytes().as_ref())
    .bind(user_id.as_bytes().as_ref())
    .execute(&pool)
    .await
    .unwrap();

    let sessions = on_this_day(&pool, user_id, 3, 12).await.unwrap();
    assert_eq!(sessions.len(), 2);
}

#[tokio::test]
async fn not_played_since_filters_by_last_played() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;

    let m1 = make_media_id();
    let m2 = make_media_id();

    sqlx::query(
        "INSERT INTO play_stats_item
             (media_id, user_id, play_count, total_ms, last_played_at)
             VALUES (?, ?, 3, 100, '2025-01-01T00:00:00Z'),
                    (?, ?, 1, 100, '2026-03-01T00:00:00Z')",
    )
    .bind(m1.as_bytes().as_ref())
    .bind(user_id.as_bytes().as_ref())
    .bind(m2.as_bytes().as_ref())
    .bind(user_id.as_bytes().as_ref())
    .execute(&pool)
    .await
    .unwrap();

    let result = not_played_since(&pool, user_id, "2026-01-01T00:00:00Z", 10)
        .await
        .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0], m1);
}

// -----------------------------------------------------------------------
// Streak tracking
// -----------------------------------------------------------------------

#[tokio::test]
async fn streak_first_play_creates_streak_of_one() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;

    update_streak(&pool, user_id, "2026-03-12").await.unwrap();

    let streak = current_streak(&pool, user_id).await.unwrap().unwrap();
    assert_eq!(streak.start, "2026-03-12");
    assert_eq!(streak.end, "2026-03-12");
    assert_eq!(streak.days, 1);
}

#[tokio::test]
async fn streak_consecutive_day_extends() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;

    update_streak(&pool, user_id, "2026-03-11").await.unwrap();
    update_streak(&pool, user_id, "2026-03-12").await.unwrap();

    let streak = current_streak(&pool, user_id).await.unwrap().unwrap();
    assert_eq!(streak.start, "2026-03-11");
    assert_eq!(streak.end, "2026-03-12");
    assert_eq!(streak.days, 2);
}

#[tokio::test]
async fn streak_same_day_is_idempotent() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;

    update_streak(&pool, user_id, "2026-03-12").await.unwrap();
    update_streak(&pool, user_id, "2026-03-12").await.unwrap();

    let streak = current_streak(&pool, user_id).await.unwrap().unwrap();
    assert_eq!(streak.days, 1);
}

#[tokio::test]
async fn streak_gap_closes_old_and_starts_new() {
    let pool = setup().await;
    let user_id = make_user_id();
    insert_user(&pool, user_id).await;

    update_streak(&pool, user_id, "2026-03-10").await.unwrap();
    update_streak(&pool, user_id, "2026-03-11").await.unwrap();
    // Gap: skip 2026-03-12
    update_streak(&pool, user_id, "2026-03-13").await.unwrap();

    let streak = current_streak(&pool, user_id).await.unwrap().unwrap();
    assert_eq!(streak.start, "2026-03-13");
    assert_eq!(streak.days, 1);

    let (closed_count,): (i32,) =
        sqlx::query_as("SELECT COUNT(*) FROM play_streaks WHERE user_id = ? AND is_current = 0")
            .bind(user_id.as_bytes().as_ref())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(closed_count, 1);
}
