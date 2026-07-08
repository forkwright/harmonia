-- Per-indexer settings override storage (Cardigann `settings:` fields).
-- JSON object string (setting name -> value), nullable — absent means "use
-- every definition default." Same treatment as caps_json in 004.

ALTER TABLE indexers ADD COLUMN settings_json TEXT;
