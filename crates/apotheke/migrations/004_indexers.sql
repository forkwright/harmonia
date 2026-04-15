-- Indexer registry tables for Zetesis
-- Stores configured indexer endpoints and their cached capabilities.

CREATE TABLE IF NOT EXISTS indexers (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    url         TEXT NOT NULL,
    protocol    TEXT NOT NULL CHECK (protocol IN ('torznab', 'newznab')),
    api_key     TEXT,
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    cf_bypass   BOOLEAN NOT NULL DEFAULT FALSE,
    status      TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active', 'degraded', 'failed')),
    last_tested TEXT,
    caps_json   TEXT,
    priority    INTEGER NOT NULL DEFAULT 50,
    added_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE TABLE IF NOT EXISTS indexer_categories (
    indexer_id  INTEGER NOT NULL REFERENCES indexers(id) ON DELETE CASCADE,
    category_id INTEGER NOT NULL,
    name        TEXT NOT NULL,
    PRIMARY KEY (indexer_id, category_id)
) STRICT;

CREATE INDEX idx_indexers_enabled_status ON indexers(enabled, status);
CREATE INDEX idx_indexer_categories_indexer ON indexer_categories(indexer_id);
