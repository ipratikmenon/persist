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
**Active module:** Phase 2 M5 Client Portal — Step 1 (spec) complete and awaiting your review. B07 cleared ahead of Step 3a.
**Last session completed:** S13 — 2026-08-05 — B07 resolved: real metadata stripping (`storage/metadata.rs`), internal-read vs client-export split. cargo test 86/86, pnpm build PASS (472 modules, 497kb).
**Last updated:** 2026-08-05

---

## 🔴 IN PROGRESS RIGHT NOW

> Fill this section at the start of a session. Clear it when done.

**M5 spec written — needs review before implementation starts.**
`specs/module-05-portal.md` is the contract. Per TASKS.md Step 1: review and
approve the spec before Step 2. If anything is wrong, fix the spec — not the
next session.

Points worth a decision during review:
  - Statutory deadlines default to client-visible; Procedural/Custom stay private (§6)
  - No client↔attorney messaging, no online payment, no multi-client users (§16)
  - Clients never see time entries or work-in-progress (§16)
  - Sync is off by default until a server URL + client cert are configured (§15.9)

✅ B07 cleared in S13 — `clean_metadata()` now strips for real and fails closed.
Chosen ahead of Step 2 because it is independent of the spec review: metadata
stripping is required for every export path, not just the portal.

⚠️ **Behaviour change to be aware of:** `get_document` no longer strips metadata
(it returns raw bytes for internal attorney use — a counterparty's tracked
changes are exactly what an attorney opens the file to read). The new
`export_document` command is the only client-facing path. DocumentList gained a
"↓ Client copy" action that reports what was removed.

Next: Step 2 (schema) — `0009_portal_sync.sql` + `server/migrations/` + RLS.
Gate: `test_rls_blocks_cross_client_read` must pass before any portal endpoint.
Nothing is pushed to main — awaiting your review.

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
| 2 | Migration: `ip_assets` table | ✅ | S11 — `0008_ip_assets.sql` + `deadlines.ip_asset_id` |
| 2 | Migration: `cascade_templates` table | ❌ | New from spec §2.9 |
| 2 | Migration: `deadline_escalations` table | ❌ | New from spec §2.11 |
| 2 | Migration: `document_intake_events` table | ❌ | New from spec §2.16 |
| 2 | Migration: `docket_errors` table | ❌ | New from spec §2.17 |
| 2 | Migration: `ip_fee_schedule` table | ❌ | New from spec §2.18 |
| 2 | `deadlines` table — add `ip_asset_id` | ✅ | S11 — via `0008_ip_assets.sql` |
| 2 | `deadlines` table — add `reference_number`, `dual_verified_by` fields | ❌ | New from spec |
| 2 | `SCHEMA.md` updated | ✅ | S11 — ip_assets + sessions documented; cascade still pending |
| 3 | `db/queries/deadlines.rs` — urgency_for(), CRUD | ✅ | S04 |
| 3 | `commands/deadlines.rs` — 7 commands + statutory templates (TM, Patent, Design, Copyright) | ✅ | S04 |
| 3 | `commands/ip_assets.rs` — CRUD for IP assets | ✅ | S11 — 5 commands + `db/queries/ip_assets.rs` (8 tests) |
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
| 2 | Migration: `sessions` table | ✅ | S11 — `0007_sessions.sql`, 8-hour expiry |
| 2 | User seeding (persist2026, bcrypt cost 12) | ✅ | S06 — in lib.rs block_on |
| 3 | `db/queries/users.rs` — 6 functions + 3 tests | ✅ | S06 |
| 3 | `commands/auth.rs` — login (bcrypt), logout, get_session | ✅ | S06 |
| 3 | Session persistence (8-hour expiry, `sessions` table) | ✅ | S11 — survives restart; keychain-held token; rate limiting + refresh_session |
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

| Step | Module | Status | Notes |
|---|---|---|---|
| 1 | `specs/module-05-portal.md` written | ✅ | S12 — sync contract, mirror schema, RLS, doc pipeline, OTP/JWT |
| 2 | Desktop migration `0009_portal_sync.sql` | ❌ | deadlines.is_client_visible, portal_users, sync_outbox, client_uploads, sync_state |
| 2 | PostgreSQL mirror schema (`server/migrations/`) | ❌ | `mirror.*` + `inbound.*` |
| 2 | RLS policies + portal_reader/portal_writer roles | ❌ | **Gate:** test_rls_blocks_cross_client_read |
| 3a | `services/sync_engine/projection.rs` — allow-list projections | ❌ | Security-relevant; tests must prove denied fields never serialise |
| 3a | `services/sync_engine/mod.rs` — outbox drain, push/pull, backoff | ❌ | |
| 3a | `commands/sync.rs` — 15 commands | ❌ | Replaces the stub |
| 3a | `clean_metadata()` made real | ✅ | S13 — B07 resolved; sync engine must use `export_document`, never `get_document` |
| 3b | Sync server Axum routes (`server/`) | ❌ | mTLS, object storage, WS |
| 3c | Portal backend: FastAPI, OTP auth, JWT (`portal/backend/`) | ❌ | |
| 4 | Portal frontend: 4 tabs (Matters/Documents/Invoices/Profile) | ❌ | |
| 5 | Validation pass | ❌ | End-to-end share → download → upload → ingest → revoke |

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
| Court cause list auto-import | ❌ | Absorbed by Module 26 (Phase 8) |

---

### Phase 2.5 — Data Protection Audit Engine (pulled forward, after M5)

| Module | Status | Notes |
|---|---|---|
| Module 31: Data Protection Audit Engine (DPDP 2023/GDPR) | ❌ | Roadmap planned (S09) — spec next. RoPA inventory, gap analysis, LaTeX audit reports, breach workflow |

---

### Phase 8 — Courtroom & Litigation Intelligence (Track A — adalat.ai-inspired)

Build order: M27 → M26 → M25 → M28 → M29. See `specs/expansion-roadmap.md` §3.

| Module | Status | Notes |
|---|---|---|
| Module 27: Document Digitization & Intake Intelligence | ❌ | LLM-OCR → structured extraction → intake review queue |
| Module 26: Cause List & Hearing Flow | ❌ | hearings table, cause_list_watcher, HearingBoard; absorbs Phase 7 cause-list item |
| Module 25: Hearing & Dictation Transcription | ❌ | Local whisper.cpp default, EN+HI, vault-encrypted; compliance flag required |
| Module 28: Research & Summarization | ❌ | Delivers M17/M18 AI surface; HPAS Opus tier |
| Module 29: Client Status Chatbot (WhatsApp) | ❌ | Two-way, mirror-subset-only answers, attorney escalation |

---

### Phase 9 — Persist Advisory: Startup Legal SaaS (Track B — visiocyber.ai-inspired)

See `specs/expansion-roadmap.md` §4–5. M31 ships earlier in Phase 2.5.

| Module | Status | Notes |
|---|---|---|
| Module 30: Startup Legal OS (SaaS, multi-tenant) | ❌ | Tenants + RLS, compliance calendar, doc generator w/ attorney review, AI Q&A, TM intake, subscriptions |
| Module 32: Compliance & AI Governance | ❌ | Framework libraries (DPDP/SOC 2/ISO 27001/CERT-In), evidence engine, responsible-AI policy generator, vendor scorecards |
| Module 33: Assessment & Advisory Toolkit | ❌ | Scored readiness assessments, advisory engagement tracker |

---

### Track C — Candidate Bets (NOT sequenced — see specs/expansion-roadmap.md §8)

Differentiators beyond feature-parity. **Nothing scheduled** — each needs an
explicit promote-into-a-phase decision + Step-1 spec before any work. Top three
bets: M35, M36, M38.

| Module | Status | Notes |
|---|---|---|
| Module 34: Matter Mind Map (Case Theory Canvas) | 🔵 candidate | Pocket-inspired; per-matter graph, nodes linked to records. Transcript/doc mind maps already folded into M25/M28 |
| Module 35: Limitation Engine ⭐ | 🔵 candidate | Deterministic deadline law (Limitation Act + court calendars as data); export via jurisdiction packs |
| Module 36: Order Watcher ⭐ | 🔵 candidate | eCourts/IP India poll + TM Journal watch; overnight order summaries → proposed docket entries |
| Module 37: Court-Rules Compiler | 🔵 candidate | Per-court formatting rules as data → filing-ready PDF via LaTeX pipeline |
| Module 38: Firm Brain ⭐ | 🔵 candidate | Precedent memory with clause provenance; compounding moat; builds on HPAS VectorMemory |
| Module 39: Bench Analytics | 🔵 candidate | India-first judge/forum insight over public data; ethics framing required |
| Module 40: Client-Held Privilege Keys | 🔵 candidate | Client-custody encryption + verifiable privilege log; strongest export/trust feature |
| Module 41: Vernacular Voice Intake | 🔵 candidate | WhatsApp voice note → structured matter brief; reuses M25/M27/M29 |

---

## 🐛 Known Issues / Blockers

| ID | Issue | Severity | Status | Notes |
|---|---|---|---|---|
| B01 | `sessions` table not built — auth is in-memory only, restart always logs out | Medium | Resolved | S11 — `0007_sessions.sql`, keychain-held token, 8h expiry + refresh, 5-attempt lockout |
| B02 | `ip_assets` table missing — deadlines not linked to specific IP assets | Medium | Resolved | S11 — `0008_ip_assets.sql`, 5 CRUD commands, `deadlines.ip_asset_id`, asset panel + filter UI |
| B03 | LaTeX stub — generate_invoice_pdf command will fail if pdflatex not installed | Medium | Partial | latex.rs now real impl; runtime requires MacTeX or bundled TeX Live sidecar |
| B04 | SCHEMA.md missing billing tables | Low | Resolved | Billing tables were already documented in SCHEMA.md from S07 |
| B05 | Keel cannot build on Linux without GTK/WebKit system libs | Low | Resolved | S11 — `libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev` after `apt-get update`. Needed in any CI image |
| B06 | Deck parses Keel DATETIME strings as local time, not UTC | Medium | Partial | S11 — `src/lib/dates.ts` (`parseKeelDateTime`) added and used for session expiry. Pre-existing `new Date(...)` call sites elsewhere still unconverted |
| B07 | `clean_metadata()` is a pass-through stub — sharing a document to the portal is an export | High | Resolved | S13 — `storage/metadata.rs`: PDF info/XMP, OOXML props/comments/tracked changes, JPEG EXIF+GPS, PNG chunks. Fails closed on unsupported types. 19 tests |

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
| Jul 2026 | Two expansion tracks: Courtroom Intelligence (M25–M29) + Startup Legal SaaS (M30–M33) | adalat.ai feature parity firm-side; visiocyber.ai-style productized data-protection/compliance services as recurring revenue |
| Jul 2026 | Bifurcated source of truth: SaaS tenant data is server-native PostgreSQL | One desktop cannot be the write path for N self-serve tenants; firm practice data stays desktop-SQLite-first; separate schemas, no cross-privilege joins (roadmap §5.1) |
| Jul 2026 | Server-side AI router in `server/` for portal/chatbot/tenant AI traffic | Same three-tier Haiku→Sonnet→Opus discipline; per-tenant token metering feeds subscription billing; tenants never see model names (roadmap §5.2) |
| Jul 2026 | HPAS ValidationGate as shared lawyer-in-the-loop primitive for all client/tenant-facing AI output | AI output is a draft until attorney approval — liability + ethics backbone of Track B (roadmap §5.3) |
| Jul 2026 | M31 DP Audit Engine pulled forward to Phase 2.5 (before Phase 3 AI) | Needs only portal + billing + LaTeX (all built after M5); nearest-term revenue; AI gap-analysis upgrades later |
| Aug 2026 | Session token = the `sessions` row id (UUID), held in OS keychain | No second secret to manage; validity is a pure DB question (`expires_at > now`), so a restart can restore without re-auth. Reverses the S06 in-memory decision now that B01 is closed |
| Aug 2026 | Keychain falls back to a 0600 file when no Secret Service exists | Linux dev/CI containers have no keychain daemon. The fallback sits beside persist.db — anyone who can read it can already read every matter, so it is no weaker in context. macOS/Windows always use the real keychain |
| Aug 2026 | Login deletes the user's prior sessions | One device holds one live token; logging in elsewhere invalidates the old session. Simple and predictable for a 2-attorney firm |
| Aug 2026 | Session refresh is activity-gated, not unconditional | An idle app expires on schedule instead of renewing itself forever; a working attorney is never logged out mid-task |
| Aug 2026 | IP asset deletion refuses while deadlines reference it | Silently cascading or orphaning statutory deadlines is the one failure mode a docketing system must not have. Attorney reassigns explicitly |
| Aug 2026 | Migrations tested by applying the real set to an empty DB | Query-layer tests hand-build their tables, so they cannot catch a bad migration. These caught two fixture drifts on the first run |
| Aug 2026 | Sync uses allow-list projections, never whole-row serialisation | Adding a desktop column must never leak it to the portal. Inverse of a deny-list; the single most important safety property in M5 |
| Aug 2026 | Client portal stays a mirror — desktop remains authoritative (NOT the roadmap §5.1 exception) | §5.1's server-native PostgreSQL applies to Track B SaaS tenants only. Portal data is a projection; separate PG schemas, no cross-joins |
| Aug 2026 | Client-authored items land in an inbound queue the desktop pulls | The server never writes to SQLite. If the firm never opens the desktop, nothing enters firm data — and the desktop can refuse |
| Aug 2026 | Statutory deadlines default client-visible; Procedural/Custom private | A client must not be surprised by a deadline they are legally affected by; strategy notes and internal steps stay private. Per-deadline override either way |
| Aug 2026 | `deadlines.notes` never syncs | It routinely holds strategy ("weak prior art, consider opposing"). Client sees the event and the date |
| Aug 2026 | Portal DB role has SELECT only on mirror.*, with FORCE ROW LEVEL SECURITY | The portal is structurally incapable of mutating the mirror; RLS applies even to the owner. `SET LOCAL` scopes client context to the transaction so pooled connections cannot leak it |
| Aug 2026 | Mirror money is NUMERIC(14,2), rounded on projection | Desktop stores REAL; an invoice total shown to a client must never drift by floating-point noise |
| Aug 2026 | Metadata stripping fails closed | An unsupported file type errors instead of passing bytes through. A caller exporting to a client must not be able to ship uncleaned bytes by accident — the old stub made the documented rule a no-op |
| Aug 2026 | Internal read and client export are separate commands | `get_document` returns raw bytes (an attorney reviewing a counterparty draft needs its tracked changes); `export_document` is the only client-facing path and always cleans. Conflating them either leaks metadata or destroys the information the attorney opened the file to read |
| Aug 2026 | Tracked changes are *accepted*, not rejected, when cleaning | Insertions become plain text, deletions vanish — which is what "send the clean copy" means to an attorney |
| Aug 2026 | Format detected by magic bytes, never by mime string or extension | A caller-supplied mime type is not evidence; a .docx renamed to .pdf must still be cleaned as a .docx |
| Aug 2026 | Export reports what it removed | "3 tracked change(s), Reviewer comments, GPS location data" is shown to the attorney. Silent stripping gives no chance to notice that a file should not have been sent at all |

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
| Jul 19 2026 | S09: Expansion roadmap (planning only, no code) — researched adalat.ai + visiocyber.ai; wrote specs/expansion-roadmap.md defining Track A Courtroom Intelligence (M25 transcription, M26 hearings/cause lists, M27 doc digitization, M28 research/summarization, M29 WhatsApp chatbot) and Track B Startup Legal SaaS (M30 Startup Legal OS, M31 DP Audit Engine → Phase 2.5, M32 Compliance & AI Governance, M33 Assessments); added Phases 2.5/8/9 to TASKS.md; 4 architecture decisions logged | specs/expansion-roadmap.md (new), TASKS.md, PROGRESS.md, SESSION-LOG/2026-07-19-S09-expansion-roadmap.md | No code changed — tests unaffected (33/33 as of S08) |
| Aug 5 2026 | S13: B07 resolved — `storage/metadata.rs` (real stripping: PDF info dict + XMP via lopdf; OOXML core/app/custom props, comments, tracked changes incl. nested, dangling-rel cleanup via zip + quick-xml; JPEG APP/COM segments incl. GPS detection; PNG text chunks; plain text). Fails closed on unsupported types. Split internal read (`get_document`, raw) from client export (`export_document`, cleaned + report). Deck: "↓ Client copy" action reporting what was removed. Deps added: zip, quick-xml, lopdf. KEEL-RULES.md and M5 spec §9.1/§15/§18 updated | src-tauri/src/storage/{metadata.rs (new),vault.rs,mod.rs}, commands/documents.rs, lib.rs, Cargo.toml, KEEL-RULES.md, specs/module-05-portal.md, src/lib/{ipc-types,tauri}.ts, src/pages/Documents/DocumentList.tsx | cargo test: 86/86 (was 67), pnpm build: PASS (472 modules, 497kb) |
| Aug 5 2026 | S12: Phase 2 M5 Step 1 — `specs/module-05-portal.md` (spec only, no code). Defines the three surfaces, allow-list sync projection + outbox/inbound queue, desktop migration 0009 additions, full PostgreSQL `mirror.*`/`inbound.*` schema, RLS policies + role separation, outbound/inbound document pipelines, OTP+JWT auth, 15 Keel commands, 8 server routes, 14 portal API routes, 4 frontend tabs, 14 constraints, 8 resolved open questions, 5 implementation sub-steps with gates. Raised B07 (clean_metadata stub blocks doc sharing) | specs/module-05-portal.md (new), PROGRESS.md, SESSION-LOG/2026-08-05-S12-portal-spec.md | No code changed — spec session (67/67 and build unchanged from S11) |
| Aug 5 2026 | S11: B01 + B02 closed. **B01:** `0007_sessions.sql`, `db/queries/sessions.rs` (9 tests), `services/keychain.rs` (5 tests, keyring + 0600 file fallback), `commands/auth.rs` rewritten (session rows, keychain token, 5-attempt/60s lockout, `refresh_session`), AppState gains keychain + login_attempts, expired sessions cleared at startup, `src/lib/dates.ts` (UTC parsing), `SessionKeepAlive` in App.tsx. **B02:** `0008_ip_assets.sql` (+ `deadlines.ip_asset_id`), `db/queries/ip_assets.rs` (8 tests), `commands/ip_assets.rs` (5 commands, 4 validation tests), deadline layer carries `ipAssetId`, `IpAssetStatusBadge.tsx` + `IpAssetDrawer.tsx`, asset panel with per-asset deadline filtering. Plus `db/mod.rs` migration tests (4) and SCHEMA.md | 0007/0008 migrations, queries/{sessions,ip_assets}.rs, services/keychain.rs, commands/{auth,ip_assets,deadlines}.rs, db/mod.rs, lib.rs, SCHEMA.md, App.tsx, lib/{dates,tauri,ipc-types}.ts, components/dockets/{IpAssetStatusBadge,IpAssetDrawer}.tsx, pages/Dockets/IPAssetRecord.tsx | cargo test: 67/67, pnpm build: PASS (472 modules, 497kb) |
| Jul 19 2026 | S10: Track C candidate bets (planning only) — researched heypocket.com (Pocket AI notes: auto mind maps, Central Theme→Branches→Nodes, tap-to-transcript). Folded transcript/document mind maps into M25/M28; added roadmap §8 + TASKS.md Track C with M34 Matter Mind Map + M35 Limitation Engine⭐ + M36 Order Watcher⭐ + M37 Court-Rules Compiler + M38 Firm Brain⭐ + M39 Bench Analytics + M40 Client-Held Privilege Keys + M41 Vernacular Voice Intake (none sequenced) | specs/expansion-roadmap.md, TASKS.md, PROGRESS.md, SESSION-LOG/2026-07-19-S10-track-c-candidate-bets.md | No code changed — tests unaffected (33/33 as of S08) |

---

## HOW TO UPDATE THIS FILE

At the end of every session:
1. Change completed tasks from `❌` to `✅`
2. Add a row to the Session Log
3. Update "Current State" at the top
4. Clear "IN PROGRESS RIGHT NOW"
5. Add new bugs to Known Issues
6. Add new architecture decisions to Decision Log
