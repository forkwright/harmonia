-- Download queue persistence for Syntaxis.
-- Tracks all queued, active, and terminal download states for restart recovery.

CREATE TABLE IF NOT EXISTS download_queue (
    id           BLOB NOT NULL PRIMARY KEY,
    want_id      BLOB NOT NULL,
    release_id   BLOB NOT NULL,
    download_url TEXT NOT NULL,
    protocol     TEXT NOT NULL CHECK (protocol IN ('torrent', 'nzb')),
    priority     INTEGER NOT NULL DEFAULT 1,
    tracker_id   INTEGER,
    info_hash    TEXT,
    status       TEXT NOT NULL DEFAULT 'queued' CHECK (status IN (
                     'queued', 'downloading', 'post_processing', 'importing', 'completed', 'failed'
                 )),
    added_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    started_at   TEXT,
    completed_at TEXT,
    failed_reason TEXT,
    retry_count  INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE INDEX idx_download_queue_status_priority ON download_queue(status, priority DESC);
