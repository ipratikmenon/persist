# Portal CLAUDE.md — portal/
# Rules for the Persist client portal (FastAPI + React).

---

## Identity

The client portal is a browser-only web application for Persistas' clients.
No installation required. Hosted on Hetzner with Cloudflare CDN.

Two parts:
- `portal/backend/` — FastAPI (Python) REST API
- `portal/frontend/` — React (TypeScript) + Vite

---

## What the client portal IS and IS NOT

IS: A read-mostly window into the client's own matters, documents, and invoices.
IS NOT: A version of the firm desktop app with features removed.

Client actions are limited to: view matter status, download documents,
upload requested documents (POA, evidence), view invoices, dispute an invoice.
That is the complete scope. Do not add features beyond this without explicit instruction.

---

## Backend (FastAPI)

```
portal/backend/
  main.py               ← FastAPI app entry point
  auth/
    otp.py              ← OTP generation + verification (email/SMS)
    jwt.py              ← JWT issue + verify (15-min expiry)
  api/
    matters.py          ← GET /matters, GET /matters/{id}
    documents.py        ← GET /documents, POST /documents/upload
    invoices.py         ← GET /invoices, POST /invoices/{id}/dispute
    notifications.py    ← GET /notifications
  db/
    session.py          ← PostgreSQL connection (reads the mirror DB)
    models.py           ← SQLAlchemy models matching PostgreSQL mirror schema
  middleware/
    auth.py             ← JWT validation middleware
    rls.py              ← Row-level security enforcement
```

### Security rules (backend)
- OTP auth only — no passwords stored anywhere
- JWT tokens expire in 15 minutes — refresh token pattern
- Row-level security: EVERY query must filter by `client_id = current_user.id`
  Do not trust the frontend to scope queries correctly — enforce in the API
- Documents served as signed time-limited URLs (5-minute expiry) — never direct file URLs
- All uploads scanned before storage (ClamAV or equivalent)
- Rate limiting on all endpoints (slowapi)
- HTTPS only — HSTS header on all responses

### What the portal API can NEVER return
- Internal notes
- Time entries or billing rates
- Other clients' data in any form
- Draft documents (not yet approved for sharing)
- Unapproved invoices

---

## Frontend (React)

The portal frontend shares the same React component library as the Deck
(same design tokens, same UI primitives) but has its own pages and routing.

```
portal/frontend/
  src/
    pages/
      Matters/          ← matter list + detail (read-only)
      Documents/        ← document list + upload
      Invoices/         ← invoice list + dispute
      Profile/          ← settings + notification preferences
    components/         ← shared with Deck where applicable
    lib/
      api.ts            ← fetch() calls to FastAPI backend
      auth.ts           ← OTP login flow
```

**No Tauri in the portal frontend** — it's a browser app, not a desktop app.
Data fetching uses standard `fetch()` to the FastAPI backend.
No `invoke()`, no Tauri imports.

### The API is namespaced under `/api` — always

`/matters` and `/invoices` are both React routes *and* API endpoints. Serving
both from `/` on one origin means the edge cannot tell a page load from an API
call, and a browser navigation gets handed raw JSON. So every call the portal
makes goes to `/api/...`, and the edge (Cloudflare in production, Vite in dev)
strips the prefix before FastAPI sees it.

### Tokens

The access token lives in a module variable in `lib/api.ts` and nowhere else.
Never `localStorage`, never `sessionStorage`: anything that can run a script on
the page could read it there, and a stolen token is a client's entire matter
file. In memory, it dies with the tab.

The refresh token is an httpOnly cookie the backend sets — unreadable from
JavaScript. It is scoped `path=/`, not `/auth`, because the app cannot know what
prefix the edge mounted it under; a narrower path silently stops the cookie
being sent and every page reload looks like a logout.

On a cold load there is no access token, so the app asks for one from the cookie
before deciding whether to show the login screen.

### Design tokens come from Deck, not a copy

`src/design-system/tokens.ts` re-exports `src/design-system/tokens.ts` from the
desktop app via a Vite alias. A client sees this portal and the firm's invoices
side by side; two drifting copies of the palette would show.

### Money is a string, end to end

The API returns decimals as strings and the portal keeps them strings all the
way to the screen. Parsing to a float to format it is how a bill drifts by a
paisa. Grouping is Indian — three digits then twos — matching the invoice PDF.

---

## Deployment

- FastAPI backend: Hetzner VPS, systemd service, uvicorn
- React frontend: Hetzner Object Storage static hosting + Cloudflare CDN
- Domain: app.persistas.com (TBC — see PRD open questions)
- PostgreSQL: reads from the mirror database populated by the sync server
