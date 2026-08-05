# Persist — Module 05: Client Portal & Sync
**Phase:** 2
**Status:** Spec — awaiting approval before Step 2 (schema)
**Last updated:** 2026-08-05

---

## 1. Purpose

Give Persistas' clients a browser window into their own matters, documents, and
invoices — without giving them, or the machine serving them, access to anything
privileged.

This module spans three surfaces:

| Surface | Location | Role |
|---|---|---|
| Sync client | `src-tauri/src/commands/sync.rs` + `services/sync_engine.rs` | Pushes the public subset from desktop SQLite; pulls client-originated items back |
| Sync server | `server/` (Axum + PostgreSQL) | Receives pushes, writes the mirror, holds the inbound queue. No business logic |
| Client portal | `portal/backend` (FastAPI) + `portal/frontend` (React) | Authenticates clients, serves their own rows only |

**The governing rule:** desktop SQLite is the source of truth. The mirror is a
*projection* of it, never an authority. Nothing a client does writes directly to
firm data — client actions land in a queue the desktop chooses to ingest.

---

## 2. Scope

**In scope.** View matter status; view upcoming deadlines; download documents the
firm has shared; upload documents the firm has requested; view invoices and
payment history; raise a dispute on an invoice; manage notification preferences.

**Out of scope.** Anything resembling the desktop app with features removed.
No messaging, no matter creation, no document editing, no AI features, no
visibility into other clients, no visibility into firm internals.

Per `portal/PORTAL-RULES.md`: *"IS NOT: A version of the firm desktop app with
features removed."* Do not add beyond this list without explicit instruction.

---

## 3. Users & Roles

| Role | Surface | Can do |
|---|---|---|
| Partner | Desktop | Invite/revoke portal users, share/unshare documents, mark deadlines client-visible, resolve disputes, force sync |
| Associate | Desktop | Share documents, mark deadlines client-visible, respond to disputes |
| Paralegal | Desktop | Request documents from a client; cannot share firm documents outward |
| Portal user | Browser | Read own matters/deadlines/documents/invoices; upload requested documents; raise disputes |

A **portal user** is a person (email + phone), not a company. One client may have
several portal users (a founder and a company secretary); one portal user maps to
exactly one `client_id`. Cross-client access is not representable in the schema —
that is deliberate.

---

## 4. Architecture

```
┌─────────────────────────────┐
│ Persist Desktop (Keel)      │   SOURCE OF TRUTH
│ SQLite + AES-256 vault      │
└──────────┬──────────────────┘
           │  mTLS, client cert pinned at build
           │  outbound: allow-listed projection of changed rows
           │  inbound:  client uploads + disputes awaiting ingest
           ▼
┌─────────────────────────────┐
│ Sync Server (server/, Axum) │   RELAY — no business logic
│ PostgreSQL mirror           │
│ Object storage (documents)  │
└──────────┬──────────────────┘
           │  PostgreSQL reads, RLS-scoped
           ▼
┌─────────────────────────────┐
│ Portal API (FastAPI)        │   JWT 15-min, OTP login
└──────────┬──────────────────┘
           │  HTTPS
           ▼
┌─────────────────────────────┐
│ Portal Frontend (React)     │   4 tabs
└─────────────────────────────┘
```

**Note on the roadmap's §5.1 exception.** `specs/expansion-roadmap.md` establishes
that Track B (SaaS tenant) data will be server-native PostgreSQL. That exception
does **not** apply here. Client portal data is a mirror; desktop remains
authoritative. The two datasets live in separate PostgreSQL schemas
(`mirror.*` and, later, `tenant.*`) and nothing joins across them.

---

## 5. The Sync Contract

### 5.1 Allow-list projection — the core safety property

The sync engine never serialises a desktop row wholesale. Each syncable entity
has an explicit projection function naming every column that leaves the machine.
A new column added to `matters` is invisible to the portal until someone edits
the projection deliberately.

This is the inverse of a deny-list, and it is the single most important rule in
this module: **adding a column must never leak it.**

```rust
// services/sync_engine/projection.rs — the ONLY place desktop rows become
// portal-visible payloads. Reviewers: treat any change here as security-relevant.
fn project_matter(m: &MatterRow) -> MatterPublic { /* named fields only */ }
```

### 5.2 What syncs outbound

| Mirror table | Source | Columns exposed |
|---|---|---|
| `matters_public` | `matters` | id, title, matter_type, status, opened_date, forum, jurisdiction, client_notes, responsible attorney **name**, next deadline (date + event) |
| `deadlines_public` | `deadlines` WHERE `is_client_visible = 1` | id, matter_id, docketing_event, due_date, status |
| `ip_assets_public` | `ip_assets` | id, matter_id, asset_type, title, application_number, registration_number, filing_date, registration_date, expiry_date, status, classes, jurisdiction |
| `documents_shared` | `documents` WHERE `is_shared_with_client = 1` | id, matter_id, filename, category, mime_type, file_size_bytes, version, description, created_at, object_key, sha256 |
| `invoices_public` | `invoices` WHERE `status != 'Draft'` | id, client_id, status, invoice_date, due_date, subtotal, cgst/sgst/igst, total_with_tax, amount_paid, notes, pdf object_key |
| `payments_public` | `payments` | id, invoice_id, amount, payment_date, method, reference |
| `client_notifications` | generated | id, client_id, kind, title, body, matter_id, created_at, read_at |

### 5.3 What NEVER syncs

`internal_notes` · `clients.notes` · `time_entries` (all of it) · billing rates ·
`firm_settings` · draft documents · draft invoices · `matters.tags` ·
`matter_parties` · `deadlines.notes` · `users` (beyond a display name) ·
`sessions` · AI chat history · audit logs · `vault_path` · the vault key.

`deadlines.notes` is on this list on purpose — it routinely holds strategy
("consider opposing, weak prior art"). The client sees the event and the date.

### 5.4 Outbox

Every write to a syncable entity records a row in `sync_outbox` (desktop). The
sync engine batches pending rows, pushes, and clears on server ack. This gives
`SyncStatus.pending_changes` an honest number and makes sync resumable after a
laptop closes mid-push.

Ordering is by `created_at`; a failed batch is retried with exponential backoff
and never silently dropped. Deletes are tombstones (`op = 'Delete'`), so
un-sharing a document actually removes it from the mirror.

### 5.5 Inbound queue

Client-originated items (uploads, disputes) are **never** applied to SQLite by the
server. They sit in PostgreSQL with `status = 'Pending'`. The desktop pulls them,
validates, applies locally, and acks — at which point the server marks them
`Ingested`. If the firm never runs the desktop app, nothing enters firm data.

### 5.6 Conflict policy

Desktop always wins. The mirror is overwritten by the projection on every push;
there is no merge. The only client-authored data lives in the inbound queue,
which the desktop treats as *input*, not as truth.

---

## 6. Desktop Schema Additions — migration `0009_portal_sync.sql`

### `deadlines` — add client visibility

| Column | Type | Notes |
|---|---|---|
| `is_client_visible` | INTEGER NOT NULL DEFAULT 0 | 1 = appears in portal |

**Default rule:** Keel sets `is_client_visible = 1` at creation when
`event_type = 'Statutory'` — a deadline the client is legally affected by and
must not be surprised by. `Procedural` and `Custom` deadlines stay private
(default 0). Either can be toggled by an attorney. Backfill in the migration
applies the same rule to existing rows.

### `portal_users`

Portal identities, managed from the desktop. The desktop owns this table; it
projects to the mirror.

| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | UUID |
| `client_id` | TEXT NOT NULL | FK → `clients(id)` ON DELETE CASCADE |
| `full_name` | TEXT NOT NULL | |
| `email` | TEXT NOT NULL | OTP destination |
| `phone` | TEXT | OTP destination (SMS), optional |
| `status` | TEXT NOT NULL DEFAULT 'Invited' | `Invited \| Active \| Suspended \| Revoked` |
| `invited_by` | TEXT NOT NULL | FK → `users(id)` |
| `invited_at` | DATETIME NOT NULL DEFAULT (datetime('now')) | |
| `last_login_at` | DATETIME | Reported back by the inbound sync |
| `created_at` / `updated_at` | DATETIME NOT NULL DEFAULT (datetime('now')) | |

**Unique index:** `idx_portal_users_email` on `(email)` — one portal identity per
email address across the whole firm, so an address can never resolve to two clients.

### `sync_outbox`

| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | UUID |
| `entity_type` | TEXT NOT NULL | `Matter \| Deadline \| IpAsset \| Document \| Invoice \| Payment \| PortalUser \| Notification` |
| `entity_id` | TEXT NOT NULL | Desktop row id |
| `op` | TEXT NOT NULL | `Upsert \| Delete` |
| `attempts` | INTEGER NOT NULL DEFAULT 0 | Retry counter |
| `last_error` | TEXT | Last failure message |
| `created_at` | DATETIME NOT NULL DEFAULT (datetime('now')) | |

**Index:** `idx_sync_outbox_created` on `(created_at)`

### `client_uploads`

Mirror of the inbound queue, for audit. A row is created when the desktop
ingests a client upload.

| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | Matches the PostgreSQL upload id |
| `client_id` | TEXT NOT NULL | FK → `clients(id)` |
| `matter_id` | TEXT | FK → `matters(id)`, if the client attributed it |
| `document_id` | TEXT | FK → `documents(id)` once ingested into the vault |
| `filename` | TEXT NOT NULL | As supplied by the client |
| `status` | TEXT NOT NULL DEFAULT 'Pending' | `Pending \| Ingested \| Rejected` |
| `rejection_reason` | TEXT | |
| `uploaded_at` | DATETIME NOT NULL | Client-side timestamp from the portal |
| `ingested_at` | DATETIME | |

### `sync_state`

Single-row bookkeeping (`id INTEGER PK DEFAULT 1`): `last_pushed_at`,
`last_pulled_at`, `last_error`, `server_url`, `is_enabled`.

---

## 7. PostgreSQL Mirror Schema (`server/migrations/`)

All mirror tables live in schema `mirror`. Every table carries `client_id` —
including ones where it is denormalised (`deadlines_public`, `payments_public`) —
because RLS policies need it locally without a join.

```sql
CREATE SCHEMA mirror;

CREATE TABLE mirror.clients (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    -- NOTE: clients.notes, gstin, pan are NOT mirrored.
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE mirror.portal_users (
    id            TEXT PRIMARY KEY,
    client_id     TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,
    full_name     TEXT NOT NULL,
    email         TEXT NOT NULL UNIQUE,
    phone         TEXT,
    status        TEXT NOT NULL DEFAULT 'Invited',
    last_login_at TIMESTAMPTZ,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE mirror.matters_public (
    id                    TEXT PRIMARY KEY,
    client_id             TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,
    title                 TEXT NOT NULL,
    matter_type           TEXT NOT NULL,
    status                TEXT NOT NULL,
    opened_date           DATE NOT NULL,
    forum                 TEXT,
    jurisdiction          TEXT NOT NULL DEFAULT 'India',
    client_notes          TEXT,          -- the client-facing note, not internal_notes
    responsible_attorney  TEXT,          -- display name only, never a user id
    next_deadline_date    DATE,
    next_deadline_event   TEXT,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE mirror.deadlines_public (
    id              TEXT PRIMARY KEY,
    client_id       TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,
    matter_id       TEXT NOT NULL REFERENCES mirror.matters_public(id) ON DELETE CASCADE,
    docketing_event TEXT NOT NULL,
    due_date        DATE NOT NULL,
    status          TEXT NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

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
    classes             JSONB NOT NULL DEFAULT '[]',
    jurisdiction        TEXT NOT NULL DEFAULT 'India',
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

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
    object_key      TEXT NOT NULL,   -- object storage key; never a public URL
    sha256          TEXT NOT NULL,   -- integrity check on download
    shared_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE mirror.invoices_public (
    id             TEXT PRIMARY KEY,
    client_id      TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,
    status         TEXT NOT NULL,
    invoice_date   DATE NOT NULL,
    due_date       DATE,
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

CREATE TABLE mirror.client_notifications (
    id         TEXT PRIMARY KEY,
    client_id  TEXT NOT NULL REFERENCES mirror.clients(id) ON DELETE CASCADE,
    kind       TEXT NOT NULL,   -- DocumentShared | DeadlineUpcoming | InvoiceIssued | PaymentRecorded | DocumentRequested
    title      TEXT NOT NULL,
    body       TEXT,
    matter_id  TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    read_at    TIMESTAMPTZ
);
```

**Monetary types.** Desktop stores money as SQLite `REAL`; the mirror uses
`NUMERIC(14,2)`. The sync engine rounds to 2 decimal places on projection. An
invoice total displayed to a client must never drift by floating-point noise.

### Inbound queue tables (client-authored — NOT part of the mirror projection)

```sql
CREATE SCHEMA inbound;

CREATE TABLE inbound.client_uploads (
    id               TEXT PRIMARY KEY,
    client_id        TEXT NOT NULL,
    portal_user_id   TEXT NOT NULL,
    matter_id        TEXT,
    filename         TEXT NOT NULL,
    mime_type        TEXT NOT NULL,
    file_size_bytes  BIGINT NOT NULL,
    object_key       TEXT NOT NULL,       -- quarantine bucket
    sha256           TEXT NOT NULL,
    scan_status      TEXT NOT NULL DEFAULT 'Pending',  -- Pending | Clean | Infected | Failed
    status           TEXT NOT NULL DEFAULT 'Pending',  -- Pending | Ingested | Rejected
    uploaded_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE inbound.invoice_disputes (
    id             TEXT PRIMARY KEY,
    client_id      TEXT NOT NULL,
    portal_user_id TEXT NOT NULL,
    invoice_id     TEXT NOT NULL,
    reason         TEXT NOT NULL,
    status         TEXT NOT NULL DEFAULT 'Pending',  -- Pending | Ingested
    raised_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE inbound.otp_challenges (
    id            TEXT PRIMARY KEY,
    email         TEXT NOT NULL,
    code_hash     TEXT NOT NULL,       -- bcrypt; never the plaintext code
    expires_at    TIMESTAMPTZ NOT NULL,
    attempts      INTEGER NOT NULL DEFAULT 0,
    consumed_at   TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

---

## 8. Row-Level Security

RLS is enabled on **every** `mirror.*` table and every client-readable
`inbound.*` table. Per root CLAUDE.md: enforced at PostgreSQL level, not just
the ORM.

```sql
ALTER TABLE mirror.matters_public ENABLE ROW LEVEL SECURITY;
ALTER TABLE mirror.matters_public FORCE ROW LEVEL SECURITY;

CREATE POLICY client_isolation ON mirror.matters_public
    FOR SELECT
    USING (client_id = current_setting('app.current_client_id', true));
-- repeat for every mirror table
```

**Request flow.** The FastAPI app connects as role `portal_reader`, which has
`SELECT` only and is **not** the table owner (hence `FORCE ROW LEVEL SECURITY`).
On each request, after JWT validation, the middleware issues:

```sql
SET LOCAL app.current_client_id = $1;   -- from the JWT, inside the transaction
```

`SET LOCAL` scopes the setting to the transaction, so a pooled connection cannot
leak one client's context into the next request. `portal_reader` has no `INSERT`,
`UPDATE`, or `DELETE` on `mirror.*` — the portal is structurally incapable of
mutating the mirror. Writes go only to `inbound.*` via a separate role
`portal_writer` with `INSERT` on those tables alone.

**Belt and braces.** The API also filters by `client_id` in every query. RLS is
the guarantee; the API filter is the second lock. A test asserts both:
disabling the API filter must still yield zero cross-client rows.

**Required test.** `test_rls_blocks_cross_client_read` — seed two clients, set
`app.current_client_id` to the first, query every mirror table unfiltered,
assert zero rows belonging to the second. This test is the module's acceptance
gate; it must exist before any portal endpoint is written.

---

## 9. Document Pipeline

### 9.1 Outbound (firm → client)

Documents are AES-256-GCM encrypted in the desktop vault and must never leave in
that form (the vault key never leaves the machine), nor in raw form.

```
attorney sets is_shared_with_client = 1
  → outbox row (Document, Upsert)
  → sync engine:
      1. decrypt_from_vault(vault_key, vault_path)
      2. clean_metadata(&bytes, &doc_type)     ← MANDATORY, see below
      3. sha256(cleaned_bytes)
      4. encrypt with a per-document key (server-side envelope encryption)
      5. PUT to object storage → object_key
      6. push documents_shared row (metadata + object_key + sha256)
```

**Step 2 is not optional.** Root CLAUDE.md: *"Metadata stripped before EVERY
document export — call `clean_metadata()`."* Sharing to the portal is an export.
`clean_metadata()` is currently a pass-through stub (S05) and **must be made
real before the first document is shared** — a Word file carrying author names,
tracked changes, or comments is a privilege incident. This is a blocking
dependency, recorded in §14.

Downloads are served as **5-minute signed URLs** generated per request. The
`object_key` is never returned to the browser.

### 9.2 Inbound (client → firm)

```
client uploads via portal
  → FastAPI: size/type check → PUT to quarantine bucket
  → ClamAV scan → scan_status
  → inbound.client_uploads row (status = Pending)
  → desktop pulls on next sync:
      1. refuse anything not scan_status = 'Clean'
      2. verify sha256
      3. clean_metadata()
      4. encrypt_to_vault(...) → documents row (uploaded_by = 'client:{portal_user_id}')
      5. client_uploads row (Ingested) + ack to server
  → server marks Ingested; quarantine object deleted after 7 days
```

The server never writes to the vault. The desktop pulls, and can refuse.

**Upload limits:** 25 MB per file; `pdf, docx, jpg, png, tiff` only; magic-byte
check, not extension trust.

---

## 10. Authentication

### Portal (OTP + JWT) — no passwords stored anywhere

```
POST /auth/request-otp   { email }
  → always 202, regardless of whether the email is known   (no account enumeration)
  → if known + status Active: 6-digit code, bcrypt-hashed into otp_challenges,
    10-minute expiry, emailed
POST /auth/verify-otp    { email, code }
  → max 5 attempts per challenge, then the challenge is dead
  → success: access JWT (15 min) + refresh token (7 days, rotating, httpOnly cookie)
POST /auth/refresh       → new access JWT
POST /auth/logout        → revoke refresh token
```

JWT claims: `sub` (portal_user_id), `cid` (client_id), `exp`, `iat`, `jti`.
`cid` is what drives RLS. Signed RS256; the public key is the only key the API
needs to verify.

Rate limits (slowapi): `request-otp` 3/email/hour and 10/IP/hour;
`verify-otp` 10/IP/hour.

A `Revoked` or `Suspended` portal user fails verification, and any live JWT is
rejected by a status check on each request — revocation from the desktop takes
effect at the next request, not at token expiry.

### Desktop ↔ server (mTLS)

Client certificate pinned at build time, per `server/SERVER-RULES.md`. The sync
endpoints accept no other authentication. A stolen portal JWT cannot reach them.

---

## 11. Keel Commands (`commands/sync.rs`)

| Command | Input | Returns | Notes |
|---|---|---|---|
| `sync_status` | `()` | `SyncStatus` | Replaces the stub; real `pending_changes` from `sync_outbox` |
| `trigger_sync` | `()` | `SyncStatus` | Manual push+pull; async, never blocks the UI |
| `set_sync_enabled` | `enabled: bool` | `SyncStatus` | Master switch — off by default until configured |
| `invite_portal_user` | `InvitePortalUserInput` | `PortalUser` | Partner/Associate only |
| `list_portal_users` | `client_id` | `Vec<PortalUser>` | |
| `revoke_portal_user` | `id` | `PortalUser` | Sets Revoked; syncs immediately |
| `share_document` | `document_id` | `DocumentMeta` | Sets flag + enqueues outbox |
| `unshare_document` | `document_id` | `DocumentMeta` | Tombstone — removes from mirror and deletes the object |
| `set_deadline_client_visible` | `id, visible: bool` | `Deadline` | |
| `request_document` | `RequestDocumentInput` | `()` | Creates a `DocumentRequested` notification |
| `list_pending_uploads` | `()` | `Vec<ClientUpload>` | Inbound queue awaiting ingest |
| `ingest_client_upload` | `id, matter_id` | `DocumentMeta` | Vault write + ack |
| `reject_client_upload` | `id, reason` | `()` | |
| `list_invoice_disputes` | `status?` | `Vec<InvoiceDispute>` | |
| `resolve_invoice_dispute` | `id, resolution` | `()` | |

`SyncStatus` keeps its existing shape (`last_synced_at`, `is_syncing`,
`pending_changes`) so the Deck sync store needs no rework.

**Background sync.** `services/sync_engine.rs` runs on a `tokio::spawn` loop —
push every 5 minutes when there is outbox work, pull every 15 minutes — plus the
WebSocket connection described in `server/SERVER-RULES.md`. Same pattern as
`deadline_watcher.rs`.

---

## 12. Sync Server Routes (`server/`, Axum)

| Route | Auth | Purpose |
|---|---|---|
| `POST /sync/push` | mTLS | Batch of projected upserts/deletes → mirror |
| `GET  /sync/pull` | mTLS | Pending inbound items since cursor |
| `POST /sync/ack` | mTLS | Mark inbound items Ingested/Rejected |
| `POST /sync/documents` | mTLS | Upload a shared document object; returns object_key |
| `DELETE /sync/documents/{key}` | mTLS | Remove an unshared document object |
| `GET  /sync/uploads/{id}/object` | mTLS | Download a quarantined client upload |
| `WS   /sync/live` | mTLS | Push notifications to connected portal sessions |
| `GET  /health` | none | Liveness |

The server validates payload shape and writes. It contains **no** rule about
what may be shared — that decision was made by the projection on the desktop.

---

## 13. Portal API (`portal/backend`, FastAPI)

| Route | Returns |
|---|---|
| `GET /matters` | Client's matters (summary) |
| `GET /matters/{id}` | Matter detail + its deadlines + IP assets + documents |
| `GET /deadlines` | Upcoming deadlines across matters |
| `GET /documents` | Shared documents, filterable by matter |
| `GET /documents/{id}/download` | 302 to a 5-minute signed URL |
| `POST /documents/upload` | Multipart; scanned, quarantined, queued |
| `GET /invoices` | Invoices (never drafts) |
| `GET /invoices/{id}` | Invoice detail + payments |
| `GET /invoices/{id}/pdf` | 302 to a 5-minute signed URL |
| `POST /invoices/{id}/dispute` | Raise a dispute |
| `GET /notifications` | Client notifications |
| `POST /notifications/{id}/read` | Mark read |
| `GET /profile` | Portal user profile + notification preferences |
| `PATCH /profile` | Update notification preferences only |

Every response passes through a Pydantic response model. No SQLAlchemy object is
ever serialised directly — that is how internal columns leak.

Headers on all responses: HSTS, `X-Content-Type-Options: nosniff`,
`Cache-Control: no-store` on anything document- or invoice-related.

---

## 14. Portal Frontend (`portal/frontend`)

Four tabs, per PROGRESS.md. Same design tokens as Deck (`colors`, `fonts`,
`fontSizes`, `radius`, `shadows`, `spacing`) — the client should feel the same
firm. No Tauri imports; data via `fetch()` in `lib/api.ts`.

| Page | Contents |
|---|---|
| `Matters/` | List (title, type, status badge, next deadline). Detail: status, attorney, deadlines timeline, IP assets, documents for that matter |
| `Documents/` | All shared documents, filter by matter/category, download; upload area showing outstanding firm requests |
| `Invoices/` | List with status + amount due; detail with line-item-free summary, GST breakdown, payment history, PDF download, "Raise a query" |
| `Profile/` | Name, email, phone (read-only — changes go through the firm), notification preferences, logout |

**Deliberately absent:** time entries, hourly rates, internal notes, other
clients, firm staff lists, document version history beyond the current version.

**Invoice detail nuance.** The client sees the invoice as issued — the PDF is
authoritative. The HTML view shows totals and GST breakdown, not a reconstruction
of individual time entries, since `time_entries` never syncs.

---

## 15. Constraints & Rules

1. **Allow-list projection.** Adding a desktop column must never expose it.
   Changes to `projection.rs` are security-relevant.
2. **RLS before endpoints.** `test_rls_blocks_cross_client_read` must pass
   before any portal route is written.
3. **`clean_metadata()` must be real** before the first document is shared.
   Currently a pass-through stub — blocking dependency (B07).
4. **The vault key never leaves the desktop.** Shared documents are decrypted
   locally, cleaned, then re-encrypted for the server under a different key.
5. **No direct object URLs.** 5-minute signed URLs only, generated per request.
6. **The server never writes to SQLite.** Inbound items are pulled and applied
   by the desktop, which may refuse.
7. **Drafts never sync.** `invoices.status = 'Draft'` and unshared documents are
   excluded at projection time, not filtered at the API.
8. **Money is `NUMERIC(14,2)`** in the mirror; rounded on projection.
9. **Sync is off by default** until a server URL and client certificate are
   configured. A firm with no server keeps working exactly as today.
10. **Revocation is immediate** — status checked per request, not at token expiry.
11. **No account enumeration.** `request-otp` returns 202 for unknown emails.
12. **Uploads are untrusted** until scanned clean, hash-verified, and ingested.
13. **Deletion propagates.** Unsharing removes the mirror row *and* the object.
14. **One email, one portal identity** — enforced by unique index on both sides.

---

## 16. Open Questions

| Question | Decision |
|---|---|
| Client-side messaging with attorneys? | **No.** Out of scope per PORTAL-RULES. Disputes and upload notes are the only client-authored text |
| Can clients see unbilled work in progress? | **No.** `time_entries` never syncs — it is the firm's most commercially sensitive data |
| Online payment (Razorpay/Stripe)? | **Deferred to Phase 3.** Invoices are viewable; payment happens by bank transfer and is recorded on the desktop |
| Multi-client users (a director of two client companies)? | **Not supported.** One email → one client. A second relationship needs a second email. Revisit if it actually occurs |
| Real-time WebSocket to the browser? | **Phase 3.** Portal polls `/notifications` on load and every 60s. The desktop↔server WS is in scope; server↔browser is not |
| iOS app data source? | Same mirror, same RLS, same API — Module 12, Phase 7 |
| Self-hosting vs Hetzner Managed PostgreSQL? | **Hetzner Managed** — backups and PITR without firm ops burden |
| What if the firm never runs the desktop for a week? | Mirror goes stale; portal shows `last_updated`. Acceptable — staleness is safer than a second write path |

---

## 17. Implementation Order

This module is too large for one session. Five sub-steps, each independently
testable, in dependency order:

**Step 2 — Schema**
- `0009_portal_sync.sql` (desktop): `deadlines.is_client_visible` + backfill,
  `portal_users`, `sync_outbox`, `client_uploads`, `sync_state`
- `server/migrations/0001_mirror.sql`: `mirror.*` + `inbound.*` schemas
- `server/migrations/0002_rls.sql`: RLS policies, `portal_reader` / `portal_writer` roles
- Update `SCHEMA.md`; write `server/SCHEMA.md`
- **Gate:** `test_rls_blocks_cross_client_read` passes

**Step 3a — Keel sync engine**
- `services/sync_engine/projection.rs` — allow-list projections + unit tests
  asserting that denied fields are absent from every payload
- `services/sync_engine/mod.rs` — outbox drain, push/pull, backoff
- `db/queries/sync.rs`, `db/queries/portal_users.rs`
- `commands/sync.rs` — all 15 commands
- Make `clean_metadata()` real (B07)
- **Gate:** `cargo test` green; projection tests prove no denied field escapes

**Step 3b — Sync server**
- Axum routes, mTLS, object storage client, WS
- **Gate:** desktop can push and pull against a local server

**Step 3c — Portal backend**
- FastAPI: OTP + JWT, RLS middleware, all routes, Pydantic response models
- **Gate:** `pytest` green, including a cross-client access test per endpoint

**Step 4 — Portal frontend**
- Four tabs, OTP login flow, shared design tokens
- **Gate:** `pnpm build` green in `portal/frontend`

**Step 5 — Validation**
- End-to-end: invite a client, share a document, verify it appears and downloads
  cleanly; upload from the portal and ingest on the desktop; raise and resolve a
  dispute; revoke a user and confirm immediate lockout
- Confirm by inspection that no privileged column exists anywhere in `mirror.*`

---

## 18. New Blocker Raised by This Spec

| ID | Issue | Severity |
|---|---|---|
| B07 | `clean_metadata()` is a pass-through stub (S05). Sharing a document to the portal is an export; shipping the portal without real metadata stripping risks disclosing author names, tracked changes, and comments to a client | **High** — blocks Step 3a |
