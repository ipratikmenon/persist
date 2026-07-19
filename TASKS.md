# TASKS.md — Persist Full Build Plan
# This is the canonical task list. PROGRESS.md tracks completion status.
# Use this file to understand what comes next and how sessions are structured.

---

## Session Workflow (Follow for Every Module)

Every module follows this five-step pattern. Never skip steps.

```
Step 1 — SPEC SESSION      (spec-writer agent or manual)
  Read the PRD section → write specs/module-XX.md
  Review and approve the spec before moving to Step 2.
  If anything is wrong, fix the spec — not the next session.

Step 2 — SCHEMA SESSION    (db-architect agent or keel-engineer)
  Read specs/module-XX.md → write migration SQL
  Run: cargo sqlx migrate run  → must pass
  Update: src-tauri/db/SCHEMA.md

Step 3 — KEEL SESSION      (keel-engineer agent)
  Read specs/module-XX.md + Step 2 migration
  Implement: commands/, services/ files
  Run: cargo test  → must pass before Step 4

Step 4 — DECK SESSION      (deck-engineer agent)
  Read specs/module-XX.md + Step 3 Keel commands
  Implement: pages/, components/ files
  Run: pnpm test  → must pass before Step 5

Step 5 — VALIDATION        (test-validator agent or manual)
  Review all output → write test report
  Fix all P0 bugs before moving to next module
  Update PROGRESS.md → mark module complete
```

---

## Phase 0 — Tauri v2 Scaffold (Weeks 1–2)

No modules — pure infrastructure. No spec files needed.

- [ ] Initialise Tauri v2 project with React 19 + TypeScript
- [ ] Configure `tauri.conf.json` (window, CSP, capabilities, auto-updater stub)
- [ ] Add sqlx to `Cargo.toml`; create initial migration (empty schema)
- [ ] Create full Keel directory structure (`commands/`, `services/`, `db/`, `storage/`, `config/`)
- [ ] Create full Deck directory structure (`pages/`, `components/`, `stores/`, `lib/`)
- [ ] Scaffold Zustand stores (matters, ui, chat, sync, mail)
- [ ] Create design system tokens (`src/design-system/tokens.ts`)
- [ ] Create animation tokens (`src/design-system/motion.ts`) — duration, ease, transition, useMotionPreference
- [ ] Create IPC types scaffold (`src/lib/ipc-types.ts`)
- [ ] Create typed Tauri wrappers scaffold (`src/lib/tauri.ts`)
- [ ] Create AI entry point (`src/lib/ai.ts` — single invoke call only)
- [ ] Create AI router stub (`src-tauri/services/ai_router.rs`)
- [ ] Create AI thresholds config (`src-tauri/config/ai_thresholds.rs`)
- [ ] Create vault scaffold (`src-tauri/storage/vault.rs` — AES-256 API stub)
- [ ] Create LaTeX service stub (`src-tauri/services/latex.rs`)
- [ ] Bundle TeX Live in Tauri build pipeline (sidecar configuration)
- [ ] GitHub Actions CI/CD: Rust tests + React tests + Tauri build (macOS + Windows)
- [ ] Hetzner VPS: provision server, deploy Axum scaffold + PostgreSQL
- [ ] Confirm: blank Persist Desktop window launches on macOS ✓
- [ ] Confirm: blank Persist Desktop window launches on Windows ✓

**Phase 0 complete when:** Desktop app launches blank on both platforms, CI green.

---

## Phase 1 — Foundation (Weeks 3–10)

### Module 1: Matter Management
- [ ] Step 1: `specs/module-01-matters.md`
- [ ] Step 2: `db/migrations/0002_matters.sql` (matters, matter_parties, clients tables)
- [ ] Step 2: `db/migrations/0002b_matter_comments.sql` (matter_comments, comment_mentions tables)
- [ ] Step 3: `commands/matters.rs` (get, create, update, close, archive, list, search)
- [ ] Step 3: `commands/clients.rs` (get, create, update, list)
- [ ] Step 3: `commands/discussion.rs` (post_comment, get_comments, mark_mention_read, resolve_comment)
- [ ] Step 3: `services/discussion_broadcaster.rs` (WebSocket real-time comment delivery)
- [ ] Step 4: `pages/Matters/MatterList.tsx`
- [ ] Step 4: `pages/Matters/MatterDetail.tsx` (tabs: Overview, Timeline, Documents, 💬 Discussion, Billing)
- [ ] Step 4: `components/matters/MatterStatusBadge.tsx`
- [ ] Step 4: `components/matters/MatterTimeline.tsx`
- [ ] Step 4: `components/discussion/DiscussionThread.tsx` (comment list, grouped by day)
- [ ] Step 4: `components/discussion/CommentComposer.tsx` (Persist Editor + voice + attach + @mention)
- [ ] Step 4: `components/discussion/CommentCard.tsx` (avatar, body, voice chip, attachment chips)
- [ ] Step 5: Validation pass

### Module 2: Docketing & Deadline Engine
- [ ] Step 1: `specs/module-02-docketing.md`
- [ ] Step 2: `db/migrations/0003_deadlines.sql` (deadlines, ip_assets tables)
- [ ] Step 3: `commands/deadlines.rs` (CRUD, urgency calc, generate_from_template, mark_complete)
- [ ] Step 3: IP statutory templates: TM, Patent, Design, Copyright, PCT, Paris Convention
- [ ] Step 3: `services/deadline_watcher.rs` (background urgency escalation + OS notifications)
- [ ] Step 4: `pages/Dockets/DocketList.tsx` (full row urgency colouring — DocketTrak pattern)
- [ ] Step 4: `pages/Dockets/IPAssetRecord.tsx` (two-panel: General + Chronology + Events)
- [ ] Step 4: `pages/Dockets/PipelineBoard.tsx` (Kanban prosecution pipeline — Triangle IP pattern)
- [ ] Step 4: `pages/Dockets/RenewalDashboard.tsx` (renewal + annuity management)
- [ ] Step 5: Validation pass

### Module 3: Document Management
- [ ] Step 1: `specs/module-03-documents.md`
- [ ] Step 2: `db/migrations/0004_documents.sql`
- [ ] Step 3: `commands/documents.rs` (upload, version, categorise, share, clean_metadata)
- [ ] Step 3: `storage/vault.rs` — AES-256 encrypt/decrypt implementation (replace stub)
- [ ] Step 4: `pages/Documents/DocumentStore.tsx`
- [ ] Step 4: `pages/Documents/DocumentViewer.tsx` (PDF + DOCX preview)
- [ ] Step 5: Validation pass

### Auth + RBAC
- [ ] Step 1: `specs/auth-rbac.md`
- [ ] Step 2: `db/migrations/0005_auth.sql` (users, roles, permissions, sessions)
- [ ] Step 3: `commands/auth.rs` (login, logout, session, RBAC enforcement)
- [ ] Step 4: Login screen + auth flow
- [ ] Step 4: Role-gated UI (partner vs associate vs paralegal views)
- [ ] Step 5: Validation pass

**Phase 1 complete when:** Attorneys can create matters, add deadlines, upload documents, and log in with RBAC.

---

## Phase 2 — Billing & Client Portal (Weeks 11–18)

### Module 4: Time Tracking & Billing
- [ ] Step 1: `specs/module-04-billing.md`
- [ ] Step 2: `db/migrations/0006_billing.sql` (time_entries, invoices, invoice_line_items, payments)
- [ ] Step 3: `commands/billing.rs` (time entry CRUD, invoice generation, GST calculation, PDF via LaTeX)
- [ ] Step 3: LaTeX invoice template (`storage/templates/invoice.tex`)
- [ ] Step 3: `services/latex.rs` — replace stub with real subprocess implementation
- [ ] Step 4: `pages/Billing/TimeTracker.tsx` (manual entry + in-app timer)
- [ ] Step 4: `pages/Billing/InvoiceList.tsx`
- [ ] Step 4: `pages/Billing/InvoiceDetail.tsx`
- [ ] Step 5: Validation pass

### Module 5: Client Portal (React + FastAPI)
- [ ] Step 1: `specs/module-05-portal.md`
- [ ] Step 2: PostgreSQL schema for mirror tables (matters_public, documents_shared, invoices, etc.)
- [ ] Step 3: `commands/sync.rs` — desktop SQLite → PostgreSQL sync implementation
- [ ] Step 3: Sync server Axum routes and WebSocket handlers (`server/`)
- [ ] Portal backend: FastAPI app, OTP auth, JWT, all API endpoints (`portal/backend/`)
- [ ] Portal frontend: all 4 tabs — Matters, Documents, Invoices, Profile (`portal/frontend/`)
- [ ] Step 5: Validation pass

**Phase 2 complete when:** Client can log in to the portal, see their matters and invoices, and download documents.

---

## Phase 3 — Intelligence Layer (Weeks 19–26)

### Module 22: AI Router (make production-ready)
- [ ] Replace `ai_router.rs` stub with full implementation
- [ ] All three tiers: Haiku, Sonnet, Opus
- [ ] Context size gate (Haiku ceiling at 180K tokens)
- [ ] Sonnet confidence escalation loop
- [ ] Prompt caching (system prompt + matter context + statute corpus)
- [ ] `ai_thresholds.rs` all constants populated
- [ ] End-to-end test: `invoke('ai_request', ...)` from Deck → correct model selected

### Module 24: HPAS — Hierarchical Parallel Agent System

HPAS is introduced in three sub-phases within Phase 3. Each sub-phase is triggered by the first Persist feature that needs that capability. Ref: `HPAS_PRD_Standalone.md` (specification), `HPAS_Persists_Integration.md` (Persist binding).

#### Phase 3A — Dispatch Foundation (alongside first AI feature)
- [ ] Step 1: `specs/module-24-hpas.md` — write spec from HPAS standalone + integration docs
- [ ] Step 2: `db/migrations/XXXX_hpas.sql` (hpas_workflows, hpas_sessions, hpas_registry tables)
- [ ] Step 2: Update `SCHEMA.md` with HPAS tables
- [ ] Step 3: `hpas/mod.rs` — `Job`, `StructuredResult`, `Priority`, `JobStatus` types, `dispatch()` router
- [ ] Step 3: `hpas/store.rs` — `WorkflowStore` trait + `SqliteStore` implementation
- [ ] Step 3: `hpas/agent.rs` — `SubAgent` (single `tokio::spawn`, Anthropic API call, typed result)
- [ ] Step 3: `hpas/compressor.rs` — `SemanticCompressor` (pure function, schema enforcement + token ceiling)
- [ ] Step 3: `commands.rs` — `hpas_dispatch` Tauri command wired to `hpas::dispatch()`
- [ ] Step 4: `hooks/useHpas.ts` — React hook (`dispatch` + `dispatchBatch` via Tauri invoke/listen)
- [ ] Step 4: Wire first real Persist feature through HPAS end-to-end
- [ ] Step 4: Verify React receives schema-conforming typed data with no null checks
- [ ] Step 5: `cargo test` — SubAgent dispatch + SemanticCompressor boundary tests
- [ ] Step 5: Validation pass — first feature works via HPAS

#### Phase 3B — Parallel Dispatch (when first multi-fetch feature needed)
- [ ] Step 3: `hpas/agent.rs` — extend with `FuturesUnordered` parallel dispatch
- [ ] Step 3: `hpas/session.rs` — `SessionAgent` with SQLite-persisted conversation history
- [ ] Step 3: `hpas/compressor_ctx.rs` — `ContextCompressor` (periodic history distillation)
- [ ] Step 5: Write 3-agent parallel dispatch test
- [ ] Step 5: Verify Parent delta activation cost <= 1,500 tokens
- [ ] Step 5: Verify parallel wall-time <= slowest agent (not sum of agents)
- [ ] Step 5: Validation pass

#### Phase 3C — Communication Layer (when cross-domain coordination needed)
- [ ] Step 3: `hpas/registry.rs` — `RegistryAgent` (SQLite table, serialised writes, conflict policy)
- [ ] Step 3: `hpas/bus.rs` — `MessageBus` (Tauri events + SQLite seq_id counter, dedup only at L1)
- [ ] Step 3: `hpas/bus.rs` — `OrchestratorBus` (separate Tauri channel, strict seq_id ordering at L0)
- [ ] Step 3: `hpas/memory.rs` — `VectorMemory` (sqlite-vss, top-k retrieval, namespace scoping)
- [ ] Step 3: Bundle sqlite-vss extension in `Contents/Resources/`
- [ ] Step 5: Write 2-Parent lateral communication test (MessageBus publish → subscribe → delta)
- [ ] Step 5: Verify MessageBus latency < 5ms (should be ~0.01ms via Tauri events)
- [ ] Step 5: Verify RegistryAgent write ordering under concurrent agents
- [ ] Step 5: Validation pass

### Module 7: AI Drafting Assistant
- [ ] Step 1: `specs/module-07-ai-drafting.md`
- [ ] Step 3: `commands/ai_drafting.rs` (draft_document, draft_reply, draft_from_template)
- [ ] Step 3: `services/context_builder.rs` — assembles matter context for AI requests
- [ ] Step 4: `pages/Drafting/AIDraftingPanel.tsx` (intake form + AI output + edit + approve)
- [ ] Step 5: Validation pass

### Module 21: Persist Chat (Hummingbird)
- [ ] Step 1: `specs/module-21-persist-chat.md`
- [ ] Step 3: `commands/chat.rs` (chat_message, get_chat_history, clear_session)
- [ ] Step 4: `components/chat/PersistChatOverlay.tsx` (Cmd+/ floating overlay)
- [ ] Step 4: `components/chat/ChatThread.tsx` (message list, action chips, AI indicators)
- [ ] Step 4: `components/chat/ChatInput.tsx` (Deep Analysis toggle, send)
- [ ] Step 5: Validation pass

### Module 8: Reports & Analytics
- [ ] Step 1: `specs/module-08-analytics.md`
- [ ] Step 4: `pages/Analytics/PortfolioAnalytics.tsx` (three-panel: filter + bar chart + table)
- [ ] Step 4: `pages/Analytics/DocketCompliance.tsx`
- [ ] Step 4: `pages/Analytics/FinancialReports.tsx`
- [ ] Step 4: `pages/Analytics/PartnerDashboard.tsx` (widget cards)
- [ ] Step 5: Validation pass

### Module 6: Notifications
- [ ] WhatsApp integration (Meta Cloud API) in Keel
- [ ] Calendar sync (Google Calendar API + Microsoft Graph) in Keel
- [ ] M365 Outlook mail sync stub (full mail module in Phase 4)

**Phase 3 complete when:** Attorneys can use Persist Chat, generate AI drafts, see the analytics dashboard, and all AI calls route through HPAS dispatch.

---

## Phase 4 — Drafting Suite (Weeks 27–38)

### Module 24 (cont.): HPAS — Full Hierarchy + ApprovalGate
- [ ] Step 3: `hpas/approval.rs` — `ApprovalGate` (Tauri notification + SQLite pending state + deadline timer)
- [ ] Step 3: Wire full GrandParent → Parent → Agent graph for a real Persist job (e.g. contract review pipeline)
- [ ] Step 3: `hpas/agent.rs` — Batch API dispatch (`batchMode: true`) + Tauri local HTTP webhook handler
- [ ] Step 4: (Optional) `components/AgentGraph.tsx` — real-time agent topology visualisation
- [ ] Step 5: Run N=20 parallel agents, measure actual vs modelled token costs
- [ ] Step 5: Verify token efficiency targets: >= 8x vs single-context at N=50
- [ ] Step 5: Validation pass

### Module 15A: Persist Editor (ProseMirror)
- [ ] Step 1: `specs/module-15a-editor.md`
- [ ] Step 4: `components/editor/PersistEditor.tsx` — ProseMirror core
- [ ] Step 4: `components/editor/plugins/markdown.ts` — live Markdown rendering
- [ ] Step 4: `components/editor/plugins/smart-tags.ts` — @ and # tag system
- [ ] Step 4: `components/editor/plugins/expansions.ts` — /shorthand system
- [ ] Step 3: `db/migrations/XXXX_expansions.sql` (expansions table)
- [ ] Step 3: `commands/editor.rs` (get_expansions, upsert_expansion, resolve_smart_tag)
- [ ] Step 5: Validation pass

### Module 9: Document Drafting Suite
- [ ] Step 1: `specs/module-09-drafting.md`
- [ ] Steps 2–5: Template engine (9.1), AI proofreading (9.2), comparison (9.3),
      metadata clean (9.4), precedent library (9.6), Smart Form Compiler (9.8)

### Module 15: Integrated Mail Module
- [ ] Step 1: `specs/module-15-mail.md`
- [ ] Steps 2–5: Mail sync, bundles, Focused tab, client tabs, all Shortwave patterns (15.15)

### Module 16: Advanced PDF Engine
- [ ] Step 1: `specs/module-16-pdf.md`
- [ ] Steps 2–5: OCR, markup toolkit, Bates stamping, Smart Text, shared sessions

**Phase 4 complete when:** Full drafting lifecycle works end-to-end. Mail is live.

---

## Phase 5 — Contract Intelligence (Weeks 39–54)

### Module 24 (cont.): HPAS — Cloud Scale Preparation (when multi-user/distributed)
- [ ] Add `restate-sdk-rust` Cargo dependency
- [ ] `hpas/store.rs` — implement `RestateStore` behind existing `WorkflowStore` trait
- [ ] Feature-flag: `HPAS_BACKEND=restate` at compile time
- [ ] Verify all existing HPAS tests pass with Restate backend
- [ ] Document cloud deployment requirements (Restate server, Postgres for Registry + VectorMemory at scale)
- [ ] Remove SQLite dependency from cloud build profile; keep for local `.app`

### Module 10: Contract Intelligence Engine
- [ ] Step 1: `specs/module-10-contracts.md`
- [ ] Steps 2–5: Smart field extraction (10.1), review workspace (10.2),
      analysis chart (10.3), AI chat with contracts (10.4), diligence reports (10.5),
      contract portfolio (10.6)

---

## Phase 6 — Daily Intelligence (Weeks 55–62)

### Module 11: Daily Intelligence & Workday Organiser
- [ ] Step 1: `specs/module-11-daily.md`
- [ ] Steps 2–5: Daily Brief (11.1), Smart Tasks (11.2), Meeting Intelligence (11.3),
      Routines (11.4), Focus Mode (11.5)

### Module 17: Legal Reference Manager
### Module 18: Legal Intelligence Layer

---

## Phase 7 — Post-Launch

### Module 12: Client iOS/iPadOS App
### E-signature integration
### Court cause list auto-import → absorbed by Module 26 (Phase 8)

---

## Phase 2.5 — Data Protection Audit Engine (pulled forward — see specs/expansion-roadmap.md §6)

### Module 31: Data Protection Audit Engine (DPDP 2023 / GDPR)
- [ ] Step 1: `specs/module-31-dp-audit.md`
- [ ] Step 2: server PostgreSQL migration — dp_audits, dp_questionnaires, dp_data_inventory (RoPA), dp_gaps, dp_evidence, dp_remediations
- [ ] Step 3: audit workflow API (server) + framework libraries (DPDP 2023, GDPR) as versioned data
- [ ] Step 3: LaTeX audit report template (`dp_audit_report.tex`) — compiled locally via latex.rs
- [ ] Step 3: breach-notification (72h) workflow checklist; consent-notice + privacy-policy generators (attorney-approved)
- [ ] Step 4: attorney-side audit UI (Deck `pages/Advisory/`); client-side questionnaire (portal)
- [ ] Step 5: validation + first real audit dry-run
- Prerequisite: Phase 2 M5 portal infra. AI gap-analysis pass upgrades in Phase 3.

---

## Phase 8 — Courtroom & Litigation Intelligence (Track A — adalat.ai-inspired)
See `specs/expansion-roadmap.md` §3. Build order: M27 → M26 → M25 → M28 → M29.
Prerequisites: Phase 3 AI layer; B02 ip_assets + cascade_engine; B01 sessions.

### Module 27: Document Digitization & Intake Intelligence
- [ ] Step 1: `specs/module-27-doc-intelligence.md`
- [ ] Steps 2–5: LLM-OCR pipeline (`services/doc_intelligence.rs`), structured extraction → document_intake_events, auto-classification, intake review queue UI (AI proposes, attorney confirms — dual verification)

### Module 26: Cause List & Hearing Flow
- [ ] Step 1: `specs/module-26-hearings.md`
- [ ] Steps 2–5: `hearings` + `cause_list_imports` tables, `services/cause_list_watcher.rs` (Delhi HC / district courts / IP offices), HearingBoard.tsx, outcome capture → cascade_engine next-deadline chain

### Module 25: Hearing & Dictation Transcription
- [ ] Step 1: `specs/module-25-transcription.md` (must include recording-compliance note)
- [ ] Steps 2–5: `services/transcription.rs` async STT jobs (local whisper.cpp sidecar default; cloud opt-in), English + Hindi launch, legal vocabulary boost, vault-encrypted audio + transcripts, recorder + transcript viewer UI
- [ ] Steps 2–5: **Transcript mind map** (Pocket-inspired) — auto-generated per transcript: Central Theme (matter/issue) → Branches (topics discussed) → Nodes (facts, instructions, admissions), every node **linked back to its transcript timestamp** (tap node → jump to that moment in the recording). Structured-output pass via HPAS; Deck renderer (`components/transcripts/TranscriptMindMap.tsx`); Copy-Markdown export

### Module 28: Research & Summarization (delivers M17 + M18 AI surface)
- [ ] Step 1: `specs/module-28-research.md`
- [ ] Steps 2–5: judgment/order summarization via HPAS (Opus tier), hearing-prep briefs, citation extraction, summary panel + brief UI
- [ ] Steps 2–5: **Document mind map** — same renderer as M25, applied to judgments/orders/long documents: Central Theme → issues/holdings → nodes (ratio, obiter, citations), each node linked to the source paragraph. Feeds the hearing-prep brief (the brief IS the map). Shared `TranscriptMindMap.tsx` component generalized to `MindMap.tsx`

### Module 29: Client Status Chatbot (WhatsApp, multilingual)
- [ ] Step 1: `specs/module-29-chatbot.md`
- [ ] Steps 2–5: Meta Cloud API webhook (server), intent handling answered ONLY from PostgreSQL mirror subset, English + Hindi, attorney-escalation to M11 tasks, server-side AI router (§5.2)

---

## Phase 9 — Persist Advisory: Startup Legal SaaS (Track B — visiocyber.ai-inspired)
See `specs/expansion-roadmap.md` §4–5. M31 already live from Phase 2.5.
Prerequisites: Phase 4 drafting/template machinery; server-side AI router.

### Module 30: Startup Legal OS (SaaS, multi-tenant)
- [ ] Step 1: `specs/module-30-startup-os.md` (must ratify §5.1 bifurcated source of truth + tenant model)
- [ ] Step 2: `tenants` + `tenant_users` (PostgreSQL RLS), subscription tables (`recurring_plans` extends M4)
- [ ] Steps 3–5: onboarding wizard + startup compliance calendar (cascade-template library), document generator (founder/ESOP/IP-assignment/NDA/DPA/privacy-policy — AI fill + attorney review queue → LaTeX PDF), scoped AI legal Q&A with "Ask an attorney" escalation, TM knock-out intake → firm matter pipeline, attorney console (`pages/Advisory/`)

### Module 32: Compliance & AI Governance
- [ ] Step 1: `specs/module-32-compliance.md`
- [ ] Steps 2–5: framework libraries as data (DPDP 2023, SOC 2, ISO 27001, CERT-In 2022, responsible-AI set), control mapping + evidence collection with deadline-engine expiry, posture dashboard, responsible-AI policy generator, vendor/tech evaluation scorecards

### Module 33: Assessment & Advisory Toolkit
- [ ] Step 1: `specs/module-33-assessments.md`
- [ ] Steps 2–5: cyber/AI-readiness scored assessments (LaTeX report card), recommendations → M32 remediation backlog, advisory engagement tracker (workshops, materials, follow-ups, M4 invoicing)

---

## Track C — Candidate Bets (differentiators — NOT yet sequenced)

> These are opinion-driven, high-differentiation ideas beyond the committed
> Phase 0–9 plan. **Nothing here is scheduled.** Each is a *candidate* to pull
> into a phase once it proves out. They are recorded with module numbers so the
> conversation has a stable vocabulary. Do NOT start any of these without an
> explicit decision to promote it into a phase (and a Step-1 spec first).
> Rationale for each is in `specs/expansion-roadmap.md` §8 (Track C).
>
> **Top three bets** (my recommendation, in order): **M35 Limitation Engine**,
> **M36 Order Watcher**, **M38 Firm Brain**. These have the strongest
> daily-pain + moat + export story.

### Module 34: Matter Mind Map — Case Theory Canvas (Pocket-inspired, matter-scale)
- [ ] Step 1: `specs/module-34-matter-mindmap.md`
- [ ] Concept: per-*matter* graph (not per-conversation). Center = the mark/patent/dispute;
      branches = parties, claims/objections, evidence, hearings, deadlines; every node
      linked to its underlying record (vault document, docket entry, transcript segment,
      order). The graph is largely a SQLite query + renderer since all objects already
      co-exist in one DB. Litigator "case theory on a canvas" — CaseMap-class, none good in India.
- Depends on: M25/M28 MindMap renderer, M26 hearings, M27 intake, existing docket/document tables.

### Module 35: Limitation Engine — Deterministic Deadline Law  ⭐ top bet
- [ ] Step 1: `specs/module-35-limitation-engine.md`
- [ ] Concept: Limitation Act rules-as-data (Ss. 4–14 exclusions, condonation windows) +
      versioned per-court holiday/vacation calendars (Delhi HC first). Deterministic answer
      to "file by when, in which court, accounting for vacations" with the statutory chain
      shown. AI only *explains* the result — it never computes it. Export play: swap in a
      Madrid Protocol / PCT / UKIPO pack → global docketing product (jurisdictions = data).
- Extends: module-02 docketing engine + cascade_engine.

### Module 36: Order Watcher — eCourts / IP India / TM Journal Robot  ⭐ top bet
- [ ] Step 1: `specs/module-36-order-watcher.md`
- [ ] Concept: scheduled watcher polling eCourts, High Court sites, and IP India for the
      firm's own matters; diffs the record; summarizes new orders overnight; proposes the
      next docket entry by morning. **TM Journal watch** — scan every journal issue for
      marks confusingly similar to clients' portfolios — is a standalone sellable
      subscription and a natural three-tier-router similarity workload.
- Extends: M26 cause_list_watcher pattern, M28 summarization, deadline engine.

### Module 37: Court-Rules Compiler — Self-Formatting E-Filing
- [ ] Step 1: `specs/module-37-filing-compiler.md`
- [ ] Concept: per-court formatting rules as data (index style, pagination, bookmarking,
      paper size, court-fee computation) → brief in, filing-ready rule-compliant bookmarked
      PDF out, per target court. Kills e-filing formatting rejections. Built on the existing
      LaTeX pipeline (latex.rs) — a substrate almost no competitor has.

### Module 38: Firm Brain — Precedent Memory with Provenance  ⭐ top bet
- [ ] Step 1: `specs/module-38-firm-brain.md`
- [ ] Concept: private retrieval over every draft/opinion/order the firm has produced.
      Differentiator is NOT RAG — it's **clause provenance**: every AI-suggested clause
      traces to a real firm precedent with its outcome. Explainable drafting converts
      skeptical senior counsel. Compounding moat: value grows with each year of firm data.
- Builds on: HPAS VectorMemory; complements M9 drafting suite + M30 doc generator.

### Module 39: Bench Analytics — Judge/Forum Insight (India-first)
- [ ] Step 1: `specs/module-39-bench-analytics.md`
- [ ] Concept: Lex-Machina-style analytics over public data (Indian Kanoon, eCourts) —
      interim-injunction tendencies in TM matters, median adjournments, time-to-disposal.
      **Ethics framing required in spec**: insight, not forum-shopping. Genuine first mover.

### Module 40: Client-Held Privilege Keys — Provable Confidentiality
- [ ] Step 1: `specs/module-40-privilege-keys.md`
- [ ] Concept: client-shared documents sealed with keys the *client* holds + a verifiable
      privilege log. "Your lawyer's software provably cannot leak your documents." A trust
      feature, not an AI feature — and the one that travels abroad best (universal AI-tool
      privilege anxiety). Interacts with portal signed-URL model (module-05) — resolve
      key-custody vs. attorney-access carefully in spec.

### Module 41: Vernacular Voice Intake — WhatsApp Voice Note → Matter Brief
- [ ] Step 1: `specs/module-41-voice-intake.md`
- [ ] Concept: clients already send Hindi/regional voice notes. Turn one into a structured
      matter brief (parties, dates, grievance, urgency). Closes the loop M25 (transcription)
      + M29 (chatbot) only skirt. India-first; portable to any multilingual market.
- Reuses: M25 STT + M27 structured extraction + M29 WhatsApp channel.

---

## Notes on Using Agent Teams (Phase 3+)

For modules where Keel and Deck are clearly independent after the spec is agreed:

```bash
export CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1

# Example: build Module 15 Mail in parallel
"Build Module 15 Mail.
 Teammate 1 (keel-engineer): src-tauri/commands/mail.rs + src-tauri/services/mail_sync.rs
 Teammate 2 (deck-engineer): src/pages/Mail/ + src/components/mail/
 Both work from specs/module-15-mail.md.
 The interface is defined in src/lib/ipc-types.ts — do not change it without coordination."
```

Merge when both teammates have passing tests.
All Agent Teams sessions currently require Opus 4.6.
