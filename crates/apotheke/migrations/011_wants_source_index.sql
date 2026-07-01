-- Index for want lookups by external source.
-- WHY: syndesmos diffs live Tidal favorites against persisted want rows
-- (source = 'tidal_sync') on every sync; without this index that read is a
-- full table scan.
CREATE INDEX idx_wants_source ON wants(source, source_ref);
