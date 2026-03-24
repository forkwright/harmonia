-- Zone grouping for multi-room synchronized playback.

CREATE TABLE renderers (
    id         TEXT NOT NULL PRIMARY KEY,
    name       TEXT NOT NULL,
    address    TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE zones (
    id         TEXT NOT NULL PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE zone_members (
    zone_id     TEXT NOT NULL REFERENCES zones(id) ON DELETE CASCADE,
    renderer_id TEXT NOT NULL REFERENCES renderers(id) ON DELETE CASCADE,
    joined_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (zone_id, renderer_id)
);

CREATE INDEX idx_zone_members_renderer ON zone_members(renderer_id);
