# Session S10 — 2026-07-19
## Track C — Candidate Bets + Pocket-inspired Mind Maps

---

## Summary

Planning-only follow-on to S09. Researched **heypocket.com** (Pocket — an AI
notes device/app: captures + transcribes + summarizes real-world conversations,
auto-generates a **mind map** per conversation with Central Theme → Main
Branches → Nodes, tap-a-node-to-jump-to-transcript, Copy-Markdown export, 100+
summary styles, action-item extraction, offline sync).

Incorporated the mind map idea two ways, plus recorded the seven "revolutionary
bet" ideas discussed earlier this session as a formal **Track C — Candidate
Bets** (M34–M41). Nothing is scheduled; Track C is an explicitly unsequenced
backlog requiring a promote-decision + Step-1 spec before any work.

**Result:** TASKS.md and specs/expansion-roadmap.md gain Track C; M25/M28 gain
mind map line items; PROGRESS.md tracking table + decision context updated. No
code changed.

---

## Files Changed

### New files
- `SESSION-LOG/2026-07-19-S10-track-c-candidate-bets.md` — this file

### Modified files
- `TASKS.md` —
  - M25: added Transcript mind map line item (nodes linked to transcript
    timestamps; `components/transcripts/TranscriptMindMap.tsx`; Copy-Markdown)
  - M28: added Document mind map line item (judgments/orders; nodes → source
    paragraph; feeds hearing-prep brief; generalizes renderer to `MindMap.tsx`)
  - New **Track C — Candidate Bets** section: M34 Matter Mind Map, M35
    Limitation Engine⭐, M36 Order Watcher⭐, M37 Court-Rules Compiler, M38 Firm
    Brain⭐, M39 Bench Analytics, M40 Client-Held Privilege Keys, M41 Vernacular
    Voice Intake. Marked NOT sequenced; top-three bets flagged.
- `specs/expansion-roadmap.md` — added §8 (Track C) with rationale per module and
  the two-lift framing of the mind map idea (transcript/doc → M25/M28; matter-
  scale → M34)
- `PROGRESS.md` — Current State → S10; new Track C tracking table (🔵 candidate
  status); session-log row

---

## Module Numbering (Track C — reserved, not scheduled)

| # | Candidate | Top bet? |
|---|---|---|
| 34 | Matter Mind Map (Case Theory Canvas) | |
| 35 | Limitation Engine (deterministic deadline law) | ⭐ |
| 36 | Order Watcher (eCourts/IP India/TM Journal) | ⭐ |
| 37 | Court-Rules Compiler (self-formatting e-filing) | |
| 38 | Firm Brain (precedent memory w/ provenance) | ⭐ |
| 39 | Bench Analytics (India-first judge insight) | |
| 40 | Client-Held Privilege Keys | |
| 41 | Vernacular Voice Intake (voice note → matter brief) | |

Existing numbering intact: committed modules 1–33 (Phases 0–9). Track C occupies
34–41 as a reserved vocabulary only.

---

## Rationale Highlights

- **Mind maps are cheap to add on our stack**: transcripts already vault-
  encrypted, summarization already routes through HPAS, and every linkable
  object (documents, dockets, hearings, transcripts) co-exists in one SQLite DB —
  so the matter-scale map (M34) is largely a query + renderer.
- **Legal mind map > generic**: tap-node-to-transcript-timestamp is evidentiary,
  not just convenient; the matter-scale map is CaseMap-class tooling with no
  strong Indian incumbent.
- **Top three bets** chosen for daily-pain × moat × export story: M35 (trust via
  determinism + jurisdiction-pack export), M36 (recurring-revenue TM Journal
  watch), M38 (compounding data moat).

---

## Commands Run

```bash
# Research
WebSearch "heypocket.com Pocket AI notes app features mind map"
WebFetch https://docs.heypocketai.com/docs/features/organization/mind-maps
# (heypocket.com homepage returned 429 — used docs + search instead)

# No builds/tests — planning session, no code changed.
# Last known good: cargo test 33/33, pnpm build PASS (S08).
```

---

## Known Issues / Status

Unchanged from S09 (B01 open, B02 open, B03 partial, B04 resolved).

---

## Next Session Options (unchanged critical path)

1. **Phase 2 M5** — Client Portal spec + scaffold (unblocks Phase 2.5 + Track B).
2. **Phase 1 M2 extended** — ip_assets, cascade_engine (B02).
3. **Auth B01** — sessions table.
4. If promoting a Track C bet: start with `specs/module-35-limitation-engine.md`
   (highest-leverage, extends existing docketing engine).
