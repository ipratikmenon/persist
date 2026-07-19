# PROGRESS.md — Persist Build Tracker
# =========================================================
# CLAUDE: READ THIS FILE AT THE START OF EVERY SESSION.
# UPDATE THIS FILE AT THE END OF EVERY SESSION.
# This file is the handoff between sessions. An unupdated
# PROGRESS.md is a broken handoff. Do not skip this.
# =========================================================

---

## Current State

**Phase:** 2 — Billing & Client Portal
**Week:** 3
**Active module:** Phase 2 M4 Billing — COMPLETE. Next: Phase 1 M2 extended features (ip_assets, cascade engine) or Phase 2 M5 (Client Portal spec).
**Last session completed:** S08 — 2026-04-19 — Billing UI complete: InvoiceDetail wired, InvoiceComposer, latex.rs real impl, invoice.tex template
**Last updated:** 2026-04-19

---

## 🔴 IN PROGRESS RIGHT NOW

> Fill this section at the start of a session. Clear it when done.

Phase 2 M4 Billing is complete. cargo test: 33/33. pnpm build: PASS (469 modules, 482kb).
Next options:
  A) Phase 1 M2 extended features: ip_assets migration, cascade_engine, abandonment_watcher
  B) Phase 2 M5: write spec + scaffold Client Portal (FastAPI + React)
  C) Auth B01: sessions table + 8-hour persistent sessions

---

## ✅ COMPLETED

### Phase 0 — Tauri v2 Scaffold

| Task | Completed | Notes |
|---|---|---|
| Tauri v2 project initialised (`src-tauri/` + `src/`) | ✅ | S01 |
| `tauri.conf.json` configured (window, CSP, capabilities) | ✅ | S01 |
| sqlx dependency added to `Cargo.toml` | ✅ | S01 |
| Initial SQLite migration created (`0001_initial.sql`) | ✅ | S01 |
| Keel directory structure created | ✅ | S01 — commands/, services/, db/, storage/, config/ |
| Deck directory structure + Vite config | ✅ | S01 — vite.config.ts, tsconfig.json |
| React Router v7 configured in Deck | ✅ | S01 |
| Zustand stores scaffold (matters, ui, chat, mail, sync, auth) | ✅ | S01 |
| Design system tokens (`src/design-system/tokens.ts`) | ✅ | S01 + S06 updated to SF Pro fonts |
| Typed `invoke()` wrappers scaffold (`src/lib/tauri.ts`) | ✅ | S01 + S07 billing wrappers |
| `ipc-types.ts` scaffold | ✅ | S01 + updates through S07 |
| AI entry point scaffold (`src/lib/ai.ts`) | ✅ | S01 |
| AI router scaffold (`src-tauri/services/ai_router.rs`) | ✅ | S01 — stub, Phase 3 |
| `ai_thresholds.rs` config file | ✅ | S01 |
| LaTeX subprocess scaffold (`src-tauri/services/latex.rs`) | ✅ | S01 — stub, Phase 2 M4 full impl |
| AES-256 vault scaffold (`src-tauri/storage/vault.rs`) | ✅ | S01 stub → S05 real AES-256-GCM |
| Blank Persist Desktop window launches on macOS ✓ | ✅ | S01 |
| TeX Live bundled in Tauri installer | ❌ | Phase 2 M4 |
| GitHub Actions CI/CD | ❌ | Deferred |
| Hetzner VPS: sync server scaffold | ❌ | Phase 2 M5 |
| Windows launch confirmed | ❌ | Post Phase 1 |

---

### Phase 1 — Foundation

#### Module 1: Matter Management

| Step | Task | Completed | Notes |
|---|---|---|---|
| 1 | `specs/module-01-matters.md` written | ✅ | S01 |
| 2 | Migration: `sequences`, `clients`, `matters`, `matter_parties` | ✅ | S02 — `0002_matters.sql` |
| 2 | Migration: `matter_comments`, `comment_mentions` | ❌ | Phase 1 M1 Step 2b |
| 2 | `SCHEMA.md` updated | ✅ | S02 |
| 3 | `db/queries/matters.rs` + `db/queries/clients.rs` | ✅ | S02 — all query fns + tests |
| 3 | `commands/matters.rs` — 14 commands (get, create, update, close, archive, list, search, assign_party, remove_party, get_client, list_clients, create_client, update_client) | ✅ | S02 |
| 3 | `commands/discussion.rs` — post, list, mention, resolve | ❌ | Phase 1 M1 |
| 3 | `services/discussion_broadcaster.rs` | ❌ | Phase 1 M1 |
| 3 | `cargo test` passes | ✅ | S02 — 10/10 |
| 4 | `pages/Matters/MatterList.tsx` (filter chips, table, NewMatterDrawer) | ✅ | S03 |
| 4 | `pages/Matters/MatterDetail.tsx` (tabs: Overview/Parties/Documents/Deadlines) | ✅ | S03 |
| 4 | `components/matters/MatterStatusBadge.tsx` | ✅ | S03 |
| 4 | `components/shell/AppShell.tsx` | ✅ | S03 + S06 live session/logout |
| 4 | `components/discussion/DiscussionThread.tsx` | ❌ | Phase 1 M1 |
| 4 | `components/discussion/CommentComposer.tsx` | ❌ | Phase 1 M1 |
| 4 | `pnpm build` passes | ✅ | S03 — 387kb |
| 5 | Validation session | ❌ | |

#### Module 2: Docketing & Deadline Engine

| Step | Task | Completed | Notes |
|---|---|---|---|
| 1 | `specs/module-02-docketing.md` written | ✅ | S07 — new detailed spec (505 lines) |
| 2 | Migration: `deadlines` table | ✅ | S04 — `0003_deadlines.sql` |
| 2 | Migration: `ip_assets` table | ❌ | New from spec — ip_asset_type, application_number, etc. |
| 2 | Migration: `cascade_templates` table | ❌ | New from spec §2.9 |
| 2 | Migration: `deadline_escalations` table | ❌ | New from spec §2.11 |
| 2 | Migration: `document_intake_events` table | ❌ | New from spec §2.16 |
| 2 | Migration: `docket_errors` table | ❌ | New from spec §2.17 |
| 2 | Migration: `ip_fee_schedule` table | ❌ | New from spec §2.18 |
| 2 | `deadlines` table — add `ip_asset_id`, `reference_number`, `dual_verified_by` fields | ❌ | New from spec |
| 2 | `SCHEMA.md` updated | ✅ | S04 (partial — needs ip_assets, cascade) |
| 3 | `db/queries/deadlines.rs` — urgency_for(), CRUD | ✅ | S04 |
| 3 | `commands/deadlines.rs` — 7 commands + statutory templates (TM, Patent, Design, Copyright) | ✅ | S04 |
| 3 | `commands/ip_assets.rs` — CRUD for IP assets | ❌ | New from spec |
| 3 | `services/cascade_engine.rs` — chain generator from templates | ❌ | New from spec §2.9 |
| 3 | `services/abandonment_watcher.rs` — 14d/7d/3d/missed alerts | ❌ | New from spec §2.11 |
| 3 | `services/deadline_watcher.rs` — 15-min poll + OS notifications | ✅ | S04 |
| 3 | Dual verification enforcement | ❌ | New from spec §2.12 |
| 3 | `services/intake_sla_tracker.rs` | ❌ | New from spec §2.16 |
| 3 | `commands/docket_audit.rs` — error log | ❌ | New from spec §2.17 |
| 3 | `db/queries/fee_schedule.rs` — fee lookups | ❌ | New from spec §2.18 |
| 3 | `cargo test` passes | ✅ | S04 — 14/14 (core deadlines) |
| 4 | `pages/Dockets/DocketList.tsx` (urgency filter chips, inline mark-complete) | ✅ | S04 |
| 4 | `pages/Dockets/IPAssetRecord.tsx` (timeline + add deadline drawer + templates) | ✅ | S04 |
| 4 | `pages/Dockets/PipelineBoard.tsx` (Kanban by status) | ✅ | S04 |
| 4 | `components/dockets/UrgencyBadge.tsx` | ✅ | S04 |
| 4 | `pages/Dockets/RenewalDashboard.tsx` | ❌ | New from spec |
| 4 | `components/dockets/CascadePreview.tsx` | ❌ | New from spec |
| 4 | `components/dockets/VerificationBadge.tsx` | ❌ | New from spec |
| 4 | `components/dockets/TriggerDocumentChip.tsx` | ❌ | New from spec |
| 4 | `components/dockets/IntakeSLABanner.tsx` | ❌ | New from spec |
| 4 | `pnpm build` passes | ✅ | S04 — 411kb |
| 5 | Validation session | ❌ | |

#### Module 3: Document Management

| Step | Task | Completed | Notes |
|---|---|---|---|
| 1 | `specs/module-03-documents.md` written + updated | ✅ | S05 + S07 update (§6 File Manager Tree) |
| 2 | Migration: `documents` table | ✅ | S05 — `0004_documents.sql` |
| 2 | `SCHEMA.md` updated | ✅ | S05 |
| 3 | `storage/vault.rs` — AES-256-GCM encrypt/decrypt (4 tests) | ✅ | S05 |
| 3 | `db/queries/documents.rs` — list, get, create, delete (4 tests) | ✅ | S05 |
| 3 | `commands/documents.rs` — upload, get, delete, list (4 commands) | ✅ | S05 |
| 3 | `clean_metadata()` function (pass-through, Phase 2 full strip) | ✅ | S05 |
| 3 | `AppState` extended with `vault_dir` | ✅ | S05 |
| 3 | Document versioning (re-upload bumps version) | ❌ | From spec |
| 3 | `cargo test` passes | ✅ | S05 — 22/22 |
| 4 | `pages/Documents/DocumentList.tsx` (all-firm vault, category filter chips) | ✅ | S05 |
| 4 | `pages/Documents/UploadDrawer.tsx` (file picker via Tauri dialog) | ✅ | S05 |
| 4 | Documents tab in `MatterDetail.tsx` | ✅ | S05 |
| 4 | `components/documents/DocumentTree.tsx` — file manager tree | ❌ | From spec §6 — Phase 1 M3.1 |
| 4 | PDF / DOCX preview (`DocumentViewer.tsx`) | ❌ | From spec |
| 4 | `pnpm build` passes | ✅ | S05 — 431kb |
| 5 | Validation pass | ❌ | |

#### User Management + RBAC

| Step | Task | Completed | Notes |
|---|---|---|---|
| 1 | `specs/auth-rbac.md` written | ✅ | S07 — new spec (183 lines): users, sessions, RBAC, first-launch wizard |
| 2 | Migration: `users` table | ✅ | S06 — `0005_users.sql` |
| 2 | Migration: `sessions` table | ❌ | In new auth spec — persistent sessions with expiry |
| 2 | User seeding (persist2026, bcrypt cost 12) | ✅ | S06 — in lib.rs block_on |
| 3 | `db/queries/users.rs` — 6 functions + 3 tests | ✅ | S06 |
| 3 | `commands/auth.rs` — login (bcrypt), logout, get_session | ✅ | S06 |
| 3 | Session persistence (8-hour expiry, `sessions` table) | ❌ | New from spec — currently in-memory only |
| 3 | First-launch wizard (set firm name, GSTIN, bank details) | ❌ | New from spec |
| 3 | `cargo test` passes | ✅ | S06 — 25/25 |
| 4 | `src/stores/auth.ts` | ✅ | S06 |
| 4 | `pages/Auth/LoginScreen.tsx` (attorney chips, form, error anim) | ✅ | S06 |
| 4 | `App.tsx` — SessionGate + LogoutHandler | ✅ | S06 |
| 4 | `AppShell.tsx` — live session (initials, name, role) + logout ⎋ | ✅ | S06 |
| 4 | SF Pro system fonts — Google Fonts removed | ✅ | S06 |
| 4 | Drafting font catalogue (8 choices + draftingFontStack helper) | ✅ | S06 |
| 4 | `pnpm build` passes | ✅ | S06 — 439kb |
| 5 | Validation pass | ❌ | |

---

### Phase 2 — Billing & Client Portal

#### Module 4: Time Tracking & Billing

| Step | Task | Completed | Notes |
|---|---|---|---|
| 1 | `specs/module-04-billing.md` written | ✅ | S07 — 349 lines, GST/INR/LaTeX spec |
| 2 | Migration: `firm_settings`, `time_entries`, `invoices`, `invoice_line_items`, `payments` | ✅ | S07 — `0006_billing.sql` |
| 2 | `SCHEMA.md` updated | ❌ | TODO — add billing tables |
| 3 | `db/queries/billing.rs` — all query functions (5 tests) | ✅ | S07 |
| 3 | `commands/billing.rs` — 13 commands (time entries, invoices, payments, PDF, settings) | ✅ | S07 |
| 3 | All billing commands registered in `lib.rs` invoke_handler | ✅ | S07 |
| 3 | `services/latex.rs` — real subprocess implementation | ✅ | S08 — finds pdflatex (sidecar → MacTeX → PATH), temp dir compile, 3 tests |
| 3 | `storage/templates/invoice.tex` — LaTeX invoice template | ✅ | S08 — GST compliant, SAC 998212, CGST/SGST/IGST breakout, amount in words |
| 3 | `cargo test` passes | ✅ | S08 — 33/33 (8 billing tests, 3 latex tests) |
| 4 | `lib/tauri.ts` — billing IPC wrappers | ✅ | S07 |
| 4 | `ipc-types.ts` — billing types added | ✅ | S07 |
| 4 | `pages/Billing/BillingHome.tsx` (Invoices/Time Entries/Settings tabs) | ✅ | S07 |
| 4 | `pages/Billing/TimeTracker.tsx` | ✅ | S07 |
| 4 | `pages/Billing/InvoiceList.tsx` | ✅ | S07 |
| 4 | `pages/Billing/FirmSettingsPanel.tsx` | ✅ | S07 |
| 4 | `pages/Billing/InvoiceDetail.tsx` | ✅ | S08 — header card, line items, GST panel, payment history, RecordPaymentModal |
| 4 | `pages/Billing/InvoiceComposer.tsx` | ✅ | S08 — client/matter/entry selection, fixed-fee lines, live totals, GST type selector |
| 4 | Billing route in `App.tsx` (enabled, not disabled) | ✅ | S07 |
| 4 | `pnpm build` passes | ✅ | S08 — 469 modules, 482kb |
| 5 | Validation pass | ❌ | |

#### Module 5: Client Portal (React + FastAPI)

| Module | Status | Notes |
|---|---|---|
| `specs/module-05-portal.md` written | ❌ | |
| PostgreSQL schema (mirror tables) | ❌ | |
| `commands/sync.rs` — desktop → PostgreSQL sync | ❌ | |
| Sync server Axum routes (`server/`) | ❌ | |
| Portal backend: FastAPI, OTP auth, JWT (`portal/backend/`) | ❌ | |
| Portal frontend: 4 tabs (Matters/Documents/Invoices/Profile) | ❌ | |

---

### Phase 3 — Intelligence Layer (Weeks 19–26)

| Module | Status | Notes |
|---|---|---|
| Module 22: AI Router (production-ready) | ❌ | Stub in ai_router.rs — replaces with 3-tier logic |
| Module 24 Phase 3A: HPAS Dispatch Foundation | ❌ | spec written (`specs/hpas-integration.md`) — build alongside first AI feature |
| Module 24 Phase 3B: HPAS Parallel Dispatch | ❌ | When first multi-fetch feature needed |
| Module 24 Phase 3C: HPAS Communication Layer | ❌ | When cross-domain coordination needed |
| Module 7: AI Drafting Assistant | ❌ | |
| Module 21: Persist Chat | ❌ | |
| Module 8: Reports & Analytics | ❌ | |
| WhatsApp notifications (Meta Cloud API) | ❌ | |
| Calendar sync (Google + Outlook) | ❌ | |
| M365 integration (Module 19) | ❌ | |

---

### Phase 4 — Drafting Suite (Weeks 27–38)

| Module | Status | Notes |
|---|---|---|
| Module 24 Phase 4: HPAS Full Hierarchy + ValidationGate | ❌ | |
| Module 9: Document Drafting Suite | ❌ | |
| Module 15: Integrated Mail Module | ❌ | |
| Module 15A: Persist Editor (ProseMirror) | ❌ | |
| Module 16: Advanced PDF Engine | ❌ | |

---

### Phase 5 — Contract Intelligence (Weeks 39–54)

| Module | Status | Notes |
|---|---|---|
| Module 10: Contract Intelligence Engine | ❌ | |

---

### Phase 6 — Daily Intelligence (Weeks 55–62)

| Module | Status | Notes |
|---|---|---|
| Module 11: Daily Intelligence & Workday Organiser | ❌ | |
| Module 17: Legal Reference Manager | ❌ | |
| Module 18: Legal Intelligence Layer | ❌ | |

---

### Phase 7 — Advanced Features (Post-launch)

| Module | Status | Notes |
|---|---|---|
| Module 12: Client iOS/iPadOS App | ❌ | |
| E-signature integration | ❌ | |
| Court cause list auto-import | ❌ | |

---

## 🐛 Known Issues / Blockers

| ID | Issue | Severity | Status | Notes |
|---|---|---|---|---|
| B01 | `sessions` table not built — auth is in-memory only, restart always logs out | Medium | Open | New auth spec adds 8-hour persistent sessions |
| B02 | `ip_assets` table missing — deadlines not linked to specific IP assets | Medium | Open | New docketing spec requires this |
| B03 | LaTeX stub — generate_invoice_pdf command will fail if pdflatex not installed | Medium | Partial | latex.rs now real impl; runtime requires MacTeX or bundled TeX Live sidecar |
| B04 | SCHEMA.md missing billing tables | Low | Resolved | Billing tables were already documented in SCHEMA.md from S07 |

---

## 📋 Decision Log

| Date | Decision | Reasoning |
|---|---|---|
| Apr 2026 | Tauri v2 over Electron for desktop | Smaller binary, local-first, Rust safety, iOS target |
| Apr 2026 | SQLite (local) as canonical data store | Offline-first, zero latency, client privilege |
| Apr 2026 | Three-tier AI routing: Haiku→Sonnet→Opus | Cost efficiency — ~24× cheaper than all-Opus |
| Apr 2026 | Sonnet (not Haiku) is escalation judge | Haiku can't reliably assess its own limits |
| Apr 2026 | Keel/Deck naming for Rust/React layers | Load-bearing convention — do not rename |
| Apr 2026 | ProseMirror for all rich text editors | Same engine as Notion/Linear — bidirectional MD |
| Apr 2026 | Matter Discussion tab over embedded messaging system | Two attorneys → messaging is over-engineering |
| Apr 2026 | HPAS as multi-agent orchestration engine above ai_router | O(n log n) token cost vs O(n²) — 127× more efficient at N=200 |
| Apr 2026 | SqliteStore (local) → RestateStore (cloud) via WorkflowStore trait | No callers change on backend swap |
| Apr 2026 | useHpas.ts for intelligence ops, ai.ts for single-model | HPAS orchestrates parallel agents; ai_router selects model inside SubAgent |
| Apr 2026 | In-memory session (AppState) for Phase 1 auth | Restart always requires re-auth. Persistent sessions (B01) added to spec for Phase 1.5 |
| Apr 2026 | SF Pro system font as default | Zero network requests, perfect macOS rendering, no FOIT |
| Apr 2026 | window.__persistLogout pattern for AppShell logout | Avoids prop-drilling through router — LogoutHandler registers fn, AppShell calls it |

---

## 🔧 Environment Notes

```bash
# Key commands
pnpm tauri dev          # Start Tauri desktop app in dev mode
pnpm tauri build        # Build production binary
cargo test              # Run Rust tests (from src-tauri/) — currently 30/30
cargo sqlx migrate run  # Apply pending migrations (from src-tauri/)
pnpm build              # Build Deck for production — currently 467 modules, 457kb

# Environment variables needed
ANTHROPIC_API_KEY=      # Claude API key for ai_router.rs
DATABASE_URL=           # SQLite path for sqlx CLI (dev only)
HETZNER_SYNC_URL=       # Sync server URL (Phase 2 M5)
```

---

## 📝 Session Log

> Brief log of what each session accomplished. Append, never delete.

| Date | Session summary | Files changed | Tests passing |
|---|---|---|---|
| Apr 11 2026 | S01: Full Phase 0 scaffold — all dirs, Keel stubs, Deck scaffold, design system, stores, IPC types | 30+ files across src-tauri/ and src/ | No tests yet |
| Apr 11 2026 | S02: Phase 1 M1 Steps 2+3 — migration 0002_matters.sql, db/queries/matters.rs + clients.rs, commands/matters.rs (14 cmds) | src-tauri/src/db/migrations/0002_matters.sql, queries/{matters,clients}.rs, commands/matters.rs, lib.rs, SCHEMA.md | cargo test: 10/10 |
| Apr 11 2026 | S03: Phase 1 M1 Step 4 — MatterList, MatterDetail, MatterStatusBadge, NewMatterDrawer, AppShell | src/App.tsx, AppShell.tsx, MatterStatusBadge.tsx, pages/Matters/*.tsx, ipc-types.ts | pnpm build: PASS |
| Apr 12 2026 | S04: Phase 1 M2 complete — migration 0003_deadlines.sql, queries/deadlines.rs (urgency_for), commands/deadlines.rs (7 cmds + statutory templates), deadline_watcher.rs, DocketList/IPAssetRecord/PipelineBoard/UrgencyBadge | All docket files | cargo test: 14/14, pnpm build: PASS (411kb) |
| Apr 13 2026 | S05: Phase 1 M3 complete — migration 0004_documents.sql, vault.rs (AES-256-GCM, 4 tests), queries/documents.rs (4 tests), commands/documents.rs (4 cmds), DocumentList/UploadDrawer | All document files | cargo test: 22/22, pnpm build: PASS (431kb) |
| Apr 15 2026 | S06: Auth complete — migration 0005_users.sql, queries/users.rs (3 tests), commands/auth.rs (login/logout/get_session), user seeding, stores/auth.ts, LoginScreen.tsx, SessionGate, AppShell live session + logout, SF Pro fonts, drafting font catalogue | All auth files, design system | cargo test: 25/25, pnpm build: PASS (439kb) |
| Apr 17 2026 | S07: Phase 2 M4 Billing — spec written, migration 0006_billing.sql (5 tables), queries/billing.rs (5 tests), commands/billing.rs (13 cmds), lib.rs billing commands registered, tauri.ts billing wrappers, BillingHome/TimeTracker/InvoiceList/FirmSettingsPanel. Also: new spec files placed in specs/ (module-02-docketing, auth-rbac, hpas-integration, module-03-documents updated), PROGRESS.md reconciled | specs/*.md, 0006_billing.sql, queries/billing.rs, commands/billing.rs, pages/Billing/*.tsx | cargo test: 30/30, pnpm build: PASS (467 modules, 457kb) |
| Apr 19 2026 | S08: Phase 2 M4 Billing UI complete — InvoiceDetail.tsx (back/actions/line items/GST panel/payment modal), InvoiceComposer.tsx (client→matter→entries→fixed-fee→GST type→live totals→create), InvoiceList wired (row click→detail, New Invoice→composer), latex.rs real impl (finds pdflatex, tempdir compile, 3 tests), invoice.tex GST-compliant template, client lookup added to generate_invoice_pdf, tempfile moved to [dependencies] | pages/Billing/InvoiceDetail.tsx, InvoiceComposer.tsx, InvoiceList.tsx, services/latex.rs, storage/templates/invoice.tex, commands/billing.rs, Cargo.toml | cargo test: 33/33, pnpm build: PASS (469 modules, 482kb) |

---

## HOW TO UPDATE THIS FILE

At the end of every session:
1. Change completed tasks from `❌` to `✅`
2. Add a row to the Session Log
3. Update "Current State" at the top
4. Clear "IN PROGRESS RIGHT NOW"
5. Add new bugs to Known Issues
6. Add new architecture decisions to Decision Log
