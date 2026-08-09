# Session S18 — 2026-08-09
## Module 5 Step 3b — Sync Server and Transport

---

## Summary

Until this session Persist could decide what to send and refuse to send it.
Step 3b built the two halves that make it move: an Axum server that writes the
PostgreSQL mirror, and the Keel-side transport that drains the outbox into it.
It also closed the deferral flagged in S16 — the remaining write paths now
enqueue, so a first sync will carry the firm's history rather than only whatever
happened after the transport landed.

The end-to-end test found a real design gap that no unit test could have: every
table in `mirror.*` has a foreign key to `mirror.clients`, and nothing projected
a client row. A first sync would have been refused outright.

**Result:** cargo test **156/156** (was 146), sync e2e **4/4 against live
PostgreSQL**, pnpm build PASS, 13 screenshots. On `claude/new-session-dbqe5o`;
nothing pushed to main.

---

## Files

### New

| File | What it is |
|---|---|
| `server/src/main.rs` | Axum routes: `/health`, `/sync/push`, `/sync/pull`, `/sync/ack`. Constant-time token compare; refuses to start without `SYNC_CLIENT_TOKEN` ≥ 32 chars |
| `server/src/types.rs` | Wire types, all `deny_unknown_fields` |
| `server/src/mirror.rs` | Upsert/delete per entity; `money()` converts f64 → `Decimal`; refuses draft invoices at the boundary as well as at projection |
| `server/Cargo.toml` | axum 0.7, sqlx/postgres, rust_decimal, tower-http |
| `src-tauri/src/services/sync_engine/transport.rs` | `build_entry`, `order_for_push`, `push_once`, `pull_once`, `ack` |
| `src-tauri/src/db/migrations/0012_outbox_client.sql` | Rebuilds `sync_outbox` to admit `entity_type = 'Client'` |
| `src-tauri/tests/sync_e2e.rs` | 4 tests against a live server; skipped when `PERSIST_SYNC_E2E_URL` is unset |

### Modified

| File | Change |
|---|---|
| `src-tauri/src/commands/sync.rs` | `trigger_sync` really syncs; `set_sync_token`; `ingest_pull`; `SyncStatus.has_token` |
| `src-tauri/src/services/sync_engine/mod.rs` | `EntityType::Client`; `note_change`; `record_sync_run` |
| `src-tauri/src/services/sync_engine/projection.rs` | `ClientPublic` + `project_client` |
| `src-tauri/src/services/keychain.rs` | Generalised to hold more than one secret, keyed by a `Secret { user, file }` pair |
| `src-tauri/src/db/queries/matters.rs` | `ClientRow` + `get_client_row` |
| `src-tauri/src/commands/{matters,deadlines,ip_assets,billing}.rs` | Outbox enqueue on every write path |
| `src-tauri/src/lib.rs` | `services` made public for the integration test; `set_sync_token` registered |
| `src-tauri/src/db/SCHEMA.md` | 0011/0012 rows; `Client` in the outbox enum; a note on push ordering |
| `src/pages/Portal/PortalHome.tsx` | Write-only sync token field; honest "Sync now" messaging |
| `src/lib/{tauri,ipc-types}.ts`, `src/stores/sync.ts` | `setToken`, `hasToken` |
| `screenshots/{mock/core.ts,capture.mjs}` | `?sync=on` fixture and a 13th capture |

---

## Decisions

**The client row is a synced entity like any other.** Every mirror table has a
foreign key to `mirror.clients`. The alternative — having the server
auto-create a stub row from a matter's `client_id` — would have put an empty
client name in the portal. `ClientPublic` carries id and name only; gstin, pan,
address, phone, email and notes are not even fetched.

**Clients sort ahead of everything else in a push batch.** The outbox is
oldest-first and clients are created before their matters, so the order usually
holds already. "Usually" is not what a first sync should rest on, so
`order_for_push` makes it explicit. The sort is stable, so within each group the
outbox order survives.

**A failed enqueue does not fail the write.** By the time `note_change` runs the
local row is committed. Returning an error would show an attorney a failure for
a change that did happen — the worse of the two failure modes. The miss is
logged and recorded on `sync_state.last_error` so the Sync tab shows the mirror
is behind, rather than diverging silently.

**A run stamps `last_pushed_at` even when a leg failed.** That field answers
"when did we last talk to the server", and a run that pushed nine rows with one
rejected did talk to it. `last_error` carries the qualification, and Deck no
longer says "Sync complete" when one is set.

**Client uploads are pulled as metadata only.** The bytes stay on the server
until an attorney reviews the upload. An unreviewed client file must not land in
the firm's vault automatically, whatever the scanner said.

**A draft invoice is refused twice.** Projection withholds it and the server
refuses it. Redundant on purpose: the second check is what catches a future
caller that bypasses the first.

**The sync token lives in the OS keychain.** Same reasoning as the session
token: the database gets backed up and copied. The keychain was single-secret,
so it was generalised — with a test that clearing one secret cannot disturb the
other, because a sync token that overwrote the session token would log the
attorney out every time sync was configured.

**Auth is a shared secret, not the mTLS the spec asks for.** Recorded as a
shortfall rather than quietly dropped. Adequate on a private network; it must be
closed before the server faces the open internet.

---

## Bugs Found and Fixed

**B08 — nothing projected the client row.** Found by the first live push:
`matters_public_client_id_fkey` refused every matter. Unit tests could not have
caught it — they never open a socket, and the desktop schema has no such
constraint. Fixed with `ClientPublic`, `EntityType::Client`, migration 0012, and
`order_for_push`.

**My own invoice tests were seeding invalid rows.** `a_draft_invoice_is_not_sent`
pointed `created_by` at a user that did not exist (FK failure), and
`a_sent_invoice_is_sent_without_internal_fields` put a sentinel in `gst_type`,
which a CHECK constraint pins to `Intra`/`Inter`. The sentinel technique does not
work on a constrained column, so that field is asserted by key name instead.

**`sqlx::migrate!` did not rebuild on a new migration file.** Adding
`0012_outbox_client.sql` left the compiled binary running the old set, and the
CHECK constraint failure was identical to the one the migration was written to
fix — which reads exactly like the fix not working. `touch src/lib.rs` forced it.
Worth remembering: after adding a migration, force a rebuild before believing a
failure.

---

## Commands Run

```bash
# Keel
cargo test --lib                        # 156/156
cargo build --lib

# Mirror, from scratch
sudo -u postgres dropdb --if-exists persist_sync_e2e
sudo -u postgres createdb persist_sync_e2e
sudo -u postgres psql -q -d persist_sync_e2e -f server/migrations/0001_mirror.sql
sudo -u postgres psql -q -d persist_sync_e2e -f server/migrations/0002_rls.sql

# Server
SYNC_CLIENT_TOKEN=<32+ chars> DATABASE_URL=postgres://sync_writer:...@localhost/persist_sync_e2e \
  BIND_ADDR=127.0.0.1:8787 ./target/debug/persist-sync-server

# End to end
PERSIST_SYNC_E2E_URL=http://127.0.0.1:8787 PERSIST_SYNC_E2E_TOKEN=<same> \
  cargo test --test sync_e2e -- --test-threads=1     # 4/4

# Deck
pnpm build                                            # PASS
npx vite build --config vite.config.screenshots.ts
node screenshots/capture.mjs                          # 13 screenshots
```

### What the mirror held afterwards

```
 mirror.clients        : e2e-client | Petalveda Botanicals
 mirror.matters_public : P&P-2026-TM-9001 | PETALVEDA | Active | Sree Lakshmi Menon | (no client_notes)
 mirror.deadlines_public : e2e-dl-1 | Reply to Examination Report | 2026-11-30
 mirror.invoices_public  : INV-2026-9001 | 70000.01 | 6300.00 | 82600.01 | Sent
```

Sentinel sweep across all four tables for `INTERNAL_LEAK`, `NOTES_LEAK` and the
user id `e2e-user`: **no matches.** Negative control run — a sentinel written
directly into `mirror.matters_public.client_notes` was detected, so the sweep is
not vacuous.

---

## Not Done / Deferred

- **mTLS.** The spec asks for it; a shared bearer token is what exists.
- **Document and payment projections.** `build_entry` logs and skips them. They
  land with the outbound document pipeline, which needs signed URLs and object
  storage — a sprint of its own.
- **Automatic background sync.** `trigger_sync` is manual only. A scheduler
  without a retry-visible UI would hide failures.
- **Ingesting upload bytes into the vault.** Deliberate — see decisions.
- **WhatsApp L2 escalation** (from the abandonment ladder), **first-launch
  wizard**, **B06 date conversion outside the session layer**.
- **`clean_metadata` is dead code**; `clean_with_report` is what documents.rs
  calls. Pre-existing, unrelated to this sprint, but the root CLAUDE.md names
  the former.

---

## Next Session Options

1. **M5 Step 3c — FastAPI portal backend.** OTP + JWT, RLS middleware setting
   `app.current_client_id` per request, the 14 routes in the spec. This is the
   half that lets a client actually see any of what now reaches the mirror.
2. **The outbound document pipeline.** Signed 5-minute URLs, object storage, and
   the `Document` projection `build_entry` currently skips.
3. **mTLS**, closing the gap above.

---
---

# Session S18 (continued) — Module 5 Step 3c
## The Portal API

---

## Summary

Step 3b got firm data into the mirror. Step 3c is the half that lets a client
read their own share of it: a FastAPI app where every request is scoped to one
client by PostgreSQL itself, and login is a one-time code because the portal
stores no passwords at all.

**Result:** portal **48/48** against live PostgreSQL, RLS gate **12/12**
(negative-control verified twice). Still on `claude/new-session-dbqe5o`.

---

## Files

### New

| File | What it is |
|---|---|
| `portal/backend/app/config.py` | Settings. Nothing has a development default |
| `portal/backend/app/db/session.py` | Three engines, three roles; `client_scope` is the only way to get a read connection |
| `portal/backend/app/db/queries.py` | Every SQL statement the portal runs |
| `portal/backend/app/models.py` | Pydantic response models, `extra="forbid"` |
| `portal/backend/app/auth/{otp,tokens,refresh,deps}.py` | The whole front door |
| `portal/backend/app/api/{auth,matters,documents,invoices,misc}.py` | Routes |
| `portal/backend/app/storage.py` | HMAC-signed, 5-minute URLs |
| `portal/backend/app/main.py` | App, security headers, error handler |
| `portal/backend/tests/{conftest,test_isolation,test_auth,test_responses}.py` | 48 tests |
| `server/migrations/0003_portal_auth.sql` | The `portal_auth` role |
| `server/migrations/0004_refresh_tokens.sql` | `inbound.refresh_tokens` |

### Modified

`server/tests/rls_test.sql` (10 → 12 assertions, exemptions derived from
grants), `server/SCHEMA.md`, `.gitignore`.

---

## Decisions

**A fourth role, `portal_auth`.** Neither existing portal role can log a client
in. `portal_reader` is scoped by `app.current_client_id`, which login has not
established yet — establishing it *is* login. `portal_writer` has no grant on
`inbound.otp_challenges` at all, deliberately, from 0002. So the auth path needs
its own role, and 0001 anticipated exactly this ("only the auth path may, via a
dedicated function or elevated role").

It is kept as narrow as the job allows: it can resolve an email to a portal
identity, stamp `last_login_at`, and manage OTP and refresh rows. It cannot read
a single matter, deadline, document or invoice, and the RLS gate now asserts
that. A compromised auth connection yields the client roster and nothing about
the firm's work.

**Refresh tokens, because 15 minutes is not a session.** The short access token
exists so revocation bites quickly, which is right — but on its own it means
asking a client to read an emailed code four times an hour. Rotation with
family-wide revocation on reuse is the standard answer and the one the spec
implies. Replay and theft are indistinguishable, so both end the session.

**404, never 403, for another client's row.** A 403 confirms the id is real.

**No `status <> 'Draft'` filter on invoices in the API.** The mirror's CHECK
already forbids a draft from existing there. Adding an API filter would paper
over a projection bug rather than surfacing it.

**The RLS gate's exemptions are derived, not listed.** The old check named
`inbound.otp_challenges` as an exception. Adding `refresh_tokens` would have
meant a second name, and a list like that grows silently until it exempts
something that mattered. The check now exempts a table only if no portal role
can reach it at all — so a future table that *is* granted and unprotected still
fails. Verified by adding exactly such a table and watching the gate fail.

**No OpenAPI schema.** The portal is not a public API.

---

## Bugs Found and Fixed

**B09 — the OTP attempt counter was rolled back by its own refusal.**
`verify-otp` raised its 401 inside `async with auth_scope()`, and an exception
inside a transaction rolls it back — including the `attempts = attempts + 1`
written moments earlier. The cap was therefore never reached and the six-digit
code was brute-forceable at whatever rate the IP limiter allowed.

What makes this one worth recording: the code carried a comment explaining that
attempts are counted *before* the check precisely so a crash cannot hand back a
free guess. The reasoning was right and the placement defeated it. Found by
`test_attempts_are_capped_and_then_the_challenge_is_dead`, which failed with
`assert 200 == 401` — the correct code still worked after five wrong ones. The
same shape of bug was latent in `/auth/refresh`, where a detected token reuse
revokes the family and then raises; that raise moved out of the transaction too.

**`:mid::text` does not survive SQLAlchemy's parameter parser.** The `::`
PostgreSQL cast collides with `:name` binding. `CAST(:mid AS text)` is
unambiguous.

**slowapi with `headers_enabled=True` requires a `response: Response`
parameter** on the decorated route, and raises a bare `Exception` if it is
missing — which the app's own catch-all turned into a 500.

---

## Commands Run

```bash
cd portal/backend
uv venv --python 3.11 && uv pip install -e . --group dev

createdb persist_portal_test
for m in 0001_mirror 0002_rls 0003_portal_auth 0004_refresh_tokens; do
  psql persist_portal_test -f ../../server/migrations/$m.sql
done

source .test-env && pytest -q          # 48 passed

server/scripts/test-rls.sh             # 12/12 PASS
```

### Negative controls

Both isolation gates were watched failing before being trusted:

```sql
ALTER TABLE mirror.matters_public DISABLE ROW LEVEL SECURITY;
```
→ `test_rls_holds_with_the_api_filter_removed` fails:
`Extra items in the left set: 'test-client-b'`. The other eight isolation tests
still passed, which is the belt-and-braces design working: the API's `client_id`
filter held while RLS was off.

```sql
CREATE TABLE mirror.forgotten (id TEXT PRIMARY KEY, client_id TEXT NOT NULL);
GRANT SELECT ON mirror.forgotten TO portal_reader;
```
→ RLS gate fails: `TABLES WITHOUT FORCED RLS: mirror.forgotten`.

---

## Not Done / Deferred

- **Object storage.** The signing is real — HMAC over key and expiry together,
  with tests that a swapped key and an extended expiry both fail to verify — but
  no bucket is provisioned, so uploaded bytes are not yet written anywhere. The
  upload row is created regardless, so nothing a client sends is silently
  dropped; it sits `Pending`.
- **Virus scanning.** Uploads are recorded `Pending` and the desktop refuses to
  ingest anything else.
- **Email/SMS delivery.** A challenge is issued and stored; nothing sends it.
  The code is never returned to a browser outside tests.
- **Notification read receipts.** `portal_reader` cannot write to `mirror.*` by
  design, so this needs an inbound queue rather than a direct update. The route
  returns 501 rather than pretending.
- **Rate-limit storage is in-memory**, so limits are per worker.
- **The portal frontend** (Step 4).

---

## Next Session Options

1. **Step 4 — the portal frontend.** Four tabs, same design tokens as Deck. The
   API is now complete enough to build against.
2. **Step 3d — object storage and email.** Closes the three "not built" items
   above and makes document sharing work end to end.
3. **mTLS** on the desktop↔server leg.
