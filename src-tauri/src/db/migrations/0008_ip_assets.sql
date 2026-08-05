-- Migration 0008: IP Assets (resolves B02)
-- Phase 1 Module 2 extended — see specs/module-02-docketing.md §"ip_assets table"
-- Never edit this file. Create new migrations for all changes.
--
-- Until now a deadline hung off a matter only. A matter can hold several IP
-- assets (a brand filed in four Nice classes, a patent plus its divisionals),
-- and renewal/annuity dates belong to the asset, not the matter. This adds the
-- asset record and links deadlines to it.

CREATE TABLE IF NOT EXISTS ip_assets (
    id                    TEXT PRIMARY KEY,
    matter_id             TEXT NOT NULL REFERENCES matters(id) ON DELETE CASCADE,

    asset_type            TEXT NOT NULL
        CHECK (asset_type IN ('Trademark','Patent','Design','Copyright','PlantVariety')),

    -- Mark name, invention title, or work title.
    title                 TEXT NOT NULL,

    -- Registry identifiers. application_number is assigned at filing;
    -- registration_number only exists once registered/granted.
    application_number    TEXT,
    registration_number   TEXT,

    filing_date           DATE,
    priority_date         DATE,   -- Paris Convention / PCT priority
    grant_date            DATE,
    registration_date     DATE,
    expiry_date           DATE,   -- next renewal falls due on this date

    applicant_entity_type TEXT NOT NULL DEFAULT 'Company'
        CHECK (applicant_entity_type IN ('Individual','Startup','SmallEntity','Company','Government')),

    jurisdiction          TEXT NOT NULL DEFAULT 'India',

    -- JSON array of Nice (TM) or Locarno (Design) class numbers, e.g. '[9,42]'.
    classes               TEXT NOT NULL DEFAULT '[]',

    status                TEXT NOT NULL DEFAULT 'Pending'
        CHECK (status IN ('Pending','Examination','Accepted','Advertised','Opposed',
                          'Registered','Granted','Lapsed','Abandoned','Cancelled')),

    notes                 TEXT,
    created_at            DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at            DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_ip_assets_matter ON ip_assets(matter_id);
CREATE INDEX IF NOT EXISTS idx_ip_assets_type   ON ip_assets(asset_type, status);
CREATE INDEX IF NOT EXISTS idx_ip_assets_expiry ON ip_assets(expiry_date);

-- Link deadlines to a specific asset. Nullable: matter-level deadlines
-- (client meetings, internal reviews) legitimately have no asset.
ALTER TABLE deadlines ADD COLUMN ip_asset_id TEXT REFERENCES ip_assets(id);

CREATE INDEX IF NOT EXISTS idx_deadlines_ip_asset ON deadlines(ip_asset_id);
