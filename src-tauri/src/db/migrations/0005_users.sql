-- Phase 1 Auth: User accounts and RBAC
-- Two firm attorneys are seeded at first startup via Rust (see lib.rs).
-- No registration flow — user management is an admin operation.

CREATE TABLE IF NOT EXISTS users (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    email         TEXT NOT NULL,
    role          TEXT NOT NULL DEFAULT 'Associate'
        CHECK (role IN ('Partner', 'Associate', 'Paralegal', 'Admin')),
    password_hash TEXT NOT NULL,           -- bcrypt hash, cost 12
    is_active     INTEGER NOT NULL DEFAULT 1,
    last_login_at DATETIME,
    created_at    DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at    DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email ON users(email);
