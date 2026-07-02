-- Widen indexers.protocol to admit 'cardigann' (definition-driven HTML
-- indexers, zetesis CardigannClient).
--
-- WHY the rebuild: SQLite cannot alter a CHECK constraint in place. Both
-- tables are copied through FK-free backups (child first) so the implicit
-- DELETE from dropping the parent cannot cascade into live category rows.

CREATE TABLE indexer_categories_backup (
    indexer_id  INTEGER NOT NULL,
    category_id INTEGER NOT NULL,
    name        TEXT NOT NULL
) STRICT;

INSERT INTO indexer_categories_backup (indexer_id, category_id, name)
SELECT indexer_id, category_id, name FROM indexer_categories;

CREATE TABLE indexers_backup (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    url         TEXT NOT NULL,
    protocol    TEXT NOT NULL,
    api_key     TEXT,
    enabled     INTEGER NOT NULL,
    cf_bypass   INTEGER NOT NULL,
    status      TEXT NOT NULL,
    last_tested TEXT,
    caps_json   TEXT,
    priority    INTEGER NOT NULL,
    added_at    TEXT NOT NULL
) STRICT;

INSERT INTO indexers_backup
    (id, name, url, protocol, api_key, enabled, cf_bypass, status,
     last_tested, caps_json, priority, added_at)
SELECT id, name, url, protocol, api_key, enabled, cf_bypass, status,
       last_tested, caps_json, priority, added_at
FROM indexers;

DROP TABLE indexer_categories;
DROP TABLE indexers;

CREATE TABLE indexers (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    url         TEXT NOT NULL,
    protocol    TEXT NOT NULL CHECK (protocol IN ('torznab', 'newznab', 'cardigann')),
    api_key     TEXT,
    enabled     INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    cf_bypass   INTEGER NOT NULL DEFAULT 0 CHECK (cf_bypass IN (0, 1)),
    status      TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active', 'degraded', 'failed')),
    last_tested TEXT,
    caps_json   TEXT,
    priority    INTEGER NOT NULL DEFAULT 50,
    added_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

INSERT INTO indexers
    (id, name, url, protocol, api_key, enabled, cf_bypass, status,
     last_tested, caps_json, priority, added_at)
SELECT id, name, url, protocol, api_key, enabled, cf_bypass, status,
       last_tested, caps_json, priority, added_at
FROM indexers_backup;

CREATE TABLE indexer_categories (
    indexer_id  INTEGER NOT NULL REFERENCES indexers(id) ON DELETE CASCADE,
    category_id INTEGER NOT NULL,
    name        TEXT NOT NULL,
    PRIMARY KEY (indexer_id, category_id)
) STRICT;

INSERT INTO indexer_categories (indexer_id, category_id, name)
SELECT indexer_id, category_id, name FROM indexer_categories_backup;

DROP TABLE indexers_backup;
DROP TABLE indexer_categories_backup;

CREATE INDEX idx_indexers_enabled_status ON indexers(enabled, status);
CREATE INDEX idx_indexer_categories_indexer ON indexer_categories(indexer_id);
