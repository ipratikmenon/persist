# Keel CLAUDE.md — src-tauri/
# Rules for working in the Rust backend of Persist Desktop.
# Read this alongside the root CLAUDE.md when working in Keel.

---

## Identity

You are working in **Keel** — the Rust backend of Persist Desktop (`src-tauri/`).
Keel handles everything system-level: database, document vault, LaTeX compilation,
AI routing, OS notifications, mail sync, and the Tauri IPC bridge.

Your scope is `src-tauri/` only. Do not touch `src/` (Deck) from here.

---

## Rust Patterns

### Async
- Always use `tokio::spawn` for async work — NEVER block the main thread
- Use `tokio::task::spawn_blocking` for CPU-intensive work (LaTeX, OCR, crypto)
- All Tauri commands must be `async fn` — even simple ones
- Use `anyhow::Result` for error handling in services; `tauri::Error` in commands

### Error handling
```rust
// Services: return anyhow::Result
pub async fn build_matter_context(db: &SqlitePool, matter_id: &str) -> anyhow::Result<MatterContext> { ... }

// Tauri commands: return Result<T, String> — Tauri serialises the error to Deck
#[tauri::command]
pub async fn get_matter(id: String, state: tauri::State<'_, AppState>) -> Result<Matter, String> {
    let db = state.db.lock().await;
    build_matter_context(&db, &id).await.map_err(|e| e.to_string())
}
```

### State management
```rust
// AppState in lib.rs — shared across all commands
pub struct AppState {
    pub db: Arc<Mutex<SqlitePool>>,
    pub vault_key: Arc<[u8; 32]>,
    pub ai_client: Arc<ClaudeClient>,
}
```

---

## Database (sqlx + SQLite)

### Rules
- All database access via sqlx — never raw SQL strings outside of `db/queries/`
- Every migration in `db/migrations/` — never edit existing migration files
- ALWAYS update `db/SCHEMA.md` after any migration change
- snake_case for all table and column names
- Every table needs `created_at DATETIME NOT NULL DEFAULT (datetime('now'))` and `updated_at`

### Migration naming
```
db/migrations/
  0001_initial.sql
  0002_add_deadlines.sql
  0003_add_documents.sql
  ...
```

### Query pattern
```rust
// db/queries/matters.rs — type-safe queries
pub async fn get_matter_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Matter>> {
    sqlx::query_as!(Matter, "SELECT * FROM matters WHERE id = ?", id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}
```

### Run migrations
```bash
cargo sqlx migrate run    # from src-tauri/
```

---

## Tauri Commands

### File location
One file per domain in `src/commands/`:
```
commands/
  matters.rs     ← matter CRUD, status, assignment
  deadlines.rs   ← deadline engine, templates, urgency
  documents.rs   ← upload, version, vault ops, metadata clean
  billing.rs     ← time entries, invoice generation
  pdf.rs         ← PDF engine, OCR, markup, compile
  latex.rs       ← LaTeX compilation pipeline
  ai.rs          ← THE ONLY AI ENTRY POINT — routes to ai_router.rs
  mail.rs        ← IMAP/SMTP, Graph API, mail sync
  contracts.rs   ← contract intelligence pipeline
  sync.rs        ← sync with Hetzner server
  auth.rs        ← login, session, RBAC
```

### Command registration
Every command must be registered in `lib.rs`:
```rust
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
        commands::matters::get_matter,
        commands::matters::create_matter,
        // ...
    ])
```

### IPC types
Types returned from commands MUST match the TypeScript types in `src/lib/ipc-types.ts`.
When you add or change a command's return type, update `ipc-types.ts` in Deck.
Never let these drift out of sync.

---

## Document Vault

```rust
// Always go through vault.rs — never raw filesystem
use crate::storage::vault::{encrypt_to_vault, decrypt_from_vault};
use crate::storage::metadata::{clean_metadata, clean_with_report};

// Store a document
let vault_path = encrypt_to_vault(&vault_dir, &vault_key, &document_bytes, &matter_id, &doc_id)?;

// Retrieve a document — RAW bytes, for internal use only
let bytes = decrypt_from_vault(&vault_dir, &vault_key, &vault_path)?;

// ALWAYS clean metadata before anything leaves the firm.
// Fails closed: returns Err for file types it cannot clean, so uncleaned bytes
// can never be shipped by accident.
let clean_bytes = clean_metadata(&document_bytes, &mime_type)?;

// Prefer clean_with_report when a human will see the result — the report names
// what was stripped ("3 tracked change(s)", "Reviewer comments", "GPS location").
let cleaned = clean_with_report(&document_bytes, &mime_type)?;
```

### Internal read vs client export — do not confuse these
- `get_document` returns **raw** bytes. An attorney reviewing a counterparty's
  draft needs its tracked changes; stripping them there destroys the thing they
  opened the file to read. Never send these bytes to a client.
- `export_document` is the **only** client-facing path. It strips metadata and
  fails closed. The sync engine (Module 5 §9.1) uses the same function.

Vault directory: `~/Library/Application Support/Persist/vault/` (macOS)
                 `%APPDATA%\Persist\vault\` (Windows)
Never hardcode this path — use `tauri::api::path::app_data_dir()`.

---

## LaTeX Compilation

```rust
// services/latex.rs — async subprocess
pub async fn compile_latex(
    template_id: &str,
    field_values: &HashMap<String, String>
) -> anyhow::Result<Vec<u8>> {
    // 1. Load .tex template from storage/templates/
    // 2. Inject field_values
    // 3. tokio::task::spawn_blocking: run pdflatex subprocess
    // 4. Read output PDF bytes
    // 5. Return bytes (never write to disk in the command layer)
}
```

TeX Live binary path is resolved at runtime from the bundled sidecar.
Never hardcode paths — use the sidecar resolver in `lib.rs`.

---

## AI Router

The AI router in `services/ai_router.rs` is the ONLY place in Keel that calls the Anthropic API.

```rust
// commands/ai.rs — the single Tauri command Deck calls
#[tauri::command]
pub async fn ai_request(
    task_type: String,
    context: String,
    prompt: String,
    deep_analysis: bool,
    state: tauri::State<'_, AppState>,
) -> Result<AIResponse, String> {
    let request = TaskRequest { task_type: task_type.parse()?, context, prompt, deep_analysis };
    services::ai_router::route(request, &state).await.map_err(|e| e.to_string())
}
```

Routing tiers (from `config/ai_thresholds.rs`):
- Haiku: ThreadSummary, SmartReplyChips, EmailAutoTag, SentinelEval, MetadataExtract, FormAutoFill
- Sonnet: PersistChat, AIDrafting, SmartForm, DocComparison, DailyBrief, ContractExtraction
- Opus: PatentClaims, DiligenceReport, explicit attorney request

Do not add new task types without updating `ai_thresholds.rs` and the routing logic.

---

## Notifications

Use Tauri's notification plugin — never a custom implementation:
```rust
use tauri_plugin_notification::NotificationExt;
app.notification()
    .builder()
    .title("Deadline overdue")
    .body("Response to Examination Report — Petalveda Scents TM")
    .show()?;
```

---

## Test Patterns

```bash
cargo test                           # Unit tests
cargo test --features integration    # Integration tests (needs test SQLite)
cargo clippy -- -D warnings          # Linting — must pass clean
```

Write tests in the same file as the code they test (`#[cfg(test)]` block).
Integration tests go in `tests/` with their own SQLite test database.

---

## Common Mistakes to Avoid

- Blocking the Tokio runtime with sync code → use `spawn_blocking`
- Returning raw error strings from services → use `anyhow::Result`
- Writing SQL outside of `db/queries/` → centralise everything
- Editing existing migration files → always create new ones
- Forgetting to update `db/SCHEMA.md` after migrations
- Forgetting to update `src/lib/ipc-types.ts` when command return types change
- Accessing the vault without going through `vault.rs`
- Compiling LaTeX synchronously → always async + spawn_blocking
