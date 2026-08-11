# Session S21 — 2026-08-11
## Module 5 Step 4 — the Portal Frontend

---

## Summary

This closes M5. Firm data reaches the mirror (S18 Step 3b), the API serves it
under four independent locks (S18 Step 3c), and now a client can actually read
it in a browser.

Four tabs, OTP login, no password anywhere. Built against the live API and a
real PostgreSQL mirror rather than fixtures, which is how both of this
session's bugs were found — neither would have shown up in a build or a unit
test.

**Result:** portal **48/48**, Deck build PASS, portal build PASS. On
`claude/new-session-dbqe5o`.

---

## What was built

`portal/frontend/` — React 19 + Vite + react-router.

| Page | Contents |
|---|---|
| Matters | List with next deadline; detail with dates, registrations and documents |
| Documents | Everything shared, download by signed URL; upload with a queue showing status |
| Invoices | List with total outstanding; detail with GST breakdown, payments, PDF, "raise a query" |
| Account | Details (read-only), recent updates, sign out |

`lib/api.ts` is the only place the portal talks to the backend — typed against
`models.py`, with one place that handles 401-and-refresh.

### Design tokens are aliased, not copied

`portal/frontend/src/design-system/tokens.ts` re-exports Deck's file through a
Vite alias. A client sees this portal and the firm's invoices side by side; two
copies kept in step by hand would drift, and the drift would be visible. The CI
build fails if the alias stops resolving.

### Token handling

The access token lives in a module variable and nowhere else. `localStorage`
would mean anything that can run a script on the page can read it, and a stolen
token is a client's entire matter file. In memory it dies with the tab.

The refresh token is an httpOnly cookie, so a page reload restores the session
instead of looking like a logout. Concurrent 401s share one refresh promise —
the backend rotates on every refresh, so a second concurrent call would present
a token already spent, be read as a replay, and revoke the whole family.

### Money

Decimals arrive as strings and stay strings to the screen. Parsing to a float to
format is how a bill drifts by a paisa. Grouping is Indian — `₹1,55,760.00` —
matching the invoice PDF exactly, so the two never disagree.

### The backend now speaks camelCase

Keel's IPC structs carry `rename_all = "camelCase"` and Deck's types match. The
portal was the one surface where a client-side developer had to remember
otherwise, so `models.py` got an alias generator and the suite was updated.

---

## Two bugs, both found only by running it

**`/matters` is both a React route and an API endpoint.** The first Playwright
run navigated to `/matters` and got `{"detail":"Not authenticated"}` rendered as
a page. A same-origin deployment — which is the documented plan, Cloudflare in
front of both — cannot tell a browser navigation from an API call when the paths
collide. Every call now goes to `/api/...` and the edge strips the prefix. A
build would never have caught this; nor would the backend suite, which never
loads a page.

**The refresh cookie stopped being sent.** It was scoped `path="/auth"` — a
deliberate narrowing from S18. Once the API moved to `/api/auth/...` the browser
correctly declined to send it, so `restoreSession` failed and every reload
logged the client out. The harness asserts on this directly, and printed
`✗ RELOAD LOGGED THE CLIENT OUT` before the fix and `✓ session survived a
reload` after.

Now `path="/"`. The app cannot know what prefix the edge mounted it under, and
path is not a security boundary in any case — any page on the origin can trigger
a request to `/auth/*`. `httponly` is what protects the value.

---

## Commands Run

```bash
cd portal/frontend && pnpm install && pnpm build

# The API, against the same mirror the tests use
uvicorn app.main:app --port 8000     # with PORTAL_* env

# End to end: full OTP login, four tabs, and a reload
node preview.mjs
```

The harness proxies `/api/*` to FastAPI and serves everything else as the
bundle — exactly what Cloudflare will do — so what is exercised is what will be
deployed. It also surfaces console errors and failed requests, because a silent
console error is how a blank screenshot gets mistaken for a working page.

Ten screenshots, all four tabs rendering real data from a real mirror.

---

## Not Done / Deferred

- **Object storage, virus scanning, OTP email delivery** — all M5 Step 3d.
  Downloads redirect correctly and uploads queue correctly; the bytes have
  nowhere to live yet.
- **Notification read receipts.** The route returns 501 rather than pretending;
  `portal_reader` cannot write to `mirror.*` by design, so it needs an inbound
  queue.
- **No frontend unit tests.** The Playwright harness covers the paths that
  matter end to end; component tests would be worth adding when the pages grow
  logic beyond fetch-and-render.
- **Mobile layout.** It reflows because the layout is simple, but it has not
  been designed for a phone, and clients will read this on phones.
- **Sidecar bundling (B03)** — needs a real build machine.

---

## Next Session Options

1. **M9 — the Smart Form Compiler UI.** The backend has been ready since S20.
2. **M5 Step 3d** — object storage and email, which makes documents actually
   flow rather than queue.
3. **Mobile layout pass** on the portal.
