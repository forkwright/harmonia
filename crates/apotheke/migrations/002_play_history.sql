-- Play sessions: one row per contiguous playback of a media item.
-- A "session" starts when playback begins and ends when the user stops,
-- skips, or the track/episode/chapter finishes.
CREATE TABLE IF NOT EXISTS play_sessions (
    id              BLOB NOT NULL PRIMARY KEY,    -- SessionId (UUIDv7)
    media_id        BLOB NOT NULL,                -- FK to media_registry
    user_id         BLOB NOT NULL REFERENCES users(id),
    media_type      TEXT NOT NULL,                 -- MediaType enum as string
    started_at      TEXT NOT NULL,                 -- ISO8601 UTC
    ended_at        TEXT,                          -- NULL if still playing
    duration_ms     INTEGER NOT NULL DEFAULT 0,    -- actual listen time (excludes pauses)
    total_ms        INTEGER,                       -- total track/episode length
    completed       INTEGER NOT NULL DEFAULT 0,    -- 1 if played to end
    percent_heard   INTEGER,                       -- 0-100, computed at session end
    source          TEXT NOT NULL DEFAULT 'local'  -- 'local', 'subsonic', 'stream'
        CHECK(source IN ('local', 'subsonic', 'stream')),

    -- Scrobble tracking
    scrobble_eligible INTEGER NOT NULL DEFAULT 0,  -- met Last.fm threshold?
    scrobbled_at      TEXT,                        -- when scrobble was submitted (NULL = not yet)
    scrobble_service  TEXT,                        -- 'lastfm', 'listenbrainz', etc.

    -- Context (for analytics/discovery)
    device_name     TEXT,                          -- which output device
    quality_score   INTEGER,                       -- quality at time of play
    dsp_active      INTEGER NOT NULL DEFAULT 0     -- was DSP chain active?
) STRICT;

CREATE INDEX idx_ps_user_time ON play_sessions(user_id, started_at DESC);
CREATE INDEX idx_ps_media ON play_sessions(media_id, started_at DESC);
CREATE INDEX idx_ps_scrobble ON play_sessions(user_id, scrobble_eligible, scrobbled_at)
    WHERE scrobble_eligible = 1 AND scrobbled_at IS NULL;
CREATE INDEX idx_ps_media_type ON play_sessions(user_id, media_type, started_at DESC);

-- Daily aggregates: pre-computed for fast analytics queries.
-- Updated by a background task after each session ends.
CREATE TABLE IF NOT EXISTS play_stats_daily (
    user_id         BLOB NOT NULL REFERENCES users(id),
    date            TEXT NOT NULL,                 -- YYYY-MM-DD
    media_type      TEXT NOT NULL,
    sessions        INTEGER NOT NULL DEFAULT 0,
    total_ms        INTEGER NOT NULL DEFAULT 0,
    unique_items    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, date, media_type)
) STRICT;

-- Per-item lifetime stats: total plays, total time, first/last played.
-- Updated incrementally when sessions complete.
CREATE TABLE IF NOT EXISTS play_stats_item (
    media_id        BLOB NOT NULL,
    user_id         BLOB NOT NULL REFERENCES users(id),
    play_count      INTEGER NOT NULL DEFAULT 0,
    total_ms        INTEGER NOT NULL DEFAULT 0,
    skip_count      INTEGER NOT NULL DEFAULT 0,    -- sessions WHERE percent_heard < 50
    first_played_at TEXT,
    last_played_at  TEXT,
    PRIMARY KEY (media_id, user_id)
) STRICT;

CREATE INDEX idx_psi_user_plays ON play_stats_item(user_id, play_count DESC);
CREATE INDEX idx_psi_user_recent ON play_stats_item(user_id, last_played_at DESC);

-- Streak tracking: consecutive days with listening activity.
CREATE TABLE IF NOT EXISTS play_streaks (
    user_id         BLOB NOT NULL REFERENCES users(id),
    streak_start    TEXT NOT NULL,                 -- YYYY-MM-DD
    streak_end      TEXT NOT NULL,                 -- YYYY-MM-DD
    days            INTEGER NOT NULL,
    is_current      INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (user_id, streak_start)
) STRICT;
