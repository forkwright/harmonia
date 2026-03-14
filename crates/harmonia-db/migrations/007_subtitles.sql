-- Prostheke subtitle track storage.
-- Tracks acquired subtitle files per media item, one row per language/forced combination.

CREATE TABLE subtitles (
    id               BLOB NOT NULL PRIMARY KEY,
    media_id         BLOB NOT NULL,
    language         TEXT NOT NULL,
    format           TEXT NOT NULL CHECK (format IN ('srt', 'ass', 'sub', 'vtt')),
    file_path        TEXT NOT NULL,
    provider         TEXT NOT NULL,
    provider_id      TEXT NOT NULL,
    hearing_impaired BOOLEAN NOT NULL DEFAULT FALSE,
    forced           BOOLEAN NOT NULL DEFAULT FALSE,
    score            REAL NOT NULL,
    acquired_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX idx_subtitles_media ON subtitles(media_id);
CREATE UNIQUE INDEX idx_subtitles_media_lang ON subtitles(media_id, language, forced);
