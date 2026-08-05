# Session S14 — 2026-08-05
## Sprint A: M5 Step 2 (Portal Schema + RLS) · Sprint B: Cascade Engine + Abandonment Watcher

---

## Summary

Two sprints, both requested. Nothing pushed to `main` — held on
`claude/new-session-dbqe5o` per the current review arrangement.

**Sprint A** built the portal's data layer and, more importantly, its isolation
guarantee: PostgreSQL mirror + inbound queue, role separation, and an RLS
acceptance gate that is verified to fail when isolation breaks.

**Sprint B** completed the remaining Phase 1 M2 work: statutory deadline chains
generated from templates stored as data, and an escalation ladder so a missed
statutory deadline is never silent.

**Result:** cargo test 86 → **113/113**. RLS gate **10/10**. pnpm build PASS
(474 modules, 506kb).

---

## Sprint A — M5 Step 2

### Files
- `src-tauri/src/db/migrations/0009_portal_sync.sql` — `deadlines.is_client_visible`
  (+ statutory backfill), `portal_users`, `sync_outbox`, `client_uploads`, `sync_state`
- `server/migrations/0001_mirror.sql` — `mirror.*` (9 tables), `inbound.*` (3 tables)
- `server/migrations/0002_rls.sql` — roles, policies, default privileges
- `server/tests/rls_test.sql` — the acceptance gate, 10 assertions
- `server/scripts/test-rls.sh` — runner
- `server/SCHEMA.md` — new
- `src-tauri/src/db/mod.rs` — 2 more migration tests
- `src-tauri/src/db/SCHEMA.md` — 0009 documented

### Decisions

**Three database roles, not one.** `portal_reader` holds `SELECT` on `mirror.*`
and nothing else — the portal is structurally incapable of mutating the mirror,
independent of any application bug. `portal_writer` may `INSERT` into exactly two
inbound tables. Neither has *any* grant on `inbound.otp_challenges`, so a leaked
portal credential cannot reach OTP hashes; that table is protected by absence of
privilege rather than by policy, and Test 5 asserts it.

**Unset context matches nothing.** `current_setting('app.current_client_id', true)`
returns NULL when unset and every policy compares with `=`, so `client_id = NULL`
is NULL and the row is filtered out. A request that forgets to set context
returns zero rows rather than every row. Test 3 covers this specifically because
it is the difference between a bug and a breach.

**`WITH CHECK` on inbound.** Without it, isolation would be read-only: a client
could file an upload or dispute under another client's id. Test 6 attempts
exactly that and expects a refusal.

**Denormalised `client_id` everywhere.** Including on `deadlines_public` and
`payments_public`, where it could be reached by a join. A policy that joins is a
policy that can be tricked; policies here compare one local column.

**Money changes type at the boundary.** Desktop `REAL` → mirror `NUMERIC(14,2)`,
rounded at projection. A total shown to a client must not drift.

### The gate is real, not decorative

Verified by negative control: disabling RLS on one mirror table makes the suite
abort with `RLS LEAK: mirror.matters_public returned 1 row(s) belonging to
another client` and exit non-zero. A gate that cannot fail is worse than none.

PostgreSQL 16 was installed in this container to run it, so the gate ran for real
rather than being asserted on paper.

---

## Sprint B — Cascade Engine + Abandonment Watcher

### Files
- `src-tauri/src/db/migrations/0010_cascade.sql` — `cascade_templates` (7 seeded),
  `deadline_escalations`, `deadlines` rebuilt
- `src-tauri/src/services/cascade_engine.rs` — 14 tests
- `src-tauri/src/services/abandonment_watcher.rs` — 10 tests
- `src-tauri/src/commands/cascade.rs` — 5 commands
- `src-tauri/src/db/queries/ip_assets.rs` + `commands/ip_assets.rs` — `list_upcoming_renewals`
- `src/pages/Dockets/RenewalDashboard.tsx`, route in `App.tsx`
- `src/lib/{ipc-types,tauri}.ts`

### Decisions

**Templates are data, not Rust.** The statutory periods live in
`cascade_templates.template_json` with a `last_verified` date and a citation.
When the Trade Marks Rules change a period, that is a DB update — not a code
change and a release. A wrong period here abandons a client's application.

**Registry-triggered events are their own anchors.** The firm cannot know at
filing when an examination report will issue, so `TMApplication` generates only
what is computable from the filing date (renewal at 10 years, grace at 10y6m).
When the report actually arrives, `TMExaminationReport` runs as its own anchor
with the report's real date. Nothing is ever dated from an event that has not
happened — the alternative is a docket full of fictional dates.

**Preview then commit.** `preview_cascade` writes nothing and returns the whole
chain plus the template's `last_verified`. Silently creating 19 annuity deadlines
from a mistyped anchor date would be a genuine mess to unpick.

**Month arithmetic clamps to month end.** 31 Jan + 1 month = 28 Feb (29 in a leap
year), not 3 March. This is the conventional treatment of statutory periods and
the wrong answer here is a missed deadline. Tested explicitly, including
29 Feb + 1 year.

**Internal buffers are always Procedural and never client-visible.** The buffer
is the firm's own discipline, not law; showing a client an internal working date
invites arguments about a date that has no legal force.

**Escalations are cumulative and idempotent.** A deadline first seen 5 days out
backfills L1 and L2, so the audit trail does not misleadingly suggest only L2 was
ever known. Idempotence comes from a UNIQUE index on
`(deadline_id, escalation_level)` rather than application logic — the watcher
runs every 30 minutes and two overlapping cycles must not both raise a level.

**Only Statutory deadlines escalate.** Escalating the firm's own working dates
would train attorneys to ignore the alerts that actually matter.

**`status = 'Missed'` required a table rebuild.** SQLite cannot alter a CHECK
constraint. `Missed` is a terminal business state — the deadline passed without
action — and is genuinely different from urgency `Overdue`, which is only a
visual signal. The rebuild is safe (no table references `deadlines`), and
`cascade_root_id` was deliberately left a plain TEXT column rather than a
self-FK to avoid re-pointing it during the rename.

---

## Commands Run

```bash
apt-get install -y postgresql-16          # to run the RLS gate for real
pg_ctl start

cd src-tauri && cargo test --lib          # 113/113 (was 86)
server/scripts/test-rls.sh                # RLS GATE: PASS (10 assertions)
pnpm build                                # 474 modules, 506kb — PASS
```

---

## Known Issues / Status

| ID | Status | Notes |
|---|---|---|
| B01, B02, B04, B05, B07 | Resolved | — |
| B03 | Partial | latex.rs real; runtime needs MacTeX or bundled TeX Live |
| B06 | Partial | UTC parsing helper exists; other call sites unconverted |

---

## Not Done / Deferred

- **The M5 spec is still unreviewed.** Step 2 implements it as written; if the
  review changes §5–§7, the mirror schema changes with it.
- **WhatsApp at escalation L2** — spec §2.11 wants it. Channel list currently
  records `["in_app"]`; WhatsApp arrives with Module 6 (Phase 3).
- **`docket_errors` (spec §2.17)** — L4 records an escalation and flips status to
  `Missed`, but does not yet write a `docket_errors` row. Separate spec item.
- **Cascade recalculation on date change** (spec §Business Rules) — editing an
  asset's filing date does not yet prompt to regenerate the chain.
- **`CascadePreview.tsx`** — the preview command exists and is wired in
  `lib/tauri.ts`, but no UI calls it yet; the dashboard covers renewals and
  escalations only.
- **Dual verification, reference numbers, fee schedule** — still open Module 2
  spec items, untouched this session.
- The Deck bundle now exceeds Vite's 500 kB warning threshold. Not addressed;
  code-splitting is a separate piece of work.

---

## Next Session Options

1. **M5 Step 3a — Keel sync engine.** `services/sync_engine/projection.rs` is the
   security-critical piece (allow-list projections), plus outbox drain and the 15
   sync commands.
2. **Review the M5 spec** and adjust Step 2 if anything changes.
3. **Finish Module 2** — docket_errors, dual verification, reference numbers,
   cascade recalculation prompt.
