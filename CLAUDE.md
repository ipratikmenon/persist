# Persist — Root CLAUDE.md
# Read this file at the start of every session, every worktree, every agent.
# This is the single source of truth for project-wide rules and conventions.

---

## FIRST ACTION EVERY SESSION

Before doing anything else, read PROGRESS.md in the root of this repo.
It tells you exactly where the project is, what was last completed, what is in progress,
and what to do next. Do not proceed with any work until you have read it.

```
cat PROGRESS.md
```

After every completed task — no matter how small — update PROGRESS.md before stopping.

At the END of every session, write a session log file:
```
SESSION-LOG/YYYY-MM-DD-SNN-description.md
```
Copy `SESSION-LOG/TEMPLATE.md` and fill every section.
This is your console output in markdown. It captures every action taken, every file
changed, every command run, every decision made, every bug found.

Both PROGRESS.md and the session log must be written before committing.
A session without a log file is an incomplete session.

---

## Project Identity

**Persist** is a legal practice management platform for Persistas & Partners, Delhi.

The application is named **Persist**. It is proprietary software and the
exclusive property of Persistas & Partners — see `COPYRIGHT.md`. Not
open-source; no licence is granted. The document templates in
`src-tauri/storage/templates/` are the firm's professional work product, not
configuration.
It is a law firm tool — not a generic SaaS product. Every decision should reflect that:
client privilege matters, reliability > features, Indian IP law context, two attorneys
(Sree Lakshmi Menon and Kajal Thakur) are the primary users.

**Three surfaces:**
- Persist Desktop (firm app) — Tauri v2, Keel (Rust) + Deck (React), macOS + Windows
- Persist Web (client portal) — React + FastAPI, browser-only, Hetzner + Cloudflare
- Persist iOS (client app) — Tauri v2 mobile, Phase 7 only

**One source of truth:** Desktop SQLite (local-first). Everything else syncs from it.

---

## Stack

### Persist Desktop
| Layer | Technology | Location |
|---|---|---|
| Rust backend | Tauri v2, Tokio, sqlx, Axum | `src-tauri/` (called **Keel**) |
| React frontend | React 19, TypeScript, Vite, Zustand | `src/` (called **Deck**) |
| Database | SQLite via sqlx | `src-tauri/db/` |
| Document vault | Local filesystem, AES-256 | `src-tauri/storage/` |
| LaTeX engine | XeLaTeX + Noto (TeX Live bundled in installer) | subprocess via `src-tauri/services/latex.rs` |
| IPC bridge | Tauri `invoke()` | `src/lib/tauri.ts` (typed wrappers) |

### Persist Web (Client Portal)
| Layer | Technology | Location |
|---|---|---|
| API | FastAPI (Python) | `portal/backend/` |
| Frontend | React + TypeScript + Vite | `portal/frontend/` |
| Database | PostgreSQL (Hetzner) | synced from desktop SQLite |
| Auth | JWT + OTP (email/SMS) | `portal/backend/auth/` |

### Sync Server
| Layer | Technology | Location |
|---|---|---|
| HTTP/WebSocket | Axum (Rust) | `server/` |
| Database | PostgreSQL (Hetzner) | mirror of desktop SQLite (client-visible subset) |

---

## Naming Conventions — Load-Bearing, Do Not Change

**Keel = `src-tauri/`** — the Rust backend of the desktop app.
**Deck = `src/`** — the React frontend of the desktop app.
These are internal names for clarity. Not a third-party framework. Do not rename.

**Rust (Keel):** snake_case everywhere. Commands in `commands/`, services in `services/`.
**TypeScript (Deck):** PascalCase components, camelCase functions, kebab-case filenames.
**SQLite:** snake_case tables and columns — no camelCase, no PascalCase in the database.
**Matter IDs:** `P&P-YYYY-TYPE-NNNN` — e.g. `P&P-2026-TM-0042`. Always this format.
**Docket reference numbers:** `P&P-DD-NNNN` — e.g. `P&P-DD-0023`.

---

## Architecture Rules — Absolute, No Exceptions

### AI calls
- **ALL** AI calls go through `invoke('ai_request', ...)` in `src/lib/ai.ts` for simple single-model requests
- For intelligence operations (multi-step, parallel, data assembly) — use `src/hooks/useHpas.ts` which calls `invoke('hpas_dispatch', ...)`
- The routing brain is in `src-tauri/services/ai_router.rs` (model selection: Haiku/Sonnet/Opus)
- HPAS (`src-tauri/src/hpas/`) is the orchestration layer above ai_router — they are complementary
- Deck NEVER calls the Anthropic API directly — ever
- Deck NEVER knows which model (Haiku/Sonnet/Opus) ran — that is Keel's business
- Three-tier routing: Haiku (lightweight tasks) → Sonnet (daily work) → Opus (deep analysis)
- See `src-tauri/config/ai_thresholds.rs` for tunable escalation constants
- See `src-tauri/src/hpas/` for the full HPAS module layout (12 Rust files)
- HPAS rule: intelligence → HPAS dispatch; pure data transport → direct Tauri IPC

### Document security
- All documents AES-256 encrypted at rest via `src-tauri/storage/vault.rs`
- NEVER read or write documents via raw filesystem from Deck
- NEVER store document bytes in SQLite — store only metadata, path to vault file
- Metadata stripped before EVERY document export — call `clean_metadata()` in `src-tauri/commands/documents.rs`

### LaTeX
- LaTeX compilation is LOCAL — TeX Live is bundled in the installer
- The engine is **XeLaTeX**, never pdfLaTeX — ₹ and Devanagari need a Unicode engine
- Template values are `latex::Field` (`text` escapes, `raw` does not) — never bare strings
- NEVER compile LaTeX server-side or via any network call
- All LaTeX jobs are async — always `tokio::spawn`, never block the main thread
- LaTeX pipeline: `src-tauri/services/latex.rs` → subprocess → reads output PDF bytes → returns to Deck

### Database
- SQLite for desktop (Keel manages via sqlx)
- NEVER write raw SQL strings outside of `src-tauri/db/queries/`
- NEVER edit existing migration files — always create a new one
- ALWAYS update `src-tauri/db/SCHEMA.md` after any migration change
- Run migrations: `cargo sqlx migrate run` from `src-tauri/`

### Sync
- Desktop SQLite is ALWAYS the source of truth
- Client-facing surfaces (portal, iOS) only see the PostgreSQL mirror subset
- Sync server lives in `server/` — it never writes to desktop directly
- What syncs to clients: matters_public, deadlines_public, documents_shared, invoices, payments
- What NEVER syncs to clients: internal notes, time entries, billing rates, draft documents, AI chat history

### Client portal API
- Row-level security enforced at PostgreSQL level — not just the ORM
- Clients can ONLY query rows where `client_id = authenticated_user_id`
- All documents served via signed time-limited URLs (5-minute expiry)
- OTP auth only — no passwords stored

---

## Directory Structure (Desktop App)

```
persist/
├── CLAUDE.md                    ← You are here
├── PROGRESS.md                  ← READ THIS FIRST EVERY SESSION
├── TASKS.md                     ← Phase/module task breakdown
├── specs/                       ← One SPEC.md per module (written before implementation)
│   ├── module-01-matters.md
│   ├── module-02-docketing.md
│   └── ...
│
├── src-tauri/  [KEEL — Rust]
│   ├── CLAUDE.md                ← Keel-specific rules (read this when working in Keel)
│   ├── src/
│   │   ├── commands/            ← Tauri IPC handlers (one file per domain)
│   │   ├── services/            ← Background services and business logic
│   │   ├── hpas/                ← HPAS multi-agent orchestration engine (12 Rust files)
│   │   │   ├── mod.rs           ← Job/StructuredResult/dispatch router
│   │   │   ├── agent.rs         ← SubAgent + IsolatedWorkspace
│   │   │   ├── compressor.rs    ← SemanticCompressor
│   │   │   ├── session.rs       ← SessionAgent
│   │   │   ├── memory.rs        ← VectorMemory (sqlite-vss)
│   │   │   ├── mistake_memory.rs← MistakeMemory
│   │   │   ├── structural_memory.rs ← StructuralMemory
│   │   │   ├── registry.rs      ← RegistryAgent
│   │   │   ├── validation_gate.rs ← ValidationGate
│   │   │   ├── bus.rs           ← MessageBus + OrchestratorBus
│   │   │   ├── store.rs         ← WorkflowStore trait + SqliteStore
│   │   │   └── compressor_ctx.rs← ContextCompressor
│   │   ├── db/
│   │   │   ├── migrations/      ← sqlx migration files (never edit existing ones)
│   │   │   ├── queries/         ← Type-safe SQL query functions
│   │   │   └── SCHEMA.md        ← Human-readable schema docs (always keep updated)
│   │   ├── storage/             ← Encrypted vault, template storage, cache
│   │   ├── config/              ← ai_thresholds.rs and other tunable constants
│   │   └── lib.rs               ← Tauri builder — registers all commands
│   └── Cargo.toml
│
├── src/  [DECK — React]
│   ├── CLAUDE.md                ← Deck-specific rules (read this when working in Deck)
│   ├── pages/                   ← Top-level route pages
│   ├── components/              ← Shared UI components
│   ├── stores/                  ← Zustand global state
│   ├── hooks/
│   │   └── useHpas.ts           ← HPAS dispatch hook (intelligence operations)
│   └── lib/
│       ├── ai.ts                ← Simple AI entry point (single-model requests via ai_request)
│       ├── tauri.ts             ← Typed invoke() wrappers for all Keel commands
│       ├── ipc-types.ts         ← Shared TypeScript types matching Rust structs
│       └── shortcuts.ts         ← Keyboard shortcuts
│
├── server/  [SYNC SERVER — Rust/Axum]
│   └── CLAUDE.md
│
├── portal/  [CLIENT PORTAL — FastAPI + React]
│   └── CLAUDE.md
│
└── specs/                       ← Module specs (pre-implementation contracts)
```

---

## Design System — Non-Negotiable Tokens

Do not hardcode any colours, fonts, or spacing. Use the design system tokens from `src/design-system/tokens.ts`.

### Colours (Deck CSS variables)
```
--color-bg-primary:     #F9F7F4   /* Warm white — main background */
--color-bg-secondary:   #F0ECE5   /* Soft sand — cards, sidebars */
--color-bg-tertiary:    #E8EDE6   /* Light sage — section differentiation */
--color-text-primary:   #2C2C2A   /* Deep charcoal — never pure black */
--color-text-secondary: #6B6862   /* Warm grey — labels, metadata */
--color-text-tertiary:  #9A9590   /* Muted stone — placeholders */
--color-accent-primary: #4A6580   /* Warm slate blue — buttons, active states */
--color-accent-secondary: #B5604A /* Terracotta — hover, selected, unread accents */
--color-status-urgent:  #C0392B   /* Muted red — overdue only */
--color-status-warning: #D4872A   /* Warm amber — due within 3-7 days */
--color-status-clear:   #4A7C59   /* Sage green — completed, safe */
--color-border:         #E2DDD8   /* Hairline border — 0.5px */
```

### Typography (loaded via Google Fonts / bundled)
```
Headings (display):  Playfair Display — 28px/600, 20px/500
UI text (all):       DM Sans — 15px/500 (card title), 14px/400 (body), 12px/400 (label)
Legal document text: Georgia — 13px/400
Monospace (IDs):     JetBrains Mono — 12px/400
```

### Components
```
Cards:   border-radius: 10px, box-shadow: 0 1px 4px rgba(0,0,0,0.06), 0.5px border #E2DDD8
Buttons: primary = slate blue bg, border-radius: 8px; secondary = transparent, slate blue border
Inputs:  border-radius: 6px, 0.5px border, cream background on focus
```

---

## What NOT To Do

- Do NOT add Electron or any Electron-like dependency
- Do NOT store secrets in SQLite — use OS keychain via `src-tauri/services/keychain.rs`
- Do NOT call Claude API from Deck — the router is in Keel, period
- Do NOT break the Keel/Deck naming convention
- Do NOT hardcode colours, fonts, or spacing in components
- Do NOT edit existing migration files — always create new ones
- Do NOT write raw SQL strings outside of `src-tauri/db/queries/`
- Do NOT compile LaTeX server-side or via network
- Do NOT make Deck aware of which Claude model is being used
- Do NOT use localStorage or sessionStorage in any Deck component
- Do NOT use Context API for global state — Zustand only (`src/stores/`)
- Do NOT send documents to clients without calling `clean_metadata()` first

---

## Test Commands

```bash
# Keel (Rust)
cd src-tauri && cargo test
cd src-tauri && cargo test --features integration
cd src-tauri && cargo sqlx migrate run   # run before testing if schema changed

# Deck (React)
cd src && pnpm test
cd src && pnpm build   # must pass before committing Deck changes

# Full desktop build (macOS)
pnpm tauri build

# Portal backend (Python)
cd portal/backend && pytest

# Server (Rust)
cd server && cargo test
```

---

## Commit Rules

- One commit per completed session task
- Commit message format: `[phase/module] verb: description`
  - e.g. `[p1/m1] feat: add matter CRUD commands to Keel`
  - e.g. `[p1/m1] feat: build MatterList and MatterDetail pages in Deck`
  - e.g. `[p0] chore: configure Tauri v2 scaffold and CI/CD`
- ALWAYS update PROGRESS.md before committing
- NEVER commit with failing tests
- NEVER commit with `cargo build` errors in Keel

---

## Session Protocol

1. Read `PROGRESS.md` — understand exactly where things are
2. Read the relevant `specs/module-XX.md` for the current work
3. Read the relevant sub-directory `CLAUDE.md` (Keel or Deck) if working in that layer
4. Do the work
5. Run the appropriate tests — they must pass
6. Update `PROGRESS.md` — mark what's done, set the next task
7. Commit

If you are unsure about anything — architecture, naming, a pattern — check the relevant
CLAUDE.md or SPEC.md before guessing. Do not invent patterns not documented here.
