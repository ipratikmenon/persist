# specs/module-03-documents.md
# SPEC for Module 3: Document Management
# Written before implementation. This is the contract Claude implements against.

---

## Overview

Every document in Persist is encrypted at rest in the local vault, versioned, and linked to its
matter. No document ever touches a server unless explicitly shared. The document store is the
secure record of every filing, correspondence, draft, and evidence file the firm handles.

The `clean_metadata()` function is non-negotiable: no document leaves the firm without metadata
stripping. This is built in at the vault layer, not as an optional step.

---

## Data Models

### `documents` table
```sql
CREATE TABLE documents (
    id TEXT PRIMARY KEY,                      -- UUID
    matter_id TEXT NOT NULL REFERENCES matters(id) ON DELETE RESTRICT,
    -- RESTRICT not CASCADE — never silently delete documents
    title TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'Correspondence'
        CHECK (category IN ('Correspondence','FiledDocument','CourtOrder','ClientDocument',
                            'Draft','InternalNote','Evidence','Invoice','Agreement','Other')),
    sub_category TEXT,                        -- e.g. 'ExaminationReport', 'POA', 'Affidavit'
    file_vault_path TEXT NOT NULL,            -- path inside AES-256 vault
    file_name TEXT NOT NULL,                  -- original filename with extension
    file_extension TEXT NOT NULL,             -- lowercase: 'pdf', 'docx', 'jpg', etc.
    file_size_bytes INTEGER NOT NULL,
    mime_type TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    parent_version_id TEXT REFERENCES documents(id), -- prior version
    is_latest_version INTEGER NOT NULL DEFAULT 1,
    -- Document status
    status TEXT NOT NULL DEFAULT 'Draft'
        CHECK (status IN ('Draft','AwaitingSignature','Executed','Filed','Superseded','Archived')),
    -- Sharing
    is_shared_with_client INTEGER NOT NULL DEFAULT 0,
    shared_at DATETIME,
    shared_by TEXT REFERENCES users(id),
    -- Metadata
    author TEXT,                              -- extracted from document metadata
    source TEXT,                             -- 'uploaded'|'generated'|'received_email'|'received_portal'
    -- Linking
    linked_deadline_id TEXT REFERENCES deadlines(id),
    linked_ip_asset_id TEXT REFERENCES ip_assets(id),
    -- Full-text search (extracted on upload)
    content_text TEXT,
    -- Permissions
    restricted_to_user_ids TEXT,             -- JSON array; null = all matter parties
    -- Tags
    tags TEXT DEFAULT '[]',
    notes TEXT,
    uploaded_by TEXT NOT NULL REFERENCES users(id),
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_docs_matter ON documents(matter_id, is_latest_version, created_at DESC);
CREATE INDEX idx_docs_category ON documents(matter_id, category);
CREATE INDEX idx_docs_shared ON documents(is_shared_with_client, matter_id);
-- Full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
    title, content_text, notes,
    content='documents',
    content_rowid='rowid'
);
```

---

## Vault Architecture

All document bytes live exclusively in the encrypted vault managed by `storage/vault.rs`.
The `documents` table stores only metadata and the vault path — never the file bytes.

```
~/Library/Application Support/com.persistas.persist/vault/
└── {matter_id}/
    └── {document_id}_{version}.enc   ← AES-256-GCM encrypted bytes
```

**Vault operations (all in `storage/vault.rs`):**

```rust
pub fn encrypt_to_vault(
    key: &[u8; 32],
    bytes: &[u8],
    matter_id: &str,
    document_id: &str,
    version: u32,
) -> Result<String>  // returns vault_path

pub fn decrypt_from_vault(
    key: &[u8; 32],
    vault_path: &str,
) -> Result<Vec<u8>>

pub fn clean_metadata(
    bytes: &[u8],
    extension: &str,
) -> Result<Vec<u8>>   // strips author, revision history, comments, tracked changes
```

**`clean_metadata` must handle:**
- PDF: strip XMP metadata, DocInfo dict, embedded thumbnails, hidden layers
- DOCX/XLSX/PPTX: strip author, company, revision history, comments, tracked changes, custom properties
- Images (JPG/PNG/TIFF): strip EXIF, GPS, camera info
- Other: pass through unchanged (no stripping attempted)

**When `clean_metadata` is called:**
- ALWAYS before any external share (share link, email attachment, client portal upload)
- ALWAYS before any document export out of Persist
- NEVER on the stored vault copy — the original is always preserved

---

## Keel Commands (`commands/documents.rs`)

```rust
// Upload
upload_document(input: UploadDocumentInput) -> Document
// input: { matter_id, title, category, bytes (base64), file_name, linked_deadline_id?, linked_ip_asset_id? }
// Keel: encrypt to vault, extract text for FTS, create DB record, return Document

// Retrieve
get_document(id: String) -> Document
get_document_bytes(id: String) -> Base64Bytes  // decrypts from vault, returns bytes
list_documents(filter: DocumentFilter) -> Vec<DocumentSummary>
search_documents(matter_id: String, query: String) -> Vec<DocumentSearchResult>

// Versioning
create_new_version(document_id: String, bytes: Base64Bytes, notes: String) -> Document
// Sets prior version is_latest_version=0, creates new version record

// Sharing
share_with_client(document_id: String) -> Document
// Sets is_shared_with_client=1, triggers sync server push to client portal
revoke_client_share(document_id: String) -> Document

// Export (always cleans metadata)
export_document(document_id: String, clean: bool) -> Base64Bytes
// If clean=true (default): decrypt → clean_metadata → return
// Deck always uses clean=true for any external-facing export

// Status management
update_document_status(id: String, status: String) -> Document
link_to_deadline(document_id: String, deadline_id: String) -> Document

// Deletion (soft only)
archive_document(id: String) -> Document  // status=Archived, removed from active views
```

---

## TypeScript Types (`src/lib/ipc-types.ts` additions)

```typescript
export interface Document {
    id: string;
    matterId: string;
    title: string;
    category: 'Correspondence' | 'FiledDocument' | 'CourtOrder' | 'ClientDocument'
        | 'Draft' | 'InternalNote' | 'Evidence' | 'Invoice' | 'Agreement' | 'Other';
    subCategory: string | null;
    fileName: string;
    fileExtension: string;
    fileSizeBytes: number;
    mimeType: string;
    version: number;
    isLatestVersion: boolean;
    status: 'Draft' | 'AwaitingSignature' | 'Executed' | 'Filed' | 'Superseded' | 'Archived';
    isSharedWithClient: boolean;
    sharedAt: string | null;
    source: 'uploaded' | 'generated' | 'received_email' | 'received_portal';
    linkedDeadlineId: string | null;
    linkedIpAssetId: string | null;
    tags: string[];
    notes: string | null;
    uploadedBy: string;
    createdAt: string;
    updatedAt: string;
}

export interface DocumentSummary {
    id: string;
    matterId: string;
    title: string;
    category: Document['category'];
    fileName: string;
    fileExtension: string;
    fileSizeBytes: number;
    version: number;
    status: Document['status'];
    isSharedWithClient: boolean;
    createdAt: string;
}

export interface UploadDocumentInput {
    matterId: string;
    title: string;
    category: Document['category'];
    bytes: string;          // base64-encoded file bytes
    fileName: string;
    linkedDeadlineId?: string;
    linkedIpAssetId?: string;
    tags?: string[];
    notes?: string;
}

export interface DocumentFilter {
    matterId?: string;
    category?: Document['category'][];
    isSharedWithClient?: boolean;
    linkedDeadlineId?: string;
    status?: Document['status'][];
}
```

---

## Business Rules

**Immutable vault:** once a document version is encrypted to the vault, its bytes are never
modified. A new upload of the same document creates a new version record. The prior version's
`is_latest_version` flag is set to 0 but the encrypted file remains in the vault permanently.

**Matter restriction on deletion:** documents cannot be deleted if their matter is Active or
has any open deadlines. Archiving is the only removal option while the matter is live.
Hard deletion (vault file removal) is only available once the matter is permanently Archived
and only by a Partner role. Even then, Keel logs the deletion to the audit table with the
deleting user, timestamp, and document title.

**Full-text search:** when a document is uploaded, Keel attempts text extraction:
- PDF: use `lopdf` crate or Tesseract if scanned
- DOCX: extract from XML
- Other: skip FTS indexing, document is findable by title/category only

Text extracted into `documents.content_text`, then indexed into `documents_fts` virtual table.
Search via `fts5` MATCH query — not LIKE. Fast enough for firm-scale document libraries.

**Sharing restriction:** only documents with `status = 'Executed'`, `'Filed'`, or explicitly
approved by a Partner can be shared with a client. Deck should prevent sharing Draft documents
and Keel should enforce this at the command level.

**Metadata always cleaned on export:** `export_document` always calls `clean_metadata()`.
If the attorney explicitly needs the original unstripped file (rare — e.g. for internal
archiving), they use `get_document_bytes()` directly, which is only accessible from the
firm desktop app, never from the client portal.

---

## Deck UI Notes

### DocumentStore page (`pages/Documents/DocumentStore.tsx`)
- List view: table rows with title, category badge, file type icon, date, status badge,
  shared-with-client indicator
- Sidebar filter: by category, by status, by IP asset, by deadline link
- Bulk actions: share selected with client, update status, archive
- Upload: drag-and-drop or file picker; category and matter auto-detected where possible
- Version history: expandable row showing all versions with diff date/uploader

### DocumentViewer component (`components/documents/DocumentViewer.tsx`)
- PDF: render via pdf.js bundled in Deck (no external service)
- DOCX: render via mammoth.js (converts to HTML for preview, original preserved in vault)
- Images: native `<img>` tag with decrypted bytes as blob URL
- Other: show file type icon + metadata, offer download only

---

## Edge Cases

- Upload of a duplicate file (same content, same matter): Keel detects by SHA-256 hash of
  the decrypted bytes. If a match is found, warn the attorney: "This file appears to already
  be in the document store as [title]. Upload as a new version?" Options: new version / new
  separate document / cancel.
- Document linked to a deadline that is subsequently waived: the document remains, the link
  is preserved. The link does not affect document status.
- Large file uploads (>50MB): PDFs for large patent families or court bundles can be large.
  Deck chunks the base64 upload. Keel streams the decryption. No size limit imposed — the
  vault is only bounded by local disk space.
- Document received via email (Module 15 integration): the mail sync service calls
  `upload_document` with `source='received_email'` and links the document to the email
  thread's matter automatically if the email is bundled to a matter.
