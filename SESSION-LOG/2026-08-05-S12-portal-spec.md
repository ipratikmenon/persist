# Session S12 — 2026-08-05
## Phase 2 Module 5 — Client Portal & Sync: Step 1 (Spec)

---

## Summary

Spec session only — no code. Per TASKS.md Step 1, the spec is written and
reviewed *before* any schema or implementation session.

`specs/module-05-portal.md` is now the contract for the client portal: the last
prerequisite standing between the current build and Phase 2.5 (M31 DP Audit) plus
all of Track B.

The module spans three surfaces (Keel sync client, Axum sync server, FastAPI +
React portal), so the spec is deliberately longer than module-04 and splits
implementation into five independently testable sub-steps.

**Result:** spec written, PROGRESS.md updated, B07 raised. Nothing built.

---

## Files Changed

### New files
- `specs/module-05-portal.md` — the module contract (18 sections)
- `SESSION-LOG/2026-08-05-S12-portal-spec.md` — this file

### Modified files
- `PROGRESS.md` — Current State → S12; M5 table expanded from 6 flat rows to 12
  step-tagged rows; B07 added; 7 decisions logged; session log row

---

## Sources Read

- `portal/PORTAL-RULES.md` — scope boundary, backend/frontend layout, security rules
- `server/SERVER-RULES.md` — sync/never-sync lists, mTLS, WebSocket, deployment
- root `CLAUDE.md` — sync rules, RLS, signed URLs, OTP-only, metadata stripping
- `specs/module-04-billing.md` — spec format and depth reference
- `specs/expansion-roadmap.md` §5.1 — to establish that its exception does NOT apply here
- `src-tauri/src/db/SCHEMA.md` — existing `documents.is_shared_with_client`,
  `clients.notes` ("never synced"), current table shapes
- `src-tauri/src/commands/sync.rs` — the existing stub whose `SyncStatus` shape is preserved

---

## Key Decisions

### Allow-list projection is the central safety property
The sync engine never serialises a desktop row wholesale. Every syncable entity
gets an explicit projection function naming each column that leaves the machine,
isolated in `services/sync_engine/projection.rs`. A column added to `matters`
next year is invisible to the portal until someone edits that file deliberately.
Deny-lists fail open as the schema grows; allow-lists fail closed.

### The portal is a mirror, not a second source of truth
Explicitly recorded that roadmap §5.1 (server-native PostgreSQL) applies to
Track B SaaS tenants **only**. Portal data is a projection of desktop SQLite.
`mirror.*` and future `tenant.*` are separate schemas with no cross-joins.

### Client-authored data lands in an inbound queue
Uploads and disputes sit in `inbound.*` with `status = 'Pending'`. The desktop
pulls, validates, applies, and acks. The server never writes to SQLite. If the
firm never opens the desktop, nothing enters firm data — and the desktop can
refuse an item outright.

### Deadline visibility defaults by type
`is_client_visible` defaults to 1 for `Statutory` deadlines (the client is
legally affected and must not be surprised) and 0 for `Procedural`/`Custom`
(internal steps). Overridable per deadline. `deadlines.notes` never syncs at all
— it routinely holds strategy.

### Database-level role separation, not just RLS policies
`portal_reader` holds `SELECT` only on `mirror.*` and is not the table owner, so
`FORCE ROW LEVEL SECURITY` applies to it. Writes go to `inbound.*` through a
separate `portal_writer`. The portal is structurally incapable of mutating the
mirror. Client context is set with `SET LOCAL` inside the transaction so a pooled
connection cannot leak one client's context into the next request.

### RLS test is the acceptance gate
`test_rls_blocks_cross_client_read` must exist and pass before any portal
endpoint is written — seed two clients, set the session client id, query every
mirror table *unfiltered*, assert zero foreign rows. The API's own `client_id`
filter is a second lock, tested independently.

### Money changes type at the boundary
Desktop stores money as SQLite `REAL`; the mirror uses `NUMERIC(14,2)` with
rounding at projection time. An invoice total shown to a client must never drift
by floating-point noise.

### Sync is off by default
No server URL and no client certificate means no sync. A firm with no server
keeps working exactly as it does today.

---

## Bug Raised

| ID | Issue | Severity |
|---|---|---|
| B07 | `clean_metadata()` is a pass-through stub (from S05). Sharing a document to the portal **is** an export under root CLAUDE.md's "metadata stripped before EVERY document export" rule. Shipping the portal without real stripping risks disclosing author names, tracked changes, and comments to a client | High — blocks Step 3a |

This surfaced from writing §9.1 (outbound document pipeline) and is the reason
the spec makes metadata stripping an explicit gate rather than an implementation
detail.

---

## Open Questions Resolved in the Spec

| Question | Decision |
|---|---|
| Client↔attorney messaging | No — out of scope per PORTAL-RULES |
| Clients see work in progress / unbilled time | No — `time_entries` never syncs |
| Online payment (Razorpay/Stripe) | Deferred to Phase 3 |
| Multi-client portal users | Not supported — one email, one client |
| Real-time WebSocket to the browser | Phase 3; desktop↔server WS is in scope |
| Hosting | Hetzner Managed PostgreSQL (backups + PITR without firm ops burden) |
| Stale mirror if desktop is offline | Acceptable; portal shows `last_updated`. Staleness is safer than a second write path |

---

## Commands Run

None — spec session, no code changed.
Last known good: `cargo test` 67/67, `pnpm build` PASS (472 modules, 497kb) from S11.

---

## Next Session Options

1. **Review and approve the spec** (TASKS.md Step 1 requires this before Step 2).
   Decision points flagged in PROGRESS.md's IN PROGRESS section.
2. **Step 2 — Schema:** `0009_portal_sync.sql`, `server/migrations/0001_mirror.sql`,
   `0002_rls.sql`, `server/SCHEMA.md`. Gate: `test_rls_blocks_cross_client_read`.
3. **B07 first** — make `clean_metadata()` real. It blocks Step 3a and is
   independently useful (it applies to every export, not just the portal).
