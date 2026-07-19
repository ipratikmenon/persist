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

---

## Deployment

- FastAPI backend: Hetzner VPS, systemd service, uvicorn
- React frontend: Hetzner Object Storage static hosting + Cloudflare CDN
- Domain: app.persistas.com (TBC — see PRD open questions)
- PostgreSQL: reads from the mirror database populated by the sync server
