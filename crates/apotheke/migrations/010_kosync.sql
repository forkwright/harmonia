-- KOSync protocol support for ebook reading progress sync.
-- Implements the KOSync wire protocol (KOReader client-server sync).

-- KOSync users: separate credential table for SHA1-based auth.
-- WHY(kosync-compat): KOReader client sends SHA1(password) in x-auth-key headers.
-- Storing password_hash as SHA1 hex enables direct comparison without decryption.
CREATE TABLE IF NOT EXISTS kosync_users (
    id              BLOB NOT NULL PRIMARY KEY,    -- UUIDv7
    username        TEXT NOT NULL UNIQUE,          -- account name (max 32 chars typical)
    password_hash   TEXT NOT NULL,                 -- SHA1(password) as hex string
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_kosync_users_username ON kosync_users(username);

-- KOSync progress: one row per (username, document_md5) pair.
-- Last-write-wins conflict resolution with updated_at guard.
-- The document field is the MD5 hash of the ebook file content.
CREATE TABLE IF NOT EXISTS kosync_positions (
    id              BLOB NOT NULL PRIMARY KEY,    -- UUIDv7
    username        TEXT NOT NULL REFERENCES kosync_users(username) ON DELETE CASCADE,
    document        TEXT NOT NULL,                 -- MD5 hash (32 hex chars)
    progress        TEXT,                          -- XPointer location string (e.g. "/body/DocFragment[20]/body/p[22]")
    percentage      REAL NOT NULL DEFAULT 0.0,     -- 0.0-1.0 completion ratio
    device          TEXT,                          -- human-readable device name
    device_id       TEXT,                          -- unique device identifier
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE(username, document)                     -- one row per user per document (LWW)
) STRICT;

CREATE INDEX idx_kosync_pos_user ON kosync_positions(username);
CREATE INDEX idx_kosync_pos_document ON kosync_positions(document);
CREATE INDEX idx_kosync_pos_user_document ON kosync_positions(username, document);
