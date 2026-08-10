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
**Active module:** LaTeX pipeline repaired — invoice PDFs now actually generate. Next: Step 3d (object storage, email delivery) or Step 4 (portal frontend).
**Last session completed:** S19 — 2026-08-10 — LaTeX/template pipeline: engine switched to XeLaTeX, escaping made unforgettable, three blocking defects fixed. cargo test 174/174 including real compilation.
**Last updated:** 2026-08-10

---

## 🔴 IN PROGRESS RIGHT NOW

> Fill this section at the start of a session. Clear it when done.

**Nothing in progress.** S19 repaired the LaTeX pipeline. On
`claude/new-session-dbqe5o`, **nothing pushed to main**.

**Invoice PDF generation had never worked.** Three independent faults, each
fatal on its own, all invisible because no test ever ran the engine — and
pdfLaTeX was not installed in CI or the dev container:

  ✅ **The firm's own name broke compilation.** `firm_name` defaults to
     `'Persistas & Partners'` and `&` is LaTeX's alignment character. Escaping
     was applied to one field out of twenty-one.
  ✅ **The rupee sign broke compilation.** pdfLaTeX fails outright on U+20B9,
     which the template hardcodes and every line item carries.
  ✅ **The documents row violated a foreign key.** `generate_invoice_pdf` bound
     the client id into `documents.matter_id`, which references `matters(id)` —
     so the PDF was encrypted into the vault and *then* the row failed, leaving
     an orphan.

What changed:

  ✅ **Engine is now XeLaTeX**, with `fontspec` + Noto Serif. Handles ₹ and
     Devanagari natively — the latter matters for Hindi filings, and switching
     later with a template library in place would have been far more expensive.
  ✅ **Escaping is no longer forgettable.** `compile_latex` takes
     `HashMap<String, Field>`, and a `Field` is either `text` (escaped on the
     way in) or `raw` (deliberate LaTeX). A bare string does not compile. The
     old `latex_escape` is deleted, not deprecated.
  ✅ **The old escaper corrupted backslashes silently** — it mapped `\` to
     `\\`, a LaTeX line break, so "In re Bajaj\Auto" rendered across two lines
     with a clean compile and no warning. Now `\textbackslash{}`.
  ✅ **An unfilled placeholder is an error**, not `{{CLIENT_ADDRESS}}` printed
     into a document that goes to a client. There was a test asserting the old
     behaviour was correct; it is gone.
  ✅ Two compilation passes (`longtable` needs it), 60s timeout, `-no-shell-escape`,
     stdin closed, job names sanitised, and LaTeX's actual error surfaced instead
     of a thousand-line log.
  ✅ **Indian digit grouping** — `₹6,00,000.00`, not `₹600000.00`. It sits beside
     `amount_in_words`, which already says "Lakh".
  ✅ **Tests that compile the real template with hostile values.** Both original
     faults were reproduced as negative controls before being trusted.

⚠️ **The installer and CI now need `xelatex` + Noto.** There is no CI workflow in
this repo yet; when one is added it needs `texlive-xetex fonts-noto-core`.
`tauri.conf.json` sidecar bundling is still outstanding (B03).

⚠️ **Still awaiting your review:** the M5 spec.

Next: **M5 Step 3d** (object storage + email) or **Step 4** (portal frontend).

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
| 2 | Migration: `cascade_templates` table | ✅ | S14 — `0010_cascade.sql`, 7 Indian templates seeded with statutory citations |
| 2 | Migration: `deadline_escalations` table | ✅ | S14 — UNIQUE(deadline_id, level) so the 30-min watcher cannot spam |
| 2 | Migration: `document_intake_events` table | ❌ | New from spec §2.16 |
| 2 | Migration: `docket_errors` table | ✅ | S15 — `0011_verification.sql` |
| 2 | Migration: `ip_fee_schedule` table | ❌ | New from spec §2.18 |
| 2 | `deadlines` table — add `ip_asset_id` | ✅ | S11 — via `0008_ip_assets.sql` |
| 2 | `deadlines` table — add `reference_number`, verification fields | ✅ | S15 — `0011_verification.sql` (+ created_by, is_verified, verified_by/at) |
| 2 | `SCHEMA.md` updated | ✅ | S11 — ip_assets + sessions documented; cascade still pending |
| 3 | `db/queries/deadlines.rs` — urgency_for(), CRUD | ✅ | S04 |
| 3 | `commands/deadlines.rs` — 7 commands + statutory templates (TM, Patent, Design, Copyright) | ✅ | S04 |
| 3 | `commands/ip_assets.rs` — CRUD for IP assets | ✅ | S11 — 5 commands + `db/queries/ip_assets.rs` (8 tests) |
| 3 | `services/cascade_engine.rs` — chain generator from templates | ✅ | S14 — 14 tests; month/year arithmetic clamps to month end |
| 3 | `services/abandonment_watcher.rs` — 14d/7d/3d/missed alerts | ✅ | S14 — 30-min poll, cumulative levels, sets status `Missed`; 10 tests |
| 3 | `services/deadline_watcher.rs` — 15-min poll + OS notifications | ✅ | S04 |
| 3 | Dual verification enforcement | ✅ | S15 — `verify_deadline` refuses your own deadline; `list_unverified_deadlines` |
| 3 | `services/intake_sla_tracker.rs` | ❌ | New from spec §2.16 |
| 3 | `commands/docket_audit.rs` — error log | ◐ | S15 — `docket_errors` table exists (0011); commands not yet built |
| 3 | `db/queries/fee_schedule.rs` — fee lookups | ❌ | New from spec §2.18 |
| 3 | `cargo test` passes | ✅ | S04 — 14/14 (core deadlines) |
| 4 | `pages/Dockets/DocketList.tsx` (urgency filter chips, inline mark-complete) | ✅ | S04 |
| 4 | `pages/Dockets/IPAssetRecord.tsx` (timeline + add deadline drawer + templates) | ✅ | S04 |
| 4 | `pages/Dockets/PipelineBoard.tsx` (Kanban by status) | ✅ | S04 |
| 4 | `components/dockets/UrgencyBadge.tsx` | ✅ | S04 |
| 4 | `pages/Dockets/RenewalDashboard.tsx` | ✅ | S14 — firm-wide renewals + open escalations with inline resolve |
| 4 | `components/dockets/CascadePreview.tsx` | ✅ | S15 — preview-then-commit drawer with last-verified date |
| 4 | `components/dockets/VerificationBadge.tsx` | ✅ | S15 — statutory only; plus ReferenceChip |
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
| 3 | RBAC enforcement at command level | ✅ | S15 — `src-tauri/src/rbac.rs`, 6 tests; unknown roles fail closed |
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
| 2 | Desktop migration `0009_portal_sync.sql` | ✅ | S14 — deadlines.is_client_visible (+statutory backfill), portal_users, sync_outbox, client_uploads, sync_state |
| 2 | PostgreSQL mirror schema (`server/migrations/`) | ✅ | S14 — `0001_mirror.sql`: mirror.* (9 tables) + inbound.* (3 tables) |
| 2 | RLS policies + portal_reader/portal_writer roles | ✅ | S14 — `0002_rls.sql`: 3 roles, FORCE RLS, WITH CHECK on inbound. **Gate passes** (10 assertions, verified to fail when RLS disabled) |
| 2 | `server/SCHEMA.md` written | ✅ | S14 |
| 3a | `services/sync_engine/projection.rs` — allow-list projections | ✅ | S16 — 12 tests asserting denied fields never reach the wire payload |
| 3a | `services/sync_engine/mod.rs` — outbox drain, backoff, state | ✅ | S16 — collapsing enqueue, Delete supersedes Upsert, capped backoff (11 tests) |
| 3a | `commands/sync.rs` — sync + portal commands | ✅ | S16 — 11 commands; the 4 ingest/dispute ones land with the transport (3b) |
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
| B03 | LaTeX engine not bundled in the installer | Medium | Partial | S19: engine is XeLaTeX + Noto and is resolved at runtime (sidecar → known paths → PATH), with a clear error naming the install command. `tauri.conf.json` sidecar bundling still outstanding |
| B10 | Invoice PDF generation had never produced a PDF — firm name `&`, rupee sign, and a `documents.matter_id` FK violation | **Critical** | ✅ Fixed | S19. None of the three could be caught by the string-only tests that existed; all three are now covered by tests that compile the real template |
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
| Aug 2026 | Portal DB roles separated: portal_reader (SELECT on mirror) vs portal_writer (INSERT on 2 inbound tables) vs sync_writer | The portal is structurally incapable of mutating the mirror. portal_writer has NO grant on otp_challenges, so a leaked portal credential cannot reach OTP hashes |
| Aug 2026 | RLS unset context matches nothing, not everything | `current_setting(...,true)` is NULL when unset and every policy uses `=`, so a request that forgets to set client context returns zero rows. Asserted by Test 3 |
| Aug 2026 | Cascade templates are DATA, not Rust | When a statutory period changes, that is a DB update and a new last_verified date — not a code change and a release |
| Aug 2026 | Registry-triggered events are their own cascade anchors | The firm cannot know at filing when an examination report will issue. TMApplication generates only what is computable from the filing date; TMExaminationReport runs later with the report's real date. Nothing is dated from an event that has not happened |
| Aug 2026 | Cascade generation is preview-then-commit | An attorney sees the whole chain, with the template's last_verified date, before any deadline exists. Silently creating 19 annuities on a wrong anchor date would be a mess to unpick |
| Aug 2026 | Escalation levels are cumulative and idempotent | A deadline first seen 5 days out backfills L1 and L2 so the audit trail is not misleading; the UNIQUE index (not app logic) stops the 30-minute watcher re-raising |
| Aug 2026 | Only Statutory deadlines escalate | Escalating the firm's own internal working dates would train attorneys to ignore the alerts that actually matter |
| Aug 2026 | RBAC is a permission→minimum-role lookup table, not scattered role checks | The matrix lives in one place, reads like the spec, and is exhaustively testable. An unknown role gets rank 0 and is denied everything — failing closed beats being helpful about a role that should not exist |
| Aug 2026 | `created_by` on a deadline comes from the session, never the payload | Dual verification is defeated if Deck can lie about who entered the date. The field is `#[serde(skip_deserializing)]` |
| Aug 2026 | Screenshots use a Vite alias to fixture data, not a running Tauri app | The desktop binary needs a display and a real Keel; aliasing `@tauri-apps/api/core` renders the actual components against representative data. Wired only by vite.config.screenshots.ts, so it can never reach a shipped build |
| Aug 2026 | Projections are plain functions, never `From<Row>` | `From` invites `..Default` and struct-update syntax, both of which can carry fields you never named. Listing every field by hand is harder to write and much harder to get wrong |
| Aug 2026 | Projection tests assert on serialised JSON, not struct fields | Checking fields only proves what we already know. Building a row with `INTERNAL_LEAK` in every unnamed column and asserting it is absent from the wire bytes proves what actually leaves |
| Aug 2026 | A Delete in the outbox supersedes a pending Upsert | Otherwise an upsert queued before an un-share resurrects the row in the mirror. Re-queuing the same op collapses, so five edits before one sync are one push |
| Aug 2026 | Enabling sync without a server URL is refused | A firm that believes sync is on and is wrong is worse off than one that sees an error |
| Aug 2026 | XeLaTeX, never pdfLaTeX | ₹ is the immediate blocker — pdfLaTeX fails outright on U+20B9 — and Devanagari for Hindi filings is the one coming. Switching with one template in place is cheap; switching after M9 builds a template library would not be |
| Aug 2026 | Template values are a `Field` type, not a `String` | Escaping as a call-site responsibility produced exactly one escaped field out of twenty-one. Making the unescaped path a distinct constructor (`Field::raw`) means forgetting is a compile error, and injection has one greppable origin |
| Aug 2026 | An unfilled placeholder fails the render | The old behaviour printed `{{CLIENT_ADDRESS}}` into the PDF, and a test asserted that was correct. A template and its caller disagreeing is a bug in one of them, not something to ship to a client |
| Aug 2026 | Templates are tested by compiling them, not by asserting on strings | Three fatal faults survived a green suite because nothing ever ran the engine — and the engine was not installed. A template test that does not compile is not a test of the template |
| Aug 2026 | Money is grouped Indian-style on invoices | `₹600000.00` beside "Six Lakh" in words reads as a mistake on a GST invoice |
| Aug 2026 | The portal gets a fourth PostgreSQL role, `portal_auth` | Neither existing role can log a client in: `portal_reader` is scoped by a client id login has not established yet, and neither may touch an OTP challenge. Kept narrow — it can resolve an email and nothing else, so a compromised auth connection yields the client roster and nothing about the firm's work |
| Aug 2026 | Refresh tokens rotate, and reuse revokes the whole family | Replay and theft are indistinguishable. Losing a session is a small cost; leaving a thief with a live one is not |
| Aug 2026 | Another client's row is a 404, never a 403 | 403 confirms the id is real. 404 tells them nothing |
| Aug 2026 | The API adds no `status <> 'Draft'` filter on invoices | The mirror's CHECK already forbids one. Filtering in the API would paper over a projection bug instead of surfacing it |
| Aug 2026 | The RLS gate derives its exemptions from grants, not table names | A named exemption list grows silently. Deriving it means a future table that *is* portal-reachable and unprotected still fails the gate — negative-control verified |
| Aug 2026 | No OpenAPI schema is served | The portal is not a public API; a schema only describes attack surface to someone with no business calling it |
| Aug 2026 | The client row is a synced entity like any other | Every table in `mirror.*` has a foreign key to `mirror.clients`. Auto-creating a stub row server-side would put an empty client name in the portal; projecting it properly costs two fields |
| Aug 2026 | Clients are sorted ahead of everything else in a push batch | The outbox is oldest-first and clients are created before their matters, so the order usually holds — "usually" is not what a first sync should rest on |
| Aug 2026 | `note_change` logs a failed enqueue instead of failing the write | The local row is already committed. Showing an attorney a failure for a change that did happen is the worse failure mode; the miss surfaces on `sync_state.last_error` instead |
| Aug 2026 | A sync run stamps `last_pushed_at` even when a leg failed | It answers "when did we last talk to the server". `last_error` carries the qualification, and Deck no longer says "Sync complete" when one is set |
| Aug 2026 | Client uploads are pulled as metadata only | An unreviewed client file must not be written into the firm's vault automatically, whatever the scanner said |
| Aug 2026 | Auth is a shared secret, not mTLS as the spec asks | Deliberate shortfall, recorded rather than quietly dropped. Adequate on a private network; must be closed before the server faces the open internet |
| Aug 2026 | Remaining outbox write-path wiring deferred to Step 3b | Enqueuing from fifteen call sites with nothing to drain them is untestable code written a sprint early. It lands with the transport so both can be tested together |
| Aug 2026 | Deck permission gating duplicates rbac.rs by hand | Per the auth spec, Deck gating is UX convenience and Keel is the enforcer. A divergence between the two degrades to "button shown, command refused" — never to a leak — so a hand-kept copy is an acceptable cost for not inventing an IPC round trip per button |
| Aug 2026 | Nav active state matches exact-or-child, not prefix | `/dockets` was lighting up on `/dockets/renewals`, so two items appeared active at once |

---

## 🔧 Environment Notes

```bash
# Key commands
pnpm tauri dev          # Start Tauri desktop app in dev mode
pnpm tauri build        # Build production binary
cargo test --lib        # Run Rust tests (from src-tauri/) — currently 156/156
cargo sqlx migrate run  # Apply pending migrations (from src-tauri/)
pnpm build              # Build Deck for production — currently 477 modules, 529kb

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
| Aug 10 2026 | S19: LaTeX/template pipeline repaired. Invoice PDF generation had never worked — three independent fatal faults (`&` in the seeded firm name, ₹ under pdfLaTeX, and a `documents.matter_id` FK violation that orphaned a vault file), none catchable by the string-only tests that existed. Engine switched to XeLaTeX + fontspec + Noto Serif (also unlocks Devanagari for Hindi filings). `compile_latex` now takes `HashMap<String, Field>` where `Field::text` escapes and `Field::raw` does not — a bare string no longer compiles, which is what made "escaped one field out of twenty-one" possible. Old `latex_escape` deleted: it mapped `\\` to a LaTeX line break and silently split citations across lines. Unfilled placeholders now fail instead of printing `{{CLIENT_ADDRESS}}` to a client. Added two passes, 60s timeout, `-no-shell-escape`, job-name sanitising, LaTeX-error extraction, and Indian digit grouping (`₹1,55,760.00`). New compilation tests run the real engine against the real template; both original faults reproduced as negative controls | src-tauri/src/services/latex.rs (rewritten), src-tauri/src/commands/billing.rs, src-tauri/storage/templates/invoice.tex, src-tauri/KEEL-RULES.md, CLAUDE.md | cargo test: 174/174 (was 170), incl. 4 real-compilation tests |
| Aug 9 2026 | S18b: M5 Step 3c — the portal API. `portal/backend/` FastAPI: OTP login (6-digit, bcrypt-hashed, 10-min expiry, 5 attempts then dead, new code kills the old), RS256 JWT, rotating refresh tokens with family-wide revocation on reuse, and every read route in spec §13. Four independent locks on privilege — RLS via `SET LOCAL`, three roles across three connections, a `client_id` filter in every query, and Pydantic models with `extra="forbid"` — each tested with the others disabled. New roles/tables: `0003_portal_auth.sql` (the login path could not run under either existing portal role), `0004_refresh_tokens.sql`. RLS gate extended 10 → 12 assertions and its exemption list derived from grants rather than table names. **Bug found by its own test:** the 401 in verify-otp was raised inside the transaction, rolling back the attempt counter and leaving the OTP brute-forceable | portal/backend/{pyproject.toml,README.md,app/**,tests/**}, server/migrations/{0003_portal_auth,0004_refresh_tokens}.sql, server/tests/rls_test.sql, server/SCHEMA.md, .gitignore | portal: 48/48 vs live PostgreSQL, RLS gate: 12/12 (negative-control verified) |
| Aug 9 2026 | S18: M5 Step 3b — the transport. **Server:** `server/src/{main,types,mirror}.rs` — Axum routes, constant-time token check, refuses to start on a short token, `deny_unknown_fields` on every wire type, money as exact NUMERIC. **Keel:** `services/sync_engine/transport.rs` (push/pull/ack, `build_entry` returns None for withheld rows, `order_for_push`), `record_sync_run`, `note_change`; `commands/sync.rs::trigger_sync` wired for real + `set_sync_token`; outbox enqueue added to every remaining write path (matters, clients, deadlines, IP assets, invoices, payments) — the S16 deferral, now closed. Keychain generalised to hold session and sync tokens without collision. **Gap the e2e test found:** nothing projected the client row that every mirror FK points at — added `ClientPublic` + migration `0012_outbox_client.sql`. **Deck:** write-only sync token field, honest "Sync now" messaging (a run that failed a leg no longer reports "complete"). New `tests/sync_e2e.rs` — 4 tests, skipped without a server, run green against live PostgreSQL | server/src/*, src-tauri/src/services/sync_engine/{mod,transport,projection}.rs, services/keychain.rs, commands/{sync,matters,deadlines,ip_assets,billing}.rs, db/queries/matters.rs, db/migrations/0012_outbox_client.sql, tests/sync_e2e.rs, lib.rs, db/SCHEMA.md, src/pages/Portal/PortalHome.tsx, src/lib/{tauri,ipc-types}.ts, src/stores/sync.ts, screenshots/* | cargo test: 156/156 (was 146), sync e2e: 4/4 vs live PostgreSQL, pnpm build: PASS |
| Aug 6 2026 | S17: Deck surface for M5. `pages/Portal/PortalHome.tsx` (Client access + Sync tabs), `lib/permissions.ts` mirroring rbac.rs for UI gating, share/unshare toggle on document rows with live SHARED badge, verify action + client-visibility toggle on the docket timeline, nav gains Renewals and Portal with fixed active-state matching. Exposed `is_client_visible` on the Deadline IPC type (row had it, wire type did not). Screenshot harness extended to 12 views | src/pages/Portal/PortalHome.tsx, src/lib/permissions.ts, src/pages/Documents/DocumentList.tsx, src/pages/Dockets/IPAssetRecord.tsx, src/components/shell/AppShell.tsx, src/App.tsx, src/lib/ipc-types.ts, src-tauri/src/commands/deadlines.rs, src-tauri/src/db/queries/deadlines.rs, screenshots/* | cargo test: 146/146, pnpm build: PASS |
| Aug 6 2026 | S16: M5 Step 3a — sync engine. `services/sync_engine/projection.rs`: MatterPublic/DeadlinePublic/IpAssetPublic/InvoicePublic, plain functions (not From), `Withheld` for drafts and non-visible deadlines, money rounded to 2dp; 12 tests that serialise to JSON and assert sensitive values are absent, plus a guard test that fails if a time-entry projection is ever added. `services/sync_engine/mod.rs`: outbox with collapsing enqueue and Delete-supersedes-Upsert, retry/backoff capped at 1h, sync state with enable-requires-URL; 11 tests. `commands/sync.rs` rewritten: 11 commands. Also added `is_client_visible` to DeadlineRow (column existed since 0009 but was never selected) | services/sync_engine/{mod,projection}.rs, commands/sync.rs, db/queries/deadlines.rs, services/mod.rs, lib.rs, src/lib/{ipc-types,tauri}.ts, src/stores/sync.ts | cargo test: 146/146 (was 122), pnpm build: PASS |
| Aug 6 2026 | S15: Three parallel sprints + screenshot harness. **RBAC:** `src-tauri/src/rbac.rs` (Permission enum, rank table, `require` guard, 6 tests); guards on close/archive matter, delete_document, create_invoice, update_firm_settings, create/verify deadline; replaced the ad-hoc Partner check in billing. **Dual verification:** `0011_verification.sql` (created_by, reference_number, is_verified, verified_by/at, docket_errors), `verify_deadline` + `list_unverified_deadlines`, P&P-DD-NNNN generator, statutory-default client visibility, 3 new query tests. **Cascade UI:** `CascadePreview.tsx`, `VerificationBadge.tsx` + `ReferenceChip`, wired into IPAssetRecord (Generate chain action, badges + refs on timeline rows); fixed `humanise` not splitting the TM acronym. **Screenshots:** `screenshots/{mock,capture.mjs}`, `vite.config.screenshots.ts`, 10 captured views | src-tauri/src/rbac.rs, 0011_verification.sql, commands/{deadlines,matters,billing,documents}.rs, db/queries/deadlines.rs, lib.rs, src/components/dockets/{CascadePreview,VerificationBadge}.tsx, src/pages/Dockets/IPAssetRecord.tsx, src/lib/{ipc-types,tauri}.ts, screenshots/*, vite.config.screenshots.ts, .gitignore | cargo test: 122/122 (was 113), pnpm build: PASS |
| Aug 5 2026 | S14: Two sprints. **M5 Step 2:** `0009_portal_sync.sql` (is_client_visible + statutory backfill, portal_users, sync_outbox, client_uploads, sync_state), `server/migrations/0001_mirror.sql` (mirror.* 9 tables + inbound.* 3), `0002_rls.sql` (3 roles, FORCE RLS, WITH CHECK), `server/tests/rls_test.sql` + `scripts/test-rls.sh` (10 assertions, negative-control verified), `server/SCHEMA.md`. **Cascade + abandonment:** `0010_cascade.sql` (cascade_templates w/ 7 seeded Indian templates, deadline_escalations, deadlines rebuilt for status Missed), `services/cascade_engine.rs` (14 tests), `services/abandonment_watcher.rs` (10 tests), `commands/cascade.rs` (5 cmds), `list_upcoming_renewals`, `RenewalDashboard.tsx` | 0009/0010 migrations, server/{migrations,tests,scripts,SCHEMA.md}, services/{cascade_engine,abandonment_watcher}.rs, commands/{cascade,ip_assets}.rs, db/{mod.rs,queries/ip_assets.rs}, lib.rs, SCHEMA.md, src/pages/Dockets/RenewalDashboard.tsx, src/lib/{ipc-types,tauri}.ts, src/App.tsx | cargo test: 113/113 (was 86), RLS gate: 10/10, pnpm build: PASS (474 modules, 506kb) |
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
