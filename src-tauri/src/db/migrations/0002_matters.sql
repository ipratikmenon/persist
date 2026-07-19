-- Migration 0002: Matter Management
-- Phase 1 Module 1
-- Never edit this file. Create new migrations for all changes.

-- sequences: used for P&P-YYYY-TYPE-NNNN Matter ID generation.
-- key format: 'TM-2026', 'PAT-2026', etc.
CREATE TABLE IF NOT EXISTS sequences (
    key       TEXT PRIMARY KEY,
    next_val  INTEGER NOT NULL DEFAULT 1
);

-- clients: the firm's client roster.
CREATE TABLE IF NOT EXISTS clients (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    type        TEXT NOT NULL DEFAULT 'Individual'
                    CHECK (type IN ('Individual','Company','Partnership','Trust','Other')),
    email       TEXT,
    phone       TEXT,
    address     TEXT,
    gstin       TEXT,
    pan         TEXT,
    notes       TEXT,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at  DATETIME NOT NULL DEFAULT (datetime('now'))
);

-- matters: every piece of legal work is a Matter.
CREATE TABLE IF NOT EXISTS matters (
    id                      TEXT PRIMARY KEY,   -- P&P-YYYY-TYPE-NNNN
    client_id               TEXT NOT NULL REFERENCES clients(id),
    title                   TEXT NOT NULL,
    matter_type             TEXT NOT NULL
                                CHECK (matter_type IN (
                                    'Trademark','Patent','Design','Copyright',
                                    'Corporate','Litigation','Paralegal'
                                )),
    sub_type                TEXT,
    status                  TEXT NOT NULL DEFAULT 'Active'
                                CHECK (status IN (
                                    'Active','OnHold','PendingClientResponse',
                                    'Closed','Archived'
                                )),
    priority                TEXT NOT NULL DEFAULT 'Normal'
                                CHECK (priority IN ('Normal','High','Urgent')),
    responsible_partner_id  TEXT,               -- FK to users(id) — enforced after auth migration
    forum                   TEXT,
    jurisdiction            TEXT NOT NULL DEFAULT 'India',
    opened_date             DATE NOT NULL,
    target_close_date       DATE,
    internal_notes          TEXT,               -- NEVER synced to client portal
    client_notes            TEXT,
    tags                    TEXT NOT NULL DEFAULT '[]',
    linked_matter_ids       TEXT NOT NULL DEFAULT '[]',
    created_at              DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at              DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_matters_client  ON matters(client_id);
CREATE INDEX IF NOT EXISTS idx_matters_status  ON matters(status);
CREATE INDEX IF NOT EXISTS idx_matters_updated ON matters(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_matters_type    ON matters(matter_type);

-- matter_parties: attorneys and staff assigned to a matter.
CREATE TABLE IF NOT EXISTS matter_parties (
    id          TEXT PRIMARY KEY,
    matter_id   TEXT NOT NULL REFERENCES matters(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL,                  -- FK to users(id) — enforced after auth migration
    role        TEXT NOT NULL
                    CHECK (role IN ('Partner','Associate','Paralegal','Admin')),
    is_primary  INTEGER NOT NULL DEFAULT 0,
    added_at    DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_matter_parties_unique ON matter_parties(matter_id, user_id);
CREATE INDEX IF NOT EXISTS idx_matter_parties_matter ON matter_parties(matter_id);
