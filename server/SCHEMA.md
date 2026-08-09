# Persist Sync Server — PostgreSQL Schema

This file is the human-readable schema reference for the sync server's mirror
database. Update it after every migration. Never let it drift from the actual
schema.

**Desktop SQLite is the source of truth.** Nothing in this database is
authoritative. See `src-tauri/src/db/SCHEMA.md` for the canonical schema and
`specs/module-05-portal.md` for the full module contract.

---

## Migration History

| Migration | Description | Date |
|---|---|---|
| `0001_mirror.sql` | `mirror.*` projection tables + `inbound.*` client-authored queue | Phase 2 M5 |
| `0002_rls.sql` | Row-level security policies, `portal_reader` / `portal_writer` / `sync_writer` roles | Phase 2 M5 |
| `0003_portal_auth.sql` | `portal_auth` role — the login path. Neither existing portal role can resolve an email or touch an OTP challenge | Phase 2 M5 |
| `0004_refresh_tokens.sql` | `inbound.refresh_tokens` — rotating refresh tokens with reuse detection | Phase 2 M5 |

---

## Two Schemas, Two Directions

```
mirror.*    A projection of desktop SQLite. Written ONLY by the sync server
            applying a desktop push. Read-only to the portal.

inbound.*   Client-authored items awaiting ingest by the desktop. Written by the
            portal, read and acked by the desktop. Never applied to firm data by
            this server.
```

Every `mirror.*` table carries `client_id` — including where it is denormalised
(`deadlines_public`, `payments_public`) — because RLS policies must scope a row
without joining. A join inside a policy is a policy that can be tricked.

---

## Roles

| Role | Grants | Purpose |
|---|---|---|
| `portal_reader` | `SELECT` on `mirror.*` only | The portal's read connection. Structurally incapable of mutating the mirror |
| `portal_writer` | `SELECT, INSERT` on `inbound.client_uploads` and `inbound.invoice_disputes` only | Client-authored writes. **No grant on `inbound.otp_challenges`** — a leaked portal credential must not reach OTP hashes |
| `portal_auth` | `SELECT/INSERT/UPDATE/DELETE` on `inbound.otp_challenges` and `inbound.refresh_tokens`; `SELECT` on `mirror.portal_users` plus `UPDATE (last_login_at)` | The login path only. Deliberately narrow: a compromised auth connection yields the client roster and **nothing about the firm's work** |
| `sync_writer` | Full DML on both schemas | The sync server, reached only from the desktop |

Neither portal role owns any table, so `FORCE ROW LEVEL SECURITY` applies to
them. FORCE is set anyway so an ownership change cannot silently disable
isolation.

---

## Client Context

The API sets the client identity per request, inside the transaction:

```sql
SET LOCAL app.current_client_id = '<client_id from validated JWT>';
```

`SET LOCAL` is essential — a pooled connection would otherwise carry one
client's context into the next request.

`current_setting('app.current_client_id', true)` returns NULL when unset, and
every policy compares with `=`. An unset context therefore matches **nothing**,
not everything. Failing closed is asserted by `tests/rls_test.sql` (Test 3).

---

## `mirror` Tables

### `mirror.clients`
| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | Desktop client id |
| `name` | TEXT NOT NULL | |
| `updated_at` | TIMESTAMPTZ NOT NULL DEFAULT now() | |

**Not mirrored:** `notes` (internal), `gstin`, `pan`, `address`, `phone`, `email`.

RLS keys on `id`, not `client_id`.

### `mirror.portal_users`
| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | |
| `client_id` | TEXT NOT NULL | FK → `mirror.clients(id)` ON DELETE CASCADE |
| `full_name` | TEXT NOT NULL | |
| `email` | TEXT NOT NULL UNIQUE | |
| `phone` | TEXT | |
| `status` | TEXT NOT NULL DEFAULT 'Invited' | `Invited \| Active \| Suspended \| Revoked` |
| `last_login_at` | TIMESTAMPTZ | |
| `updated_at` | TIMESTAMPTZ NOT NULL DEFAULT now() | |

### `mirror.matters_public`
| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | |
| `client_id` | TEXT NOT NULL | FK → `mirror.clients(id)` |
| `title` | TEXT NOT NULL | |
| `matter_type` | TEXT NOT NULL | |
| `status` | TEXT NOT NULL | |
| `opened_date` | DATE NOT NULL | |
| `forum` | TEXT | |
| `jurisdiction` | TEXT NOT NULL DEFAULT 'India' | |
| `client_notes` | TEXT | The client-facing note. `matters.internal_notes` is **never** projected |
| `responsible_attorney` | TEXT | Display name only — never a user id |
| `next_deadline_date` | DATE | |
| `next_deadline_event` | TEXT | |
| `updated_at` | TIMESTAMPTZ NOT NULL DEFAULT now() | |

### `mirror.deadlines_public`
| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | Only rows with `deadlines.is_client_visible = 1` are projected |
| `client_id` | TEXT NOT NULL | |
| `matter_id` | TEXT NOT NULL | FK → `mirror.matters_public(id)` |
| `docketing_event` | TEXT NOT NULL | |
| `due_date` | DATE NOT NULL | |
| `status` | TEXT NOT NULL | |
| `updated_at` | TIMESTAMPTZ NOT NULL DEFAULT now() | |

**Not mirrored:** `deadlines.notes` — it routinely holds strategy ("weak prior
art, consider opposing"). The client sees the event and the date.

### `mirror.ip_assets_public`
| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | |
| `client_id` / `matter_id` | TEXT NOT NULL | |
| `asset_type` | TEXT NOT NULL | |
| `title` | TEXT NOT NULL | |
| `application_number` / `registration_number` | TEXT | |
| `filing_date` / `registration_date` / `expiry_date` | DATE | |
| `status` | TEXT NOT NULL | |
| `classes` | JSONB NOT NULL DEFAULT '[]' | Nice/Locarno class numbers |
| `jurisdiction` | TEXT NOT NULL DEFAULT 'India' | |
| `updated_at` | TIMESTAMPTZ NOT NULL DEFAULT now() | |

### `mirror.documents_shared`
| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | Only `documents.is_shared_with_client = 1` is projected |
| `client_id` / `matter_id` | TEXT NOT NULL | |
| `filename` | TEXT NOT NULL | |
| `category` / `mime_type` | TEXT NOT NULL | |
| `file_size_bytes` | BIGINT NOT NULL | |
| `version` | INTEGER NOT NULL DEFAULT 1 | |
| `description` | TEXT | |
| `object_key` | TEXT NOT NULL | Object storage key. **Never returned to a browser** — the API issues a 5-minute signed URL per request |
| `sha256` | TEXT NOT NULL | Integrity check; also proves the object matches what the desktop cleaned |
| `shared_at` / `updated_at` | TIMESTAMPTZ NOT NULL DEFAULT now() | |

Bytes reach this store only after `export_document` / `clean_with_report` has
stripped metadata on the desktop (B07).

### `mirror.invoices_public`
| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | |
| `client_id` | TEXT NOT NULL | |
| `status` | TEXT NOT NULL CHECK (status <> 'Draft') | Drafts are excluded at projection time; the CHECK is a backstop |
| `invoice_date` / `due_date` | DATE | |
| `subtotal`, `cgst_amount`, `sgst_amount`, `igst_amount`, `total_with_tax`, `amount_paid` | NUMERIC(14,2) | **NUMERIC, not float** — desktop stores REAL, but a total shown to a client must never drift by floating-point noise. Rounded on projection |
| `notes` | TEXT | |
| `pdf_object_key` | TEXT | |
| `updated_at` | TIMESTAMPTZ NOT NULL DEFAULT now() | |

### `mirror.payments_public`
| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | |
| `client_id` | TEXT NOT NULL | Denormalised for RLS |
| `invoice_id` | TEXT NOT NULL | FK → `mirror.invoices_public(id)` |
| `amount` | NUMERIC(14,2) NOT NULL | |
| `payment_date` | DATE NOT NULL | |
| `method` | TEXT NOT NULL | |
| `reference` | TEXT | |
| `updated_at` | TIMESTAMPTZ NOT NULL DEFAULT now() | |

### `mirror.client_notifications`
| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | |
| `client_id` | TEXT NOT NULL | |
| `kind` | TEXT NOT NULL | `DocumentShared \| DeadlineUpcoming \| InvoiceIssued \| PaymentRecorded \| DocumentRequested` |
| `title` | TEXT NOT NULL | |
| `body` | TEXT | |
| `matter_id` | TEXT | |
| `created_at` | TIMESTAMPTZ NOT NULL DEFAULT now() | |
| `read_at` | TIMESTAMPTZ | |

---

## `inbound` Tables

### `inbound.client_uploads`
| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | |
| `client_id` / `portal_user_id` | TEXT NOT NULL | |
| `matter_id` | TEXT | If the client attributed it |
| `filename` / `mime_type` | TEXT NOT NULL | Client-supplied, untrusted |
| `file_size_bytes` | BIGINT NOT NULL | |
| `object_key` | TEXT NOT NULL | **Quarantine bucket** — moves to the vault only when the desktop pulls it |
| `sha256` | TEXT NOT NULL | |
| `scan_status` | TEXT NOT NULL DEFAULT 'Pending' | `Pending \| Clean \| Infected \| Failed` |
| `status` | TEXT NOT NULL DEFAULT 'Pending' | `Pending \| Ingested \| Rejected` |
| `uploaded_at` | TIMESTAMPTZ NOT NULL DEFAULT now() | |

The desktop refuses anything not `scan_status = 'Clean'`.

### `inbound.invoice_disputes`
| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | |
| `client_id` / `portal_user_id` | TEXT NOT NULL | |
| `invoice_id` | TEXT NOT NULL | |
| `reason` | TEXT NOT NULL | |
| `status` | TEXT NOT NULL DEFAULT 'Pending' | `Pending \| Ingested` |
| `raised_at` | TIMESTAMPTZ NOT NULL DEFAULT now() | |

### `inbound.otp_challenges`
| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | |
| `email` | TEXT NOT NULL | |
| `code_hash` | TEXT NOT NULL | bcrypt. The plaintext code is never stored |
| `expires_at` | TIMESTAMPTZ NOT NULL | 10 minutes |
| `attempts` | INTEGER NOT NULL DEFAULT 0 | Max 5, then the challenge is dead |
| `consumed_at` | TIMESTAMPTZ | |
| `created_at` | TIMESTAMPTZ NOT NULL DEFAULT now() | |

**Only `portal_auth` has any grant on this table** (0003). `portal_reader` and
`portal_writer` have none, so a leaked portal credential cannot reach OTP hashes,
and `tests/rls_test.sql` Test 5 asserts that. There is no RLS here: the table
holds no client-scoped rows, and a challenge is keyed by an address that has not
yet been proven to belong to anyone.

### `inbound.refresh_tokens`
| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | |
| `portal_user_id` | TEXT NOT NULL | FK → `mirror.portal_users(id)` ON DELETE CASCADE |
| `client_id` | TEXT NOT NULL | FK → `mirror.clients(id)` ON DELETE CASCADE |
| `token_hash` | TEXT NOT NULL UNIQUE | SHA-256. High-entropy random value, so a plain hash suffices where the OTP needs bcrypt |
| `family_id` | TEXT NOT NULL | Every token minted from one login. Reuse kills the family, not merely the token presented |
| `expires_at` | TIMESTAMPTZ NOT NULL | 7 days |
| `used_at` / `revoked_at` / `replaced_by` | | Rotation chain |
| `created_at` | TIMESTAMPTZ NOT NULL DEFAULT now() | |

Presenting a spent token means either a replay or a theft the legitimate client
has already rotated past. Nothing can tell those apart, so the whole family is
revoked and the client logs in again.

**Only `portal_auth` has any grant here.** A session credential is not client
data, and the connection that serves matters has no business reading one.

---

## Row-Level Security

RLS is enabled **and forced** on every `mirror.*` table and both client-readable
`inbound.*` tables.

| Policy | Applies to | Rule |
|---|---|---|
| `client_isolation` | `portal_reader`, all `mirror.*` | `SELECT` where `client_id` (or `id` on `mirror.clients`) matches the session context |
| `client_isolation_read` | `portal_writer`, `inbound.*` | `SELECT` scoped to own rows |
| `client_isolation_write` | `portal_writer`, `inbound.*` | `INSERT` with `WITH CHECK` — stops a client filing an upload or dispute under another client's id |
| `sync_full_access` | `sync_writer`, all tables | Unscoped; the sync server holds no client context and applies pushes across all clients |

`WITH CHECK` on the inbound tables is what makes isolation more than read-only.
Without it a client could file a dispute as someone else.

### New tables inherit grants, not policies

`ALTER DEFAULT PRIVILEGES` gives a future `mirror` table `SELECT` for
`portal_reader` automatically — but it has **no policy** until someone writes
one, and with RLS enabled and no policy a table returns zero rows. A forgotten
policy therefore hides data rather than exposing it. Enabling RLS on new tables
is still required, and `tests/rls_test.sql` Test 7 fails if it is skipped.

---

## Running the RLS Gate

```bash
server/scripts/test-rls.sh
```

Creates a throwaway database, applies both migrations, and runs 10 assertions:
cross-client reads blocked on every mirror table, context switching, fail-closed
on unset context, portal cannot mutate the mirror (UPDATE/DELETE/INSERT all
refused), OTP hashes unreachable, clients cannot file inbound items as another
client, and every table has RLS enabled and forced.

**This must pass before any portal endpoint is written** (spec §8).

Verified to fail correctly: disabling RLS on one mirror table makes Test 1 abort
with `RLS LEAK: mirror.matters_public returned 1 row(s) belonging to another
client` and a non-zero exit.
