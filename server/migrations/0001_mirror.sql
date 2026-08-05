-- Migration 0001: PostgreSQL mirror + inbound queue
-- Persist sync server — see specs/module-05-portal.md §7
--
-- TWO SCHEMAS, TWO DIRECTIONS:
--
--   mirror.*   a projection of desktop SQLite. Written ONLY by the sync server
--              applying a desktop push. Read-only to the portal.
--
--   inbound.*  client-authored items awaiting ingest by the desktop. Written by
--              the portal, read and acked by the desktop. Never applied to firm
--              data by this server.
--
-- Desktop SQLite remains the source of truth. Nothing here is authoritative.
--
-- Every mirror table carries client_id — including where it is denormalised
-- (deadlines, payments) — because the RLS policies in 0002_rls.sql must scope a
-- row without joining. A join in a policy is a policy that can be tricked.

CREATE SCHEMA IF NOT EXISTS mirror;
CREATE SCHEMA IF NOT EXISTS inbound;

-- ===========================================================================
-- mirror — the client-visible projection
-- ===========================================================================

CREATE TABLE mirror.clients (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    -- NOT mirrored: notes (internal), gstin, pan, address, phone, email.
    -- The portal has no need for them and they are commercially sensitive.
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE mirror.portal_users (
    id            TEXT PRIMARY KEY,
    client_id     TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,
    full_name     TEXT NOT NULL,
    email         TEXT NOT NULL UNIQUE,
    phone         TEXT,
    status        TEXT NOT NULL DEFAULT 'Invited'
                      CHECK (status IN ('Invited','Active','Suspended','Revoked')),
    last_login_at TIMESTAMPTZ,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_mirror_portal_users_client ON mirror.portal_users(client_id);
CREATE INDEX idx_mirror_portal_users_email  ON mirror.portal_users(lower(email));

CREATE TABLE mirror.matters_public (
    id                   TEXT PRIMARY KEY,
    client_id            TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,
    title                TEXT NOT NULL,
    matter_type          TEXT NOT NULL,
    status               TEXT NOT NULL,
    opened_date          DATE NOT NULL,
    forum                TEXT,
    jurisdiction         TEXT NOT NULL DEFAULT 'India',
    -- The client-facing note. matters.internal_notes is NEVER projected here.
    client_notes         TEXT,
    -- Display name only — never a user id, which would leak firm structure.
    responsible_attorney TEXT,
    next_deadline_date   DATE,
    next_deadline_event  TEXT,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_mirror_matters_client ON mirror.matters_public(client_id);

CREATE TABLE mirror.deadlines_public (
    id              TEXT PRIMARY KEY,
    client_id       TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,
    matter_id       TEXT NOT NULL REFERENCES mirror.matters_public(id) ON DELETE CASCADE,
    docketing_event TEXT NOT NULL,
    due_date        DATE NOT NULL,
    status          TEXT NOT NULL,
    -- NOT mirrored: deadlines.notes — it routinely holds strategy
    -- ("weak prior art, consider opposing"). The client sees event and date.
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_mirror_deadlines_client ON mirror.deadlines_public(client_id);
CREATE INDEX idx_mirror_deadlines_matter ON mirror.deadlines_public(matter_id, due_date);

CREATE TABLE mirror.ip_assets_public (
    id                  TEXT PRIMARY KEY,
    client_id           TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,
    matter_id           TEXT NOT NULL REFERENCES mirror.matters_public(id) ON DELETE CASCADE,
    asset_type          TEXT NOT NULL,
    title               TEXT NOT NULL,
    application_number  TEXT,
    registration_number TEXT,
    filing_date         DATE,
    registration_date   DATE,
    expiry_date         DATE,
    status              TEXT NOT NULL,
    classes             JSONB NOT NULL DEFAULT '[]'::jsonb,
    jurisdiction        TEXT NOT NULL DEFAULT 'India',
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_mirror_ip_assets_client ON mirror.ip_assets_public(client_id);
CREATE INDEX idx_mirror_ip_assets_matter ON mirror.ip_assets_public(matter_id);

CREATE TABLE mirror.documents_shared (
    id              TEXT PRIMARY KEY,
    client_id       TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,
    matter_id       TEXT NOT NULL REFERENCES mirror.matters_public(id) ON DELETE CASCADE,
    filename        TEXT NOT NULL,
    category        TEXT NOT NULL,
    mime_type       TEXT NOT NULL,
    file_size_bytes BIGINT NOT NULL,
    version         INTEGER NOT NULL DEFAULT 1,
    description     TEXT,
    -- Object storage key. NEVER returned to a browser — the API issues a
    -- 5-minute signed URL per request instead.
    object_key      TEXT NOT NULL,
    -- Integrity check on download; also proves the object matches what the
    -- desktop cleaned and uploaded.
    sha256          TEXT NOT NULL,
    shared_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_mirror_documents_client ON mirror.documents_shared(client_id);
CREATE INDEX idx_mirror_documents_matter ON mirror.documents_shared(matter_id);

CREATE TABLE mirror.invoices_public (
    id             TEXT PRIMARY KEY,
    client_id      TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,
    -- Drafts are excluded at projection time, not filtered here. A draft invoice
    -- must never reach this table at all.
    status         TEXT NOT NULL CHECK (status <> 'Draft'),
    invoice_date   DATE NOT NULL,
    due_date       DATE,
    -- NUMERIC, not float: desktop stores REAL, but an invoice total shown to a
    -- client must never drift by floating-point noise. Rounded on projection.
    subtotal       NUMERIC(14,2) NOT NULL DEFAULT 0,
    cgst_amount    NUMERIC(14,2) NOT NULL DEFAULT 0,
    sgst_amount    NUMERIC(14,2) NOT NULL DEFAULT 0,
    igst_amount    NUMERIC(14,2) NOT NULL DEFAULT 0,
    total_with_tax NUMERIC(14,2) NOT NULL DEFAULT 0,
    amount_paid    NUMERIC(14,2) NOT NULL DEFAULT 0,
    notes          TEXT,
    pdf_object_key TEXT,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_mirror_invoices_client ON mirror.invoices_public(client_id);

CREATE TABLE mirror.payments_public (
    id           TEXT PRIMARY KEY,
    client_id    TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,
    invoice_id   TEXT NOT NULL REFERENCES mirror.invoices_public(id) ON DELETE CASCADE,
    amount       NUMERIC(14,2) NOT NULL,
    payment_date DATE NOT NULL,
    method       TEXT NOT NULL,
    reference    TEXT,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_mirror_payments_client  ON mirror.payments_public(client_id);
CREATE INDEX idx_mirror_payments_invoice ON mirror.payments_public(invoice_id);

CREATE TABLE mirror.client_notifications (
    id         TEXT PRIMARY KEY,
    client_id  TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,
    kind       TEXT NOT NULL
                   CHECK (kind IN ('DocumentShared','DeadlineUpcoming','InvoiceIssued',
                                   'PaymentRecorded','DocumentRequested')),
    title      TEXT NOT NULL,
    body       TEXT,
    matter_id  TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    read_at    TIMESTAMPTZ
);

CREATE INDEX idx_mirror_notifications_client ON mirror.client_notifications(client_id, created_at DESC);

-- ===========================================================================
-- inbound — client-authored, awaiting desktop ingest
-- ===========================================================================

CREATE TABLE inbound.client_uploads (
    id              TEXT PRIMARY KEY,
    client_id       TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,
    portal_user_id  TEXT NOT NULL REFERENCES mirror.portal_users(id) ON DELETE CASCADE,
    matter_id       TEXT,
    filename        TEXT NOT NULL,
    mime_type       TEXT NOT NULL,
    file_size_bytes BIGINT NOT NULL,
    -- Quarantine bucket. Only moves to the vault once the desktop pulls it.
    object_key      TEXT NOT NULL,
    sha256          TEXT NOT NULL,
    scan_status     TEXT NOT NULL DEFAULT 'Pending'
                        CHECK (scan_status IN ('Pending','Clean','Infected','Failed')),
    status          TEXT NOT NULL DEFAULT 'Pending'
                        CHECK (status IN ('Pending','Ingested','Rejected')),
    uploaded_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_inbound_uploads_client  ON inbound.client_uploads(client_id);
CREATE INDEX idx_inbound_uploads_pending ON inbound.client_uploads(status, uploaded_at);

CREATE TABLE inbound.invoice_disputes (
    id             TEXT PRIMARY KEY,
    client_id      TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,
    portal_user_id TEXT NOT NULL REFERENCES mirror.portal_users(id) ON DELETE CASCADE,
    invoice_id     TEXT NOT NULL,
    reason         TEXT NOT NULL,
    status         TEXT NOT NULL DEFAULT 'Pending'
                       CHECK (status IN ('Pending','Ingested')),
    raised_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_inbound_disputes_client  ON inbound.invoice_disputes(client_id);
CREATE INDEX idx_inbound_disputes_pending ON inbound.invoice_disputes(status, raised_at);

-- OTP challenges. Never client-readable — the portal role cannot SELECT here,
-- only the auth path may, via a dedicated function or elevated role.
CREATE TABLE inbound.otp_challenges (
    id          TEXT PRIMARY KEY,
    email       TEXT NOT NULL,
    -- bcrypt hash of the 6-digit code. The plaintext code is never stored.
    code_hash   TEXT NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    attempts    INTEGER NOT NULL DEFAULT 0,
    consumed_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_inbound_otp_email   ON inbound.otp_challenges(lower(email), created_at DESC);
CREATE INDEX idx_inbound_otp_expires ON inbound.otp_challenges(expires_at);
