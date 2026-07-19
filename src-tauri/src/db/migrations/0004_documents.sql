-- Phase 1 Module 3: Document Management
-- Stores encrypted-at-rest document metadata.
-- Document BYTES live in the vault (src-tauri/storage/vault.rs).
-- NEVER store raw bytes in this table.

CREATE TABLE IF NOT EXISTS documents (
    id                   TEXT PRIMARY KEY,
    matter_id            TEXT NOT NULL REFERENCES matters(id) ON DELETE CASCADE,
    filename             TEXT NOT NULL,
    category             TEXT NOT NULL DEFAULT 'Other'
        CHECK (category IN ('Correspondence','Filing','Certificate','SearchReport','Invoice','Contract','Other')),
    mime_type            TEXT NOT NULL DEFAULT 'application/octet-stream',
    file_size_bytes      INTEGER NOT NULL DEFAULT 0,
    version              INTEGER NOT NULL DEFAULT 1,
    vault_path           TEXT NOT NULL,                      -- relative path inside vault dir
    uploaded_by          TEXT NOT NULL DEFAULT 'system',
    is_shared_with_client INTEGER NOT NULL DEFAULT 0,        -- 1 = visible in client portal
    description          TEXT,                               -- optional user note
    created_at           DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at           DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_documents_matter   ON documents(matter_id);
CREATE INDEX IF NOT EXISTS idx_documents_category ON documents(category);
CREATE INDEX IF NOT EXISTS idx_documents_created  ON documents(created_at DESC);
