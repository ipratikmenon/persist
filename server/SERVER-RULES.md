# Server CLAUDE.md — server/
# Rules for the Persist sync server (Axum + PostgreSQL on Hetzner).

---

## Identity

This is the Persist sync server — a lightweight Rust service running on Hetzner VPS.
Its sole job: bridge the desktop SQLite (canonical) and the client-facing surfaces
(web portal, iOS app). It does NOT contain business logic. It is a sync relay.

---

## Stack

- Axum (HTTP + WebSocket server)
- sqlx + PostgreSQL (the mirror database)
- tokio (async runtime)

---

## Rules

- This server NEVER writes to the desktop SQLite directly
- It receives pushes from the desktop and writes to PostgreSQL
- It serves reads to the client portal (FastAPI reads PostgreSQL directly)
- Row-level security enforced in PostgreSQL — not in this server's code
- mTLS between desktop and this server — client certificate pinned at build time
- JWT tokens (15-minute expiry) for client portal API calls

---

## What syncs TO PostgreSQL (from desktop)

```
matters_public      ← matter name, status, stage, responsible attorney
deadlines_public    ← upcoming deadlines (date, event name, status only)
documents_shared    ← documents explicitly shared with client
invoices            ← sent invoices (not drafts)
payments            ← payment records
client_notifications ← notification events for client
```

## What NEVER syncs to PostgreSQL

```
internal_notes      ← attorney-only
time_entries        ← billing-sensitive
billing_rates       ← confidential
draft_documents     ← not yet approved for client
ai_chat_sessions    ← session-private
audit_log           ← internal only
```

---

## WebSocket

Real-time sync uses WebSocket connections. Desktop keeps a persistent WS connection
to this server. When a syncable record changes on the desktop, it pushes a diff.
The server applies the diff to PostgreSQL and notifies connected client portal sessions.

---

## Deployment

Hetzner VPS (Ubuntu 24) behind Cloudflare proxy.
Deployed via GitHub Actions — `server/` subdirectory.
PostgreSQL managed via Hetzner Managed Databases.
