# Persist Expansion Roadmap — Litigation Intelligence & Startup Legal SaaS
# Status: PLAN — approved module numbering, pre-spec
# Written: 2026-07-19 (S09)
# Each module below still requires its own specs/module-XX.md before implementation
# (per the 5-step workflow in TASKS.md). This document is the strategic contract.

---

## 1. Purpose

Persist today is an internal practice-management platform for Persistas & Partners
(matters, docketing, documents, billing, portal). This roadmap adds two new product
tracks **on top of** — never replacing — the existing Phase 0–7 plan:

- **Track A — Courtroom & Litigation Intelligence** (inspired by adalat.ai):
  hearing transcription, cause-list-driven case flow, LLM-powered document
  digitization, judgment summarization, and a multilingual client status chatbot.

- **Track B — Persist Advisory: Startup Legal SaaS + Data Protection**
  (inspired by visiocyber.ai): a productized, subscription SaaS offering for new
  startups — AI-assisted legal operations with attorney review — plus a Data
  Protection (DPDP Act 2023 / GDPR) audit engine, compliance & AI-governance
  tooling, and cyber/AI-readiness assessment toolkits the firm can sell as
  recurring engagements.

Feature-source mapping:

| Source product feature | Persist module |
|---|---|
| adalat.ai Real-Time Transcription (15+ Indian languages, legal vocab) | M25 Hearing & Dictation Transcription |
| adalat.ai Live Case Flow Management | M26 Cause List & Hearing Flow |
| adalat.ai LLM-Based Scanning / Legal Document Intelligence | M27 Document Digitization & Intake Intelligence |
| adalat.ai AI-Powered Legal Summarization / Judicial Research | M28 Research & Summarization (delivers M17/M18) |
| adalat.ai AI Chatbots for Litigants (WhatsApp, regional languages) | M29 Client Status Chatbot |
| visiocyber.ai "AI legal for startups" positioning | M30 Startup Legal OS (SaaS) |
| visiocyber.ai data protection / privacy compliance (GDPR/CCPA → DPDP for India) | M31 Data Protection Audit Engine |
| visiocyber.ai Compliance & Governance automation (SOC 2, ISO 27001, responsible-AI policy) | M32 Compliance & AI Governance |
| visiocyber.ai Cybersecurity Assessment / Technology Evaluation / Executive Education | M33 Assessment & Advisory Toolkit |

---

## 2. Strategic Constraints (carry over from CLAUDE.md — non-negotiable)

1. All AI calls route through Keel (`ai_router.rs` / HPAS). Deck never calls
   Anthropic. New server-side surfaces get their own router (see §5.2) — SaaS
   tenants never call Anthropic directly either.
2. Desktop SQLite remains the source of truth **for firm practice data**. SaaS
   tenant data is a new, deliberate exception (see §5.1).
3. Client privilege wall: privileged firm-matter data and SaaS tenant data never
   mix. Nothing beyond the existing public-subset tables syncs outward.
4. Every AI-generated legal output delivered to a client or tenant is a DRAFT
   until an attorney approves it (reuse HPAS ValidationGate). Lawyer-in-the-loop
   is a product feature, not a disclaimer.

---

## 3. Track A — Courtroom & Litigation Intelligence (Phase 8, Modules 25–29)

Adalat.ai is court-side software (judges, court staff). Persist is firm-side, so
each feature is adapted to the attorney's seat at the bar table.

### Module 25 — Hearing & Dictation Transcription
Firm-side speech-to-text: attorney dictation, client meetings, counsel
conferences, and transcription of official hearing recordings.

- **Keel**: `services/transcription.rs` — async job queue (tokio, same pattern as
  latex.rs), pluggable STT backend: local whisper.cpp sidecar (privileged
  audio never leaves the machine — default) or cloud STT (opt-in per recording).
  Legal vocabulary boost list (Indian statutes, IP terms, party names from the
  linked matter).
- **Languages**: English + Hindi at launch; regional languages follow the STT
  backend's coverage.
- **Storage**: audio + transcript both AES-256 in the vault; transcript indexed
  into `documents` with `category = 'transcript'`, linked to `matter_id` and
  optionally a hearing row (M26).
- **Deck**: `pages/Transcripts/` — recorder widget (mic capture via Tauri),
  job status list, transcript viewer with speaker labels + inline correction.
- **Guardrail**: in-court recording by parties is restricted in most Indian
  courts. The recorder UI carries a context flag (`dictation | client_meeting |
  official_recording_import`) and the spec must include the compliance note.
  We transcribe what the firm may lawfully record or already lawfully holds.

### Module 26 — Cause List & Hearing Flow
The firm-side twin of adalat.ai's case-flow dashboard, built on the existing
docketing engine.

- **Schema**: `hearings` table (matter_id, court, bench, item_no, hearing_date,
  purpose, outcome, next_date, order_document_id) + `cause_list_imports`.
- **Keel**: `services/cause_list_watcher.rs` — scheduled fetch/parse of Delhi HC
  and district-court cause lists + IP office hearing boards (absorbs the Phase 7
  "Court cause list auto-import" line item); matching by case number to matters;
  outcome capture cascades the next deadline chain via `cascade_engine.rs`
  (module-02 spec §2.9 — hard dependency, see §6).
- **Deck**: `pages/Hearings/HearingBoard.tsx` — "today at a glance": all
  listed matters, item numbers, bench, files-ready checklist; post-hearing
  outcome quick-entry that sets next date + triggers cascade.

### Module 27 — Document Digitization & Intake Intelligence
LLM-powered OCR replacing manual data entry — adalat.ai's scanning/document
intelligence, pointed at firm intake.

- **Keel**: `services/doc_intelligence.rs` — PDF/image → text extraction →
  structured extraction pass through ai_router (dates, parties, application
  numbers, deadlines mentioned); auto-classification into matter + category;
  extracted dates feed `document_intake_events` and propose docket entries
  (dual-verification rules from module-02 spec §2.12 apply — AI proposes,
  attorney confirms).
- **Deck**: intake review queue — side-by-side original vs extracted fields,
  one-click accept into docketing.
- Extends module-03; does not replace the vault or `clean_metadata()` rules.

### Module 28 — Research & Summarization
This module IS the delivery vehicle for the already-planned M17 (Legal Reference
Manager) and M18 (Legal Intelligence Layer) — upgraded to adalat.ai-grade
summarization. No duplicate module; M28 subsumes their AI surface.

- **Keel/HPAS**: judgment + order summarization (Opus tier via ai_router),
  matter-timeline synthesis ("brief me for tomorrow's hearing" — pulls M26
  hearing history + M27 extracted documents + deadlines), citation extraction.
- **Deck**: summary panel on document viewer; hearing-prep brief on HearingBoard.
- Depends on Phase 3 (ai_router production + HPAS 3A/3B).

### Module 29 — Client Status Chatbot (WhatsApp, multilingual)
Adalat.ai's litigant chatbot, adapted to firm clients. Extends the Phase 3
"WhatsApp notifications (Meta Cloud API)" item from one-way to two-way.

- **Server** (`server/`): webhook receiver for Meta Cloud API; intent handling
  (case status, next hearing, invoice status, document received?) answered
  **only** from the PostgreSQL mirror subset (matters_public, deadlines_public,
  invoices, documents_shared). The bot physically cannot leak privileged data
  because privileged data never syncs.
- **Languages**: English + Hindi at launch; template-based regional responses
  after. Translation runs server-side through the server AI router (§5.2).
- **Escalation**: anything the bot can't answer opens a task in the attorney's
  daily organiser (M11) — never a hallucinated legal answer.

---

## 4. Track B — Persist Advisory: Startup Legal SaaS + Data Protection (Phase 9, Modules 30–33)

Visiocyber.ai sells cybersecurity/compliance services into law firms. Persist
inverts this: the **firm** sells AI-accelerated legal + data-protection services
to **startups**, productized as SaaS. Track B is a revenue product, not internal
tooling.

### Module 31 — Data Protection Audit Engine  *(build FIRST in Track B — see §6)*
DPDP Act 2023 + GDPR audit tooling. Sellable immediately as a firm-delivered
engagement; later self-serve inside M30.

- **Core objects** (server-native PostgreSQL, per §5.1): `dp_audits`,
  `dp_questionnaires`, `dp_data_inventory` (RoPA: what personal data, where,
  why, lawful basis, retention), `dp_gaps`, `dp_evidence`, `dp_remediations`.
- **Flow**: questionnaire (framework-versioned: DPDP 2023, GDPR, both) →
  data-flow inventory builder → AI-assisted gap analysis against framework
  controls (ai_router; attorney reviews every finding — ValidationGate) →
  remediation tracker → **LaTeX audit report** (reuse latex.rs + a
  `dp_audit_report.tex` template — same pipeline as invoices, compiled on the
  firm desktop, never server-side).
- **Extras**: 72-hour breach-notification workflow checklist; consent-notice and
  privacy-policy generators (template + AI draft + attorney approval); annual
  re-audit scheduling via the deadline engine.
- **Billing**: audits invoice through the existing M4 billing module.

### Module 30 — Startup Legal OS (SaaS, multi-tenant)
The flagship: "a law firm in your browser" for new Indian startups, with
Persistas & Partners attorneys in the loop.

- **Tenant model**: new `tenants` + `tenant_users` in server PostgreSQL with
  row-level security (same discipline as the client portal — RLS at the
  database, not just the ORM). A tenant may later convert to a full firm client
  (matter opened in desktop Persist).
- **Feature set (MVP)**:
  1. Onboarding wizard → company profile → auto-generated **compliance
     calendar** (ROC filings, GST, labour, DPDP obligations) powered by a
     startup-flavoured cascade-template library (reuses the cascade engine
     concept from module-02).
  2. **Document generator**: founder agreements, ESOP pool docs, IP assignment,
     NDAs, DPAs, privacy policies, employment offers — template + AI fill +
     attorney review queue → signed-off PDF (LaTeX pipeline).
  3. **AI legal Q&A**: scoped assistant answering from the tenant's own
     documents + Indian startup-law knowledge; every answer labeled; anything
     consequential routes to "Ask an attorney" (paid escalation).
  4. **IP starter**: trademark knock-out search request + filing intake that
     lands directly in the firm's matter pipeline (Persist desktop).
  5. **Data protection self-serve**: M31 questionnaire lite as an upsell funnel.
- **Attorney console** (Deck): `pages/Advisory/` — review queues, tenant list,
  escalations, SLA timers. Attorneys work in the desktop app; tenants live in
  the web portal codebase (`portal/` grows a `startup/` app or a second Vite
  target — decided in the module spec).
- **Billing**: subscription tiers (recurring invoices — extends M4 with
  `recurring_plans` + auto-generated GST invoices; SAC code for legal services
  vs SaaS to be settled in spec with the accountants).

### Module 32 — Compliance & AI Governance
Visiocyber's compliance-automation offer, as a firm service tool + M30 add-on.

- Framework libraries as data: DPDP 2023, SOC 2, ISO 27001, CERT-In directions
  (2022), and an AI-governance control set (responsible-AI policy areas).
- Control mapping ("one evidence, many frameworks"), evidence collection with
  expiry/refresh dates driven by the deadline engine, posture dashboard.
- **Responsible-AI policy generator**: interview → AI draft → attorney approval
  → versioned policy document per tenant.
- **Vendor / technology evaluation scorecards** (visiocyber "Technology
  Evaluation"): structured questionnaires for a tenant's SaaS vendors — DPDP
  processor obligations, data-residency, sub-processor chains.

### Module 33 — Assessment & Advisory Toolkit
The lightest module — productizes visiocyber's assessment/education services.

- Cyber/AI-readiness assessment: weighted questionnaire → scored report card
  (LaTeX) → recommendations backlog that feeds M32 remediation.
- Engagement tracker for advisory work (workshops, executive-education
  sessions) — scheduling, materials vault, follow-ups; invoiced via M4.

---

## 5. Architecture Decisions Required (to be ratified in module specs)

### 5.1 Bifurcated source of truth
Firm practice data: desktop SQLite remains canonical (unchanged).
SaaS tenant data (M30–M33 tenant-facing objects): **server-native PostgreSQL is
canonical** — a single desktop cannot be the write path for N self-serve
tenants. The desktop app becomes a *client* of the advisory API for Track B
(attorney console reads/writes via authenticated API, not via sync).
This is a scoped exception to the "one source of truth" rule and must be stated
in every Track B spec. The two datasets live in separate schemas; nothing joins
across the privilege wall.

### 5.2 Server-side AI router
`server/` gains `ai_router` (Rust port of the Keel three-tier logic —
Haiku/Sonnet/Opus, shared thresholds via a common crate if practical). All
portal/chatbot/tenant AI traffic flows through it with per-tenant token
metering (feeds subscription billing) and per-tenant rate limits. Tenants and
the portal frontend never see model names — same rule as Deck.

### 5.3 Lawyer-in-the-loop as product architecture
HPAS ValidationGate becomes the shared approval primitive: AI output targeting
a client/tenant enters `pending_review` and is only released by an attorney
action, which is logged (who, when, what changed). This is the ethical and
liability backbone of Track B and the M29 chatbot.

### 5.4 Recording compliance (M25)
Recording context flag + jurisdiction note in spec; no covert-recording
features; official recordings are imported, not captured.

---

## 6. Sequencing & Dependencies

Existing Phases 0–7 keep their order. New work interleaves at two points:

```
Phase 2 M5  (Client Portal)            ← unchanged, next major build
   ↓
Phase 2.5   M31 Data Protection Audit  ← pulled forward: needs only portal
            (firm-delivered mode)         infra + billing + LaTeX (all built);
                                          AI gap-analysis upgrades later in P3.
   ↓
Phase 3     AI Layer (M22 router, HPAS, M7, M21, M8, M6)   ← unchanged
   ↓
Phase 8     Track A: M27 → M26 → M25 → M28 → M29
            (order: digitization first — it feeds docketing immediately;
             transcription after STT backend selection; chatbot last —
             needs server AI router + WhatsApp infra from Phase 3)
   ↓
Phase 4–5   Drafting Suite + Contract Intelligence          ← unchanged
   ↓
Phase 9     Track B: M30 Startup Legal OS → M32 → M33
            (M30 needs the drafting/template machinery of Phase 4 for its
             document generator; M31 is already live by now and plugs in
             as the self-serve upsell)
   ↓
Phase 6–7   Daily Intelligence, iOS, e-sign                 ← unchanged
            (Phase 7 "cause list auto-import" line is absorbed by M26)
```

Hard prerequisites before any Track A/B module starts:
1. **Phase 2 M5 portal** — sync server, PostgreSQL, RLS, OTP auth (Track B foundation).
2. **B02 `ip_assets` + cascade_engine** (module-02 extended) — M26/M27 cascade
   integration depends on it.
3. **B01 persistent sessions** — attorney console and review queues need
   sessions that survive restarts.

### Suggested next-session order (unchanged near-term plan, now with a reason)
1. Phase 2 M5 spec + scaffold (portal) — unblocks everything in this roadmap.
2. Phase 1 M2 extended (ip_assets, cascade_engine, abandonment_watcher) — B02.
3. Auth B01 (sessions table) — small, high leverage.
4. `specs/module-31-dp-audit.md` — first Track B spec session.

---

## 7. What This Roadmap Does NOT Change

- No renaming, no restructure of Keel/Deck, no new frameworks.
- Existing Phase 0–7 module numbering and content stay as-is (M26 absorbs one
  Phase 7 line item; M28 delivers M17/M18 — noted in TASKS.md).
- Design system, privilege rules, migration rules, LaTeX-is-local rule: untouched.
- Nothing in this document is implemented yet. Each module gets its own spec
  session (Step 1) before any schema or code session.

---

## 8. Track C — Candidate Bets (differentiators, NOT sequenced)

These go beyond feature parity with adalat.ai/visiocyber.ai/Pocket — they are
where Persist could *lead* rather than match. **None is scheduled.** Each is a
candidate to promote into a phase after an explicit decision + a Step-1 spec.
Module numbers (M34–M41) are reserved in TASKS.md for a stable vocabulary only.

**Top three bets** (recommended order): M35 Limitation Engine, M36 Order
Watcher, M38 Firm Brain — strongest combination of daily pain relieved,
defensible moat, and clean export-to-abroad story.

### M34 — Matter Mind Map (Case Theory Canvas)
Pocket ships an auto mind map *per conversation* (Central Theme → Branches →
Nodes, tap-node-to-transcript, Copy-Markdown export). Two lifts for Persist:
1. **Transcript/document mind maps** — the direct analogue, already folded into
   M25 and M28 as line items (shared `MindMap.tsx` renderer, nodes linked to
   transcript timestamps / source paragraphs).
2. **Matter-scale map (M34)** — the bigger prize: a per-matter graph with the
   mark/patent/dispute at center; branches for parties, claims/objections,
   evidence, hearings, deadlines; every node linked to its real record. Because
   all those objects already live in one SQLite DB, the graph is largely a query
   plus a renderer. This is CaseMap-class litigation tooling with no strong
   Indian incumbent, and it pairs naturally with M38 (Firm Brain) and M28's
   hearing-prep briefs (the brief *is* the map).

### M35 — Limitation Engine ⭐
Deterministic deadline *law*, not just a tracker: Limitation Act exclusion rules
(Ss. 4–14), condonation windows, and versioned per-court holiday/vacation
calendars as data packs. Answers "file by when, in which court, accounting for
vacations" deterministically with the statutory chain shown; AI only explains,
never computes. Export play: Madrid/PCT/UKIPO packs turn it into a global
docketing product where jurisdictions are data, not code. Attorneys trust it
*because* it isn't an LLM guessing.

### M36 — Order Watcher (eCourts / IP India / TM Journal) ⭐
Indian orders and registry changes post online late and are checked manually.
A watcher that polls for the firm's matters, diffs the record, summarizes new
orders overnight, and proposes the next docket entry by morning would be the
single most-loved feature. **TM Journal watch** (scan every issue for marks
confusingly similar to clients' portfolios) is a standalone subscription and an
ideal three-tier-router similarity workload.

### M37 — Court-Rules Compiler (self-formatting e-filing)
Per-court formatting rules as data → brief in, filing-ready rule-compliant
bookmarked PDF (+ court-fee computation) out. Kills e-filing formatting
rejections; built on the LaTeX pipeline almost no competitor has.

### M38 — Firm Brain (precedent memory with provenance) ⭐
Private retrieval over every firm draft/opinion/order. Differentiator is not RAG
but **clause provenance**: every suggested clause traces to a real firm
precedent with its outcome. Explainable drafting converts skeptical senior
counsel; the moat compounds with each year of firm data. Builds on HPAS
VectorMemory; complements M9 and M30's document generator.

### M39 — Bench Analytics (India-first)
Lex-Machina-style insight over public data (Indian Kanoon, eCourts): interim-
injunction tendencies in TM matters, median adjournments, time-to-disposal.
Spec must frame this as insight, not forum-shopping. Genuine first mover.

### M40 — Client-Held Privilege Keys
Client-shared documents sealed with keys the *client* holds + a verifiable
privilege log: "your lawyer's software provably cannot leak your documents." A
trust feature that travels abroad best. Resolve key-custody vs. attorney-access
against the module-05 signed-URL model in spec.

### M41 — Vernacular Voice Intake
WhatsApp voice note (Hindi/regional) → structured matter brief (parties, dates,
grievance, urgency). Closes the loop M25 + M29 only skirt. Reuses M25 STT + M27
extraction + M29 channel. India-first, portable to any multilingual market.
