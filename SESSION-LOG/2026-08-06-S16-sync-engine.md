# Session S16 — 2026-08-06
## M5 Step 3a — Keel Sync Engine

---

## Summary

Built everything that decides **what** would sync and **whether** syncing is on.
The transport itself (push/pull over mTLS) is Step 3b and deliberately not here.

**Result:** cargo test 122 → **146/146**. pnpm build PASS. On
`claude/new-session-dbqe5o`; nothing pushed to main.

---

## Files

### New
- `src-tauri/src/services/sync_engine/projection.rs` — the allow-list (12 tests)
- `src-tauri/src/services/sync_engine/mod.rs` — outbox, backoff, state (11 tests)
- `SESSION-LOG/2026-08-06-S16-sync-engine.md`

### Modified
- `src-tauri/src/commands/sync.rs` — stub replaced with 11 commands (2 tests)
- `src-tauri/src/db/queries/deadlines.rs` — `is_client_visible` added to
  `DeadlineRow`; the column existed since 0009 but was never selected
- `src-tauri/src/{services/mod.rs, lib.rs}` — registration
- `src/lib/{ipc-types,tauri}.ts`, `src/stores/sync.ts` — types and wrappers

---

## The projection layer

This is the piece the spec calls "the single most important rule in this module".

**Plain functions, never `From<Row>`.** `From` invites `..Default` and
struct-update syntax, both of which can silently carry fields you never named.
Listing every field by hand is more tedious and much harder to get wrong.

**Tests assert on serialised JSON, not struct fields.** Checking that
`MatterPublic` has no `internal_notes` field only proves what the type already
says. Instead each test builds a row with a sentinel in every unnamed column —
`INTERNAL_LEAK`, `USERID_LEAK`, `NOTES_LEAK` — serialises the projection, and
asserts those strings are absent from the bytes. That proves what actually
leaves the machine.

**Withholding is explicit.** Draft invoices and non-client-visible deadlines
return `Err(Withheld::…)` rather than being silently skipped, so a caller logs a
reason. Drafts are excluded *at projection time*, not filtered later in the API —
a draft must never reach the mirror at all.

**A guard test for the thing that must never exist.** `time_entries` is the
firm's most commercially sensitive data. There is a test that greps this file's
own source and fails if a time-entry projection is ever added, so doing so is a
deliberate act with a failing test in front of it rather than an oversight.
(The first version of that test failed against itself, because the needle
appeared literally in its own source. It now builds the string with `concat!`.)

---

## The outbox

**Enqueue collapses.** Five edits before one sync are one push, not five — the
push sends current state, so stacking is pointless.

**Delete supersedes a pending Upsert.** Without this, an upsert queued just
before an un-share would resurrect the row in the mirror. Tested directly.

**Failures never drop an entry.** A change the firm made must not vanish because
the network was down; `record_failure` increments a counter and keeps the row.
Backoff is exponential, capped at an hour so a long outage does not push the
next attempt days out.

**Sync is off by default, and enabling without a server URL is refused.** A firm
that believes sync is on and is wrong is worse off than one that sees an error.

---

## Commands

11 replacing the stub: `sync_status`, `trigger_sync`, `set_sync_enabled`,
`set_sync_server`, `invite_portal_user`, `list_portal_users`,
`revoke_portal_user`, `share_document`, `unshare_document`,
`set_deadline_client_visible`, `list_shared_documents`.

`share_document` only sets the flag and queues the change. The bytes are cleaned
and uploaded by the transport using `export_document` — sharing is an export, and
export means metadata stripping (B07). Putting the upload here would have meant
two paths to the same rule.

`SyncStatus` keeps its original three fields so the Deck store needed no rework;
three were added (`isEnabled`, `serverUrl`, `lastError`) because Deck has to be
able to show *why* nothing is syncing.

---

## Commands Run

```bash
cd src-tauri && cargo test --lib     # 146/146 (was 122)
pnpm build                            # PASS
```

---

## Not Done / Deferred

- **The transport (Step 3b).** `trigger_sync` reports how many changes are queued
  and returns an explicit error rather than pretending to have sent them. Nothing
  leaves the machine yet.
- **Outbox wiring on the remaining write paths.** Only the sharing and portal-user
  commands enqueue today. Matters, deadlines, IP assets, invoices and payments do
  not. This is deliberate: enqueuing from fifteen call sites with nothing to drain
  them is untestable code written a sprint early, so it lands with the transport.
  **It must not be forgotten** — without it the first sync would miss history.
- **Document and payment projections.** `Withheld::DocumentNotShared` exists;
  the projections themselves land with the document pipeline in 3b.
- **The 4 ingest commands** (`list_pending_uploads`, `ingest_client_upload`,
  `reject_client_upload`, dispute handling) — they need the pull transport.
- **No UI for any of this.** The commands and wrappers exist; no Deck screen
  calls them. A portal-users panel and a sync settings section are still to build.
- Same as last session: no verify action in the UI, no Deck role gating.

---

## Next Session Options

1. **M5 Step 3b** — the Axum sync server plus push/pull transport, and the
   remaining outbox write-path wiring alongside it.
2. **Deck surface for M5** — portal users panel, sync settings, share toggles.
3. **Close the older loose ends** — verify action, role-gated UI, B06.
