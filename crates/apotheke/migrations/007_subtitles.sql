-- Prostheke subtitle track storage.
-- Tracks acquired subtitle files per media item, one row per language/forced combination.

CREATE TABLE IF NOT EXISTS subtitles (
    id               BLOB NOT NULL PRIMARY KEY,
    media_id         BLOB NOT NULL,
    language         TEXT NOT NULL,
    format           TEXT NOT NULL CHECK (format IN ('srt', 'ass', 'sub', 'vtt')),
    file_path        TEXT NOT NULL,
    provider         TEXT NOT NULL,
    provider_id      TEXT NOT NULL,
    hearing_impaired INTEGER NOT NULL DEFAULT 0 CHECK (hearing_impaired IN (0, 1)),
    forced           INTEGER NOT NULL DEFAULT 0 CHECK (forced IN (0, 1)),
    score            REAL NOT NULL,
    acquired_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_subtitles_media ON subtitles(media_id);
CREATE UNIQUE INDEX idx_subtitles_media_lang ON subtitles(media_id, language, forced);
