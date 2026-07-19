-- Migration 0003: Docketing & Deadline Engine
-- Phase 1 Module 2
-- Never edit this file. Create new migrations for all changes.

-- deadlines: every docketing event / statutory deadline for a matter.
CREATE TABLE IF NOT EXISTS deadlines (
    id              TEXT PRIMARY KEY,
    matter_id       TEXT NOT NULL REFERENCES matters(id) ON DELETE CASCADE,

    -- The docketing event name (e.g. "Examination Report Response", "FER Response Deadline")
    docketing_event TEXT NOT NULL,

    -- Statutory = mandated by law / registry rules (TM, Patent, Design)
    -- Procedural = firm internal deadline (file cover letter, client instruction)
    -- Custom     = one-off attorney-set deadline
    event_type      TEXT NOT NULL DEFAULT 'Custom'
                        CHECK (event_type IN ('Statutory','Procedural','Custom')),

    due_date        DATE NOT NULL,

    status          TEXT NOT NULL DEFAULT 'Pending'
                        CHECK (status IN ('Pending','Complete','Waived')),

    -- Recalculated by deadline_watcher every 15 minutes and at query time.
    urgency         TEXT NOT NULL DEFAULT 'Normal'
                        CHECK (urgency IN ('Overdue','Critical','Warning','Normal')),

    notes           TEXT,
    completed_at    DATETIME,
    completed_by    TEXT,           -- FK to users(id) — enforced after auth migration
    created_at      DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_deadlines_matter  ON deadlines(matter_id);
CREATE INDEX IF NOT EXISTS idx_deadlines_due     ON deadlines(due_date);
CREATE INDEX IF NOT EXISTS idx_deadlines_status  ON deadlines(status);
CREATE INDEX IF NOT EXISTS idx_deadlines_urgency ON deadlines(urgency);
