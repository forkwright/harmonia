-- Drop the live FK on haves.upgraded_from_id.
--
-- WHY: the import finalize path's upgrade sequence (archon::ImportAdapter,
-- forkwright/harmonia#602 keystone) must DELETE the have row it is replacing
-- (idx_haves_file_path is UNCONDITIONAL UNIQUE — the old and new row can
-- never coexist for a Music album directory) while the NEW row's
-- upgraded_from_id records that deleted row's former id. A hard
-- `REFERENCES haves(id)` cannot express this: SQLite validates the FK
-- immediately against live table state, so an INSERT whose upgraded_from_id
-- points at an id already removed in the same transaction is rejected —
-- verified empirically, including with `PRAGMA defer_foreign_keys = ON`
-- (deferred-to-COMMIT still fails, because the referenced row genuinely
-- does not exist in the final state; there is no ordering of
-- DELETE-then-INSERT vs INSERT-then-DELETE that satisfies a live FK here).
-- upgraded_from_id is lineage metadata (which prior have this one
-- replaced), not a referential-integrity link to a still-present row, so it
-- stays a plain BLOB — informational, unenforced.
--
-- WHY the rebuild: SQLite cannot drop a column's REFERENCES clause in place.

CREATE TABLE haves_backup (
    id               BLOB NOT NULL PRIMARY KEY,
    want_id          BLOB NOT NULL,
    release_id       BLOB,
    media_type       TEXT NOT NULL,
    media_type_id    BLOB NOT NULL,
    quality_score    INTEGER NOT NULL,
    file_path        TEXT NOT NULL,
    file_size_bytes  INTEGER NOT NULL,
    status           TEXT NOT NULL,
    imported_at      TEXT NOT NULL,
    upgraded_from_id BLOB
) STRICT;

INSERT INTO haves_backup
    (id, want_id, release_id, media_type, media_type_id, quality_score,
     file_path, file_size_bytes, status, imported_at, upgraded_from_id)
SELECT id, want_id, release_id, media_type, media_type_id, quality_score,
       file_path, file_size_bytes, status, imported_at, upgraded_from_id
FROM haves;

DROP TABLE haves;

CREATE TABLE haves (
    id               BLOB NOT NULL PRIMARY KEY,
    want_id          BLOB NOT NULL REFERENCES wants(id),
    release_id       BLOB REFERENCES releases(id),
    media_type       TEXT NOT NULL,
    media_type_id    BLOB NOT NULL,
    quality_score    INTEGER NOT NULL,
    file_path        TEXT NOT NULL,
    file_size_bytes  INTEGER NOT NULL,
    status           TEXT NOT NULL DEFAULT 'pending' CHECK(status IN (
                         'pending', 'downloading', 'importing', 'complete', 'failed'
                     )),
    imported_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    upgraded_from_id BLOB
) STRICT;

INSERT INTO haves
    (id, want_id, release_id, media_type, media_type_id, quality_score,
     file_path, file_size_bytes, status, imported_at, upgraded_from_id)
SELECT id, want_id, release_id, media_type, media_type_id, quality_score,
       file_path, file_size_bytes, status, imported_at, upgraded_from_id
FROM haves_backup;

DROP TABLE haves_backup;

CREATE INDEX idx_haves_want ON haves(want_id);
CREATE INDEX idx_haves_type_id ON haves(media_type, media_type_id);
CREATE UNIQUE INDEX idx_haves_file_path ON haves(file_path);
