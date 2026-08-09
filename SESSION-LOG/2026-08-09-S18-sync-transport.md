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
