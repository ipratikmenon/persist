# Session S17 — 2026-08-06
## Deck Surface for Module 5

---

## Summary

Four sprints of backend had accumulated with no way for an attorney to reach any
of it: RBAC enforced but no gated UI, verification badges with no verify action,
11 sync commands with no caller, and a Renewals dashboard reachable only by
typing a URL. This session closed all of that.

**Result:** cargo test **146/146**, pnpm build PASS, 12 screenshots. On
`claude/new-session-dbqe5o`; nothing pushed to main.

---

## Files

### New
- `src/pages/Portal/PortalHome.tsx` — Client access + Sync tabs
- `src/lib/permissions.ts` — UI gating, mirrors `rbac.rs`
- `SESSION-LOG/2026-08-06-S17-portal-deck-surface.md`

### Modified
- `src/pages/Documents/DocumentList.tsx` — share/unshare toggle, role-gated
  Delete, SHARED badge now reflects live state rather than the stale prop
- `src/pages/Dockets/IPAssetRecord.tsx` — verify action, client-visibility toggle
- `src/components/shell/AppShell.tsx` — Renewals + Portal nav, active-state fix
- `src/App.tsx` — `/portal` route
- `src/lib/ipc-types.ts`, `src-tauri/src/commands/deadlines.rs`,
  `src-tauri/src/db/queries/deadlines.rs` — expose `is_client_visible` on the
  wire type; the row carried it but the IPC type did not
- `screenshots/{mock/core.ts, capture.mjs}` — two new views

---

## Decisions

**Deck permission gating duplicates `rbac.rs` by hand.** The auth spec is
explicit that Deck gating is UX convenience and Keel is the enforcer, so a
divergence between the two degrades to "button shown, command refused" — never
to a leak. That makes a hand-kept copy an acceptable cost for not adding an IPC
round trip per button. The file says so at the top, loudly.

**The SHARED badge had to become live state.** It was reading `doc.isSharedWithClient`
from the prop, so toggling share left the badge stale until a reload. Now both
the badge and the button read the same local state.

**Nav active-state was matching on prefix.** `/dockets` lit up whenever the route
was `/dockets/renewals`, so two items appeared active. Changed to exact-or-child.
This only surfaced because Renewals was added as a sibling — it had been latent.

**Verify errors are surfaced verbatim.** Keel refuses when the verifier is the
person who entered the date; the UI shows that message rather than pre-guessing
who may verify, because the author is not something Deck reliably knows.

---

## Commands Run

```bash
cd src-tauri && cargo test --lib     # 146/146
pnpm build                            # PASS
npx vite build --config vite.config.screenshots.ts && node screenshots/capture.mjs
```

---

## Not Done / Deferred

- **The transport (Step 3b).** Sync can now be configured and enabled from the
  UI, but `trigger_sync` still errors explicitly rather than sending. Nothing
  leaves the machine.
- **Outbox write-path wiring** — unchanged from S16. Matters, deadlines, IP
  assets, invoices and payments still do not enqueue. Lands with the transport.
- **Portal user status never becomes `Active`.** Invitations are created as
  `Invited`; only a real portal login flips that, which needs Step 3c.
- **No invitation is actually sent.** `invite_portal_user` records the row; the
  email goes out from the portal backend, which does not exist yet.
- **B06** — `parseKeelDateTime` still only used for session expiry.
- **Screenshots remain fixture-driven** — layout and states are faithful, but
  they are not proof the IPC wiring works end to end.

---

## Next Session Options

1. **M5 Step 3b** — Axum sync server, push/pull transport, outbox wiring.
2. **M5 Step 3c** — FastAPI portal backend (OTP + JWT), which is what makes
   invitations real.
3. **Close remaining older gaps** — B06, first-launch wizard, user management.
