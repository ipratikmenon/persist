-- Migration 0007: Persistent sessions (resolves B01)
-- Phase 1 Auth — see specs/auth-rbac.md §"sessions table"
-- Never edit this file. Create new migrations for all changes.
--
-- Before this migration the session lived only in AppState (in-memory), so every
-- app restart forced a re-login. Sessions are now rows with an 8-hour expiry; the
-- session id (a UUID, used as the bearer token) is held in the OS keychain by
-- services/keychain.rs — never in SQLite alongside the data it protects.

CREATE TABLE IF NOT EXISTS sessions (
    id               TEXT PRIMARY KEY,   -- UUID v4 — the session token itself
    user_id          TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- 8 hours from creation; extended by refresh_session on user activity.
    expires_at       DATETIME NOT NULL,
    last_activity_at DATETIME NOT NULL DEFAULT (datetime('now')),

    created_at       DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at       DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_sessions_user    ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);
