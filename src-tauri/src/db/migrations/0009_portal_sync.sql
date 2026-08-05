-- Migration 0009: Client portal sync (Phase 2 Module 5, Step 2)
-- See specs/module-05-portal.md §6
-- Never edit this file. Create new migrations for all changes.
--
-- Desktop-side additions only. The PostgreSQL mirror lives in server/migrations/.
-- Desktop SQLite remains the source of truth; these tables track what has been
-- projected outward and what the client has sent back awaiting ingest.

-- ---------------------------------------------------------------------------
-- deadlines: client visibility
-- ---------------------------------------------------------------------------
-- Default deny. Statutory deadlines are then opted in by the backfill below and
-- by Keel at creation time: a client is legally affected by a statutory deadline
-- and must not be surprised by it, whereas Procedural/Custom deadlines are the
-- firm's internal steps. An attorney can toggle either way per deadline.
ALTER TABLE deadlines ADD COLUMN is_client_visible INTEGER NOT NULL DEFAULT 0;

UPDATE deadlines SET is_client_visible = 1 WHERE event_type = 'Statutory';

CREATE INDEX IF NOT EXISTS idx_deadlines_client_visible
    ON deadlines(is_client_visible, due_date);

-- ---------------------------------------------------------------------------
-- portal_users: who may log into the client portal
-- ---------------------------------------------------------------------------
-- A portal user is a person, not a company. One client may have several
-- (a founder and a company secretary); each maps to exactly one client_id.
-- Cross-client access is not representable here, deliberately.
CREATE TABLE IF NOT EXISTS portal_users (
    id            TEXT PRIMARY KEY,
    client_id     TEXT NOT NULL REFERENCES clients(id) ON DELETE CASCADE,

    full_name     TEXT NOT NULL,
    email         TEXT NOT NULL,          -- OTP destination
    phone         TEXT,                   -- OTP destination (SMS), optional

    status        TEXT NOT NULL DEFAULT 'Invited'
                      CHECK (status IN ('Invited','Active','Suspended','Revoked')),

    invited_by    TEXT NOT NULL REFERENCES users(id),
    invited_at    DATETIME NOT NULL DEFAULT (datetime('now')),
    -- Reported back by the inbound sync; the desktop never observes a login itself.
    last_login_at DATETIME,

    created_at    DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at    DATETIME NOT NULL DEFAULT (datetime('now'))
);

-- One portal identity per email address across the whole firm, so an address
-- can never resolve to two clients. Mirrored by a UNIQUE constraint in PostgreSQL.
CREATE UNIQUE INDEX IF NOT EXISTS idx_portal_users_email  ON portal_users(email);
CREATE INDEX        IF NOT EXISTS idx_portal_users_client ON portal_users(client_id);

-- ---------------------------------------------------------------------------
-- sync_outbox: pending outbound changes
-- ---------------------------------------------------------------------------
-- Every write to a syncable entity enqueues a row here. The sync engine drains
-- it in created_at order and clears on server ack, which makes sync resumable
-- after the laptop closes mid-push and gives SyncStatus.pending_changes an
-- honest number.
CREATE TABLE IF NOT EXISTS sync_outbox (
    id          TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL
                    CHECK (entity_type IN ('Matter','Deadline','IpAsset','Document',
                                           'Invoice','Payment','PortalUser','Notification')),
    entity_id   TEXT NOT NULL,

    -- Delete is a tombstone: un-sharing a document must actually remove it from
    -- the mirror, not merely stop refreshing it.
    op          TEXT NOT NULL DEFAULT 'Upsert'
                    CHECK (op IN ('Upsert','Delete')),

    attempts    INTEGER NOT NULL DEFAULT 0,
    last_error  TEXT,

    created_at  DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at  DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_sync_outbox_created ON sync_outbox(created_at);
CREATE INDEX IF NOT EXISTS idx_sync_outbox_entity  ON sync_outbox(entity_type, entity_id);

-- ---------------------------------------------------------------------------
-- client_uploads: inbound queue mirror, for audit
-- ---------------------------------------------------------------------------
-- The server never writes to this database. The desktop pulls pending uploads,
-- validates them, writes the bytes into the vault, and records the outcome here.
CREATE TABLE IF NOT EXISTS client_uploads (
    id               TEXT PRIMARY KEY,    -- matches the PostgreSQL upload id
    client_id        TEXT NOT NULL REFERENCES clients(id) ON DELETE CASCADE,
    -- NULL when the client did not attribute the upload to a matter.
    matter_id        TEXT REFERENCES matters(id),
    -- Set once the bytes are in the vault.
    document_id      TEXT REFERENCES documents(id),

    filename         TEXT NOT NULL,       -- as supplied by the client, untrusted
    status           TEXT NOT NULL DEFAULT 'Pending'
                         CHECK (status IN ('Pending','Ingested','Rejected')),
    rejection_reason TEXT,

    uploaded_at      DATETIME NOT NULL,   -- client-side timestamp from the portal
    ingested_at      DATETIME,

    created_at       DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at       DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_client_uploads_status ON client_uploads(status, uploaded_at);
CREATE INDEX IF NOT EXISTS idx_client_uploads_client ON client_uploads(client_id);

-- ---------------------------------------------------------------------------
-- sync_state: single-row bookkeeping
-- ---------------------------------------------------------------------------
-- is_enabled defaults to 0: a firm with no server configured keeps working
-- exactly as it does today, and nothing leaves the machine until someone
-- deliberately turns sync on.
CREATE TABLE IF NOT EXISTS sync_state (
    id             INTEGER PRIMARY KEY CHECK (id = 1),
    is_enabled     INTEGER NOT NULL DEFAULT 0,
    server_url     TEXT,
    last_pushed_at DATETIME,
    last_pulled_at DATETIME,
    last_error     TEXT,
    updated_at     DATETIME NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO sync_state (id) VALUES (1);
