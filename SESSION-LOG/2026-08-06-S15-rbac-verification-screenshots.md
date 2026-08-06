# Session S15 — 2026-08-06
## Three Parallel Sprints + Screenshot Harness

---

## Parallelism Analysis

Asked which sprints could run in parallel. The outstanding work splits into five
groups that are independent *by design* — no shared ordering, no shared files:

| Group | Scope | Touches |
|---|---|---|
| A — Keel / auth | RBAC enforcement, first-launch wizard, user management | `rbac.rs`, `commands/auth.rs` |
| B — Keel / docketing | Dual verification, reference numbers, docket_errors | `commands/deadlines.rs`, new migration |
| C — Deck | Cascade preview UI, verification badges, B06 date conversion | `src/` only |
| D — Portal backend | FastAPI (M5 Step 3c) | `portal/` — separate tree, separate language |
| E — Sync server | Axum routes (M5 Step 3b) | `server/` — separate crate |

**Coordination points** (why "parallel" still needs sequencing in one session):
migration numbering (only one group may claim `0011`), `lib.rs` invoke_handler
registration, and `ipc-types.ts` when a Keel type changes.

Ran A, B and C. D and E are larger new surfaces and depend on the M5 spec review.

---

## Sprint A — RBAC Enforcement

`specs/auth-rbac.md` specified a permission matrix that nothing checked: any
signed-in user could close a matter or issue an invoice.

`src-tauri/src/rbac.rs` implements it as a lookup table rather than scattered
`if role ==` checks, so the matrix lives in one place, reads like the spec, and
is exhaustively testable. Guards applied to: close/archive matter,
delete_document, create_invoice, update_firm_settings, create_deadline,
verify_deadline. The ad-hoc Partner check in billing was replaced by the shared
guard.

**Unknown roles fail closed** — rank 0, denied everything. Being helpful about a
role that should not exist is how holes appear.

`require()` returns the session, so callers get attribution (`created_by`,
`verified_by`) without a second lookup — the two things almost always go
together, and fetching them separately invites using one without the other.

## Sprint B — Dual Verification + Reference Numbers

`0011_verification.sql`: `created_by`, `reference_number`, `is_verified`,
`verified_by`, `verified_at` on deadlines, plus the `docket_errors` table.

`created_by` had to come first: without knowing who entered a date, "someone
else checked it" is unprovable. It is set from the session and marked
`#[serde(skip_deserializing)]` — dual verification is defeated if Deck can lie
about authorship.

`verify_deadline` refuses when the verifier is the author. `P&P-DD-NNNN`
numbering is sequential per firm (shared `sequences` table). Statutory deadlines
now default to client-visible at creation, matching the M5 projection rule.

## Sprint C — Cascade Preview UI

`CascadePreview.tsx` — preview-then-commit drawer showing the whole chain, the
template's `last_verified` date, and which rows the client will see.
`VerificationBadge.tsx` + `ReferenceChip` on timeline rows; badge is suppressed
on Procedural/Custom deadlines, which are not subject to the control.

Found and fixed a real bug while capturing screenshots: `humanise` did not split
the `TM` acronym, so anchor chips rendered as literal `TMApplication`.

---

## Screenshot Harness

The desktop app is Tauri: it needs a display and a live Keel, neither of which
exists here. `vite.config.screenshots.ts` aliases `@tauri-apps/api/core` (and the
dialog/fs plugins) to fixture modules, so the **real components** render against
representative data in a browser. Playwright captures 10 views at 2× scale.

```bash
npx vite build --config vite.config.screenshots.ts
node screenshots/capture.mjs      # -> screenshots/out/*.png
```

The alias is wired only by that config, so fixtures can never reach `pnpm build`.
An unmocked command logs a warning rather than failing silently.

Two corrections made after looking at the output: the document mock now scopes by
`matterId` (the vault view was showing the same fixtures repeated per matter),
and the documents shot hovers a row so the per-row actions — including the B07
"Client copy" export — are actually visible.

---

## Commands Run

```bash
cd src-tauri && cargo test --lib     # 122/122 (was 113)
pnpm build                           # PASS
npx vite build --config vite.config.screenshots.ts && node screenshots/capture.mjs
```

---

## Not Done / Deferred

- **`commands/docket_audit.rs`** — the `docket_errors` table exists; the commands
  to read and write it do not. The abandonment watcher still records L4 as an
  escalation only.
- **First-launch wizard, change_password, create_user, list_users** — the rest of
  the auth spec.
- **B06** — `parseKeelDateTime` still only used for session expiry.
- **Deck role gating** — Keel now enforces RBAC, but Deck does not yet hide the
  buttons a Paralegal cannot use. Per spec that is UX convenience, not security,
  so the order is right; it is still missing.
- **A verify action in the UI** — `verify_deadline` and `list_unverified_deadlines`
  are wired in `lib/tauri.ts`, but no screen calls them yet. The badge shows
  state; nothing acts on it.
- Screenshots use fixtures, not a live database — they show layout and states
  faithfully, but they are not proof the wiring works end-to-end.

---

## Next Session Options

1. **M5 Step 3a** — Keel sync engine: `projection.rs` (security-critical), outbox
   drain, 15 sync commands.
2. **Finish the verification loop in Deck** — an unverified queue with a verify
   action, plus role-gated UI.
3. **Portal backend (3c) / sync server (3b)** — the two large parallel surfaces.
