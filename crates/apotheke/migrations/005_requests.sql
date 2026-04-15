-- Aitesis request management tables
-- Tracks household media requests from submission through fulfillment.

CREATE TABLE IF NOT EXISTS requests (
    id          BLOB NOT NULL PRIMARY KEY,
    user_id     BLOB NOT NULL,
    media_type  TEXT NOT NULL,
    title       TEXT NOT NULL,
    external_id TEXT,
    status      TEXT NOT NULL DEFAULT 'submitted'
                    CHECK (status IN ('submitted', 'approved', 'denied', 'monitoring', 'fulfilled', 'failed')),
    decided_by  BLOB,
    decided_at  TEXT,
    deny_reason TEXT,
    want_id     BLOB,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_requests_user_status ON requests(user_id, status);
CREATE INDEX idx_requests_status ON requests(status);
