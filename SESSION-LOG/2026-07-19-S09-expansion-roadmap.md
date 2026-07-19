# Session S09 — 2026-07-19
## Expansion Roadmap — Courtroom Intelligence (Track A) + Startup Legal SaaS (Track B)

---

## Summary

Planning-only session. Researched two reference products — **adalat.ai** (court-side
AI: real-time transcription in 15+ Indian languages, live case-flow management,
LLM-based scanning/translation, legal document intelligence, WhatsApp litigant
chatbots, AI summarization) and **visiocyber.ai** (AI-legal/cyber services: data
protection & privacy compliance, compliance & governance automation for SOC 2/ISO
27001, cybersecurity/AI-readiness assessments, technology evaluation, executive
education) — and wrote the strategic plan for folding both feature sets into
Persist without touching the existing Phase 0–7 plan.

**Result:** `specs/expansion-roadmap.md` created; TASKS.md gains Phases 2.5, 8, 9;
PROGRESS.md gains the matching tracking tables and five decision-log entries.
No code changed.

---

## Files Changed

### New files
- `specs/expansion-roadmap.md` — the strategic contract: feature-source mapping
  table, Track A modules M25–M29, Track B modules M30–M33, four architecture
  decisions (§5), sequencing/dependency plan (§6)
- `SESSION-LOG/2026-07-19-S09-expansion-roadmap.md` — this file

### Modified files
- `TASKS.md` — added Phase 2.5 (M31 DP Audit), Phase 8 (M27→M26→M25→M28→M29),
  Phase 9 (M30→M32→M33) with per-module step checklists; Phase 7 cause-list item
  marked absorbed by M26
- `PROGRESS.md` — Current State updated to S09; IN PROGRESS section updated with
  roadmap-ordered next options; Phase 2.5/8/9 tracking tables added; 5 decision
  log rows; session log row

---

## Module Numbering

Existing modules occupy 1–24 (13, 14, 20, 23 unused but skipped to avoid
ambiguity). New modules are numbered 25–33:

| # | Module | Track | Phase |
|---|---|---|---|
| 25 | Hearing & Dictation Transcription | A | 8 |
| 26 | Cause List & Hearing Flow | A | 8 |
| 27 | Document Digitization & Intake Intelligence | A | 8 |
| 28 | Research & Summarization (delivers M17/M18) | A | 8 |
| 29 | Client Status Chatbot (WhatsApp) | A | 8 |
| 30 | Startup Legal OS (SaaS, multi-tenant) | B | 9 |
| 31 | Data Protection Audit Engine (DPDP/GDPR) | B | **2.5** |
| 32 | Compliance & AI Governance | B | 9 |
| 33 | Assessment & Advisory Toolkit | B | 9 |

---

## Architecture Decisions (ratification pending in module specs)

1. **Bifurcated source of truth** — firm practice data stays desktop-SQLite-first;
   SaaS tenant data (Track B) is server-native PostgreSQL. Separate schemas; no
   cross-privilege joins. A single desktop cannot be the write path for N tenants.
2. **Server-side AI router** — `server/` gets the three-tier Haiku→Sonnet→Opus
   router for portal/chatbot/tenant traffic, with per-tenant token metering and
   rate limits. Tenants never see model names (same rule as Deck).
3. **Lawyer-in-the-loop as architecture** — HPAS ValidationGate is the shared
   approval primitive: all client/tenant-facing AI output is `pending_review`
   until attorney release, logged.
4. **Recording compliance (M25)** — context flag (dictation / client meeting /
   official-recording import); no covert-recording capability; spec must carry
   the jurisdiction note.
5. **M31 pulled forward to Phase 2.5** — DP audit engine needs only portal infra
   + billing + LaTeX (all exist after M5); it is the nearest-term revenue product
   and its AI gap-analysis pass can upgrade later in Phase 3.

---

## Sequencing (roadmap §6)

```
M5 portal → Phase 2.5 (M31) → Phase 3 AI → Phase 8 (M27→M26→M25→M28→M29)
→ Phase 4–5 → Phase 9 (M30→M32→M33) → Phase 6–7
```

Hard prerequisites before any new-track module: M5 portal, B02
(ip_assets + cascade_engine), B01 (persistent sessions).

---

## Commands Run

```bash
# Research
WebFetch https://visiocyber.ai/legal, https://visiocyber.ai,
         https://adalat.ai, https://adalat.ai/products

# No builds/tests — planning session, no code changed.
# Last known good: cargo test 33/33, pnpm build PASS (S08).
```

---

## Known Issues / Status

| ID | Status | Notes |
|---|---|---|
| B01 | Open | Now a hard prerequisite for Phase 8/9 attorney console |
| B02 | Open | Now a hard prerequisite for M26/M27 cascade integration |
| B03 | Partial | Unchanged |
| B04 | Resolved | Unchanged |

---

## Next Session Options (roadmap-ordered)

1. **Phase 2 M5** — Client Portal spec + scaffold. Critical path: unblocks
   Phase 2.5 M31 and all of Track B.
2. **Phase 1 M2 extended** — ip_assets, cascade_engine, abandonment_watcher (B02).
3. **Auth B01** — sessions table, persistent sessions.
4. **specs/module-31-dp-audit.md** — first Track B spec session (after M5).
