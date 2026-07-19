# specs/module-01-matters.md
# SPEC for Module 1: Matter Management
# Written before implementation. This is the contract Claude implements against.
# Do not start coding without an approved spec.

---

## Overview

The Matter is the central entity in Persist. Every other entity — deadlines, documents,
time entries, invoices, emails, AI sessions — is anchored to a Matter.
This module defines Matter CRUD, the Client entity, and the basic firm dashboard view.

---

## Data Models

### `clients` table
```sql
CREATE TABLE clients (
    id TEXT PRIMARY KEY,                    -- UUID
    name TEXT NOT NULL,                     -- Full legal name (individual or company)
    type TEXT NOT NULL DEFAULT 'Individual'
        CHECK (type IN ('Individual', 'Company', 'Partnership', 'Trust', 'Other')),
    email TEXT,
    phone TEXT,
    address TEXT,
    gstin TEXT,                             -- GST registration number (for billing)
    pan TEXT,                               -- PAN for Indian entities
    notes TEXT,                             -- Internal notes — never shown to client
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
```

### `matters` table
```sql
CREATE TABLE matters (
    id TEXT PRIMARY KEY,                    -- Format: P&P-YYYY-TYPE-NNNN
    client_id TEXT NOT NULL REFERENCES clients(id),
    title TEXT NOT NULL,                    -- Short name of the matter
    matter_type TEXT NOT NULL
        CHECK (matter_type IN ('Trademark', 'Patent', 'Design', 'Copyright', 'Corporate', 'Litigation', 'Paralegal')),
    sub_type TEXT,                          -- e.g. 'TM Application', 'Opposition', 'NDA Review'
    status TEXT NOT NULL DEFAULT 'Active'
        CHECK (status IN ('Active', 'OnHold', 'PendingClientResponse', 'Closed', 'Archived')),
    priority TEXT NOT NULL DEFAULT 'Normal'
        CHECK (priority IN ('Normal', 'High', 'Urgent')),
    responsible_partner_id TEXT REFERENCES users(id),
    forum TEXT,                             -- e.g. 'Trade Marks Registry', 'Delhi HC', 'NCLT'
    jurisdiction TEXT NOT NULL DEFAULT 'India',
    opened_date DATE NOT NULL,
    target_close_date DATE,
    internal_notes TEXT,                    -- Never visible to clients
    client_notes TEXT,                      -- Visible in client portal
    tags TEXT,                              -- JSON array of tag strings
    linked_matter_ids TEXT,                 -- JSON array of related matter IDs
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_matters_client ON matters(client_id);
CREATE INDEX idx_matters_status ON matters(status);
```

### `matter_parties` table
```sql
CREATE TABLE matter_parties (
    id TEXT PRIMARY KEY,
    matter_id TEXT NOT NULL REFERENCES matters(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id),
    role TEXT NOT NULL
        CHECK (role IN ('Partner', 'Associate', 'Paralegal', 'Admin')),
    is_primary INTEGER NOT NULL DEFAULT 0,  -- 1 = primary responsible attorney
    added_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
CREATE UNIQUE INDEX idx_matter_parties_unique ON matter_parties(matter_id, user_id);
```

---

## Keel Commands to Implement (`commands/matters.rs`)

```rust
// All return Result<T, String> for Tauri IPC

get_matter(id: String) -> Matter
list_matters(filter: MatterFilter) -> Vec<MatterSummary>
create_matter(input: CreateMatterInput) -> Matter
update_matter(id: String, input: UpdateMatterInput) -> Matter
update_matter_status(id: String, status: MatterStatus, note: Option<String>) -> Matter
assign_party(matter_id: String, user_id: String, role: String) -> MatterParty
remove_party(matter_id: String, user_id: String) -> ()
close_matter(id: String, reason: String) -> Matter
archive_matter(id: String) -> Matter
search_matters(query: String) -> Vec<MatterSummary>

// Clients
get_client(id: String) -> Client
list_clients() -> Vec<Client>
create_client(input: CreateClientInput) -> Client
update_client(id: String, input: UpdateClientInput) -> Client
```

### Filter struct
```rust
pub struct MatterFilter {
    pub status: Option<Vec<MatterStatus>>,
    pub matter_type: Option<Vec<MatterType>>,
    pub responsible_user_id: Option<String>,
    pub client_id: Option<String>,
    pub priority: Option<Vec<MatterPriority>>,
}
```

---

## TypeScript Types (`src/lib/ipc-types.ts` additions)

```typescript
export interface Matter {
    id: string;              // P&P-YYYY-TYPE-NNNN
    clientId: string;
    title: string;
    matterType: 'Trademark' | 'Patent' | 'Design' | 'Copyright' | 'Corporate' | 'Litigation' | 'Paralegal';
    subType: string | null;
    status: 'Active' | 'OnHold' | 'PendingClientResponse' | 'Closed' | 'Archived';
    priority: 'Normal' | 'High' | 'Urgent';
    responsiblePartnerId: string | null;
    forum: string | null;
    jurisdiction: string;
    openedDate: string;       // ISO date string
    targetCloseDate: string | null;
    internalNotes: string | null;   // Never shown in client portal
    clientNotes: string | null;
    tags: string[];
    linkedMatterIds: string[];
    parties: MatterParty[];
    createdAt: string;
    updatedAt: string;
}

export interface MatterSummary {
    id: string;
    title: string;
    clientName: string;
    matterType: Matter['matterType'];
    status: Matter['status'];
    priority: Matter['priority'];
    responsibleAttorney: string | null;
    nextDeadlineDate: string | null;   // ISO date — nearest upcoming deadline
    nextDeadlineEvent: string | null;
    updatedAt: string;
}

export interface Client {
    id: string;
    name: string;
    type: 'Individual' | 'Company' | 'Partnership' | 'Trust' | 'Other';
    email: string | null;
    phone: string | null;
    address: string | null;
    gstin: string | null;
    pan: string | null;
    isActive: boolean;
}

export interface MatterParty {
    userId: string;
    role: 'Partner' | 'Associate' | 'Paralegal' | 'Admin';
    isPrimary: boolean;
    name: string;           // Denormalised for display
}
```

---

## Business Rules

**Matter ID generation:**
- Format: `P&P-{YYYY}-{TYPE_CODE}-{NNNN}`
- TYPE_CODE: TM, PAT, DES, CPY, CORP, LIT, PARA
- NNNN: sequential, per year, per type — e.g. P&P-2026-TM-0042
- Generated by Keel, never by Deck

**Status transitions (valid moves only):**
```
Active → OnHold, PendingClientResponse, Closed, Archived
OnHold → Active, Closed, Archived
PendingClientResponse → Active, Closed, Archived
Closed → Archived
Archived → (no transitions — final state)
```

**Soft deletes only:** no matter is ever hard-deleted from SQLite.
Archive is the final state.

**Internal notes visibility:** `internal_notes` is NEVER returned to the client portal API.
The sync server strips this field before writing to PostgreSQL mirror.

**Responsibility:** every Active matter must have at least one party with `is_primary = 1`.
Keel enforces this on create and on party removal.

---

## Deck UI Notes

### MatterList page
- Default sort: by `updated_at` descending (most recently active first)
- Grouping option: by status, by client, by type
- Each row: title, client name, type badge, status badge, responsible attorney, next deadline
- Priority indicator: left border accent — terracotta (Urgent), amber (High), none (Normal)
- Filter bar: status multi-select, type multi-select, attorney dropdown, client search

### MatterDetail page
Five tabs: Overview, Documents, Deadlines, Communications, Billing
The Overview tab shows: status, parties, linked matters, notes, timeline

### Status badge colours
- Active → sage green
- On Hold → amber
- Pending Client Response → slate blue
- Closed → muted grey
- Archived → muted grey (dimmed)

---

## Edge Cases

- A matter can belong to only one client — no multi-client matters in v1
- A user can be a party on any number of matters simultaneously
- `responsible_partner_id` is a denormalised field for quick access —
  it should always match the user in `matter_parties` with `is_primary = 1` and role 'Partner'
  Keel keeps these in sync on party assignment/removal
- Bulk reassign: `reassign_matters(old_user_id, new_user_id, matter_ids)` batch command
  needed for when an attorney leaves — implement in v1 as a single-command bulk update
