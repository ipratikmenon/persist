-- 0012 — admit 'Client' to the sync outbox.
--
-- Every table in the PostgreSQL mirror has a foreign key to mirror.clients, so
-- a matter, deadline, invoice or portal user for a client the mirror has never
-- seen is refused outright. The client row therefore has to be pushed like any
-- other entity — but 0009's CHECK constraint predates that and rejects it.
--
-- SQLite cannot alter a CHECK constraint in place, so the table is rebuilt.
-- Pending rows are carried across: an outbox entry is a change the firm made and
-- has not yet published, and dropping one would silently lose it from the portal
-- until something happened to touch the same record again.

PRAGMA foreign_keys = OFF;

CREATE TABLE sync_outbox_new (
    id          TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL
                    CHECK (entity_type IN ('Client','Matter','Deadline','IpAsset',
                                           'Document','Invoice','Payment',
                                           'PortalUser','Notification')),
    entity_id   TEXT NOT NULL,

    op          TEXT NOT NULL DEFAULT 'Upsert'
                    CHECK (op IN ('Upsert','Delete')),

    attempts    INTEGER NOT NULL DEFAULT 0,
    last_error  TEXT,

    created_at  DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at  DATETIME NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO sync_outbox_new
    (id, entity_type, entity_id, op, attempts, last_error, created_at, updated_at)
SELECT id, entity_type, entity_id, op, attempts, last_error, created_at, updated_at
FROM sync_outbox;

DROP TABLE sync_outbox;
ALTER TABLE sync_outbox_new RENAME TO sync_outbox;

CREATE INDEX IF NOT EXISTS idx_sync_outbox_created ON sync_outbox(created_at);
CREATE INDEX IF NOT EXISTS idx_sync_outbox_entity  ON sync_outbox(entity_type, entity_id);

PRAGMA foreign_keys = ON;
