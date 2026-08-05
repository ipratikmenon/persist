-- Migration 0010: Cascade engine + abandonment escalations
-- Phase 1 Module 2 extended — see specs/module-02-docketing.md §2.9, §2.11
-- Never edit this file. Create new migrations for all changes.
--
-- Two additions:
--   1. cascade_templates — statutory deadline chains as DATA, not Rust. When the
--      Patents Act fee schedule or a rule changes, that is a DB update and a new
--      last_verified date, not a code change and a release.
--   2. deadline_escalations — the audit trail of abandonment warnings. A missed
--      statutory IP deadline is usually irreversible, so every warning that was
--      raised (and who it went to) has to be provable after the fact.

-- ---------------------------------------------------------------------------
-- cascade_templates
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS cascade_templates (
    id                TEXT PRIMARY KEY,

    -- The event that starts the chain, e.g. 'TMApplication', 'TMExaminationReport'.
    -- Registry-triggered events get their own template rather than being guessed
    -- from the filing date: the firm cannot know when an examination report will
    -- issue, so that chain is generated when the report actually arrives.
    anchor_event_type TEXT NOT NULL,

    ip_type           TEXT NOT NULL
                          CHECK (ip_type IN ('Trademark','Patent','Design','Copyright','PlantVariety')),
    jurisdiction      TEXT NOT NULL DEFAULT 'India',

    -- JSON array of rule objects. See services/cascade_engine.rs for the shape.
    template_json     TEXT NOT NULL,

    -- When the statutory rules in template_json were last confirmed against the
    -- Act. Surfaced in the UI so a stale template is visible, not silent.
    last_verified     DATE NOT NULL,
    notes             TEXT,

    created_at        DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at        DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_cascade_templates_anchor
    ON cascade_templates(anchor_event_type, ip_type, jurisdiction);

-- ---------------------------------------------------------------------------
-- deadlines: rebuild to admit the 'Missed' status and carry cascade linkage
-- ---------------------------------------------------------------------------
-- SQLite cannot alter a CHECK constraint, so the table is rebuilt. 'Missed' is a
-- terminal business state — the deadline passed without action — and is
-- genuinely different from urgency 'Overdue', which is only a visual signal.
--
-- Safe to rebuild: no other table references deadlines. cascade_root_id is a
-- plain TEXT column rather than a self-FK, deliberately: a self-referencing FK
-- would have to be re-pointed during the rename dance for no enforcement value
-- the engine does not already provide.

CREATE TABLE deadlines_rebuilt (
    id                  TEXT PRIMARY KEY,
    matter_id           TEXT NOT NULL REFERENCES matters(id) ON DELETE CASCADE,
    ip_asset_id         TEXT REFERENCES ip_assets(id),

    docketing_event     TEXT NOT NULL,

    event_type          TEXT NOT NULL DEFAULT 'Custom'
                            CHECK (event_type IN ('Statutory','Procedural','Custom')),

    due_date            DATE NOT NULL,

    -- 'Missed' added here. Reached only by abandonment_watcher, never by a user.
    status              TEXT NOT NULL DEFAULT 'Pending'
                            CHECK (status IN ('Pending','Complete','Waived','Missed')),

    urgency             TEXT NOT NULL DEFAULT 'Normal'
                            CHECK (urgency IN ('Overdue','Critical','Warning','Normal')),

    notes               TEXT,
    completed_at        DATETIME,
    completed_by        TEXT,

    is_client_visible   INTEGER NOT NULL DEFAULT 0,

    -- Cascade linkage (spec §2.9). cascade_root_id is the anchor deadline that
    -- spawned this one; NULL for hand-entered deadlines.
    cascade_root_id     TEXT,
    cascade_template_id TEXT REFERENCES cascade_templates(id),

    created_at          DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at          DATETIME NOT NULL DEFAULT (datetime('now'))
);

-- ---------------------------------------------------------------------------
-- Copy legacy rows, then swap
-- ---------------------------------------------------------------------------
INSERT INTO deadlines_rebuilt (
    id, matter_id, ip_asset_id, docketing_event, event_type, due_date,
    status, urgency, notes, completed_at, completed_by, is_client_visible,
    created_at, updated_at
)
SELECT
    id, matter_id, ip_asset_id, docketing_event, event_type, due_date,
    status, urgency, notes, completed_at, completed_by, is_client_visible,
    created_at, updated_at
FROM deadlines;

DROP TABLE deadlines;
ALTER TABLE deadlines_rebuilt RENAME TO deadlines;

CREATE INDEX IF NOT EXISTS idx_deadlines_matter         ON deadlines(matter_id);
CREATE INDEX IF NOT EXISTS idx_deadlines_due            ON deadlines(due_date);
CREATE INDEX IF NOT EXISTS idx_deadlines_status         ON deadlines(status);
CREATE INDEX IF NOT EXISTS idx_deadlines_urgency        ON deadlines(urgency);
CREATE INDEX IF NOT EXISTS idx_deadlines_ip_asset       ON deadlines(ip_asset_id);
CREATE INDEX IF NOT EXISTS idx_deadlines_client_visible ON deadlines(is_client_visible, due_date);
CREATE INDEX IF NOT EXISTS idx_deadlines_cascade_root   ON deadlines(cascade_root_id);

-- ---------------------------------------------------------------------------
-- deadline_escalations
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS deadline_escalations (
    id                    TEXT PRIMARY KEY,
    deadline_id           TEXT NOT NULL REFERENCES deadlines(id) ON DELETE CASCADE,

    -- 1 = 14 days out, 2 = 7 days, 3 = 3 days, 4 = missed
    escalation_level      INTEGER NOT NULL CHECK (escalation_level BETWEEN 1 AND 4),

    triggered_at          DATETIME NOT NULL DEFAULT (datetime('now')),
    notified_user_ids     TEXT NOT NULL DEFAULT '[]',   -- JSON array
    notification_channels TEXT NOT NULL DEFAULT '["in_app"]',  -- JSON array

    resolution_action     TEXT,
    resolved_at           DATETIME,
    resolved_by           TEXT REFERENCES users(id)
);

-- One escalation per level per deadline: the watcher runs every 30 minutes and
-- must not raise the same warning 48 times a day.
CREATE UNIQUE INDEX IF NOT EXISTS idx_escalations_unique
    ON deadline_escalations(deadline_id, escalation_level);
CREATE INDEX IF NOT EXISTS idx_escalations_open
    ON deadline_escalations(resolved_at, escalation_level);

-- ---------------------------------------------------------------------------
-- Seed: Phase 1 Indian statutory cascade templates
-- ---------------------------------------------------------------------------
-- Rule shape (services/cascade_engine.rs::DeadlineRule):
--   event_name            display name of the generated deadline
--   event_type            Statutory | Procedural
--   offset                integer, with offset_unit days|months|years
--   internal_buffer_days  if > 0, also generate a Procedural deadline this many
--                         days earlier, so the firm works to its own date
--   client_visible        whether the client sees it in the portal
--   repeat_years          optional: generate one deadline per year from `offset`
--                         to this bound (patent annuities)
--
-- Sources cited in notes. last_verified is deliberately explicit — these are
-- statutory periods and a stale value must be visible.

INSERT OR IGNORE INTO cascade_templates
    (id, anchor_event_type, ip_type, jurisdiction, template_json, last_verified, notes)
VALUES
(
    'tpl-tm-application-in',
    'TMApplication',
    'Trademark',
    'India',
    '[
      {"event_name":"Expect examination report","event_type":"Procedural",
       "offset":12,"offset_unit":"months","internal_buffer_days":0,"client_visible":0},
      {"event_name":"Trademark renewal due (10-year term)","event_type":"Statutory",
       "offset":10,"offset_unit":"years","internal_buffer_days":90,"client_visible":1},
      {"event_name":"Renewal grace period expires (with surcharge)","event_type":"Statutory",
       "offset":126,"offset_unit":"months","internal_buffer_days":30,"client_visible":1}
    ]',
    '2026-04-01',
    'Trade Marks Act 1999. Renewal 10 years from filing; 6-month grace with surcharge (126 months = 10y6m).'
),
(
    'tpl-tm-exam-report-in',
    'TMExaminationReport',
    'Trademark',
    'India',
    '[
      {"event_name":"Response to Examination Report","event_type":"Statutory",
       "offset":30,"offset_unit":"days","internal_buffer_days":7,"client_visible":1}
    ]',
    '2026-04-01',
    'Trade Marks Rules 2017, Rule 45 — 30 days from date of notice.'
),
(
    'tpl-tm-advertised-in',
    'TMAdvertised',
    'Trademark',
    'India',
    '[
      {"event_name":"Opposition period expires","event_type":"Statutory",
       "offset":4,"offset_unit":"months","internal_buffer_days":14,"client_visible":1}
    ]',
    '2026-04-01',
    'Trade Marks Act 1999 s.21 — 4 months from date of advertisement in the Journal.'
),
(
    'tpl-patent-application-in',
    'PatentApplication',
    'Patent',
    'India',
    '[
      {"event_name":"Request for Examination (RFE) due","event_type":"Statutory",
       "offset":48,"offset_unit":"months","internal_buffer_days":30,"client_visible":1},
      {"event_name":"Annuity due","event_type":"Statutory",
       "offset":2,"offset_unit":"years","internal_buffer_days":30,"client_visible":1,
       "repeat_years":20}
    ]',
    '2026-04-01',
    'Patents Act 1970. RFE within 48 months of priority (Rule 24B). Annuities from the 2nd anniversary through year 20.'
),
(
    'tpl-patent-fer-in',
    'PatentFER',
    'Patent',
    'India',
    '[
      {"event_name":"Response to First Examination Report","event_type":"Statutory",
       "offset":12,"offset_unit":"months","internal_buffer_days":45,"client_visible":1}
    ]',
    '2026-04-01',
    'Patents Act 1970 — 12 months from date of FER issuance to place the application in order.'
),
(
    'tpl-design-application-in',
    'DesignApplication',
    'Design',
    'India',
    '[
      {"event_name":"Design renewal due (Form 6)","event_type":"Statutory",
       "offset":10,"offset_unit":"years","internal_buffer_days":60,"client_visible":1},
      {"event_name":"Second renewal / maximum term expires","event_type":"Statutory",
       "offset":15,"offset_unit":"years","internal_buffer_days":60,"client_visible":1}
    ]',
    '2026-04-01',
    'Designs Act 2000 — initial 10 years from registration, extendable by 5 (maximum term 15 years).'
),
(
    'tpl-copyright-registration-in',
    'CopyrightRegistration',
    'Copyright',
    'India',
    '[
      {"event_name":"Await Registrar objections window","event_type":"Procedural",
       "offset":30,"offset_unit":"days","internal_buffer_days":0,"client_visible":0}
    ]',
    '2026-04-01',
    'Copyright Act 1957 — no prosecution chain; registration is largely administrative.'
);
