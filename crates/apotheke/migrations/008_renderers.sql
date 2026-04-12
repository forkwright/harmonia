-- Renderer registry: tracks paired playback renderers.
CREATE TABLE renderers (
    id               TEXT    PRIMARY KEY,
    name             TEXT    NOT NULL,
    api_key_hash     TEXT    NOT NULL,
    cert_fingerprint TEXT    NOT NULL,
    last_seen        TEXT,
    paired_at        TEXT    NOT NULL,
    enabled          INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_renderers_enabled ON renderers (enabled);
