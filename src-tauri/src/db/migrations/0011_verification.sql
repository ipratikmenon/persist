-- Migration 0011: Dual verification + docket reference numbers + error log
-- Phase 1 Module 2 extended — see specs/module-02-docketing.md §2.12, §2.17
-- Never edit this file. Create new migrations for all changes.
--
-- Three additions, all about accountability for statutory dates:
--   1. reference_number — P&P-DD-NNNN, so a deadline can be cited in
--      correspondence and found again.
--   2. dual verification — a second attorney confirms every statutory date.
--      The person who entered it cannot be the person who checks it; that is
--      the entire point of the control, and it is enforced in Keel.
--   3. docket_errors — a permanent record of what went wrong. A missed
--      statutory deadline is usually irreversible, so the firm needs the
--      history, not just the current state.

-- ---------------------------------------------------------------------------
-- deadlines: authorship, reference, verification
-- ---------------------------------------------------------------------------
-- created_by is needed before verification can mean anything: without knowing
-- who entered a date, "someone else checked it" is unprovable.
ALTER TABLE deadlines ADD COLUMN created_by       TEXT REFERENCES users(id);
ALTER TABLE deadlines ADD COLUMN reference_number TEXT;
ALTER TABLE deadlines ADD COLUMN is_verified      INTEGER NOT NULL DEFAULT 0;
ALTER TABLE deadlines ADD COLUMN verified_by      TEXT REFERENCES users(id);
ALTER TABLE deadlines ADD COLUMN verified_at      DATETIME;

CREATE UNIQUE INDEX IF NOT EXISTS idx_deadlines_reference
    ON deadlines(reference_number) WHERE reference_number IS NOT NULL;

-- Finding the unverified statutory dates is the daily question this control
-- exists to answer, so it gets its own index.
CREATE INDEX IF NOT EXISTS idx_deadlines_unverified
    ON deadlines(is_verified, event_type, due_date);

-- ---------------------------------------------------------------------------
-- docket_errors
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS docket_errors (
    id            TEXT PRIMARY KEY,
    deadline_id   TEXT REFERENCES deadlines(id) ON DELETE SET NULL,
    matter_id     TEXT REFERENCES matters(id)   ON DELETE SET NULL,

    error_type    TEXT NOT NULL
                      CHECK (error_type IN ('Missed','WrongDate','Duplicate',
                                            'UnverifiedFiling','TemplateError','Other')),

    -- What happened, in the attorney's words. Not optional: an error log with
    -- no description is a row that teaches nobody anything.
    description   TEXT NOT NULL,

    -- Filled in later, once the firm knows what it is changing as a result.
    remediation   TEXT,
    severity      TEXT NOT NULL DEFAULT 'Medium'
                      CHECK (severity IN ('Low','Medium','High','Critical')),

    detected_by   TEXT REFERENCES users(id),   -- NULL when raised by a watcher
    detected_at   DATETIME NOT NULL DEFAULT (datetime('now')),
    resolved_at   DATETIME,
    resolved_by   TEXT REFERENCES users(id),

    created_at    DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at    DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_docket_errors_open     ON docket_errors(resolved_at, severity);
CREATE INDEX IF NOT EXISTS idx_docket_errors_matter   ON docket_errors(matter_id);
CREATE INDEX IF NOT EXISTS idx_docket_errors_deadline ON docket_errors(deadline_id);
