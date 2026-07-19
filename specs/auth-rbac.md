# specs/auth-rbac.md
# SPEC for Authentication & Role-Based Access Control
# Written before implementation. This is the contract Claude implements against.

---

## Overview

Persist Desktop is a single-firm tool used by 2–10 attorneys, associates, and paralegals.
Authentication is local-first: users log in with their firm credentials stored locally in
the OS keychain. There is no central auth server for the desktop app.

The client portal uses a separate OTP-based auth system (covered in Module 5 spec).

---

## Data Models

### `users` table
```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    role TEXT NOT NULL DEFAULT 'Associate'
        CHECK (role IN ('Partner','Associate','Paralegal','Admin')),
    initials TEXT NOT NULL,                   -- e.g. 'SL', 'KT' — for avatars
    colour_hex TEXT NOT NULL DEFAULT '#4A6580', -- for avatar background
    is_active INTEGER NOT NULL DEFAULT 1,
    password_hash TEXT NOT NULL,              -- bcrypt hash
    last_login_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
```

### `sessions` table
```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,                      -- UUID token
    user_id TEXT NOT NULL REFERENCES users(id),
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    expires_at DATETIME NOT NULL,             -- 8 hours from creation
    last_activity_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_sessions_user ON sessions(user_id);
```

### Initial firm users (seeded at first launch)

Partners at Persistas & Partners:
- Sree Lakshmi Menon — Partner — initials: `SL` — colour: `#4A6580` (slate blue)
- Kajal Thakur — Partner — initials: `KT` — colour: `#B5604A` (terracotta)

These two users are seeded as part of the initial migration. First-launch wizard sets their
passwords. Additional users (associates, paralegals) created by a Partner via Settings.

---

## Keel Commands (`commands/auth.rs`)

```rust
// Authentication
login(email: String, password: String) -> SessionToken
// Hash password with bcrypt, compare to stored hash.
// On success: create session, store token in OS keychain, return SessionToken.
// On failure: return Err("Invalid credentials"). Rate limit: 5 attempts then 60s lockout.

logout(session_id: String) -> ()
// Delete session record from DB, clear token from OS keychain.

get_current_user(session_id: String) -> User
// Validate session not expired, return user. Called on every app open.

refresh_session(session_id: String) -> SessionToken
// Extend expiry by 8 hours from now. Auto-called on user activity.

// User management (Partner + Admin only)
create_user(input: CreateUserInput) -> User
update_user(id: String, input: UpdateUserInput) -> User
deactivate_user(id: String) -> User           // Soft disable — never delete users
list_users() -> Vec<User>

// Password
change_password(current: String, new: String) -> ()
// Validate current password, hash new password, update. Never store plaintext.
```

---

## RBAC Rules

Four roles with escalating permissions:

| Permission | Paralegal | Associate | Partner | Admin |
|---|---|---|---|---|
| View all matters | ✓ | ✓ | ✓ | ✓ |
| Create/edit matters | ✗ | ✓ | ✓ | ✓ |
| Close/archive matters | ✗ | ✗ | ✓ | ✓ |
| View all documents | ✓ | ✓ | ✓ | ✓ |
| Upload documents | ✓ | ✓ | ✓ | ✓ |
| Share documents with client | ✗ | ✓ | ✓ | ✓ |
| Hard-delete archived documents | ✗ | ✗ | ✓ | ✓ |
| Create/edit deadlines | ✓ | ✓ | ✓ | ✓ |
| Verify statutory deadlines | ✗ | ✓ | ✓ | ✓ |
| Waive deadlines | ✗ | ✗ | ✓ | ✓ |
| View billing / time entries | ✗ | Own only | ✓ | ✓ |
| Create invoices | ✗ | ✗ | ✓ | ✓ |
| Manage users | ✗ | ✗ | ✓ | ✓ |
| View analytics dashboard | ✗ | ✗ | ✓ | ✓ |
| Access AI drafting | ✗ | ✓ | ✓ | ✓ |
| Access Persist Chat | ✓ | ✓ | ✓ | ✓ |

**RBAC enforcement:** permission checks run in Keel at the command level. Commands that
require elevated permissions check the session's user role before executing. If the role
is insufficient, return `Err("Insufficient permissions: [required role] required")`.

Deck gates UI elements based on role stored in Zustand after login, but Keel is the
authoritative enforcer. Deck gating is UX convenience only — never relied on for security.

---

## TypeScript Types

```typescript
export interface User {
    id: string;
    name: string;
    email: string;
    role: 'Partner' | 'Associate' | 'Paralegal' | 'Admin';
    initials: string;
    colourHex: string;
    isActive: boolean;
    lastLoginAt: string | null;
    createdAt: string;
}

export interface SessionToken {
    sessionId: string;
    userId: string;
    expiresAt: string;
    user: User;
}
```

---

## Session Management in Deck

On login success, Deck stores the `SessionToken` in the Zustand `ui` store.
The session ID is also stored in the OS keychain via Tauri's `keychain` API.

On every app open:
1. Read session ID from OS keychain
2. Call `get_current_user(session_id)` — if valid, populate Zustand user store
3. If expired or not found: show login screen

Auto-lock: after 10 minutes of inactivity (configurable in Settings), Persist clears the
Zustand session and shows the lock screen. The OS keychain token is NOT cleared on auto-lock
(just on explicit logout) — the attorney unlocks by re-entering their password, which
validates against the DB and re-creates the session.

---

## First-Launch Setup Wizard

When the app opens for the first time (no users in DB):
1. Welcome screen — "Setting up Persist for Persistas & Partners"
2. Create Partner 1: pre-filled name "Sree Lakshmi Menon", email, password setup
3. Create Partner 2: pre-filled name "Kajal Thakur", email, password setup
4. Confirm: launch to the main application

This wizard is a special Deck route shown only when `list_users()` returns an empty array.
After setup it is never accessible again.

---

## Security Rules

- Passwords: bcrypt with cost factor 12. Never store plaintext. Never log passwords.
- Session tokens: UUIDs stored in OS keychain (`src-tauri/services/keychain.rs`). Never in SQLite plaintext. Never in environment variables.
- The `vault_key` (AES-256 document encryption key) is derived from the user's password using Argon2id. Never stored directly. Re-derived on each login from the stored Argon2 params + the entered password.
- Rate limiting on login: 5 failed attempts → 60-second lockout. State stored in memory (Tokio mutex), not DB — resets on app restart (acceptable for local desktop app).
- The auto-lock screen must NOT show matter names, document titles, or any data. Only the firm name and unlock prompt.
