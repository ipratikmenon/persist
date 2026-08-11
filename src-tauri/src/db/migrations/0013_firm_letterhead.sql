-- Phase 4 Module 9: the firm's identity as it prints on a letter.
--
-- firm_settings held the firm's *billing* identity — GSTIN, PAN, bank account,
-- hourly rates — and nothing that appears on correspondence. The Legal Notice
-- template declares the letterhead as `computed` fields, meaning Keel assembles
-- them; Keel could not, because there was nowhere to assemble them from. Every
-- notice therefore came out with a blank header: no partner names, no numbers,
-- no website, no registered office in the footer, and no signature block.
--
-- This migration gives those values a home and seeds the firm's real ones, so
-- the app produces a serveable notice out of the box rather than after someone
-- fills in a settings form nobody told them about.

-- ---------------------------------------------------------------------------
-- firm_settings — the four letterhead values that belong to the firm itself
-- ---------------------------------------------------------------------------
-- Website, contact and the two registered-office lines are single-valued and
-- genuinely firm-level, so they are columns on the existing single row.
--
-- The office is two columns rather than one multi-line TEXT because the footer
-- sets them as two centred lines of different weight; splitting a stored
-- address on newlines would put the layout at the mercy of how someone typed
-- it into a textarea.
ALTER TABLE firm_settings ADD COLUMN firm_website          TEXT;
ALTER TABLE firm_settings ADD COLUMN firm_contact_email    TEXT;
ALTER TABLE firm_settings ADD COLUMN firm_office_line_one  TEXT;
ALTER TABLE firm_settings ADD COLUMN firm_office_line_two  TEXT;

UPDATE firm_settings SET
    firm_website         = 'www.persistas.com',
    firm_contact_email   = 'persistas.pnp@outlook.com',
    firm_office_line_one = '80-A, Pocket-A, Mayuri Enclave,',
    firm_office_line_two = 'Mayur Vihar Phase-III, Delhi - 110096'
WHERE id = 1;

-- ---------------------------------------------------------------------------
-- firm_partners — the partners as they appear on the letterhead
-- ---------------------------------------------------------------------------
-- WHY A TABLE AND NOT COLUMNS ON firm_settings
--
-- The obvious cheap thing is partner_one_name … partner_two_email: eight
-- columns on the single settings row. It was rejected for three reasons.
--
--   1. The firm has two partners today. A third joining would then be a
--      migration, eight more columns, and every read site learning that
--      "partner three" exists. As rows it is an INSERT.
--   2. A partner is not an attribute of the firm. An enrolment number belongs
--      to the individual and appears on everything they sign; a phone number is
--      theirs. Flattening them onto firm_settings says otherwise.
--   3. The signature block has to identify *which* partner signed. That needs a
--      link to the user who is logged in, and a link points at a row.
--
-- The letterhead is not limited to two: it reads the roster in sort_order and
-- fills as many partner slots as the template has places for. That the current
-- template happens to have two is a fact about the template, not the schema.
CREATE TABLE IF NOT EXISTS firm_partners (
    id               TEXT    PRIMARY KEY,
    -- The login this partner signs in with, where they have one. Nullable: a
    -- name on the letterhead does not have to be a user of the software, and a
    -- partner who leaves the app but not the firm should not vanish from it.
    user_id          TEXT    UNIQUE REFERENCES users(id),
    name             TEXT    NOT NULL,
    -- "Advocate & Partner". Printed under the name, so it is stored as it
    -- should read rather than derived from users.role — the letterhead says
    -- what the profession calls them, not what the RBAC matrix calls them.
    role             TEXT    NOT NULL DEFAULT 'Advocate & Partner',
    phone            TEXT,
    email            TEXT,
    -- Bar Council enrolment, e.g. D/6361/2020. On the signature block because a
    -- notice served without it invites a challenge to the signatory's standing.
    enrolment_number TEXT,
    -- Position on the letterhead. The senior partner prints first, and that
    -- order is a decision the firm makes, not one alphabetical order makes.
    sort_order       INTEGER NOT NULL DEFAULT 0,
    is_active        INTEGER NOT NULL DEFAULT 1,
    created_at       DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at       DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_firm_partners_order ON firm_partners(sort_order);

-- The firm's two partners, exactly as they appear on the notices this format
-- was modelled on. Seeded here so a fresh install produces a document that
-- could actually be served.
INSERT OR IGNORE INTO firm_partners
    (id, name, role, phone, email, enrolment_number, sort_order)
VALUES
    ('partner-slm', 'Sreelakshmi Menon', 'Advocate & Partner',
     '+91 99535 31789', 'sreelakshmimenon.pnp@outlook.com', 'D/6361/2020', 1),
    ('partner-kt',  'Kajal Thakur',      'Advocate & Partner',
     '+91 93153 67642', 'kajalthakur.pnp@outlook.com',      NULL,          2);

-- Link each partner to their login where it already exists.
--
-- On a fresh install it does not: users are seeded from Rust at first launch,
-- after migrations have run, so these updates match nothing and the link stays
-- NULL. That is handled — services/firm.rs falls back to matching the signed-in
-- user by name, ignoring case and spacing, because the letterhead spells it
-- "Sreelakshmi Menon" and the login spells it "Sree Lakshmi Menon" and they are
-- the same person. The EXISTS guard is what keeps the foreign key satisfied on
-- an empty users table.
UPDATE firm_partners SET user_id = 'user-slm'
 WHERE id = 'partner-slm' AND EXISTS (SELECT 1 FROM users WHERE id = 'user-slm');
UPDATE firm_partners SET user_id = 'user-kt'
 WHERE id = 'partner-kt'  AND EXISTS (SELECT 1 FROM users WHERE id = 'user-kt');
