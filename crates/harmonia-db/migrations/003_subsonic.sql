-- =============================================================================
-- 003_subsonic.sql — Subsonic/OpenSubsonic API support tables
-- =============================================================================

-- Per-user plaintext password for Subsonic legacy token auth.
-- Subsonic legacy auth is MD5-based and incompatible with Argon2id storage.
-- Users who want legacy Subsonic client support set this separately.
CREATE TABLE subsonic_passwords (
    user_id  BLOB NOT NULL PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    password TEXT NOT NULL
);

-- Playlists
CREATE TABLE subsonic_playlists (
    id         BLOB NOT NULL PRIMARY KEY,
    owner_id   BLOB NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    comment    TEXT,
    public     INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX idx_sp_owner ON subsonic_playlists(owner_id);

CREATE TABLE subsonic_playlist_tracks (
    playlist_id BLOB    NOT NULL REFERENCES subsonic_playlists(id) ON DELETE CASCADE,
    track_id    BLOB    NOT NULL REFERENCES music_tracks(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, position)
);

CREATE INDEX idx_spt_playlist ON subsonic_playlist_tracks(playlist_id);

-- Stars (track, album/release-group, artist/registry)
CREATE TABLE subsonic_stars (
    user_id    BLOB NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    item_id    BLOB NOT NULL,
    item_type  TEXT NOT NULL CHECK(item_type IN ('track', 'album', 'artist')),
    starred_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (user_id, item_id)
);

CREATE INDEX idx_ss_user ON subsonic_stars(user_id);

-- Ratings (1-5)
CREATE TABLE subsonic_ratings (
    user_id BLOB    NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    item_id BLOB    NOT NULL,
    rating  INTEGER NOT NULL CHECK(rating BETWEEN 1 AND 5),
    PRIMARY KEY (user_id, item_id)
);

CREATE INDEX idx_sr_user ON subsonic_ratings(user_id);
