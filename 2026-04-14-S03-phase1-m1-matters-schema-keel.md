# Session S03 — Phase 1 Module 1: Matters schema + Keel CRUD commands

**Date:** 2026-04-14
**Session #:** S03
**Phase:** Phase 1 — Foundation
**Module:** Module 1 — Matter Management
**Step:** Steps 2 + 3 (Schema + Keel) — combined because schema was small
**Duration:** ~55 minutes
**Model used:** Sonnet 4.6
**Commit:** `a3f8c21` — [p1/m1] feat: matters + clients schema and Keel CRUD commands

---

## 🎯 Goal

Create the SQLite migration for matters and clients, then implement all Keel CRUD
commands for matters management so Deck can create, list, and update matters.

---

## 📋 Starting State

From PROGRESS.md at session start:
- Phase 0 scaffold: ✅ complete (S01, S02)
- Module 1 spec: ✅ `specs/module-01-matters.md` approved
- Module 1 schema: ❌ not started — this session
- Module 1 Keel: ❌ not started — this session

---

## 🔧 Actions Taken

### 1. Read spec and KEEL-RULES.md

**What:** Read `specs/module-01-matters.md` and `src-tauri/KEEL-RULES.md` before writing any code.
**Why:** Spec defines exact column names and constraints. Keel rules define async and sqlx patterns.

Noted from spec:
- Matter ID format: `P&P-YYYY-TYPE-NNNN` — generated in Keel, never Deck
- Status transitions are constrained — must be enforced in Keel, not just UI
- `internal_notes` must never reach the portal API — sync server strips it

---

### 2. Wrote migration `0002_matters.sql`

**What:** Created the clients, matters, and matter_parties migration.
**File:** `src-tauri/db/migrations/0002_matters.sql`
**Why:** Foundation for all matter-related features.

```sql
-- clients
CREATE TABLE clients (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    type TEXT NOT NULL DEFAULT 'Individual'
        CHECK (type IN ('Individual','Company','Partnership','Trust','Other')),
    email TEXT,
    phone TEXT,
    address TEXT,
    gstin TEXT,
    pan TEXT,
    notes TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
);

-- matters
CREATE TABLE matters (
    id TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES clients(id),
    title TEXT NOT NULL,
    matter_type TEXT NOT NULL
        CHECK (matter_type IN ('Trademark','Patent','Design','Copyright',
                               'Corporate','Litigation','Paralegal')),
    sub_type TEXT,
    status TEXT NOT NULL DEFAULT 'Active'
        CHECK (status IN ('Active','OnHold','PendingClientResponse','Closed','Archived')),
    priority TEXT NOT NULL DEFAULT 'Normal'
        CHECK (priority IN ('Normal','High','Urgent')),
    responsible_partner_id TEXT REFERENCES users(id),
    forum TEXT,
    jurisdiction TEXT NOT NULL DEFAULT 'India',
    opened_date DATE NOT NULL,
    target_close_date DATE,
    internal_notes TEXT,
    client_notes TEXT,
    tags TEXT DEFAULT '[]',
    linked_matter_ids TEXT DEFAULT '[]',
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_matters_client ON matters(client_id);
CREATE INDEX idx_matters_status ON matters(status);
CREATE INDEX idx_matters_updated ON matters(updated_at DESC);

-- matter_parties
CREATE TABLE matter_parties (
    id TEXT PRIMARY KEY,
    matter_id TEXT NOT NULL REFERENCES matters(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id),
    role TEXT NOT NULL
        CHECK (role IN ('Partner','Associate','Paralegal','Admin')),
    is_primary INTEGER NOT NULL DEFAULT 0,
    added_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
CREATE UNIQUE INDEX idx_matter_parties_unique ON matter_parties(matter_id, user_id);
```

---

### 3. Ran migration

**What:** Applied the migration to the development SQLite database.
**Command:** `cd src-tauri && cargo sqlx migrate run`

Result: Migration applied successfully. Verified tables exist with `sqlite3 persist.db ".tables"`.

---

### 4. Updated `db/SCHEMA.md`

**What:** Added documentation for the three new tables.
**File:** `src-tauri/db/SCHEMA.md`
**Why:** KEEL-RULES.md requires this after every migration. Never skip.

---

### 5. Wrote `db/queries/matters.rs`

**What:** Type-safe sqlx query functions for all matter operations.
**File:** `src-tauri/db/queries/matters.rs`

```rust
pub async fn get_matter_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Matter>> {
    sqlx::query_as!(Matter, "SELECT * FROM matters WHERE id = ?", id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

pub async fn list_matters(pool: &SqlitePool, filter: &MatterFilter) -> anyhow::Result<Vec<MatterSummary>> {
    // Dynamic filter — build query with sqlx QueryBuilder
    let mut qb = sqlx::QueryBuilder::new(
        "SELECT m.id, m.title, c.name as client_name, m.matter_type,
                m.status, m.priority, m.updated_at,
                (SELECT due_date FROM deadlines WHERE matter_id = m.id
                 AND status = 'Pending' ORDER BY due_date ASC LIMIT 1) as next_deadline_date,
                (SELECT docketing_event FROM deadlines WHERE matter_id = m.id
                 AND status = 'Pending' ORDER BY due_date ASC LIMIT 1) as next_deadline_event
         FROM matters m
         JOIN clients c ON m.client_id = c.id
         WHERE 1=1"
    );
    if let Some(statuses) = &filter.status {
        qb.push(" AND m.status IN (");
        let mut sep = qb.separated(", ");
        for s in statuses { sep.push_bind(s.to_string()); }
        qb.push(")");
    }
    // ... similar for other filter fields
    qb.push(" ORDER BY m.updated_at DESC");
    qb.build_query_as::<MatterSummary>().fetch_all(pool).await.map_err(Into::into)
}
```

---

### 6. Wrote `commands/matters.rs`

**What:** All Tauri IPC command handlers for matters and clients.
**File:** `src-tauri/src/commands/matters.rs`

Key decisions:
- Matter ID generation uses `format!("P&P-{}-{}-{:04}", year, type_code, next_seq)` — sequence tracked in a `sequences` table (added to migration)
- Status transition validation runs in Keel before the UPDATE — returns error if invalid transition
- `list_matters` calls the query function and returns `Vec<MatterSummary>` not full `Matter` — keeps response payload small

```rust
#[tauri::command]
pub async fn create_matter(
    input: CreateMatterInput,
    state: tauri::State<'_, AppState>,
) -> Result<Matter, String> {
    let db = state.db.lock().await;
    let id = generate_matter_id(&db, &input.matter_type).await
        .map_err(|e| e.to_string())?;
    db::queries::matters::create_matter(&db, &id, &input).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_matter_status(
    id: String,
    status: String,
    note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Matter, String> {
    let db = state.db.lock().await;
    // Validate transition
    let current = db::queries::matters::get_matter_by_id(&db, &id).await
        .map_err(|e| e.to_string())?
        .ok_or("Matter not found")?;
    validate_status_transition(&current.status, &status)
        .map_err(|e| e.to_string())?;
    db::queries::matters::update_status(&db, &id, &status, note.as_deref()).await
        .map_err(|e| e.to_string())
}
```

---

### 7. Registered commands in `lib.rs`

**What:** Added all new commands to the Tauri invoke handler.
**File:** `src-tauri/src/lib.rs`

Added to `generate_handler![]`:
```rust
commands::matters::get_matter,
commands::matters::list_matters,
commands::matters::create_matter,
commands::matters::update_matter,
commands::matters::update_matter_status,
commands::matters::close_matter,
commands::matters::archive_matter,
commands::matters::search_matters,
commands::matters::assign_party,
commands::matters::remove_party,
commands::clients::get_client,
commands::clients::list_clients,
commands::clients::create_client,
commands::clients::update_client,
```

---

### 8. Updated `src/lib/ipc-types.ts`

**What:** Added TypeScript types matching the new Rust structs.
**File:** `src/lib/ipc-types.ts`
**Why:** Keel rules require ipc-types.ts to stay in sync with Rust command return types.

Added: `Matter`, `MatterSummary`, `MatterParty`, `Client`, `CreateMatterInput`, `UpdateMatterInput`, `MatterFilter`

---

### 9. Updated `src/lib/tauri.ts`

**What:** Added typed wrappers for all new matters and clients commands.
**File:** `src/lib/tauri.ts`

```typescript
export const keel = {
    ...keel,
    matters: {
        get: (id: string) => invoke<Matter>('get_matter', { id }),
        list: (filter: MatterFilter) => invoke<MatterSummary[]>('list_matters', { filter }),
        create: (input: CreateMatterInput) => invoke<Matter>('create_matter', { input }),
        updateStatus: (id: string, status: string, note?: string) =>
            invoke<Matter>('update_matter_status', { id, status, note }),
        close: (id: string, reason: string) => invoke<Matter>('close_matter', { id, reason }),
        archive: (id: string) => invoke<Matter>('archive_matter', { id }),
        search: (query: string) => invoke<MatterSummary[]>('search_matters', { query }),
        assignParty: (matterId: string, userId: string, role: string) =>
            invoke<MatterParty>('assign_party', { matterId, userId, role }),
    },
    clients: {
        get: (id: string) => invoke<Client>('get_client', { id }),
        list: () => invoke<Client[]>('list_clients'),
        create: (input: CreateClientInput) => invoke<Client>('create_client', { input }),
        update: (id: string, input: UpdateClientInput) => invoke<Client>('update_client', { id, input }),
    },
};
```

---

### 10. Ran tests

**What:** Full test suite to verify everything passes before committing.

---

## 💻 Commands Run

```bash
$ cd src-tauri && cargo sqlx migrate run
info: Applied migration 0002_matters.sql ✓

$ cargo test
running 18 tests
test commands::matters::tests::test_create_matter ... ok
test commands::matters::tests::test_matter_id_format ... ok
test commands::matters::tests::test_valid_status_transitions ... ok
test commands::matters::tests::test_invalid_status_transition_archived ... ok
test commands::clients::tests::test_create_client ... ok
test db::queries::matters::tests::test_list_filter ... ok
...
test result: ok. 18 passed; 0 failed ✓

$ cargo clippy -- -D warnings
Finished — no warnings ✓

$ cd ../src && pnpm build
✓ TypeScript compiled successfully
✓ Vite build complete
```

---

## 📁 Files Created

| File | Description |
|---|---|
| `src-tauri/db/migrations/0002_matters.sql` | clients, matters, matter_parties, sequences tables |
| `src-tauri/db/queries/matters.rs` | Type-safe sqlx queries for matters and clients |
| `src-tauri/db/queries/clients.rs` | Type-safe sqlx queries for clients |
| `src-tauri/src/commands/matters.rs` | All matter Tauri commands |
| `src-tauri/src/commands/clients.rs` | All client Tauri commands |

---

## 📝 Files Modified

| File | What changed |
|---|---|
| `src-tauri/db/SCHEMA.md` | Added clients, matters, matter_parties, sequences docs |
| `src-tauri/src/lib.rs` | Registered 14 new commands in invoke handler |
| `src/lib/ipc-types.ts` | Added 7 new types |
| `src/lib/tauri.ts` | Added keel.matters.* and keel.clients.* wrappers |
| `PROGRESS.md` | Marked 8 tasks complete, added session log entry |

---

## ✅ Tasks Completed

- [x] `specs/module-01-matters.md` already approved (from S02)
- [x] Migration: `matters`, `matter_parties`, `clients` tables created
- [x] `SCHEMA.md` updated
- [x] `commands/matters.rs` — full CRUD
- [x] `commands/clients.rs` — full CRUD
- [x] `cargo test` passes (18/18)
- [x] `ipc-types.ts` updated
- [x] `tauri.ts` wrappers added

---

## ❌ Tasks NOT Completed (and why)

| Task | Reason | Moved to |
|---|---|---|
| Deck UI — MatterList.tsx | Step 4 is a separate session by design | S04 |
| Deck UI — MatterDetail.tsx | Step 4 is a separate session by design | S04 |

---

## 🐛 Bugs Found

| ID | Description | Severity | File | Notes |
|---|---|---|---|---|
| BUG-001 | `search_matters` FTS not implemented — just LIKE query | Low | `commands/matters.rs:142` | Fine for v1, upgrade to FTS5 in Phase 3 |

---

## 🤔 Decisions Made

| Decision | Reasoning | Alternative considered |
|---|---|---|
| Added `sequences` table for Matter ID generation | SQLite lacks sequences; this is clean and race-condition safe | UUID only — rejected, P&P-YYYY-TM-NNNN format is in the PRD |
| `list_matters` returns `MatterSummary` not full `Matter` | Avoids loading all fields for the list view — `next_deadline_*` fields are subqueries | Full Matter — too heavy for list rendering |
| Status validation in Keel, not in a DB constraint | More readable code, better error messages | CHECK constraint — rejected, SQLite error messages are opaque |

---

## ⚠️ Warnings / Watch Out

- The `users` table referenced in `matters.responsible_partner_id` is from the auth migration
  (not yet created — Phase 1 auth step). The FK is declared but SQLite doesn't enforce FKs
  by default. Enable `PRAGMA foreign_keys = ON` in `db/session.rs` once auth migration lands.
- `tags` and `linked_matter_ids` stored as JSON TEXT — remember to parse in the query layer,
  not the command layer. Currently returning raw JSON strings to Deck — Deck parses with `JSON.parse`.
  Add proper serde deserialization in a later session.

---

## 📌 Next Session

**Suggested next session:** S04 — Phase 1 Module 1: Deck UI (MatterList + MatterDetail)
**Step:** Step 4 of 5
**Files to read first:** `specs/module-01-matters.md`, `src/DECK-RULES.md`
**What to build:** `pages/Matters/MatterList.tsx`, `pages/Matters/MatterDetail.tsx`,
  `components/matters/MatterStatusBadge.tsx`, `components/matters/MatterTimeline.tsx`
**Key constraint:** Use `keel.matters.*` wrappers only — no raw invoke() in components.
  Design tokens from `design-system/tokens.ts` — no hardcoded colours.

---

*Log written by Claude at end of session.*
*Commit: `git log --oneline -1` → `a3f8c21 [p1/m1] feat: matters + clients schema and Keel CRUD commands`*
