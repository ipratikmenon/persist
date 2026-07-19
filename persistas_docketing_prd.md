# Persistas Docketing Platform — Product Requirements Document

**Version:** 2.9  
**Date:** April 2026  
**Author:** Pratik (Product Owner)  
**Status:** Draft — Section 24 added: HPAS (Hierarchical Parallel Agent System) — multi-agent orchestration engine, phase assignments, success metrics, relationship to Section 22 AI router  

---

## 1. Executive Summary

The Persistas Docketing Platform (codename: **Persist**) is a proprietary legal practice management and docketing tool built exclusively for Persistas & Partners. It serves as a core USP of the firm — offering clients a branded, self-service transparency portal while giving the firm a powerful internal command centre for matter management, deadline tracking, billing, and document workflows.

This platform draws inspiration from commercial tools like **Bilr**, **Litera Transact**, **Litera Foundation 365**, **Litera Draft** (formerly Desktop), and **Kira** (AI contract intelligence), but is purpose-built for Persistas' multi-practice structure spanning IP, Corporate, Litigation, and Paralegal services. Critically, it includes a full in-house **Document Drafting Suite** and a **Contract Intelligence Engine** — covering template-based document creation, AI-powered proofreading, document comparison, metadata cleaning, PDF workflows, a firm-wide precedent library, and deep AI-driven contract analysis and due diligence — removing any dependency on expensive third-party tools.

---

## 2. Problem Statement

Currently, Persistas & Partners operates without a unified docketing system. Deadline tracking, client communication, document versioning, and billing are likely handled across spreadsheets, email threads, and ad hoc tools. This creates:

- Risk of missed statutory deadlines (especially critical in IP prosecution)
- Lack of real-time visibility for clients into their matter status
- No structured billing and time tracking workflow
- No audit trail for matter history and document versions
- Difficulty scaling paralegal and associate workload management
- Contract review done entirely by hand — slow, inconsistent, and error-prone at volume (due diligence, supplier audits, IP licensing reviews)

**Persist** solves this holistically — unifying firm operations and elevating the client experience simultaneously.

---

## 3. Goals & Success Metrics

| Goal | Metric |
|------|--------|
| Zero missed critical deadlines | 0 deadline breach incidents post-deployment |
| Client self-service adoption | 60%+ clients actively using portal within 90 days |
| Billing efficiency | 30% reduction in billing cycle time |
| Associate workload visibility | All open matters have assigned attorney + next action |
| Document accountability | 100% of filed documents linked to matters |
| Drafting speed | 50%+ reduction in time from blank page to first draft |
| Document quality | Zero documents sent to clients with metadata or formatting errors |
| Contract review speed | 70%+ reduction in time for standard due diligence contract set |
| Review consistency | 100% of reviewed contracts have a completed extraction before report |
| Contract review throughput | 70%+ reduction in manual contract review time for due diligence and M&A mandates |
| Clause extraction accuracy | 85%+ accuracy on key clause identification across standard agreement types |

---

## 4. User Personas

### 4.1 Firm Users (Internal)

**Partner / Senior Advocate**
- Needs a high-level dashboard showing all active matters, upcoming critical deadlines, outstanding invoices, and associate utilisation.
- Approves documents, sets billing rates, reviews time entries.

**Associate / Junior Advocate**
- Works on assigned matters day-to-day.
- Logs time, uploads documents, updates matter status, drafts correspondence.

**Paralegal**
- Manages docketing calendars, tracks prosecution deadlines (especially IP).
- Enters dates from official communications, manages deadline reminders.

**Admin / Billing Staff**
- Generates invoices, tracks payments, runs financial reports.

### 4.2 Client Users (External)

**Corporate Client (GC / In-House Counsel)**
- Needs real-time visibility into all their matters.
- Wants to upload documents, approve invoices, download status reports.

**Individual / SME Client**
- Less technically sophisticated; needs a clean, simple view of their single or few matters.
- Wants to know: what's happening, what's next, what they owe.

---

## 5. Platform Architecture Overview

Persist is a **dual-portal SaaS web application** with a shared backend:

```
                        ┌─────────────────────────────┐
                        │       Persist Backend         │
                        │  (Node.js / Python FastAPI)   │
                        │  PostgreSQL + S3-compatible   │
                        │  document storage             │
                        └──────────┬──────────────────┘
                                   │
               ┌───────────────────┴───────────────────┐
               │                                       │
    ┌──────────▼──────────┐               ┌───────────▼───────────┐
    │   FIRM PORTAL        │               │   CLIENT PORTAL        │
    │  (Internal Staff)    │               │  (External Clients)    │
    │                      │               │                        │
    │  Full matter CRUD    │               │  Read-only + limited   │
    │  Billing & time log  │               │  uploads               │
    │  Deadline engine     │               │  Invoice approval      │
    │  Reports & analytics │               │  Matter status         │
    │  AI drafting tools   │               │  Notifications         │
    └──────────────────────┘               └────────────────────────┘
```

---

## 6. Feature Specification

### Module 1: Matter Management

This is the core of the platform. Every action — billing, deadlines, documents — is anchored to a **Matter**.

**Matter object contains:**
- Matter ID (auto-generated, human-readable, e.g. `P&P-2026-TM-0042`)
- Matter type: IP / Corporate / Litigation / Paralegal
- Sub-type: e.g. Trademark Application, Patent Prosecution, Contract Review, Writ Petition
- Client linked (one or more)
- Responsible partner
- Assigned associate(s) and paralegal(s)
- Matter status: `Active`, `On Hold`, `Pending Client Response`, `Closed`, `Archived`
- Jurisdiction / Forum (e.g. Trade Marks Registry, Delhi HC, NCLT)
- Priority: Normal / High / Urgent
- Opening date, target close date
- Internal notes (not visible to client)
- Client-visible notes / status updates
- Tags (for filtering and search)

**Firm portal capabilities:**
- Create, edit, close, archive matters
- Bulk reassign matters across staff
- Matter timeline view (milestones plotted chronologically)
- Linked matters (e.g. trademark + subsequent opposition matter)
- Matter search with full-text and filter support

**Client portal capabilities:**
- View their own matters only
- See matter status, responsible attorney, last update date
- View client-facing notes and status updates
- Cannot see internal notes, billing rates, or staff commentary

---

### Module 1.4 — Matter Discussion Tab (Internal Collaboration)

Every matter has a **Discussion** tab — a real-time, matter-scoped comment thread visible to all attorneys and paralegals assigned to that matter. This is the primary internal communication layer for Persist, designed for the quick, contextual exchanges that happen around live matters: questions about strategy, instructions between a partner and associate, status confirmations, handoff notes.

This is deliberately **not** a general messaging system. Every comment is permanent context for the matter it belongs to. There are no DMs, no channels, no general inbox. Discussion is scoped, archived, and searchable — always attached to the matter it concerns.

---

#### 1.4.1 Why This Instead of a Messaging System

At Persistas' current scale (2 attorneys), a Slack/Teams-style messaging platform would be significant over-engineering. The industry pattern confirms this — even well-funded platforms like Clio use Slack as a separate tool rather than building it in. Matter Discussion covers the real collaboration need without the infrastructure burden of a general messaging system.

When Persist scales to 5+ attorneys or becomes a multi-firm product, the Discussion infrastructure (WebSocket layer, message store, @mention system) becomes the foundation for a general messaging layer — either extended internally or bridged to Teams via Module 19. No rewrite required, just scope expansion.

---

#### 1.4.2 The Discussion Tab — UI

```
┌──────────────────────────────────────────────────────────────────────┐
│  Petalveda Scents — TM Opposition                  P&P-2026-TM-0042  │
│  Overview │ Documents │ Deadlines │ 💬 Discussion │ Billing          │
├──────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  ─── Today ──────────────────────────────────────────────────────    │
│                                                                       │
│  [SL]  Sree Lakshmi  •  2:14 PM                                      │
│        Akash's reply came in — I've read it. They're disputing       │
│        the passing off ground entirely. Should we go for a           │
│        hearing or file a written response?                            │
│                                                                       │
│  [KT]  Kajal Thakur  •  2:31 PM                                      │
│        Written response first. If Registry isn't satisfied           │
│        they'll call a hearing anyway. I'll draft by tomorrow.        │
│        @Sree can you check if we have packaging samples as           │
│        evidence? Need at least 3 from different years.               │
│                                                                       │
│  [SL]  Sree Lakshmi  •  2:33 PM                                      │
│        ✓ Done — found 4 samples, 2019–2023. Uploaded to             │
│        Documents → Evidence. Also flagged the deadline:              │
│        response due 12 Apr.                                           │
│        📎 packaging-sample-2019.pdf  📎 packaging-sample-2023.pdf   │
│                                                                       │
│  ─── Yesterday ──────────────────────────────────────────────────    │
│                                                                       │
│  [KT]  Kajal Thakur  •  6:15 PM                                      │
│        🎙 0:24  "Check the counter-statement deadline — I think      │
│        it's closer than we think. Verify against Registry notice."   │
│                                                                       │
├──────────────────────────────────────────────────────────────────────┤
│  Reply...                                         🎙  📎  @  [Send] │
└──────────────────────────────────────────────────────────────────────┘
```

**Comment card anatomy:**
- Attorney avatar initials (coloured by user — Sree = slate blue, Kajal = terracotta)
- Attorney name + timestamp
- Message body — rendered Markdown via the Persist Editor (Module 15A)
- Inline attachment chips — documents attached from the matter store
- Voice note chip — if posted via Persist Voice, shows `🎙 0:24` play button + AI-corrected transcript preview

**Compose bar:**
- Full Persist Editor (Module 15A) — Markdown, `@mention`, `/expansions`, `#case:` tags all work
- `🎙` — opens Persist Voice scoped to this matter; output type = Discussion comment automatically
- `📎` — attach from matter document store
- `@` — triggers autocomplete showing all attorneys assigned to this matter
- `Enter` to send (Shift+Enter for new line)

---

#### 1.4.3 @Mentions and Notifications

When an attorney types `@Kajal` or `@Sree` in a comment, the mention renders as a coloured pill and triggers a targeted notification:

- **In-app notification** (bell icon, top-right): "Kajal mentioned you in Petalveda Scents TM Opposition"
- **OS system notification** (Tauri notification plugin): same message, action button "Open matter"
- **WhatsApp alert** (optional, configurable per attorney): "📌 Mention — Kajal Thakur in Petalveda Scents TM Opposition. Open Persist."

Unread comment count shows on the Discussion tab badge. The matter list view shows a `💬 N` indicator on matters with unread discussion comments.

Mentions without a specific attorney (`@all` or no mention) notify all attorneys assigned to the matter.

---

#### 1.4.4 Real-Time Delivery

Comments are delivered in real-time when both attorneys are online simultaneously — no page refresh needed. The WebSocket connection from the sync server (used for matter data sync) is extended to handle comment delivery between desktop clients.

When an attorney is offline, comments queue on the sync server and deliver the next time they open Persist. The unread badge and notification fire on reconnect.

This uses the **same WebSocket infrastructure** as the sync server — no new server component needed. The message bus is the foundation that scales into a general messaging system if Persist ever needs one.

---

#### 1.4.5 Voice Comments

When an attorney uses Persist Voice (`🎙`) from within the Discussion tab, the capture routes automatically as a Discussion comment — no output type selection needed. The comment shows:

```
[SL]  Sree Lakshmi  •  6:15 PM
      🎙 0:24   "Check the counter-statement deadline — I think
      it's closer than we think. Verify against Registry notice."
      [▶ Play]  [Show raw transcript]
```

The AI-corrected text is shown as the comment body. The audio clip and raw transcript are stored in the vault and accessible via the expand controls. This makes voice notes first-class citizens in the matter discussion — not a separate system.

---

#### 1.4.6 Search and History

All discussion comments are stored permanently in SQLite, linked to the matter. They appear in:

- The matter's Discussion tab — full chronological history, grouped by day
- The matter timeline — comments appear alongside deadlines, documents, and emails as matter events
- Global full-text search — "find all discussions mentioning Akash Kumar Srivastava"
- The Daily Brief (Module 11.1) — unread mentions surface as morning priorities

Comments are never deleted (attorneys can mark them resolved, which dims them, but the record persists). Every comment is part of the permanent matter record.

---

#### 1.4.7 Data Model

```sql
CREATE TABLE matter_comments (
    id TEXT PRIMARY KEY,
    matter_id TEXT NOT NULL REFERENCES matters(id) ON DELETE CASCADE,
    author_id TEXT NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,               -- Markdown — rendered in Deck
    body_plain TEXT NOT NULL,         -- Plain text — for search and notifications
    voice_capture_id TEXT REFERENCES voice_captures(id), -- if posted via voice
    attachment_ids TEXT DEFAULT '[]', -- JSON array of document IDs from matter store
    is_resolved INTEGER NOT NULL DEFAULT 0,
    resolved_by TEXT REFERENCES users(id),
    resolved_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_comments_matter ON matter_comments(matter_id, created_at DESC);
CREATE INDEX idx_comments_author ON matter_comments(author_id);

CREATE TABLE comment_mentions (
    id TEXT PRIMARY KEY,
    comment_id TEXT NOT NULL REFERENCES matter_comments(id) ON DELETE CASCADE,
    mentioned_user_id TEXT NOT NULL REFERENCES users(id),
    is_read INTEGER NOT NULL DEFAULT 0,
    read_at DATETIME,
    notified_at DATETIME               -- when the push notification was sent
);
CREATE INDEX idx_mentions_user ON comment_mentions(mentioned_user_id, is_read);
```

---

#### 1.4.8 Scale Path — From Matter Comments to Messaging

The table structure and WebSocket infrastructure above are deliberately designed as the foundation for a future general messaging layer:

| Current (Matter Discussion) | Future (General Messaging) |
|---|---|
| `matter_comments` table | + `conversations` table (matter-linked or general) |
| Matter-scoped threads | + Firm-wide channels + direct messages |
| @mention of assigned attorneys | + @mention of any firm user |
| WebSocket per-matter | WebSocket per-firm (same server, broader scope) |
| Notifications per mention | Notifications per conversation subscription |

**Phase 5+:** if Persist expands beyond Persistas & Partners to serve other firms, or if the attorney count reaches 5+, the conversation layer is added on top of this foundation — not instead of it.

**Alternative scale path (Module 19):** use Microsoft Teams as the general messaging layer via the Teams-per-matter integration already specced in Module 19.3. Firms that already live in Teams can keep their communication there; Persist syncs matter context and action items back. The Discussion tab then becomes the matter-native record while Teams handles the real-time chat.

---

This is the highest-criticality module in Persist — particularly for IP prosecution where missing a statutory deadline results in immediate and irreversible abandonment of the application. The design draws directly from DocketTrak (the leading US IP docketing platform) as a reference for information architecture, adapted to the Persist design system and Indian IP practice context.

---

#### 2.1 Deadline Types

- **Statutory deadline** — hard, immovable. Set by law (e.g. TM response to examination report: 30 days from date of notice; patent FER response: 12 months). Cannot be waived without formal extension.
- **Internal deadline** — soft, firm-set. First draft of reply due 10 days before statutory deadline; partner review due 5 days before. Entirely configurable.
- **Court filing deadline** — jurisdiction-specific. Delhi HC, IPAB, NCLT each have their own procedural rules. Linked to the matter's forum.
- **Auto-generated deadline** — created automatically by the docketing engine when a trigger event occurs (examination report received → 30-day response deadline auto-generated).
- **Reminder** — not a deadline itself, but a scheduled notification firing before an existing deadline (configurable: 30 days, 14 days, 7 days, 3 days, 1 day before).
- **Custom deadline** — freeform deadline manually created by the attorney.

---

#### 2.2 Deadline Object (Data Model)

Each deadline record contains:

```
matter_id           → linked matter (P&P-2026-TM-0042)
docketing_event     → human-readable event name ("Response to Examination Report")
reference_number    → internal docket code (P&P-DD-0023)
deadline_type       → Statutory / Internal / Court / Custom
ip_type             → Trademark / Patent / Design / Copyright / Corporate
due_date            → exact date (and time if applicable)
responsible_user    → attorney or paralegal assigned
status              → Pending / In Progress / Completed / Overdue / Waived
priority            → Normal / High / Critical
country             → India (default) / multi-jurisdiction
completion_date     → when marked complete
completion_notes    → free text (e.g. "Filed on TM Registry portal, ref. TM-XX")
evidence_doc        → linked filing receipt or confirmation document
created_from        → Manual / Auto-generated / Template
template_id         → which IP template generated this (if auto)
reminders[]         → array of reminder rules for this deadline
audit_log[]         → timestamped history of every status change
```

---

#### 2.3 The Docket List View — UI Specification

The Docket List View is the attorney's primary command view for all upcoming and active deadlines. Inspired by DocketTrak's "My Dockets" screen, redesigned in the Persist visual language.

**Layout:**

```
┌─────────────────────────────────────────────────────────────────────┐
│  MY DOCKETS                                    Records: 17  Future: 12 │
│  View: [My Dockets ▾]  Date Filter: [Next 30 Days ▾]  [+ New Docket] │
├─────────────────────────────────────────────────────────────────────┤
│  Filter bar (collapsed by default, expandable):                       │
│  Description: [___]  Client: [___▾]  Type: [___▾]  Status: [___▾]   │
│  Reference #: [___]  Country: [___▾]  Attorney: [___▾]  [Search]    │
├──────────┬────────────────────────────┬────────┬──────┬──────────────┤
│ Due Date │ Docketing Event            │ Ref #  │ Type │ Matter/Title │
├──────────┼────────────────────────────┼────────┼──────┼──────────────┤
│          │ ...rows...                 │        │      │              │
└──────────┴────────────────────────────┴────────┴──────┴──────────────┘
```

**Column structure** — directly informed by DocketTrak's proven column set, adapted for Indian practice:

| Column | Content | Notes |
|---|---|---|
| Due Date | `12 Apr 2026` — clickable, opens deadline record | Sorted ascending by default |
| Docketing Event | "Response to Examination Report" — the action required | Primary descriptive column |
| Ref # | `P&P-DD-0023` — internal docket reference | Short, scannable |
| Type | `Trademark` / `Patent` / `Design` / `Corporate` | Text label, not badge |
| Matter / Title | Full IP asset name or matter name | May wrap to 2 lines for long titles |
| Client | Client name — linked to client record | |
| Country | `India` / multi-jurisdiction indicator | Persistas is primarily India |
| Attorney | Responsible attorney initials | |
| Status | `Active` / `Completed` / `Overdue` / `Waived` | Final column |

**Row urgency colouring — the visual alarm system:**

This is the most critical visual decision in the entire module. Borrowed directly from DocketTrak's red-row convention, applied with the Persist colour palette:

- **Overdue (past due date):** entire row background `#FEF0EF`, all text in `#C0392B` (muted red) — unmistakably urgent, impossible to miss
- **Critical (due within 3 days):** entire row background `#FEF6ED`, all text in `#D4872A` (warm amber)
- **Warning (due within 7 days):** left accent bar only in amber `#D4872A`, no background tint — text remains standard
- **Normal (due in 7–30 days):** standard row, no colour treatment
- **Completed:** row dimmed — text in secondary colour `#9A9590`, status badge shows green `Completed`

The row colouring is the only place in Persist where colour treatment covers an entire row. Everywhere else in the platform, only individual elements are coloured. This exception exists because the docket list is a safety-critical view — an overdue IP deadline cannot be overlooked.

**Bulk actions toolbar** (appears when one or more rows are selected via checkbox):
- Edit Multiple — bulk-update status, assigned attorney, or reminder settings
- Mark Complete — bulk-complete selected deadlines
- Export — export selected rows to Excel or PDF
- Delete — soft-delete with confirmation

**Summary counters** (persistent footer bar, always visible):
- `Future Dockets: 12` — deadlines beyond the current filter window
- `Records shown: 17` — total rows in current filtered view
- `Overdue: 2` — count of overdue items in red (always shown, never filtered away)

**View selector** (top-left dropdown):
- My Dockets — filtered to the logged-in attorney's responsible deadlines
- All Dockets — all firm deadlines
- By Client — grouped by client
- By Practice Area — grouped by TM / Patent / Design / Corporate
- Overdue Only
- This Week
- Next 30 Days / 60 Days / 90 Days

---

#### 2.4 The IP Asset Record — UI Specification

When an attorney clicks a docket row or navigates to an IP asset (trademark, patent, design), they see the full IP asset record. This is the detailed view inspired by DocketTrak's Patent Details screen.

**Header bar** (always visible, sticky):
```
┌─────────────────────────────────────────────────────────────────────┐
│  ← TRADEMARK DETAILS     P&P-2026-TM-0042   Ref: TM-1234567        │
│  [← Prev] [Next →]  [Save]  [Save & Exit]  [Save & Copy]           │
│  [Cancel] [Delete]  [History]  [Generate PDF]  [View at Registry]  │
└─────────────────────────────────────────────────────────────────────┘
```

The `[View at Registry]` button opens the Indian Trade Marks Registry (ipindia.gov.in) or Patent Office (ipindia.gov.in/cgpdtm) directly to the specific application — equivalent to DocketTrak's `[View at PTO]` button. The application number is passed as a URL parameter.

**Two-panel layout (General + IP Numbers and Chronology):**

The record body uses a two-panel layout at the top — identical in concept to DocketTrak's split layout, but rendered in Persist's card style (warm white cards, `border-radius: 10px`, soft shadow):

**Left panel — General:**

```
┌─────────────────────────────────────────────────────┐
│  GENERAL                                             │
│                                                      │
│  Client Name          [Petalveda Scents ▾] [add]    │
│  Reference Number     TM-1234567                     │
│  Country              [India ▾]                      │
│  Mark / Title         [__________________________]   │
│                       [__________________________]   │
│  Status               [Active ▾]    Status Date: ___ │
│  Active               [Yes ▾]                        │
│                                                      │
│  ┌─ Responsible Team ──────────────────────────┐    │
│  │ Name              Role              [+]      │    │
│  │ Sree Lakshmi      Partner           [−]      │    │
│  │ Kajal Thakur      Partner           [−]      │    │
│  └──────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────┘
```

**Right panel — TM Numbers and Chronology (Trademark):**

```
┌──────────────────────────────────────────────────────────────────┐
│  TRADEMARK NUMBERS AND CHRONOLOGY                                 │
│                                                                   │
│  Application Type    [Word Mark ▾]  Nice Classes  [03, 05 ▾]     │
│  Application Number  1234567        Filing Date   15 Jan 2024    │
│  Journal Number      ___            Journal Date  ___            │
│  Opposition End Date ___            Registration Number ___      │
│  Registration Date   ___            Renewal Date  ___            │
│  Expiration Date     ___                                         │
│                                                                   │
│  Description of Goods/Services                                   │
│  [_______________________________________________]               │
│  [_______________________________________________]               │
│                                                                   │
└──────────────────────────────────────────────────────────────────┘
```

For **Patent** records, the right panel shows the DocketTrak-equivalent Patent Numbers and Chronology fields: Application Type (Provisional/Complete/PCT), Application SubType (Paris Convention/PCT National Phase), Application Number, Filing Date, Earliest Priority Filing Date, Effective Filing Date, Publication Number, Publication Date, Publication Kind Code, Patent Number, Date Granted, Patent Term Adjustment, Expiration Date, Abstract (scrollable text area).

**Events section** (below the two panels):

This is the deadline engine's UI, directly inspired by DocketTrak's Events section:

```
┌─────────────────────────────────────────────────────────────────────┐
│  EVENTS                                                              │
│  Auto Docket Template: [Select Template ▾]  [Generate Dockets]      │
├───┬──────────────────────────────────┬────────────┬──────┬──────────┤
│ ⚑ │ Docketing Event                  │ Event Date │ Done │ Notes    │
├───┼──────────────────────────────────┼────────────┼──────┼──────────┤
│ ⚑ │ Response to Examination Report   │ 12 Apr 26  │ [ ]  │ [_____]  │
│ ⚑ │ Internal Draft Deadline          │ 05 Apr 26  │ [ ]  │ [_____]  │
│ ⚑ │ Partner Review                   │ 08 Apr 26  │ [✓]  │ Reviewed │
├───┴──────────────────────────────────┴────────────┴──────┴──────────┤
│  [+ Add Event]   [Hide Completed Events]                            │
└─────────────────────────────────────────────────────────────────────┘
```

The `⚑` icon is the docket pin — the same concept as DocketTrak's location pin icon. It visually marks each row as a docket event (actionable deadline), distinguishing it from a general note. Clicking it opens the full deadline record.

The `[Generate Dockets]` button is the power-user feature: select a template (e.g. "TM Application — Standard Prosecution") and the engine auto-populates all future events for this matter in the correct statutory sequence with calculated dates, exactly as DocketTrak does.

**Below Events — additional panels:**

- **IP Record Details** — jurisdiction-specific fields (e.g. Vienna Classification for figurative marks, Colour claim, Disclaimers)
- **Registry Data** — pulled from ipindia.gov.in if integration is live; shows last known status from the registry
- **Documents** — all documents linked to this IP asset
- **Communications** — email thread history for this matter
- **Notes** — internal attorney notes with the Persist Editor (Module 15A — Markdown + smart tags)
- **Billing** — time entries and invoices linked to this matter
- **Audit Log** — every change to every field, with timestamp and user

---

#### 2.5 Deadline Engine Behaviour

**Automatic deadline generation:** When a triggering document is linked to a matter (e.g. an examination report is uploaded or arrives via email), the engine detects the trigger type and auto-creates the downstream deadline chain:

```
Examination Report received (TM)
  → Response to Examination Report: +30 days (statutory)
  → Internal Draft Deadline: +20 days (internal, configurable)
  → Partner Review: +25 days (internal, configurable)
  → WhatsApp Alert to responsible attorney: immediately
```

**Urgency escalation:**
- `T-5 days`: email alert to responsible attorney
- `T-3 days`: email alert + in-app banner + WhatsApp message to responsible attorney
- `T-1 day`: email alert + WhatsApp + SMS (configurable) + notification to responsible partner
- `Overdue`: escalates to both the responsible attorney and the managing partner; row turns red in all docket views; persists until marked Complete or Waived

**Conflict detection:** if two Critical deadlines fall on the same day for the same attorney, the system flags this at the time of creation and in the Daily Brief.

**Calendar sync:** every deadline syncs to iCal / Google Calendar / Outlook via the calendar integration (Module 19). Deadline appears as a calendar event with the matter name and docket reference in the event title.

---

#### 2.6 IP-Specific Deadline Templates

Pre-loaded statutory timelines for Indian IP prosecution. Selecting a template on a new matter auto-populates all events with correctly calculated due dates:

**Template: Trademark Application (India)**
```
1. Filing Date                         → Day 0
2. Examination Report Expected         → +12 months (approx, tracked)
3. Response to Examination Report      → Exam Report date + 30 days (statutory)
4. Hearing (if called)                 → Notice date + as specified
5. Advertisement in Journal            → Post-acceptance
6. Opposition Window                   → Advertisement date + 4 months
7. Counter-statement (if opposed)      → Opposition date + 2 months
8. Evidence in Support (Opponent)      → Counter-statement + 2 months
9. Evidence in Support (Applicant)     → Opponent evidence + 2 months
10. Registration                       → Post-opposition resolution
11. Renewal                            → Registration date + 10 years
```

**Template: Patent Application — Complete Specification (India)**
```
1. Filing Date                         → Day 0
2. Publication                         → Filing date + 18 months
3. Request for Examination             → Filing date + 48 months (statutory)
4. First Examination Report Expected   → RFE date + 6 months (approx)
5. Response to FER                     → FER date + 12 months (statutory)
6. Hearing (if required)               → As scheduled
7. Grant / Pre-grant Opposition        → Post-examination
8. Renewal (Annuity)                   → Grant date + year increments
```

**Template: Trademark Opposition (India)**
```
1. Notice of Opposition Filed          → Day 0
2. Counter-statement Due               → Day 0 + 2 months (statutory)
3. Evidence in Support of Opposition   → Counter-statement + 2 months
4. Evidence in Support of Application  → Opponent evidence + 2 months
5. Evidence in Reply                   → Applicant evidence + 1 month
6. Hearing Date                        → As fixed by Registry
```

**Template: Design Registration (India)**
```
1. Filing Date                         → Day 0
2. Examination Report Expected         → +3 months (approx)
3. Response to Objection               → Report date + 3 months
4. Registration                        → Post-acceptance
5. Renewal                             → Registration + 10 years (×2 renewals)
```

Additional templates: Copyright Registration, PCT National Phase Entry (India 30-month), Paris Convention Priority.

---

#### 2.7 IP Prosecution Pipeline — Kanban View

Inspired by Triangle IP's drag-and-drop pipeline view, Persist includes a **Kanban-style IP prosecution board** alongside the docket list. Where the docket list is deadline-focused (what needs action now), the pipeline board is stage-focused (where is each matter in the prosecution lifecycle).

**What it is:** A horizontal board of columns, each representing a prosecution stage. Every IP matter is a card. Cards move right as prosecution progresses.

**Default pipeline stages for Trademark:**

```
┌───────────────┬──────────────┬───────────────┬──────────────┬──────────────┬──────────────┐
│  APPLICATION  │  EXAMINATION │  ADVERTISED   │  OPPOSITION  │  REGISTERED  │  RENEWAL DUE │
│     FILED     │   PENDING    │               │   WINDOW     │              │              │
├───────────────┼──────────────┼───────────────┼──────────────┼──────────────┼──────────────┤
│ [Petalveda    │ [Zenith      │               │ [Tata Sons   │ [Rahul Ind.  │              │
│  Scents       │  Brands      │               │  Colgate     │  ALOVERA     │              │
│  ALOVERA]     │  MOONLIGHT]  │               │  Mark]       │  Mark]       │              │
│               │              │               │              │              │              │
│ [Oak Creek    │              │               │              │              │              │
│  OAKSEED]     │              │               │              │              │              │
└───────────────┴──────────────┴───────────────┴──────────────┴──────────────┴──────────────┘
```

**Card anatomy:**
```
┌──────────────────────────────────┐
│ ● PETALVEDA SCENTS               │
│ ALOVERA (Word Mark)              │
│ TM No. 1234567 · Class 03       │
│                                  │
│ Filed: 15 Jan 2024               │
│ Next: Response due 12 Apr 2026   │
│ ⚠ Due in 2 days                 │
│                     [Sree L.]   │
└──────────────────────────────────┘
```

Each card shows: client name, mark name and type, TM/patent application number, class(es), filing date, next action due date, urgency indicator (amber/red based on deadline proximity), responsible attorney initials.

**Card urgency states:** a left border accent on the card (not the full card background — subtler than the docket list) — terracotta for overdue, amber for within 7 days, none for normal.

**Drag and drop:** partners and associates can drag a card from one stage to the next when a prosecution milestone is reached (e.g. drag from "Examination Pending" to "Advertised" when the mark is advertised in the TM Journal). Dragging automatically logs the stage transition in the matter's audit trail and prompts: "Log the advertisement date?"

**Per-portfolio stage configuration (Minuet / Triangle IP pattern):** each client's matters can have their own stage set. A corporate client with a large patent portfolio might have a "Prior Art Search" stage before "Patent Drafting" that a TM-only client doesn't need. Partners configure stage sets per practice area, not per matter individually.

**Bottleneck detection (Triangle IP pattern):** if a matter card sits in the same stage for longer than the expected duration (configurable per stage — e.g. "Examination Pending" should not exceed 14 months for TM), the card is flagged with a clock icon and surfaces in the Daily Brief as "stalled matters needing attention."

**Toggle between views:** the attorney can switch between Docket List (Section 2.3), Pipeline Board (this section), and Calendar view at any time — all three show the same underlying data, just structured differently.

---

#### 2.8 Renewal & Annuity Management

Inspired by Wellspring/Astria's IP renewals management approach. Indian IP prosecution has specific renewal obligations:

- **Trademarks:** renewal every 10 years (two renewals possible before re-filing); renewal application must be filed before expiry or within 6 months of expiry with a surcharge
- **Patents:** annuity fees due every year from the 2nd anniversary of the filing date (or grant date for PCT national phase entries); missed annuity = automatic lapse
- **Designs:** renewal every 5 years, maximum of 15 years total protection

**Renewal dashboard** — a dedicated view within Module 2:

```
┌──────────────────────────────────────────────────────────────────────┐
│  RENEWALS DUE                          [Filter: Next 12 months ▾]    │
├──────────────┬───────────────┬──────────┬───────────┬───────────────┤
│ Due Date     │ Matter        │ Type     │ Year/No.  │ Status        │
├──────────────┼───────────────┼──────────┼───────────┼───────────────┤
│ 15 Jan 2027  │ Rahul Ind.    │ TM       │ 1st Renew │ ○ Pending     │
│              │ ALOVERA       │ Renewal  │           │               │
├──────────────┼───────────────┼──────────┼───────────┼───────────────┤
│ 28 Mar 2027  │ Petalveda     │ Patent   │ Yr 3      │ ○ Pending     │
│              │ FORMULA-X     │ Annuity  │ Annuity   │               │
├──────────────┼───────────────┼──────────┼───────────┼───────────────┤
│ 01 Jun 2026  │ Oak Creek     │ TM       │ 1st Renew │ ⚠ Due Soon    │
│              │ OAKSEED       │ Renewal  │           │               │
└──────────────┴───────────────┴──────────┴───────────┴───────────────┘
```

**Renewal instruction workflow** (Wellspring/Astria pattern):

When a renewal is due, the attorney goes through a structured instruction flow rather than just marking a deadline complete:

1. **Client instruction received** — attorney marks that client has confirmed renewal intent
2. **Filing prepared** — renewal application or annuity payment prepared
3. **Filed / Paid** — renewal submitted; official receipt uploaded
4. **Confirmed** — registry confirmation received; renewal date and new expiry date updated on the IP record

Each step is a status transition in the renewal record, not just a checkbox. This creates a full audit trail showing exactly when the client confirmed, when it was filed, and when the registry confirmed — critical for malpractice protection.

**Annuity fee calculator:** for patents, the system calculates the annuity fee due based on the Patent Office fee schedule (updated annually), the year number, and whether any fee surcharge applies (for late payment). Displayed on the renewal card so the attorney can give the client an accurate cost estimate without checking the Patent Office website.

**Renewal forecast report:** shows all renewal obligations for the next 24 months with estimated official fees — enabling partners to give corporate clients a projected IP maintenance budget at the start of each financial year.

**Calculated fields (Minuet pattern):** renewal date, expiry date, and annuity year number are all calculated fields — they auto-update based on the filing date and grant date stored in the IP asset record. No manual calculation required. If the filing date is corrected, all downstream dates recalculate automatically.

**Sentinel-style renewal alerts (Minuet Sentinel pattern):** rule-based alerts trigger automatically based on renewal data:
- 12 months before TM renewal → draft client advisory letter automatically created in the matter
- 6 months before patent annuity → email alert to responsible attorney + client notification
- 3 months before → WhatsApp alert + escalate to partner
- 30 days before → urgent flag in Daily Brief
- Overdue → immediate escalation + malpractice risk flag in the matter record

---

**Core capabilities:**
- Upload any file type; auto-tag by matter
- Version control: each re-upload creates a new version; all versions retained
- Document categories: Correspondence, Filed Document, Court Order, Client Document, Draft, Internal Note
- Full-text search across documents
- Document preview (PDF, DOCX, images)
- Document sharing: generate time-limited secure link for client or external party

**Firm portal only:**
- Edit / delete / archive documents
- Mark documents as `Awaiting Client Signature`, `Executed`, `Filed`
- Set document-level permissions (which staff can view sensitive docs)
- Link documents to specific deadlines (e.g. "this TM response filing satisfies this deadline")

**Client portal:**
- View and download documents shared explicitly with them
- Upload documents in response to requests (e.g. power of attorney, invoices, supporting evidence)
- E-sign integration (future phase — DocuSign / DigiSigner)

---

### Module 4: Time Tracking & Billing

Inspired by Bilr's workflow, tailored for Indian legal billing norms.

**Time tracking:**
- Manual time entry: matter, date, description, hours (in 6-minute increments standard, or custom)
- Timer: start/stop in-app stopwatch linked to a matter
- Billable vs non-billable designation
- Rate types: hourly (partner / associate / paralegal rates), fixed fee, retainer drawdown

**Invoice generation:**
- Auto-compile unbilled time entries into a draft invoice
- Invoice line items: time entries, disbursements, court fees, government charges
- GST handling: add 18% GST on legal services, generate GST-compliant invoice
- Invoice numbering: sequential, firm-controlled
- Invoice status: `Draft`, `Sent`, `Viewed`, `Partially Paid`, `Paid`, `Disputed`
- Payment recording: manual entry + future UPI / bank transfer reconciliation
- Overdue follow-up: automated reminder emails at 7, 14, 30 days past due

**Client portal:**
- View all invoices for their matters
- Download PDF invoices
- Mark invoices as disputed with a note (triggers firm notification)
- View payment history

**Billing reports (firm only):**
- Revenue by matter, by client, by attorney, by practice area
- Outstanding receivables aging report
- Utilisation report: billable hours per attorney per month

---

### Module 5: Client Portal

The client-facing interface is a distinct, branded experience. It should feel like a polished client service tool, not a stripped-down version of the firm's internal software.

**Dashboard (client view):**
- Summary cards: Total active matters, Pending actions (documents needed, invoices unpaid), Upcoming milestones
- Matter list: filterable by status, type, date
- Notification feed: all recent updates across their matters
- Profile and contact info management

**Matter detail (client view):**
- Status badge + current stage in the workflow
- Timeline: key milestones plotted visually (e.g. Application Filed → Examination Report Received → Response Filed → Advertised)
- Responsible attorney + contact details
- Latest status note from firm
- Documents shared with client
- Pending actions required from client (e.g. "Please upload executed POA")
- Invoices tab

**Access & security:**
- Separate client login (email + password + OTP)
- Each client sees only their matters
- Multi-entity clients (e.g. a corporate group with subsidiaries) can have a parent account with child matter segregation
- Audit log: every client login and document download is logged

---

### Module 6: Notifications & Communications

**In-app notifications:**
- For firm users: deadline alerts, new client uploads, invoice payment, matter assigned
- For clients: status updates, new document shared, invoice sent, action required

**Email notifications:**
- Configurable per user and per notification type
- Templated HTML emails, Persistas branded
- Deadline reminders (configurable lead times)
- Invoice delivery with PDF attachment
- Client onboarding welcome email

**Future: WhatsApp notifications**
- High-priority deadline alerts to attorneys via WhatsApp (building on existing Meta Cloud API setup)
- Client milestone notifications via WhatsApp with opt-in

---

### Module 7: AI Drafting Assistant (Firm Only)

Building on the IP drafting bot concept previously explored for Persistas. This module is the intelligent layer sitting on top of the Document Drafting Suite (Module 9) — it uses Claude to generate and review content, while Module 9 handles the structural and formatting lifecycle of the resulting document.

**Capabilities:**
- Matter-context-aware drafting: AI is given the full matter details, client profile, and relevant deadlines before drafting
- Document types supported: Cease & Desist letters, TM examination report replies, patent prosecution responses, opposition filings, client advisory memos, status update letters, affidavits, licensing agreement drafts
- Jurisdiction awareness: Indian IP law (Trade Marks Act 1999, Patents Act 1970, Copyright Act 1957), Indian Contract Act, CPC, and IPAB/IP Division rules
- Clause library integration: AI selects and inserts pre-approved boilerplate from the firm's clause library (see Module 9) contextually based on document type and matter facts
- AI review mode: upload an existing document; AI flags missing arguments, inconsistent legal positions, formatting issues, or required statutory references
- Precedent search: AI surfaces relevant precedent language from the firm's document library before drafting begins
- Multi-document synthesis: AI can read multiple uploaded documents (e.g. examination report + prior art + client instructions) and generate a response in one pass
- Output: DOCX download, fully formatted, ready for attorney review and send

**Guardrails:**
- All AI output is stamped `DRAFT — ATTORNEY REVIEW REQUIRED` in the document header
- No AI output can be shared with clients directly without an attorney explicitly approving and removing the draft stamp
- All AI-generated content is logged: prompt, model response, matter linked, attorney who triggered it, and approval status
- Uploaded reference documents are never stored beyond the session unless explicitly saved to the matter

---

### Module 8: Reports & Analytics (Firm Only)

The analytics layer gives partners a real-time view of the firm's IP portfolio health, operational performance, and financial position. The IP Portfolio Analytics view is directly inspired by the DocketTrak Analytics dashboard — a three-panel linked layout combining filter sidebar, bar chart, and drill-down data table — redesigned in the Persist visual language.

---

#### 8.1 IP Portfolio Analytics — Primary Dashboard

**Layout:** Three-panel design, all panels linked — selecting a filter or clicking a chart bar instantly updates the other panels:

```
┌─────────────────┬────────────────────────────┬──────────────────────────┐
│  FILTER SIDEBAR │  IP APPLICATIONS FILED     │  APPLICATION DETAILS     │
│                 │  (Bar chart — by FY)        │  (Drill-down data table) │
│  Financial Year │                             │                          │
│  [ ] FY 2024-25 │  ████████████████  2024-25  │  FY   Q  Type    Status  │
│  [✓] FY 2023-24 │  ████████████      2023-24  │  25  Q4  TM      Active  │
│  [✓] FY 2022-23 │  ██████████████    2022-23  │  25  Q3  Patent  Filed   │
│  [ ] FY 2021-22 │  ████████          2021-22  │  24  Q4  Design  Issued  │
│                 │                             │  ...                     │
│  Client         │                             │                          │
│  [All ▾]        │      0   5   10  15   20   │                          │
│                 │      Applications Filed     │                          │
│  Type           │                             │                          │
│  [All ▾]        │                             │                          │
│                 │                             │                          │
│  Status         │                             │                          │
│  [All ▾]        │                             │                          │
│                 │                             │                          │
│  Attorney       │                             │                          │
│  [All ▾]        │                             │                          │
└─────────────────┴────────────────────────────┴──────────────────────────┘
```

**Left sidebar — Filter panel:**
- Financial Year multi-select checkboxes (Indian FY: Apr–Mar) — all years available since firm's first filing
- Client filter dropdown (All / specific client)
- IP Type multi-select: Trademark, Patent, Design, Copyright
- Status multi-select: Filed, Pending, Examination, Opposed, Registered/Granted, Abandoned, Expired
- Attorney of Record dropdown
- Country filter (India default, + any multi-jurisdiction matters)

All filters are linked — selecting any filter updates both the bar chart and the detail table simultaneously.

**Centre — Bar chart:**
- Horizontal bar chart, one bar per selected financial year
- Each bar coloured distinctly per year (using Persist's warm palette — slate blue, sage green, terracotta, amber, charcoal variants)
- X-axis: count of applications filed
- Y-axis: financial year labels
- Hovering a bar shows a tooltip: `FY 2023-24 · 14 applications · 8 TM · 4 Patent · 2 Design`
- Clicking a bar drills into that FY — the detail table on the right updates to show only that year's records
- Multi-select: Cmd+click multiple bars to compare years side-by-side in the detail table

**Right panel — Application Details table:**

Columns: Filed FY, Filed Quarter (Q1–Q4), Application Type, Application Number, Client, Attorney, Country, Status

Status column uses colour-coded badges (Persist design system):
- `Registered / Granted` → sage green badge with ✓
- `Filed / Pending` → slate blue badge
- `Under Examination` → amber badge
- `Opposed` → terracotta badge
- `Abandoned` → muted grey badge with ✗
- `Expired` → muted grey badge

Each row in the detail table is clickable — clicking opens the full IP Asset Record (Section 2.4) for that application.

**Export:** the entire analytics view (current filter state) can be exported as a PDF report or Excel spreadsheet.

---

#### 8.2 Operational Reports

**Docket Compliance Report:**
- % of deadlines met on time vs late vs missed, by attorney, by matter type, by time period
- Trend line: is compliance improving or declining quarter-over-quarter?
- Drill-down: click any attorney bar → see their individual deadline history

**Matter Aging Report:**
- All active matters grouped by how long they've been open: 0–30 days, 31–90 days, 91–180 days, 181–365 days, 365+ days
- Colour-coded — long-aged matters in amber/red as attention flags
- Filter by type and attorney

**Portfolio Status Summary:**
- Snapshot view of the entire IP portfolio by status — counts and percentages:
  - Trademarks: Filed / Under Examination / Advertised / Opposed / Registered / Abandoned
  - Patents: Filed / Published / Under Examination / Granted / Expired
- Visual: donut charts per IP type

**Deadline Forecast:**
- Rolling 90-day view of all upcoming deadlines grouped by week
- Bar chart: number of deadlines per week for the next 12 weeks
- Useful for capacity planning — spikes in a particular week are visible immediately

---

#### 8.3 Financial Reports

- Monthly revenue summary: billed vs collected, by month, trailing 12 months
- Invoice aging: outstanding invoices in 30 / 60 / 90 / 90+ day buckets
- Realisation rate: billed vs collected percentage by client and by matter type
- Attorney productivity: hours logged vs target per attorney per month
- Top clients by revenue: trailing 12-month revenue ranking

---

#### 8.4 Partner Dashboard Widgets

Live summary cards visible on the Firm Dashboard (landing page):

- `Overdue Dockets` — count with red indicator; click → opens Docket List filtered to overdue
- `Due This Week` — count of deadlines in the next 7 days
- `Renewals Due (12 months)` — count and estimated fees for TM and Patent renewals in the next 12 months
- `Stalled Matters` — matters where pipeline stage hasn't changed in longer than expected (Triangle IP bottleneck detection pattern)
- `Matters Inactive 14+ Days` — at-risk flag for matters with no attorney activity
- `Revenue This Month` — running total vs same period last month
- `Outstanding Invoices` — total unpaid amount
- `Top 5 Clients by Active Matters` — quick-access list

---

#### 8.5 Role-Specific Dashboards (Triangle IP Pattern)

Different users see different default dashboard views based on their role. The underlying data is the same; the lens changes.

**Partner dashboard** — firm-wide visibility: revenue, overdue dockets across all attorneys, stalled matters, top clients, monthly filing activity. Focused on oversight and business health.

**Associate / Attorney dashboard** — personal docket view: my deadlines this week, my open matters, my pending drafts, my time entries pending billing. Focused on personal workload and daily priorities. Feeds directly into Module 11's Daily Brief.

**Paralegal dashboard** — task-focused: assigned tasks across matters, documents awaiting filing, evidence to compile, deadlines assigned. No billing or revenue data.

**Client-view dashboard (client portal)** — matter status, documents to action, invoices pending payment. Entirely separate surface; read-only and client-scoped.

---

#### 8.6 IP Portfolio Geo-Map (Minuet pattern)

A **world map dashboard** showing the geographic distribution of the firm's IP portfolio. Each country where a matter is active or filed has a coloured dot or shaded region. Clicking a country filters the portfolio analytics to show only that jurisdiction's matters.

Primarily relevant for clients with multi-jurisdiction filings (PCT international phase, Paris Convention filings, international TM registrations under Madrid Protocol). As Persistas' client base grows and multi-jurisdiction work increases, this view provides at-a-glance portfolio geography.

Dimensions shown: count of filings per jurisdiction, types (TM/Patent/Design), status distribution (filed/pending/granted/expired).

---

#### 8.7 Sentinel-Style Automated Notifications (Minuet pattern)

Beyond the standard deadline reminder system (Section 2.5), Persist supports rule-based automated actions triggered by key database events — equivalent to Minuet's Sentinel Notifications system:

**Trigger events:**
- Matter status changes to a specific value
- Deadline is marked Complete
- Document is uploaded to a matter
- Invoice is marked Paid
- A field value changes (e.g. TM status changes from "Advertised" to "Registered")
- A date field is reached (e.g. TM application date + 10 years)

**Actions that can be triggered:**
- Send email automatically (to attorney, client, or custom address) with a template
- Create a new deadline automatically
- Add a remark to the matter timeline
- Create a task for a specific user
- Send WhatsApp message

Partners configure Sentinel rules in Settings. Example firm-wide rules:

```
IF  TM Status = "Examination Report Received"
THEN  Create deadline: Response due in 30 days
      Send WhatsApp to responsible attorney: "Examination Report received — response due in 30 days"
      Add remark to matter: "Examination Report received — auto-deadline created"

IF  TM Status = "Registered"
THEN  Send email to client: "Your trademark [Mark Name] has been registered..."
      Create deadline: First Renewal due in [Registration Date + 10 years]
      Add remark to matter: "Registration confirmed — renewal deadline auto-created"

IF  Patent annuity due date = TODAY + 180 days
THEN  Create task for responsible attorney: "Advise client on patent annuity renewal for [Matter]"
      Send email to client: "Annual maintenance fee advisory for [Patent Title]"
```

These rules eliminate the need for attorneys to manually remember to create follow-up deadlines or send routine client communications after milestone events.

---

#### 8.8 Report Export

All reports export to PDF, Excel (XLSX), and CSV. The IP Portfolio Analytics view also exports to PowerPoint (a pre-formatted slide with the chart and data table — useful for client portfolio review meetings). Export format toggles are available on every report view.

---

### Module 9: Document Drafting Suite (Litera Draft Equivalent — Firm Only)

This is Persist's equivalent of Litera Draft (formerly Litera Desktop) — a complete document drafting lifecycle tool built directly into the platform. Rather than relying on a separate, expensive third-party drafting tool, Persist consolidates the entire document creation, checking, comparing, cleaning, and publishing workflow in one place.

The drafting suite covers five functional pillars:

---

#### 9.1 Document Creation — Template Engine (Litera Create equivalent)

The foundation of the suite. Attorneys and paralegals should never start a document from a blank page.

**Template library:**
- Firm-approved templates for every document type: pleadings, agreements, IP prosecution letters, client advisories, court affidavits, power of attorney, notices, internal memos
- Templates are versioned and managed by the admin — associates always use the current approved version
- Templates support variable placeholders (matter name, client name, court name, date, opposing party, etc.) that auto-fill from the linked matter record
- Template packets: a single action triggers generation of a set of documents (e.g. "File TM Application" generates the application form, POA, and cover letter simultaneously, all pre-filled)
- Templates tagged by practice area, document type, jurisdiction

**Content library (clause library):**
- A searchable repository of pre-approved legal language blocks: standard clauses, boilerplate paragraphs, recitals, definitions, standard conditions
- Organised by practice area and document type
- Attorneys can search, preview, and insert clauses directly into any open document
- AI-suggested clauses: based on the document context, the AI (Module 7) can suggest relevant clauses from the library
- Clause versioning: old versions of clauses retained for reference; only the current approved version is insertable by default

**Document generation workflow:**
1. Attorney selects a template
2. System auto-fills all available fields from the linked matter
3. Attorney fills in any remaining fields via a guided intake form (no blank-page paralysis)
4. Document generated as DOCX, automatically saved to the matter's document store
5. Status set to `Draft` — moves through `Under Review` → `Approved` → `Sent / Filed`

---

#### 9.2 Document Proofreading & Repair (AI-powered)

Every document created in the platform is eligible for automated quality checking before it leaves the firm.

**Proofreading checks:**
- Spelling and grammar (legal-vocabulary-aware, not generic spell-check)
- Defined terms consistency: flags where a term is defined once but used differently elsewhere (e.g. "Agreement" vs "the agreement")
- Cross-reference integrity: checks that internal references like "Clause 4.2" or "Annexure A" actually exist in the document
- Numbering and formatting integrity: sequential clause numbering, consistent heading hierarchy, table alignment
- Date and party name consistency: flags mismatches in party names, dates, or court names across the document

**Document repair:**
- One-click fix for common structural issues: inconsistent spacing, mixed fonts, broken numbering sequences, misaligned indentation
- Style normalisation: applies the firm's approved document style (font, margins, paragraph spacing) to any uploaded or pasted document in a single action
- Remove conflicting formatting: cleans up documents that were copy-pasted from multiple sources with mixed styles

**Letterhead management:**
- Apply or remove Persistas letterhead at print time without modifying the underlying document
- Draft stamp: apply a "DRAFT — NOT FOR CIRCULATION" watermark for internal review copies, removed automatically when document status advances to Approved

---

#### 9.3 Document Comparison (Litera Compare equivalent)

Critical for contract negotiation, court filing revisions, and tracking changes between drafts.

**Core comparison:**
- Side-by-side or inline redline view comparing any two versions of a document
- Comparison of DOCX, PDF, and plain text formats
- Renders additions, deletions, moved text, and formatting changes distinctly
- Ignores insignificant whitespace and formatting-only changes (configurable)

**Advanced comparison features:**
- Batch comparison: compare multiple document pairs simultaneously (e.g. compare 10 executed contracts against a standard template in one action)
- AI comparison summary: after comparison, Claude generates a plain-English summary of the key substantive changes — what was added, removed, and materially altered — so attorneys can review the delta at a glance rather than reading the full redline
- Three-way comparison: original, your draft, counterparty draft — all three rendered together
- Comparison report: export a PDF summary of all changes with a change log and change statistics

**Integration with matter workflow:**
- Every comparison is saved to the linked matter with both source documents retained
- Comparison results can be forwarded to clients with a summary note

---

#### 9.4 Metadata Cleaning (Litera Metadact / Clean equivalent)

One of the highest-risk areas in legal document sharing. Metadata embedded in a DOCX or PDF can expose prior versions, author names, internal comments, revision history, and redacted content.

**Metadata cleaning capabilities:**
- Strip all hidden metadata before any document is sent externally: author names, tracked changes, comments, revision history, document properties, custom XML data, embedded paths
- Clean on send: when an attorney shares a document with a client or external party from within Persist, the system automatically applies metadata cleaning as a default step — attorney must actively override to send with metadata intact
- Batch clean: process a folder of documents before a court filing or deal closing
- PDF redaction: black out sensitive text permanently from PDFs (not just visually hidden — structurally removed so it cannot be copied or extracted)
- Inspection report: before cleaning, show the attorney exactly what metadata is present and what will be removed

**Sensitivity scanner:**
- Before sending any document externally, flag if it contains: tracked changes still visible, comments, a prior author name different from the sending attorney, any content marked as confidential in a prior version

---

#### 9.5 PDF Workflows (pdfDocs equivalent)

Legal work is ultimately published as PDFs. This sub-module handles the full PDF lifecycle.

**Core PDF capabilities:**
- Convert DOCX, XLSX, images, and HTML to PDF with formatting fidelity
- PDF editing: add text, annotations, stamps (FILED, EXECUTED, CONFIDENTIAL), and signature blocks
- PDF binding / bundling: compile multiple documents into a single paginated PDF bundle (court book / deal bible / client report) with a clickable table of contents auto-generated
- Page manipulation: reorder pages, insert pages from another PDF, extract specific pages, rotate, crop
- PDF forms: create fillable PDF forms (e.g. client intake forms, KYC forms) and extract submitted data into the matter record
- Optical character recognition (OCR): convert scanned PDFs into searchable, text-selectable documents

**Legal-specific PDF features:**
- Bates stamping: auto-number pages across a document set for litigation disclosure (format configurable: `P&P/2026/0001`)
- Exhibit stamping: mark pages as Exhibit A, B, C etc. for court bundles
- Pagination: add / reset page numbers across multi-document PDF bundles
- Redaction: court-grade permanent redaction of sensitive text and images

---

#### 9.6 Precedent Library & Knowledge Management

This is the institutional memory of the firm — making every good document ever drafted reusable.

**Precedent library:**
- Every finalised, approved document in the system is eligible to be saved as a precedent
- Precedents tagged by: document type, practice area, jurisdiction, outcome (won/lost/settled for litigation), client industry
- Searchable by keyword, clause text, matter type
- AI precedent search (Module 7): attorney describes what they need in plain language ("standard indemnity clause for SaaS agreement under Indian law"); AI surfaces the most relevant precedents from the firm's own library first, then suggests from general knowledge

**Knowledge management:**
- Partners can annotate precedents with guidance notes ("Use this version for FMCG clients; for pharma see version B")
- Precedents marked as `Current`, `Deprecated`, or `Jurisdiction-specific`
- Usage tracking: which precedents are used most, which have never been used

---

#### 9.7 Drafting Suite — Firm Portal Integration Points

The drafting suite is not a standalone tool — it is woven into every workflow in the platform:

- Creating a matter auto-suggests the relevant document templates to start
- Every deadline in the docketing engine can have a linked document template (e.g. the "TM Examination Response" deadline automatically offers the correct template)
- Document status changes (`Draft` → `Approved` → `Filed`) feed directly into the matter timeline
- All documents generated or processed through the suite are auto-saved to the linked matter's document store (Module 3)
- AI drafting assistant (Module 7) reads from and writes to the suite's template and clause libraries

---

#### 9.8 Smart Form Compiler — LaTeX-Powered Document Generation (Invisible to User)

This sub-module is the implementation of the guided-form-to-perfect-document concept. The attorney or paralegal fills a structured, intelligent form. A court-ready, perfectly formatted PDF is produced automatically. They never see LaTeX, never touch formatting, never worry about margins or heading styles — the engine is completely hidden.

**The user experience (what the attorney sees):**

A split-screen interface with two panels:

- **Left panel — Guided intake form:** A clean, structured set of fields relevant to the document type. Fields are pre-filled from the linked matter record wherever possible (client name, TM application number, filing date, class of goods, opposing party). Remaining fields are filled manually using date pickers, dropdowns, checkboxes, and free-text inputs.
- **Right panel — Live document preview:** The document renders in real time as fields are filled. The preview looks exactly like the final PDF — Persistas letterhead, correct margins, numbered clauses, page numbers, everything. The attorney is reviewing content, not formatting.

When ready: one click on **Generate PDF** produces the final court-ready document, saved automatically to the linked matter.

**What makes the form intelligent:**

- **Auto-fill from matter record:** Client name, matter number, jurisdiction, court name, opposing party, application number, filing date — all pulled silently from the matter record. Zero retyping.
- **Conditional fields:** The form adapts based on selections. Selecting "Trademark Opposition" as the document type shows different fields than "TM Examination Reply." Choosing "Grounds: Prior Use" reveals a date field for first use. Choosing "Grounds: No Likelihood of Confusion" hides the similarity sub-fields. The attorney only ever sees the fields relevant to their specific document.
- **Clause selection via checkboxes / dropdowns:** For documents with standard legal arguments (examination replies, oppositions, affidavits), the attorney selects which grounds or clauses apply — each selection injects the corresponding pre-approved legal paragraph into the document automatically. The attorney writes the unique facts; the standard legal language is inserted by the system.
- **Free-text zones:** For content that is always unique (description of goods/services, specific facts, evidence summary), a plain text area is provided. Character/word count guidance shown where courts have limits.
- **Field validation:** Inline, real-time validation. TM application number must be numeric and 7 digits. Hearing date cannot precede filing date. Required fields are highlighted before Generate PDF is enabled.
- **Multiple template variants:** For each document type, multiple approved variants may exist (e.g. "Standard C&D — Trademark Infringement", "Urgent C&D — Online Infringement", "C&D — Design Piracy"). The attorney selects the most relevant variant before the form loads.

**What happens behind the scenes (fully hidden from user):**

```
Attorney fills form → React state holds all field values
        ↓
On each field change → values sent to backend API
        ↓
Backend injects values into the .tex template
(placeholder variables: {{client_name}}, {{tm_number}}, etc.)
        ↓
LaTeX compiles to PDF (~1–2 seconds)
        ↓
PDF rendered in live preview panel
        ↓
[Generate PDF] → Final PDF saved to matter document store
```

The `.tex` templates are managed exclusively by the platform admin (Pratik). Attorneys never see, access, or edit them. When a court changes its format requirements, the admin updates the template once — every document generated thereafter uses the new format automatically. No attorney training required.

**Compile errors are handled silently:** If a LaTeX compile fails (e.g. a special character in a free-text field breaks the template), the backend sanitises the input and retries automatically. If it still fails, the user sees only: *"Preview temporarily unavailable — your content is saved."* Never a LaTeX error message.

**Template library for Smart Form Compiler (initial set for Persistas):**

*IP / Trademark:*
- TM Application Cover Letter
- Reply to Examination Report (TM Registry)
- TM Opposition Notice
- TM Opposition Counter-Statement
- Affidavit of Use / Non-Use
- Power of Attorney
- Assignment / Transmission Deed
- Cease & Desist Letter (Trademark Infringement)

*Patent:*
- Reply to First Examination Report (Patent Office)
- Request for Examination
- Patent Assignment Deed

*Litigation:*
- Court Affidavit (standard format)
- Vakalatnama
- Application / Interlocutory Application
- Written Statement

*Corporate / General:*
- Client Engagement Letter
- Status Update Letter
- Advisory Memo (standard format)
- NDA (mutual / one-way)

Each template is versioned, approved by a partner, and tagged with the applicable court/registry format and last-revised date. Associates can always see which version they are using.

---

### Module 10: Contract Intelligence Engine (Kira Equivalent — Firm Only)

This is Persist's equivalent of Kira — an AI-powered contract review and analysis engine designed for high-volume, high-stakes contract work. While Module 3 handles document storage and Module 9 handles document creation and drafting, Module 10 is about *reading and understanding* contracts at scale — extracting obligations, identifying risks, comparing clauses across document sets, and producing client-ready diligence outputs.

This module is most heavily used by the Corporate, IP licensing, and Litigation practice areas, and is the engine behind due diligence mandates, contract portfolio reviews, M&A support, lease reviews, employment agreement audits, and licensing deal analysis.

---

#### 10.1 Smart Field Extraction

The foundation of contract intelligence. Rather than reading a contract line by line, attorneys define what they need to find — and the engine finds it across every document in a project.

**Built-in smart fields (pre-trained):**

A library of pre-configured extraction models covering the most common legal provisions, trained on Indian and cross-jurisdictional contract language:

- Parties and execution: party names, signatories, execution date, governing law, jurisdiction, dispute resolution mechanism
- Commercial terms: contract value, payment terms, payment milestones, currency, price escalation clauses
- Duration and termination: commencement date, expiry date, renewal terms (auto-renewal clauses), notice periods, termination for convenience, termination for cause
- IP and ownership: IP ownership provisions, assignment restrictions, licensing scope (exclusive / non-exclusive), sublicensing rights, work-for-hire clauses
- Liability and indemnity: limitation of liability clauses, indemnity obligations, insurance requirements, warranties, representations
- Confidentiality: NDA provisions, confidentiality obligations, carve-outs, survival clauses
- Change of control: change of control triggers, assignment on change of control, right of first refusal on transfer
- Employment-specific: non-compete clauses, non-solicitation provisions, garden leave, severance terms, bonus and equity provisions
- Real estate / lease: rent amounts, rent escalation, CAM charges, break clauses, fit-out obligations, security deposit terms
- Regulatory and compliance: data protection obligations (PDPA / GDPR references), anti-bribery clauses, sanctions provisions, force majeure

**Generative smart fields (custom — no training required):**
- Attorneys create custom extraction fields by writing a natural language prompt: e.g. "Extract any clause that restricts the firm's ability to act for competing clients"
- No coding, no training cycles — the AI interprets the prompt and searches all documents in the project
- Covers edge cases and jurisdiction-specific provisions not in the built-in library
- Results delivered with source citations (exact page and clause reference)

---

#### 10.2 Contract Review Projects

All contract intelligence work is organised around **Projects** — a structured workspace for a specific review mandate.

**Project object:**
- Project name and type (M&A due diligence / lease review / employment audit / IP licensing review / contract portfolio audit / bespoke)
- Linked matter (the project feeds into the parent matter record)
- Document set: all contracts uploaded to this project for analysis
- Smart fields activated for this project (selected from built-in library + any custom fields)
- Assigned reviewers (which associates / paralegals are working on this project)
- AI governance toggle: per-project control to enable or disable generative AI features (for clients with outside counsel guidelines restricting GenAI usage)
- Status: `Setup` → `In Review` → `QC` → `Delivered`
- Deadline: project delivery date with countdown

**Document ingestion:**
- Accepts DOCX, PDF (including scanned — OCR applied automatically), and plain text
- Automatic document type classification: NDA, shareholder agreement, employment contract, lease, licence agreement, loan agreement, service agreement, power of attorney — identified on upload
- Automatic grouping: contracts and their amendments / side letters / schedules are grouped together
- Language detection: identifies contracts in languages other than English (Hindi, regional languages, foreign counterparty contracts); flags for manual review or AI-assisted translation summary
- Bulk upload: drag-and-drop of hundreds of documents; ingestion runs in the background

---

#### 10.3 Analysis Chart (Grid-Based Review Interface)

The primary working view for attorneys reviewing a document set. Replaces spreadsheet-based diligence trackers entirely.

**Grid layout:**
- Rows = documents / contracts in the project
- Columns = smart fields being extracted (parties, governing law, termination notice, limitation of liability, etc.)
- Each cell = the extracted clause text, with a confidence indicator
- Attorneys can click any cell to see the full extracted clause in context, with the source document highlighted

**Review workflow:**
- Attorneys mark each extraction as `Confirmed`, `Needs Review`, or `Override` (if the AI extraction is wrong, attorney corrects it manually — the correction improves the model over time)
- Flag cells: mark specific clauses as `Risk`, `Negotiation Point`, `Unusual`, `Missing` for inclusion in the diligence report
- Filter and sort: view only flagged items, only high-risk clauses, only documents missing a specific provision
- Bulk actions: apply a flag or status to multiple documents simultaneously

**Concept search:**
- Search for a legal concept across all documents in the project using a single example clause or a plain-language description
- Instantly surfaces all documents containing similar or identical language
- Used for: issue spotting ("which contracts have a unilateral variation right?"), clause comparison ("show me all the limitation of liability caps across this portfolio"), rapid consistency checking

**Rapid Clause Analysis:**
- For a selected provision type, instantly identify identically drafted clauses across the entire document set
- Flag deviations from standard language — useful for checking whether non-standard terms have crept in across a portfolio
- Group contracts by clause similarity to identify which were drafted from the same template

---

#### 10.4 AI Chat with Contracts

Rather than searching for a specific clause, attorneys can interrogate the entire document set — or a single contract — in natural language.

**Document-level chat:**
- Open any contract and ask questions: "What are the termination rights of the licensor?", "Does this agreement restrict assignment?", "What happens on insolvency of either party?"
- Answers delivered with linked citations: every response highlights the exact clause in the source document
- Follow-up questions supported within the same session: full conversational context maintained

**Project-level chat:**
- Ask questions across all documents in a project simultaneously: "Which contracts in this data room do not have a limitation of liability?", "List all contracts where the governing law is not Indian law", "Which employment agreements have non-compete clauses exceeding 12 months?"
- Ideal for senior partner review — quickly orient on the risk landscape without reading every document
- Answers cite which documents support each finding

**Governance:**
- If AI governance is toggled off for a project, chat is disabled; only traditional smart field extraction is available
- All chat queries and AI responses are logged per session for audit purposes

---

#### 10.5 Smart Summaries & Diligence Reports

The output layer — converting raw extractions into client-ready deliverables.

**Smart summaries:**
- For any contract or clause, generate a plain-English summary of the key extracted provisions
- Summary length configurable: executive summary (3–5 bullet points) or detailed summary (clause-by-clause narrative)
- Summaries auto-populate the first draft of a diligence memo — attorneys edit and approve, not write from scratch
- Clause-level summaries: for a specific provision type (e.g. all change-of-control clauses), generate a comparative summary across all documents in the project

**Diligence report generation:**
- One-click generation of a structured diligence report from the analysis chart
- Report structure: executive summary → key risks flagged → clause-by-clause findings by category → document index
- Output formats: DOCX (editable, flows into Module 9 drafting suite for final formatting) and PDF
- Report includes: document name, clause reference, extracted text, attorney flag, and attorney commentary for each finding
- Branded with Persistas letterhead (via Module 9.2 letterhead management)

**Client memo generation:**
- Shorter, client-facing format: key findings only, no raw clause extracts
- Plain language, non-technical tone
- Generated from the same analysis data as the diligence report, reformatted for client readability

---

#### 10.6 Contract Portfolio Management

Beyond project-based reviews, this sub-module provides ongoing visibility into the firm's clients' entire contract portfolios — turning one-time review into a continuous intelligence service.

**Portfolio view:**
- All contracts ever reviewed for a client, stored and indexed in Persist
- Filter by: agreement type, expiry date, governing law, renewal status, risk flag
- Expiry and renewal tracker: automatic alerts when contracts are approaching expiry, renewal notice deadlines, or auto-renewal trigger dates — fed directly into the deadline engine (Module 2)
- Obligation tracker: extract ongoing obligations from contracts (e.g. annual reporting obligations, milestone payments, regulatory filings) and create deadline reminders for each one

**Risk dashboard (per client or per portfolio):**
- Visual summary of risk concentration: how many contracts have unlimited liability, no cap on damages, unilateral termination rights in favour of counterparty, unusual IP assignment provisions
- Trend view: how the risk profile of a client's contract portfolio changes over time as new agreements are added

**Ongoing monitoring:**
- When a client uploads a new contract for any matter, the system can automatically run the standard smart field extraction and flag it for attorney review
- Attorneys are alerted if a new contract deviates materially from the client's standard terms

---

#### 10.7 Contract Intelligence — Integration Points

Module 10 is tightly integrated across the rest of the platform:

- Every contract review project is linked to a parent matter (Module 1), so diligence work is tracked, billed, and visible in the matter timeline
- Extraction results feed the deadline engine (Module 2): expiry dates, renewal windows, and obligation dates extracted from contracts automatically create docketed deadlines
- Completed diligence reports are saved to the matter's document store (Module 3) and are eligible to become precedents (Module 9.6)
- Time spent on contract review is billable through the time tracking module (Module 4): project-level time logging, with entries tagged to the review project
- AI drafting assistant (Module 7) can read the smart field extractions for a contract and use them as context when drafting a negotiation response, a termination notice, or a contract amendment
- Client portal (Module 5): clients with a portfolio management subscription can view their contract expiry calendar and risk dashboard — a powerful value-add service

### Module 10: Contract Intelligence Engine (Kira Equivalent — Firm Only)

This module is Persist's equivalent of Kira — an AI-powered contract review and analysis engine purpose-built for the volume and types of contracts Persistas handles: M&A due diligence, commercial agreements, IP licensing, employment contracts, real estate leases, and corporate/commercial matters. Rather than subscribing to Kira at significant cost, Persist embeds equivalent intelligence directly into the platform using Claude's API with a structured legal extraction layer on top.

The distinction between this module and the AI Drafting Assistant (Module 7) is important: Module 7 creates and checks documents the firm is *producing*. Module 10 analyses contracts and documents the firm is *receiving* — from clients, counterparties, data rooms, and courts.

---

#### 10.1 Smart Field Extraction (Clause & Data Point Extraction)

The heart of the contract intelligence engine. When a contract or set of contracts is uploaded, the system automatically reads through every document and extracts the key provisions, obligations, dates, and risk factors — without the attorney having to read line by line.

**Pre-built extraction fields (mapped to Indian legal context):**

*General / Cross-practice:*
- Parties (names, roles, governing entities)
- Effective date, expiry date, renewal terms
- Governing law and jurisdiction clause
- Dispute resolution mechanism (arbitration / litigation / mediation; seat; rules)
- Termination rights: for cause, for convenience, notice periods
- Liability cap and limitation of liability clauses
- Indemnification obligations (who indemnifies whom, for what)
- Force majeure clause (presence, scope, carve-outs)
- Assignment and change of control provisions
- Confidentiality / NDA obligations and duration
- Most favoured nation (MFN) clauses
- Non-compete and non-solicitation
- Representations and warranties

*IP / Licensing:*
- IP ownership provisions (who owns what is created)
- License grant (scope, exclusivity, territory, field of use)
- Royalty rates, milestone payments, audit rights
- Background IP vs foreground IP definitions
- IP infringement indemnity
- Technology transfer obligations
- Publication restrictions

*Corporate / M&A:*
- Purchase price and adjustment mechanisms
- Representations and warranties (seller / buyer)
- Material adverse change (MAC) definitions
- Closing conditions
- Earn-out provisions
- Escrow arrangements
- Non-compete / non-solicitation (post-closing)
- Indemnification caps and baskets
- Data room reference obligations

*Employment:*
- Compensation and benefits
- Notice period and garden leave
- Post-termination restrictions
- IP assignment by employee
- Confidentiality obligations

*Real estate / Lease:*
- Rent amount, escalation clause
- Lease term, renewal options
- Security deposit terms
- Permitted use and exclusivity
- Fit-out obligations and reinstatement
- Break clauses

**Custom smart fields:**
- Attorneys can define their own extraction fields using a plain-language prompt — no coding, no training cycle needed (Claude interprets the field definition and applies it to all documents in the project)
- Custom fields saved to the firm's field library for reuse across future matters
- Fields tagged by practice area and matter type for quick application

---

#### 10.2 Contract Review Workflow

A structured, end-to-end workflow for reviewing a contract set — from upload to client-ready report.

**Project-based structure:**
- Every contract review is organised as a **Review Project** linked to a matter
- A project contains one or many documents (individual contract, or a full data room)
- Project types: Due Diligence, Single Contract Review, Portfolio Review, Compliance Review
- Each project has an assigned reviewer and approver

**Review workflow stages:**
1. **Upload** — drag-and-drop single or bulk documents (DOCX, PDF, scanned PDF with OCR)
2. **Auto-extraction** — smart fields run automatically on upload; results visible in seconds
3. **Triage** — attorney reviews extracted fields, confirms or corrects each extraction, adds notes
4. **Issue flagging** — attorney marks specific clauses as: `Acceptable`, `Negotiation Point`, `Risk — Escalate`, `Missing — Needs to be Added`, `Non-standard`
5. **Collaboration** — review tasks can be assigned to associates; partner reviews and approves final positions
6. **Report generation** — one-click generation of a client-ready diligence summary or issue report (see 10.5)

**Rapid Clause Analysis:**
- Across a multi-document project (e.g. 50 supplier contracts), instantly identify which documents share identically or near-identically drafted clauses for a given field
- Useful for standardisation analysis: "How many of our supplier contracts have a force majeure clause that excludes pandemic events?"
- Cluster view: documents grouped by clause similarity so the attorney can review one representative and apply a position to all similar ones

**Concept search:**
- Search across all documents in a project using a plain-language concept, not just a keyword
- Example: "Find all provisions that restrict the company from working with competitors" — surfaces relevant clauses even if the word "competitor" never appears
- Results ranked by relevance with the exact passage highlighted and linked to the source document

---

#### 10.3 Risk Scoring & Issue Spotting

After extraction, the system assigns a risk profile to each reviewed contract and surfaces the most important issues without the attorney having to read everything.

**Contract-level risk score:**
- Overall risk rating: `Low`, `Medium`, `High`, `Critical` — based on the number and severity of flagged clauses
- Risk breakdown by category: liability, IP, termination, competition, compliance
- Risk score rationale: plain-English explanation of what drove the rating ("Liability cap is below the contract value; force majeure clause is unusually broad; no audit rights on royalty payments")

**Issue flags (pre-configured for Indian legal context):**
- Unlimited liability exposure (no liability cap, or cap is too low relative to contract value)
- Unilateral termination rights for the counterparty
- IP ownership ambiguity (no clear assignment; could be interpreted as licensor retaining all rights)
- Governing law outside India with no submission to Indian jurisdiction
- Automatic renewal clauses with short opt-out windows
- One-sided indemnity obligations
- Overly broad non-compete restricting firm's clients' business post-contract
- Missing standard protective clauses (e.g. no confidentiality clause in a commercially sensitive agreement)
- Change of control triggers that affect the client adversely

**Comparative risk (portfolio view):**
- When reviewing multiple contracts of the same type, rank them by risk score side by side
- Highlight which contracts deviate from the firm's standard positions
- Export risk ranking as a summary table for client presentation

---

#### 10.4 AI Chat with Contracts

Attorneys can have a natural-language conversation with any contract or document set in a review project — getting instant answers without manually reading through the document.

**Capabilities:**
- Ask plain-language questions: "What is the notice period for termination for convenience?", "Does this agreement allow the client to sublicense?", "What happens to IP if the contract is terminated early?"
- AI responds with a concise, direct answer and a citation pointing to the exact clause and page in the source document
- Multi-document chat: "Across all 30 contracts in this project, which ones have automatic renewal clauses?" — AI scans all documents and returns a structured answer
- Follow-up questions in context: the conversation maintains awareness of previous questions in the session
- Every AI response includes: the answer, the exact quoted passage from the document, the document name and page number

**Governance controls:**
- AI chat can be toggled on or off per review project (for clients who have outside counsel guidelines restricting GenAI use)
- All AI chat sessions are logged: question, answer, source citation, attorney who asked, timestamp
- AI responses are always displayed with a `Verify — AI-generated` badge until the attorney marks the answer as confirmed

---

#### 10.5 Automated Report Generation

The final deliverable of every contract review is a structured report for the client or for internal records. Persist automates this entirely.

**Report types:**

*Due Diligence Summary Report:*
- Executive summary: overall risk assessment and key findings in plain English
- Issues table: clause-by-clause breakdown of every flagged issue, with risk level, the exact extracted language, and the recommended position
- Missing provisions: list of standard clauses not found in the reviewed documents
- Acceptable provisions: list of clauses reviewed and found to be within acceptable parameters (confirms what was reviewed, builds confidence)
- Formatted as a professional DOCX or PDF, Persistas branded, ready to send

*Issues Report (for negotiation):*
- Focused solely on the clauses that need to be negotiated
- Each issue listed with: current language, why it's problematic, and the firm's suggested replacement language
- Structured as a negotiation aide-mémoire for the attorney going into discussions

*Portfolio Comparison Report:*
- For multi-contract projects: a matrix showing each contract against each extracted field
- Highlights outliers and non-standard provisions across the portfolio
- Useful for contract standardisation projects, supplier audits, and compliance reviews

**AI-assisted report drafting:**
- AI (Module 7) drafts the narrative sections of the report (executive summary, issue explanations) based on the extracted fields and flagged issues
- Attorney reviews and edits the draft before finalising
- All AI-drafted content marked as draft until attorney approves

---

#### 10.6 Document Intelligence Beyond Contracts

The same extraction and analysis engine applies to non-contract legal documents:

- **Court orders and judgments**: extract parties, court, date, relief granted, compliance obligations, appeal deadlines
- **Regulatory notices and show-cause notices**: extract issuing authority, allegations, response deadline, applicable regulations
- **IP prosecution documents (Examination Reports)**: extract objections raised, required amendments, statutory basis, response deadline — automatically pre-populates the linked docketing deadline
- **Employment documents**: extract key terms, probation period, notice requirements
- **Lease documents**: extract rent, term, break clauses, deposit, maintenance obligations

This means that when a client uploads an examination report or a court order into a matter, Persist can automatically read it, extract the key obligations, and pre-populate the relevant deadlines in the docketing engine (Module 2) — turning a document upload into an automatic deadline creation event.

---

#### 10.7 Contract Intelligence — Integration Points

- A contract uploaded to any matter can be sent to the Contract Intelligence Engine with one click
- Extracted fields auto-populate the matter record (e.g. contract expiry date becomes a docketing deadline)
- Issue flags from a review can be converted directly into matter tasks with assigned attorneys
- Completed due diligence reports are saved to the matter's document store (Module 3)
- The firm's smart field library grows over time: every custom field created is available for future matters
- AI chat sessions with documents are saved to the matter audit log

---

### Module 11: Daily Intelligence & Workday Organiser (Littlebird Equivalent — Firm Only)

This module is Persist's equivalent of Littlebird — a context-aware daily intelligence layer that sits across every other module in the platform and helps each attorney organise, prioritise, and execute their workday without having to hunt through multiple screens.

The problem it solves: a senior associate at Persistas might have 15 active matters, 30+ open tasks, 8 upcoming deadlines, 4 pending document approvals, and 3 client meetings in a given week. Without an intelligent organiser, they open the platform and face a wall of data. Module 11 reads everything Persist already knows about that attorney — their matters, their deadlines, their document actions, their time logs, their calendar, their meeting notes — and presents a single, clear, personalised view of what matters right now.

---

#### 11.1 Daily Brief

The first thing every attorney sees when they log in each morning is their **Daily Brief** — a single-screen, AI-assembled summary of their entire day, built from live platform data.

**What the Daily Brief contains:**

- **Today's priority actions** — the 3–5 most important things the attorney needs to do today, ranked by urgency (deadline proximity, matter priority, escalations)
- **Upcoming deadlines** — all deadlines due in the next 7 days, colour-coded by urgency, with one-click access to the relevant matter and document
- **Pending approvals** — documents waiting for their review or signature
- **Matter activity overnight** — anything that changed while they were away: new client uploads, documents filed by colleagues, new court orders, messages from clients
- **Today's meetings** — pulled from the integrated calendar, with AI-prepared context for each (see Section 11.3)
- **Time tracking nudge** — if yesterday's hours were not logged, a gentle prompt to complete them before starting today

**Example Daily Brief for a senior associate:**

```
Good morning, Meera.  —  Thursday, 9 April

━━━ TODAY'S PRIORITIES ━━━━━━━━━━━━━━━━━━━━━

⚠  TM Examination Reply — Zenith Brands          DUE IN 2 DAYS
   Draft is complete. Awaiting partner approval.
   → Nudge Sree Lakshmi →

◈  Contract Review — Prism Technologies          REVIEW PENDING
   3 flagged clauses need your decision before report.
   → Open review →

◈  Affidavit — Highgate Foods matter              DUE TOMORROW
   Not started. Smart form ready to fill.
   → Open form →

━━━ TODAY'S MEETINGS ━━━━━━━━━━━━━━━━━━━━━━━

11:00 AM   Rahul Industries — matter status call
           → 3 active matters. Invoice overdue 28 days.
              Exam report received yesterday — unread.
           Prep summary ready →

3:30 PM    Internal — TM portfolio review
           → 6 matters in opposition window

━━━ PENDING YOUR ACTION ━━━━━━━━━━━━━━━━━━━━

  2 documents awaiting your approval
  1 time entry from Tuesday incomplete
  4 tasks assigned to you across 3 matters

━━━ THIS WEEK ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Hours logged: 8.5 / 40 target
  Deadlines this week: 4 (2 complete, 2 pending)
```

Every item is a live link. The attorney clicks into the action directly — no navigation required.

---

#### 11.2 Smart Task Management

Tasks in Persist exist across multiple modules — docketing deadlines (Module 2), document approvals (Module 3), review project triage (Module 10), billing actions (Module 4). Module 11 unifies all of them into a single, intelligent task surface.

**Task views:**
- **My Day** — tasks the AI has selected as most important for today, ranked and explained ("This is ranked first because the deadline is in 48 hours and the document is not yet drafted")
- **My Week** — full week view, with tasks distributed across days, capacity-aware (if Thursday already has 6 high-effort tasks, new tasks are suggested for Friday)
- **All Tasks** — complete list with filter by matter, practice area, priority, due date, status
- **Team View (Partner/Manager only)** — all tasks across all associates, with capacity heatmap showing who is overloaded and who has bandwidth

**Task intelligence:**
- **Auto-generated tasks:** when a new deadline is created in the docketing engine, Persist automatically generates the prerequisite tasks in reverse order (e.g. TM response due in 30 days → auto-creates: "Draft response" due day 20, "Partner review" due day 25, "File response" due day 29)
- **Dependency chains:** mark a task as blocked by another; the blocked task is greyed out until the dependency is resolved
- **Effort estimation:** when creating a task, attorney sets estimated hours; Module 11 uses this to flag capacity conflicts
- **Task templates:** for recurring matter types, save a standard task sequence as a template (e.g. every new TM application matter auto-creates the same 12-task checklist on opening)
- **AI task suggestions:** when a new document is uploaded or a new stage is reached in a matter, the AI suggests what the next action should be ("Examination report received — suggest creating task: Draft Reply, assign to: [associate], due: [28 days from today]")

---

#### 11.3 Meeting Intelligence & Preparation

Every meeting linked to a matter gets an AI-prepared context brief — so the attorney walks in knowing exactly where things stand, without spending 20 minutes reading back through notes.

**Meeting prep brief (auto-generated before each meeting):**

For a client meeting, the brief includes:
- All active matters for that client: current stage, last action, next deadline
- Any unread documents or unreviewed uploads from the client
- Outstanding invoices and payment status
- Last communication date and what was discussed (from meeting notes)
- Any flags or escalations on their matters
- A suggested agenda based on the above

For an internal matter review, the brief includes:
- Matter summary: current stage, all open tasks, upcoming deadlines
- Team assignments: who is responsible for what
- Any bottlenecks or overdue actions
- Document status: drafts in progress, approvals pending

**Meeting notes and action items:**
- During or after the meeting, the attorney can open a meeting note directly in Module 11
- Notes are linked to the matter
- After saving, the AI reads the notes and extracts action items — converting them directly into tasks assigned to the named attorneys, with suggested due dates
- Example: attorney notes "Sree to send draft NDA to Rahul Industries by next Friday" → AI creates a task: "Send draft NDA — Rahul Industries", assigned to Sree Lakshmi, due next Friday, linked to the matter

---

#### 11.4 Routines — Scheduled Intelligence Briefings

Inspired directly by Littlebird's Routines feature. Attorneys configure a set of automated intelligence reports that Persist generates and delivers on a schedule — without the attorney having to pull any reports manually.

**Pre-built routines (available from day one):**

- **Daily Morning Brief** (delivered at login or by email at 8:00 AM): Today's priorities, deadlines, meetings, pending actions — as described in 11.1
- **End-of-Day Summary** (delivered at 6:00 PM or on logout): What was completed today, what is still open, time logged vs target, anything that moved from green to amber urgency
- **Weekly Work Summary** (delivered Monday morning): Last week's completed matters, time logged, deadlines met vs missed, top 5 upcoming deadlines this week, billing summary
- **Monday Deadline Radar** (delivered Monday AM): Every deadline in the next 30 days across all the attorney's matters, sorted by urgency — the full forward visibility view

**Custom routines:**
- Any attorney can define their own routine with a plain-language instruction: "Every Friday at 5 PM, show me all matters where no action has been taken in the last 7 days."
- Partners can create firm-wide routines: "Every Monday, generate a practice area health report: open matters, deadline compliance last week, hours logged by associate."

---

#### 11.5 Focus Mode

A single-task, distraction-free work mode for when the attorney needs to concentrate on one thing.

- Attorney selects one task or matter to focus on
- The interface strips away all other notifications, pending items, and sidebar alerts
- A focus timer runs (Pomodoro-style: 25 or 50 minutes, configurable)
- Time is automatically logged against the linked matter while Focus Mode is active
- At the end of the focus session, a prompt: "Did you complete this task? Add a note?" — one click to update the task status and log the time entry

---

#### 11.6 Daily Intelligence — Integration Points

Module 11 reads from every other module in Persist but does not store its own separate data:

- **Matter data (Module 1):** matter status, priority, last activity — surfaces stale matters in daily brief
- **Deadline engine (Module 2):** all deadline dates and urgency — drives priority ranking in My Day and Daily Brief
- **Document store (Module 3):** pending approvals, unreviewed uploads — surfaces in Daily Brief
- **Billing (Module 4):** time logged vs target, unbilled entries — drives daily/weekly time nudges
- **Notifications (Module 6):** escalations and alerts — folded into Daily Brief rather than shown as separate pings
- **Calendar integration:** Google Calendar / Outlook calendar events pulled into meeting intelligence
- **AI assistant (Module 7):** meeting prep briefs drafted by Claude; action item extraction from meeting notes
- **Contract reviews (Module 10):** pending review decisions surface as priority tasks in My Day

The Daily Brief is not a separate page to navigate to — it is the **default landing screen** of the firm portal for every attorney. The first thing they see, every time they log in.

---

### Module 12: Client iOS / iPadOS App

The client iOS/iPadOS app gives Persistas' clients a native mobile experience for checking their matters, downloading documents, and managing invoices — on iPhone and iPad. It mirrors the client web portal exactly in terms of capability, but feels native on iOS rather than a mobile browser experience.

This is a **client-only app**. Firm users (attorneys, associates, paralegals) use the **Persist Desktop** application (Section 8.1). There is no firm iOS app — the desktop app handles everything for internal users, including full offline capability.

**Implementation:** Tauri v2 mobile target (Option A) — the same React codebase used for the client web portal, compiled for iOS via `npx tauri ios build`. Ships as a standalone app on the App Store. If native feel demands it in a later phase, the Deck layer can be replaced with SwiftUI while Keel (Rust) remains intact.

**Distribution:** Public App Store listing under Persistas & Partners branding.

---

#### 12.1 Design Philosophy

- Calm, premium feel matching the Persist design system (Section 14)
- Read-heavy — clients check, they don't manage
- Fast load: matter status should be visible within 2 seconds of opening the app
- iPad layout: split view — matter list on left, detail on right

---

#### 12.2 App Structure — 4 Tabs

```
My Matters  |  Documents  |  Invoices  |  Profile
```

**Tab 1 — My Matters**
- All active matters with status badge and last update
- Matter detail: visual workflow progress bar, responsible attorney + contact, last firm note, upcoming milestones, pending actions required from client

**Tab 2 — Documents**
- Documents shared by the firm with the client
- In-app PDF viewer (read-only)
- Upload: camera or Files app for documents requested by the firm (POA, evidence, KYC)
- Download to Files app

**Tab 3 — Invoices**
- All invoices with status badges (Draft / Sent / Paid / Overdue)
- Invoice detail: line items, total, GST breakdown, due date
- PDF download
- Dispute: flag with a note → notifies firm attorney immediately

**Tab 4 — Profile**
- Contact details, notification preferences, log out

---

#### 12.3 iOS-Specific Features

- **Face ID / Touch ID:** biometric authentication on every app open
- **Push notifications (APNs):** matter status update, new document shared, invoice sent, action required from client
- **Lock Screen widget:** active matters count + any pending client action
- **Offline:** previously loaded matter data and documents cached locally for reading

---

#### 12.4 Phase Assignment

**Phase 7 (post-launch):** Full client iOS/iPad app on the App Store, once the client web portal is stable and the client experience is validated.

---

## 7. Dual Dashboard Design Principles

| Dimension | Firm Portal | Client Portal |
|---|---|---|
| Primary metaphor | Command centre — everything visible, dense, actionable | Status window — clean, reassuring, limited |
| Navigation | Full sidebar with all modules | Top nav with 4 items: Matters, Documents, Invoices, Profile |
| Colour language | Neutral / functional (data-dense) | Persistas brand colours (trust, premium) |
| Information density | High — tables, filters, bulk actions | Low — cards, timelines, simple status |
| Default landing | Daily Brief (Module 11) — AI-assembled priorities, deadlines, meetings | Matter overview + pending actions |
| Action capacity | Create, edit, delete, assign, bill | View, download, upload, dispute |
| Notifications | Real-time, all matter types | Targeted — only what affects the client |
| AI tools | Full drafting assistant | None (future: AI Q&A about their matter) |
| Billing visibility | All rates, time entries, disbursements, margins | Only final invoice + payment status |

---

## 8. Technical Architecture

### 8.0 Architecture Overview — Three-Surface Platform

Persist is built across three distinct surfaces, each purpose-built for its users:

```
┌─────────────────────────────────────────────────────────────────┐
│  PERSIST DESKTOP — Firm App                                      │
│  Tauri v2 — Keel (Rust backend) + Deck (React frontend)         │
│  Target: macOS + Windows (attorneys, associates, paralegals)     │
│  Data: Local-first — SQLite on device, optional cloud sync      │
│  All 21 modules — full capability                                │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  PERSIST WEB — Client Portal                                     │
│  React (TypeScript) + FastAPI backend                           │
│  Browser-only: no installation required for clients             │
│  Target: Persistas' clients — individuals + corporates          │
│  Scoped capability: matter status, documents, invoices          │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  PERSIST iOS/iPadOS — Client Mobile App                         │
│  Tauri v2 mobile target — same React + Rust codebase as web     │
│  OR separate Swift/SwiftUI if native feel preferred             │
│  Target: clients checking matters on the go                     │
│  Mirrors client web portal — same scoped capability             │
└─────────────────────────────────────────────────────────────────┘
```

The **sync layer** connects the three surfaces: the desktop app holds the canonical data store (SQLite); a lightweight Rust-based sync server on Hetzner propagates changes to the web client portal and iOS app, and receives client uploads/actions back to the desktop.

---

### 8.1 Persist Desktop — Tauri v2 (Keel + Deck)

Persist Desktop is a native desktop application built on **Tauri v2** — the Rust + React desktop framework. Within the project, the two layers are named by convention:

- **Keel** = the Rust backend (`src-tauri/`) — the structural spine; handles everything system-level, performance-critical, and security-sensitive
- **Deck** = the React frontend (`src/`) — what the attorney sees and interacts with; built in React 19 (TypeScript) with Vite and the full Persist design system (Section 14)

These are internal naming conventions for clarity within the Persist codebase — not a third-party framework dependency.

**Why Tauri v2 over Electron for a law firm:**

| Concern | Electron | Tauri v2 |
|---|---|---|
| Binary size | 150–300 MB (ships full Chromium) | ~12–18 MB (uses OS native WebView) |
| Memory usage | 400–800 MB at runtime | 80–150 MB at runtime |
| Client privilege | Data can sit on cloud servers | Data lives on the attorney's machine |
| LaTeX compilation | Server round-trip (~2s) | Local, instant (~200ms) |
| PDF processing | Upload → process → download | Direct file system access, in-memory |
| Security model | Node.js process, broad permissions | Tauri capability system — fine-grained, deny-by-default |
| Rust compile-time safety | ❌ | ✓ — Rust compiler catches errors at build time |
| iOS/Android from same codebase | ❌ (Electron is desktop only) | ✓ — Tauri v2 targets iOS and Android natively |

**Keel — Rust backend responsibilities:**

The Keel layer is an async Rust application using **Tokio** for the async runtime. It handles every operation that benefits from native OS access, performance, or security isolation:

```
keel/
├── commands/           # Tauri command handlers — the IPC bridge to Deck
│   ├── matters.rs      # Matter CRUD, status, assignment
│   ├── deadlines.rs    # Deadline engine, reminders, urgency calculation
│   ├── documents.rs    # Document store, versioning, metadata cleaning
│   ├── billing.rs      # Time entries, invoice generation
│   ├── pdf.rs          # Full PDF engine: OCR, markup, compile, compare
│   ├── latex.rs        # LaTeX compilation pipeline — invisible to users
│   ├── chat.rs         # Persist Chat context assembly + Claude API calls
│   ├── mail.rs         # IMAP/SMTP — Outlook Graph API, Gmail sync
│   ├── ai_drafting.rs  # Document generation from templates
│   ├── contracts.rs    # Contract intelligence extraction pipeline
│   └── sync.rs         # Sync layer — push/pull with Hetzner sync server
│
├── db/                 # SQLite via sqlx — embedded, local-first
│   ├── migrations/     # Schema migrations
│   └── queries/        # Type-safe SQL queries
│
├── storage/            # Local file system operations
│   ├── documents.rs    # Document store on local disk — AES-256 encrypted
│   ├── templates.rs    # LaTeX .tex template management
│   └── cache.rs        # Context cache for Persist Chat
│
├── services/           # Long-running background services
│   ├── deadline_watcher.rs  # Polls deadlines, fires OS notifications
│   ├── mail_sync.rs         # Background email sync daemon
│   ├── auto_updater.rs      # Tauri auto-update service
│   └── context_builder.rs   # Assembles matter context for Persist Chat
│
└── lib.rs              # Tauri builder — registers all commands, plugins
```

**Deck — React frontend responsibilities:**

The Deck layer is a React 19 + TypeScript application, bundled with Vite, styled with the Persist design system (Section 14). It handles all rendering, state management, and user interaction — communicating with Keel exclusively through Tauri's `invoke()` IPC mechanism:

```
deck/
├── pages/              # Top-level route pages
│   ├── Today.tsx       # Daily Brief (Module 11.1)
│   ├── Matters/        # Matter list + detail
│   ├── Tasks.tsx       # Smart Task Management (Module 11.2)
│   ├── Documents/      # Document store + PDF viewer
│   ├── Billing/        # Time tracking + invoices
│   ├── Mail.tsx        # Integrated mail module (Module 15)
│   ├── Chat.tsx        # Persist Chat floating overlay (Module 21)
│   ├── Drafting/       # Smart Form Compiler (Module 9.8)
│   ├── Contracts/      # Contract Intelligence (Module 10)
│   ├── Research/       # Legal Intelligence Layer (Module 18)
│   └── Settings.tsx
│
├── components/         # Shared UI components
│   ├── pdf/            # PDF viewer, markup tools, shared sessions
│   ├── forms/          # Smart intake forms (LaTeX-backed)
│   ├── chat/           # Chat pane, Hummingbird overlay
│   ├── deadlines/      # Deadline cards, urgency indicators
│   └── design-system/  # Tokens, typography, colour palette (Section 14)
│
├── stores/             # Zustand global state
│   ├── matters.ts
│   ├── chat.ts
│   ├── ui.ts           # Theme, sidebar state, active matter
│   └── sync.ts         # Sync status
│
└── lib/
    ├── tauri.ts        # Type-safe invoke() wrappers for all Keel commands
    ├── shortcuts.ts    # Keyboard shortcuts (Cmd+/ for Chat, etc.)
    └── ipc-types.ts    # Shared types between Keel and Deck
```

**IPC communication pattern — Keel ↔ Deck:**

```rust
// Keel (Rust) — example command
#[tauri::command]
async fn get_matter_context(
    matter_id: String,
    state: tauri::State<'_, AppState>
) -> Result<MatterContext, AppError> {
    let db = state.db.lock().await;
    let ctx = build_matter_context(&db, &matter_id).await?;
    Ok(ctx)
}
```

```typescript
// Deck (React/TypeScript) — calling the command
import { invoke } from '@tauri-apps/api/core';

const context = await invoke<MatterContext>('get_matter_context', {
    matterId: currentMatter.id
});
```

Tauri's IPC is type-safe end-to-end: Rust structs serialise to JSON via `serde`, TypeScript receives them fully typed. No REST API, no HTTP server, no network round-trips for local operations.

**Local data storage — SQLite via sqlx:**

All matter data, deadlines, documents metadata, billing records, and user data are stored in a local SQLite database on the attorney's machine. Documents themselves are stored as encrypted files in a local vault directory (`~/Library/Application Support/Persist/vault/` on macOS, `%APPDATA%\Persist\vault\` on Windows).

Benefits:
- Zero latency for all reads/writes — no network call needed
- Works fully offline — attorneys can work on planes, in court, anywhere
- Client-privileged data never leaves the machine unless the attorney explicitly syncs or shares
- SQLite is battle-tested for this data scale — a firm with 500 active matters and 50,000 documents is well within SQLite's performance envelope

**LaTeX compilation — fully local on Keel:**

TeX Live is bundled as part of the Persist Desktop installer (adds ~80MB to install size). When the attorney fills a smart form in Deck and hits the live preview, Deck calls `invoke('compile_latex', { template_id, field_values })` → Keel receives the call → injects values into the `.tex` template → runs `pdflatex` as a subprocess → reads the output PDF bytes → returns them to Deck → Deck renders the PDF in the viewer. Total round-trip: ~200ms. No server. No internet required.

**OS integrations via Keel:**

- **Native OS notifications:** Tauri's notification plugin — deadline alerts appear as native macOS/Windows system notifications with action buttons ("Open Matter", "Snooze")
- **Menu bar / system tray:** Persist sits in the system tray when minimised — shows urgent deadline count badge, quick-launch chat
- **Global keyboard shortcut:** `Cmd+Shift+P` (macOS) / `Ctrl+Shift+P` (Windows) — opens Persist Chat overlay even when Persist is not in focus
- **File system access:** Tauri's FS plugin gives Keel direct access to the local file system for document imports, exports, and the local vault
- **Auto-updater:** Tauri's built-in updater checks for new versions on launch and installs silently in the background

---

### 8.2 Persist Web — Client Portal Stack

The client portal is a **browser-only** web application. Clients open it in any browser — no installation, no download. It is hosted on Hetzner with Cloudflare CDN.

| Layer | Technology |
|---|---|
| Frontend | React (TypeScript) + Vite — separate codebase from the desktop Deck |
| Backend | FastAPI (Python) — REST API serving the client portal |
| Database | PostgreSQL on Hetzner — synced from desktop via Keel sync service |
| Auth | JWT + OTP (email or SMS); no Face ID (browser limitation) |
| Storage | Hetzner Object Storage — documents synced from desktop vault |
| Email | SendGrid |
| Deployment | Hetzner VPS + Cloudflare CDN |

The React component library is **shared** between the desktop Deck and the client web portal — same design tokens, same UI components, different pages and routes. This means any design system update applies everywhere simultaneously.

The client portal is intentionally **read-heavy**: clients view matters, download documents, view invoices. Write actions are limited to uploading documents and disputing invoices. The backend API enforces these permissions server-side regardless of the frontend state.

---

### 8.3 Persist iOS/iPadOS — Client Mobile App

The client iOS/iPadOS app mirrors the client web portal — same scoped capability, native mobile experience.

**Implementation decision (to be confirmed):**

Option A — **Tauri v2 mobile target:** The same React + Rust codebase used for the web portal is compiled for iOS via Tauri v2's mobile build target (`npx tauri ios build`). This produces a native iOS app with the same UI components. Faster to build; shares the client web portal codebase exactly.

Option B — **Native SwiftUI:** A separate Swift app providing a more native iOS feel (native navigation gestures, better iPad multitasking, tighter system integration). More work but higher quality iOS UX.

Given that the client app is relatively simple (4 tabs, mostly read-only), **Option A (Tauri v2 mobile)** is recommended for Phase 7 — ship faster, maintain one codebase, upgrade to SwiftUI in a later phase if the client experience needs it.

**Client app capabilities (same as web portal):**
- My Matters: status, timeline, responsible attorney
- Documents: view, download, upload
- Invoices: view, dispute, payment history
- Profile: settings, notifications, biometric login

**iOS-specific:**
- Face ID / Touch ID authentication
- Push notifications via APNs (matter updates, new document, invoice)
- Offline: cached matter data readable without connectivity

---

### 8.4 Sync Architecture — Desktop ↔ Web ↔ iOS

The sync layer is a lightweight Rust service (also built on Keel) running on Hetzner. It handles bidirectional sync between the firm's desktop instances and the client-facing surfaces:

```
Persist Desktop (Keel — local SQLite + vault)
        │
        │  Sync over HTTPS (WebSocket for real-time)
        ▼
Hetzner Sync Server (Rust — Axum + PostgreSQL mirror)
        │                    │
        ▼                    ▼
Client Web Portal     Client iOS App
(FastAPI + React)     (Tauri v2 mobile)
```

**What syncs:**
- Matter status updates → client sees updated status within seconds
- Documents shared with client → appear in client portal immediately
- Invoices sent → client receives push notification + sees invoice
- Client uploads (POA, evidence) → appear in desktop app within seconds

**What never syncs to client surfaces:**
- Internal notes
- Attorney time entries and billing rates
- Unapproved drafts
- Matter-internal communications between attorneys
- AI chat session history

**Conflict resolution:** Desktop is always the source of truth. If a client uploads a document while the attorney is offline, it queues in the sync server and is pulled to the desktop on next connection.

---

### 8.5 Full Stack Summary

| Surface | Framework | Language | Data | Offline |
|---|---|---|---|---|
| Firm Desktop | Tauri v2 | Rust (Keel) + React/TS (Deck) | Local SQLite + encrypted vault | ✓ Full offline |
| Client Web Portal | React + FastAPI | TypeScript + Python | PostgreSQL on Hetzner | ✗ (browser) |
| Client iOS/iPad | Tauri v2 mobile | React/TS (+ Rust) | Cached from sync server | ✓ Read-only |
| Sync Server | Axum (Rust) | Rust | PostgreSQL mirror | N/A (server) |
| LaTeX Engine | TeX Live (local) | — | Bundled in installer | ✓ Always local |
| AI (Claude API) | Anthropic API | — | No data stored | ✗ (API call) |

---

### 8.6 Database Schema (Core Tables)

**Desktop SQLite (canonical — Keel manages):**

```sql
-- Core matter data
clients, matters, matter_parties, deadlines, documents,
time_entries, invoices, invoice_line_items, payments,
notifications, audit_log

-- Drafting suite (Module 9)
templates, clauses, generated_documents, latex_jobs

-- Contract intelligence (Module 10)
review_projects, project_documents, smart_fields,
extractions, extraction_flags, chat_sessions,
diligence_reports

-- Daily intelligence (Module 11)
tasks, task_templates, meeting_notes, routines,
routine_outputs, focus_sessions

-- Mail (Module 15)
mail_accounts, mail_threads, mail_messages,
mail_attachments, mail_matter_links

-- Reference manager (Module 17)
references, reference_tags, reference_notes,
reference_citations
```

**PostgreSQL mirror (Hetzner sync server — client-facing subset only):**

```sql
-- Synced from desktop — client-visible data only
matters_public, deadlines_public, documents_shared,
invoices, payments, client_notifications
```

---

### 8.7 Security Architecture

**Desktop (Keel) security:**
- Tauri capability system: each Deck component declares exactly which Keel commands it can invoke — deny-by-default, no ambient authority
- Document vault: AES-256 encryption at rest; key derived from user login credentials via Argon2
- No data leaves the machine without explicit attorney action (share / sync / export)
- Auto-lock: Persist re-requires authentication after 10 minutes of inactivity (configurable)
- Rust memory safety: no buffer overflows, no use-after-free vulnerabilities by construction

**Sync server security:**
- mTLS between desktop and sync server — client certificate pinned in Keel at build time
- JWT tokens with 15-minute expiry for all client portal API calls
- Row-level security in PostgreSQL: clients can only query rows where `client_id = authenticated_user_id`
- All documents served via signed time-limited URLs (5-minute expiry)

**Client portal security:**
- OTP authentication (email or SMS) — no passwords stored
- HTTPS only; HSTS preloaded
- CSP headers prevent XSS; all client-uploaded documents scanned before storage
- Rate limiting on all API endpoints

---

## 9. Practice-Area-Specific Workflows

### 9.1 IP / Trademark

Matter workflow stages: `Intake → Application Filed → Examination Report Received → Response Drafted → Response Filed → Advertised → Opposition Window → Registered / Hearing`

Deadline engine pre-loads: TM Registry statutory deadlines, Vienna/Nice classification lookups (future), opposition period countdown.

### 9.2 Corporate / Contracts

Matter workflow stages: `Instructions Received → Draft Circulated → Client Review → Counterparty Negotiation → Executed → Filed / Registered`

Document-heavy module: contract version comparison, redline tracking, execution checklist.

### 9.3 Litigation / Court Deadlines

Matter workflow stages: `Brief Received → Court Filed → Notices Served → Reply/Written Statement → Hearing Dates → Order / Judgment → Execution / Appeal`

Deadline engine: court date calendar, next hearing date tracking, cause list integration (future).

### 9.4 Paralegal Services

Workflow: `Instruction Received → Research / Drafting → Attorney Review → Filed / Delivered`

Used as a sub-workflow inside IP and Litigation matters; also bookable as a standalone service.

---

## 10. Phased Rollout Plan

### Phase 0 — Tauri v2 Scaffold (Weeks 1–2)
This is the technical foundation phase — setting up the Tauri v2 project structure (Keel + Deck) before any features are built. Nothing ships to users in this phase; everything built in Phases 1–7 depends on it being done right.

- Initialise Tauri v2 project: `src-tauri/` (Rust workspace / Keel) + `src/` (React/Vite/TypeScript / Deck)
- Configure Tauri v2: `tauri.conf.json` — window settings, CSP, capability declarations, auto-updater endpoint
- Set up Keel database layer: SQLite via `sqlx`, initial schema migrations
- Set up Keel command scaffold: empty command modules for each feature area (`matters.rs`, `deadlines.rs`, etc.)
- Set up Deck routing: React Router v7, Zustand stores, type-safe `invoke()` wrapper library
- Configure design system tokens in Deck: colours, typography, spacing (Section 14)
- Set up Hetzner VPS: sync server scaffold (Axum + PostgreSQL), staging environment
- CI/CD pipeline: GitHub Actions — Rust tests + React tests + Tauri build for macOS + Windows
- Install and bundle TeX Live in the Keel build process (local LaTeX — adds ~80MB to installer)
- First build: blank Persist Desktop window launches on macOS and Windows ✓

### Phase 1 — Foundation (Weeks 3–10)
First real features on top of the Houston scaffold. Desktop app only — no client portal yet.

- Matter management: create, edit, close, archive matters (Keel CRUD + Deck UI)
- Deadline engine: manual deadline entry, urgency calculation, OS native notifications
- Document store: upload, version, categorise — stored in local encrypted vault via Keel
- User management: firm user accounts, RBAC (Partner / Associate / Paralegal / Admin)
- Basic firm dashboard: matter list, deadline feed, recent documents
- Auto-updater: working update pipeline — attorney gets a toast when a new version is available

### Phase 2 — Billing & Client Portal (Weeks 11–18)
First external surface: the client browser portal, plus billing for internal use.

- Time tracking: manual entry + in-app timer, linked to matters
- GST-compliant invoice generation: PDF output from Keel LaTeX pipeline
- Keel sync service: initial sync from desktop SQLite → Hetzner PostgreSQL mirror
- Client web portal (React + FastAPI): matter status, document access, invoice view
- Client authentication: OTP (email/SMS), JWT session management
- Email notifications: SendGrid — invoice delivery, client status updates
- Matter-to-client permission model: what each client can see, enforced server-side

### Phase 3 — Intelligence Layer (Weeks 19–26)
AI features, calendar sync, WhatsApp — the platform becomes actively intelligent.

- Persist Chat (Module 21): Keel context assembly + Claude API integration, Hummingbird `Cmd+/` shortcut
- AI Drafting Assistant (Module 7): first Claude-powered document drafting via intake form
- Deadline engine: IP statutory templates (TM, Patent prosecution workflows)
- Reports and analytics dashboard (Module 8): operational + financial reporting
- WhatsApp notifications: Meta Cloud API — deadline alerts to attorneys
- Calendar sync: Google Calendar API + Microsoft Graph API (Outlook) via Keel
- M365 integration (Module 19): Outlook mail sync via Graph API → feeds into Phase 4 Mail module

### Phase 4 — Drafting Suite (Weeks 27–38)
The full document creation and management lifecycle.

- Template engine + clause library (Module 9.1): template vault in Keel, form builder in Deck
- Smart Form Compiler (Module 9.8): Keel LaTeX pipeline, Deck split-screen form + live PDF preview
- Initial template library: Reply to Legal Notice + 19 additional templates (TM, Patent, Litigation, Corporate)
- AI proofreading + document repair (Module 9.2)
- Document comparison with AI summary (Module 9.3): Keel diff engine + Claude summary
- Metadata cleaning — clean-on-send enforced (Module 9.4): Keel strips metadata before any export
- PDF engine (Module 16): full markup toolkit, OCR (Keel via Tesseract), Bates stamping, bookmarks
- Smart Text (Module 16.8): click-to-edit on Persist-generated PDFs, Keel recompiles LaTeX
- Precedent library + knowledge management (Module 9.6)
- Integrated Mail Module (Module 15): Inbox-by-Google pattern, matter-bundled email in Deck

### Phase 5 — Contract Intelligence Engine (Weeks 39–54)
High-value contract review and analysis capabilities.

- Smart field extraction engine: built-in Indian contract language library (Module 10.1)
- Review project workspace: upload → extract → triage → flag (Module 10.2)
- Generative smart fields: custom extraction via natural language prompts
- Rapid clause analysis + concept search (Module 10.3)
- AI chat with contracts: document + project level with citations (Module 10.4)
- Smart summaries + diligence report generation (Module 10.5)
- Contract portfolio management: obligation → deadline auto-creation (Module 10.6)
- Shared PDF markup sessions (Module 16.7): real-time collaborative review via WebSocket

### Phase 6 — Daily Intelligence & Workday Organiser (Weeks 55–62)
The platform becomes proactive — organising every attorney's day automatically.

- Daily Brief on desktop launch (Module 11.1): Keel assembles from all local data, Deck renders
- Smart Task Management (Module 11.2): unified task surface, dependency chains, capacity heatmap
- Meeting intelligence (Module 11.3): calendar-linked prep briefs, AI action item extraction
- Routines (Module 11.4): scheduled intelligence briefings, custom routines
- Focus Mode (Module 11.5): distraction-free single-task mode with auto time logging
- Legal Reference Manager (Module 17): per-matter reference library, Indian Kanoon integration
- Legal Intelligence Layer (Module 18): constitutional corpus, judgment search, AI legal research

### Phase 7 — Advanced Features (Post-launch)
Post-launch expansion and mobile.

- Client iOS/iPadOS app (Module 12): Tauri v2 mobile build of client web portal → App Store
- E-signature integration
- Court cause list auto-import
- Multi-firm / white-label mode (if productising for external law firms)
- ML model fine-tuning pipeline (Section 18.5): using the training corpus built from Day 1
- Shared PDF sessions on client iOS app
- SCC Online / Manupatra integration (evaluated based on demand)

---

## 11. Competitive Positioning

| Feature | Bilr | Litera Transact | Litera Foundation 365 | Litera Draft | Kira | Littlebird | **Persist** |
|---|---|---|---|---|---|---|---|
| IP docketing workflows | ❌ | Limited | ✓ | ❌ | ❌ | ❌ | ✓ |
| Branded client portal | Limited | ✓ | ✓ | ❌ | ❌ | ❌ | ✓ (fully branded) |
| Indian GST billing | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✓ |
| AI drafting assistant | ❌ | Limited | Limited | ✓ (Lito) | ❌ | ❌ | ✓ (Claude-powered) |
| Smart form → PDF compiler (LaTeX) | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✓ |
| Template engine + clause library | ❌ | ❌ | ❌ | ✓ (Litera Create) | ❌ | ❌ | ✓ |
| Document comparison + AI summary | ❌ | ❌ | ❌ | ✓ (Litera Compare) | ❌ | ❌ | ✓ |
| Metadata cleaning on send | ❌ | ❌ | ❌ | ✓ (Metadact) | ❌ | ❌ | ✓ |
| PDF bind / Bates stamp / OCR | ❌ | ❌ | ❌ | ✓ (pdfDocs) | Limited | ❌ | ✓ |
| Precedent library | ❌ | ❌ | Limited | ✓ | ❌ | ❌ | ✓ |
| AI proofreading + doc repair | ❌ | ❌ | ❌ | ✓ | ❌ | ❌ | ✓ |
| Smart field clause extraction | ❌ | ❌ | ❌ | ❌ | ✓ (1,400+ fields) | ❌ | ✓ (built-in + custom) |
| AI chat with contracts | ❌ | ❌ | ❌ | ❌ | ✓ | ❌ | ✓ |
| Diligence report generation | ❌ | Limited | ❌ | ❌ | ✓ | ❌ | ✓ |
| Obligation → deadline auto-creation | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✓ |
| Daily Brief (AI workday organiser) | ❌ | ❌ | ❌ | ❌ | ❌ | ✓ | ✓ (legal-context-aware) |
| Smart task management + chains | ❌ | ❌ | ❌ | ❌ | ❌ | Limited | ✓ |
| Meeting prep brief + action items | ❌ | ❌ | ❌ | ❌ | ❌ | ✓ | ✓ (matter-linked) |
| Scheduled intelligence routines | ❌ | ❌ | ❌ | ❌ | ❌ | ✓ | ✓ |
| Focus mode + auto time logging | ❌ | ❌ | ❌ | ❌ | ❌ | Limited | ✓ |
| WhatsApp alerts | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✓ |
| Indian IP deadline templates | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✓ |
| Self-hosted option | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✓ (Hetzner) |
| Cost | High SaaS | Very high | Very high | Very high | Very high | $20/mo/user | Internal build |

---

## 12. Open Questions

1. Should the client portal be a subdomain of persistas.com (e.g. `app.persistas.com`) or a dedicated domain?
2. Will the firm adopt time-based billing immediately, or continue fixed-fee for most matters initially?
3. What is the integration requirement with the existing Persistas & Partners website (GoDaddy / Cloudflare setup)?
4. Is there a requirement for multi-language support (English + regional languages for certain client types)?
5. Should the AI drafting assistant reference uploaded precedent documents from the firm's own library, or rely solely on the base model?
6. For the drafting suite, should document templates be built in-house (Pratik + partners define them) or should existing firm templates be imported as the starting set?
7. Is clean-on-send (mandatory metadata stripping before every external share) acceptable as a default, or should attorneys have opt-out control per document?
8. Does the firm want the comparison tool to work on court-uploaded PDFs that are scanned (requires OCR pre-processing before comparison)?
9. For the contract intelligence engine: what is the initial priority — M&A / transaction due diligence, or ongoing contract portfolio management for existing clients?
10. Should the built-in smart field library be seeded with Indian-law-specific provisions from day one, or start with a general international library and localise in iteration two?
11. Should clients be able to initiate a contract review request directly from their portal (upload a contract, request analysis), or is contract intelligence strictly an internal firm tool?
12. For the Smart Form Compiler: should the initial template library be built by Pratik from scratch using the firm's existing precedent documents, or should templates be drafted fresh to court/registry format specifications?
13. For Daily Brief routines: should the morning brief be delivered by email at a set time (e.g. 8 AM), shown only on login, or both?
14. Should Focus Mode automatically start a billable time entry against the linked matter, or require the attorney to confirm billing intent first?
15. For the iOS firm app: should it be distributed via TestFlight (invite-only) initially, or submitted to the App Store as an unlisted app from day one?
16. Should the iOS client app use Persistas branding directly, or a distinct app name (e.g. "Persist by Persistas")?
17. For offline mode on iOS: should matter data sync automatically in the background, or only on manual pull-to-refresh to conserve battery?
18. For the Mail Module: will attorneys connect their existing @persistas.com Outlook / Exchange accounts, or will Persist provision separate email accounts?
19. For shared PDF markup sessions: should external clients (via client portal) be able to join sessions, or are sessions firm-only?
20. For ML training data: which matters / documents does the firm want to designate as training-eligible from the outset? A formal data governance policy is required before Phase 7.
21. For the Legal Intelligence Layer: should the Indian Kanoon API subscription be acquired at launch (Phase 3), or is the built-in public corpus sufficient for early phases?

---

## 13. Out of Scope (v1)

- Court e-filing integration (API access to court portals)
- Multi-firm or white-label mode (Phase 7 evaluation only)
- Android app — iOS only for client mobile; Android evaluated post-launch
- Conflict of interest check engine
- Full accounting / tally integration
- Public-facing client intake form (separate project)
- Microsoft Word ribbon add-in (Graph API light-touch approach used instead — Section 19)
- Real-time co-authoring within Persist (handled via OneDrive / Word Online bridge)
- In-app video / voice calling between attorney and client
- Self-hosted ML model training infrastructure (Phase 7 decision)
- SCC Online / Manupatra paid database integration (evaluated post-launch)
- Electron-based desktop app (replaced by Tauri v2 / Houston — lighter, more secure)

---

## 14. Design System — Visual Language & Aesthetic Direction

### 14.1 Overall Design Philosophy

Persist's visual design is inspired by **Littlebird's calm, focused aesthetic** — an interface that feels reassuring rather than urgent, organised rather than dense, and premium rather than corporate. The goal is for every attorney opening the platform to feel a sense of clarity and control, not anxiety.

The design language is built on five principles:

**1. Calm over stimulation.** No aggressive red alerts everywhere, no cluttered sidebars. Urgency is communicated through hierarchy and placement, not flashing colours. Only genuinely critical items (deadline overdue, document rejected) use red. Everything else uses neutral or muted tones.

**2. Generous whitespace.** Content breathes. Cards have visible padding. Lines are not packed. The layout makes cognitive work easier by giving each element room to be read without strain.

**3. Soft, natural colour palette.** Inspired by Littlebird's warm neutrals and nature-adjacent tones — warm whites, soft creams, muted sage greens, warm slate blues, and earthy terracottas as accent colours. These are not pastels — they have depth. But they are never saturated or aggressive.

**4. Refined typography.** A single, characterful serif or soft sans font family (not Inter, not Roboto). Display headings use weight and size to create hierarchy. Body text is set at a comfortable reading size with generous line height. Legal documents deserve to be read without eye strain.

**5. Functional minimalism.** Every visible element earns its place. No decorative chrome. No gratuitous gradients. Icons are small, purposeful, and consistent. The UI gets out of the way of the work.

### 14.2 Colour Palette

| Role | Colour | Notes |
|---|---|---|
| Background (primary) | Warm white `#F9F7F4` | Slightly cream, never harsh white |
| Background (secondary) | Soft sand `#F0ECE5` | Cards, sidebars, panels |
| Background (tertiary) | Light sage `#E8EDE6` | Subtle section differentiation |
| Text (primary) | Deep charcoal `#2C2C2A` | Never pure black |
| Text (secondary) | Warm grey `#6B6862` | Labels, metadata, secondary info |
| Text (tertiary) | Muted stone `#9A9590` | Placeholders, disabled states |
| Accent (primary) | Warm slate blue `#4A6580` | Primary buttons, active states, links |
| Accent (secondary) | Soft terracotta `#B5604A` | Hover states, selected items |
| Status — Urgent | Muted red `#C0392B` | Only for overdue / critical |
| Status — Warning | Warm amber `#D4872A` | Due within 3 days |
| Status — Clear | Sage green `#4A7C59` | Completed, safe |
| Status — Neutral | Warm grey `#8A8A85` | Pending, informational |
| Border | `#E2DDD8` | Hairline, 0.5px — light and airy |

### 14.3 Typography

| Element | Font | Size | Weight |
|---|---|---|---|
| Display heading (page title) | Playfair Display | 28px | 600 |
| Section heading | Playfair Display | 20px | 500 |
| Card title | DM Sans | 15px | 500 |
| Body text | DM Sans | 14px | 400 |
| Label / metadata | DM Sans | 12px | 400 |
| Legal document body | Georgia | 13px | 400 |
| Monospace (code, IDs) | JetBrains Mono | 12px | 400 |

*Playfair Display for headings gives the platform a premium, considered editorial quality appropriate for a law firm. DM Sans is clean and modern for UI elements. Georgia for document bodies maintains the familiar reading experience attorneys expect from legal text.*

### 14.4 Component Style

- **Cards:** soft shadow (`box-shadow: 0 1px 4px rgba(0,0,0,0.06)`), `border-radius: 10px`, warm white background, 0.5px border in `#E2DDD8`
- **Buttons (primary):** warm slate blue background, white text, `border-radius: 8px`, no border — subtle hover darkening
- **Buttons (secondary):** transparent background, slate blue text, 0.5px slate blue border — hover fills lightly
- **Inputs:** `border-radius: 6px`, 0.5px border, cream background on focus — not white
- **Sidebar navigation:** warm sand background, active item has a left accent bar in terracotta, text in deep charcoal
- **Status badges:** pill-shaped, small, coloured by status with same-ramp text (never black text on coloured badge)
- **Tables:** no row borders — alternating very subtle cream/white background rows, with a hairline separator only under the header
- **Modals:** centred card on a 40% opacity warm overlay, `border-radius: 12px`, max width 640px

### 14.5 Dark Mode

Full dark mode support with a warm dark palette — dark charcoal `#1E1C1A` background, not cold grey. All colour variables invert with preserved warmth. Dark mode is system-preference-aware and user-overridable.

### 14.6 iOS App Design

The iOS app follows the same palette and typography but adapted for native SwiftUI. SF Rounded is used in place of DM Sans for navigation elements to maintain iOS feel. Haptic feedback confirms all destructive or important actions. All screens support Dynamic Type for accessibility.

---

### 14.7 Motion & Animation System

Animation in Persist is **purposeful, not decorative**. Every animated element communicates something — a state change, a relationship between elements, the result of an action, the direction of a transition. Animations that exist purely for visual richness are excluded. Animations that make the interface feel alive, responsive, and oriented are included at every interaction point.

The goal is the feel of high-end professional software — Notion, Linear, Arc — where motion adds polish and communicates intelligence without drawing attention to itself. An attorney should never consciously notice the animations. They should notice that the app feels exceptionally smooth.

**Library: Motion v12 (formerly Framer Motion)**

Motion v12 (`motion/react`) is the animation engine for Deck. It is the only animation library — no GSAP, no anime.js, no CSS keyframes for anything Motion can handle. Consistency across all animated surfaces requires one engine.

```bash
# package.json
"motion": "^12.0.0"
```

```typescript
// All animation imports from one path
import { motion, AnimatePresence, useSpring, useMotionValue,
         useTransform, useInView, LayoutGroup } from 'motion/react';
```

---

#### 14.7.1 The Animation Tokens — The Foundation

Just as colour and typography have tokens, so does animation. Never hardcode durations, easings, or spring configs in components — always import from the animation tokens file.

```typescript
// src/design-system/motion.ts
// The single source of truth for all animation values in Persist

export const duration = {
  instant:   0.08,   // State flips, badge counts — imperceptible but not jarring
  fast:      0.15,   // Hover states, small element transitions
  normal:    0.25,   // Most transitions — the default
  slow:      0.40,   // Page transitions, panel slides
  deliberate: 0.60,  // Emphasis moments — status changes, completion states
} as const;

export const ease = {
  // Standard easing — most UI transitions
  standard: [0.4, 0, 0.2, 1],         // Material Design standard — smooth in/out
  // Entering the screen — starts fast, decelerates
  enter:    [0, 0, 0.2, 1],
  // Leaving the screen — starts slow, accelerates
  exit:     [0.4, 0, 1, 1],
  // Elastic — for confirmations, completions, success states
  spring:   { type: 'spring', stiffness: 400, damping: 28 },
  // Gentle spring — for larger panels, sidebars
  gentle:   { type: 'spring', stiffness: 200, damping: 24 },
  // Snappy spring — for buttons, checkboxes, toggles
  snappy:   { type: 'spring', stiffness: 600, damping: 35 },
} as const;

// Pre-built transition objects — use these in components
export const transition = {
  fast:       { duration: duration.fast,   ease: ease.standard },
  normal:     { duration: duration.normal, ease: ease.standard },
  slow:       { duration: duration.slow,   ease: ease.standard },
  enter:      { duration: duration.normal, ease: ease.enter },
  exit:       { duration: duration.fast,   ease: ease.exit },
  spring:     ease.spring,
  gentle:     ease.gentle,
  snappy:     ease.snappy,
} as const;

// Reduced motion — always respect this
// Wrap all animations with this check
export const prefersReducedMotion = () =>
  window.matchMedia('(prefers-reduced-motion: reduce)').matches;
```

---

#### 14.7.2 Card Animations — The Primary Surface

Cards are the most common element in Persist — matter cards, deadline cards, email cards, document cards. Every card has the same animation behaviour.

**Hover state:**
```typescript
// components/ui/Card.tsx
import { motion } from 'motion/react';
import { transition } from '@/design-system/motion';

export function Card({ children, onClick, ...props }) {
  return (
    <motion.div
      className="card"
      whileHover={{
        y: -2,                          // Lifts 2px — subtle elevation
        boxShadow: '0 4px 16px rgba(0,0,0,0.10)',  // Shadow deepens
        transition: transition.fast,
      }}
      whileTap={{
        y: 0,
        scale: 0.99,                    // Slight press — tactile feel
        transition: transition.fast,
      }}
      {...props}
    >
      {children}
    </motion.div>
  );
}
```

**Enter animation — cards entering a list:**
```typescript
// Staggered list — each card enters after the previous
// Used in MatterList, DocketList, DocumentStore, Voice Inbox

const listVariants = {
  hidden: {},
  visible: {
    transition: {
      staggerChildren: 0.05,   // 50ms between each card
      delayChildren: 0.05,
    },
  },
};

const cardVariants = {
  hidden:  { opacity: 0, y: 12 },
  visible: {
    opacity: 1,
    y: 0,
    transition: transition.enter,
  },
};

// Usage:
<motion.ul variants={listVariants} initial="hidden" animate="visible">
  {matters.map(matter => (
    <motion.li key={matter.id} variants={cardVariants}>
      <MatterCard matter={matter} />
    </motion.li>
  ))}
</motion.ul>
```

**Exit animation — card being archived/done/removed:**
```typescript
// AnimatePresence handles mount/unmount
// Used when marking a deadline done, archiving a matter, marking email Done

<AnimatePresence mode="popLayout">
  {items.map(item => (
    <motion.div
      key={item.id}
      layout                              // Animates reflow of remaining items
      exit={{
        opacity: 0,
        x: -20,                           // Slides left — intentional direction
        scale: 0.97,
        transition: transition.exit,
      }}
    >
      <Card item={item} />
    </motion.div>
  ))}
</AnimatePresence>
```

---

#### 14.7.3 Status Badge Animations

Status badges change throughout a matter's lifecycle. The change itself is the communication — it tells the attorney something happened.

```typescript
// components/matters/MatterStatusBadge.tsx

const badgeVariants = {
  initial: { scale: 0.85, opacity: 0 },
  animate: {
    scale: 1,
    opacity: 1,
    transition: ease.snappy,             // Snaps in with spring
  },
  exit: {
    scale: 0.85,
    opacity: 0,
    transition: transition.fast,
  },
};

// The badge text transitions with a vertical clip — old text slides up,
// new text slides in from below. Communicates progress direction.
export function MatterStatusBadge({ status }) {
  return (
    <AnimatePresence mode="wait">
      <motion.span
        key={status}                      // Re-mounts on status change
        className={`badge badge--${status}`}
        variants={badgeVariants}
        initial="initial"
        animate="animate"
        exit="exit"
      >
        {statusLabels[status]}
      </motion.span>
    </AnimatePresence>
  );
}
```

**Urgency badge pulse — overdue deadlines:**

Overdue deadline rows have a subtle continuous pulse on the left accent bar — not distracting, but alive. It draws the eye without screaming.

```typescript
// components/deadlines/UrgencyAccent.tsx

export function UrgencyAccent({ urgency }) {
  if (urgency !== 'overdue') return <div className="accent-bar" />;

  return (
    <motion.div
      className="accent-bar accent-bar--overdue"
      animate={{
        opacity: [1, 0.5, 1],           // Breathes — never fully disappears
      }}
      transition={{
        duration: 2.4,
        repeat: Infinity,
        ease: 'easeInOut',
      }}
    />
  );
}
```

---

#### 14.7.4 Panel & Sidebar Transitions

Panels slide in from a consistent direction. This spatial consistency means attorneys always know where something came from and where it went.

**Rule: panels slide from the direction they live in the layout.**
- Right panel (thread view, AI panel, detail view) → slides from the right
- Bottom panel (compose, snooze picker, voice output) → slides from the bottom
- Modal → scales up from centre

```typescript
// src/design-system/panelVariants.ts

export const rightPanelVariants = {
  hidden:  { x: '100%', opacity: 0 },
  visible: {
    x: 0,
    opacity: 1,
    transition: { ...ease.gentle, opacity: { duration: duration.fast } },
  },
  exit: {
    x: '100%',
    opacity: 0,
    transition: transition.exit,
  },
};

export const bottomPanelVariants = {
  hidden:  { y: '100%', opacity: 0 },
  visible: {
    y: 0,
    opacity: 1,
    transition: { ...ease.gentle, opacity: { duration: duration.fast } },
  },
  exit: {
    y: '100%',
    opacity: 0,
    transition: transition.exit,
  },
};

export const modalVariants = {
  hidden:  { scale: 0.95, opacity: 0 },
  visible: {
    scale: 1,
    opacity: 1,
    transition: ease.snappy,
  },
  exit: {
    scale: 0.97,
    opacity: 0,
    transition: transition.fast,
  },
};

// Backdrop always fades — never slides
export const backdropVariants = {
  hidden:  { opacity: 0 },
  visible: { opacity: 1, transition: transition.fast },
  exit:    { opacity: 0, transition: transition.fast },
};
```

**Sidebar module switching:**

When the attorney switches between modules (Matters → Mail → Dockets) in the left sidebar, the content area transitions with a directional fade — not a full-screen slide, just a soft opacity and slight Y shift.

```typescript
// The content area responds to route changes
// src/pages/RootLayout.tsx

<AnimatePresence mode="wait">
  <motion.div
    key={currentModule}                   // Re-mounts on module change
    initial={{ opacity: 0, y: 8 }}
    animate={{ opacity: 1, y: 0, transition: transition.enter }}
    exit={{ opacity: 0, y: -8, transition: transition.exit }}
  >
    <Outlet />
  </motion.div>
</AnimatePresence>
```

---

#### 14.7.5 Persist Chat — The Hummingbird Overlay

The `Cmd+/` chat overlay is the most premium-feeling surface in the entire app. Its animation sets the tone.

```typescript
// components/chat/PersistChatOverlay.tsx

// Opening: scales up from a point near the trigger, not from screen centre
// Closing: reverses — collapses back to origin

const overlayVariants = {
  hidden: {
    opacity: 0,
    scale: 0.92,
    y: 12,
    filter: 'blur(4px)',                  // Focuses in as it appears
  },
  visible: {
    opacity: 1,
    scale: 1,
    y: 0,
    filter: 'blur(0px)',
    transition: {
      ...ease.gentle,
      filter: { duration: duration.fast },
      opacity: { duration: duration.fast },
    },
  },
  exit: {
    opacity: 0,
    scale: 0.95,
    y: 8,
    filter: 'blur(2px)',
    transition: transition.fast,
  },
};

// Each AI message streams in token by token — but the container
// grows with a smooth layout animation
<motion.div layout transition={ease.gentle}>
  <AIMessage content={message} />
</motion.div>
```

**Message streaming — the typing feel:**

As Claude generates a response, the text streams in. The container height grows smoothly with `layout` animation so it never jumps.

```typescript
// components/chat/AIMessage.tsx

export function AIMessage({ content, isStreaming }) {
  return (
    <motion.div
      layout                              // Grows smoothly as content arrives
      transition={ease.gentle}
      className="message message--ai"
    >
      {content}
      {isStreaming && (
        <motion.span
          className="cursor"
          animate={{ opacity: [1, 0] }}
          transition={{ duration: 0.7, repeat: Infinity }}
        >
          ▋
        </motion.span>
      )}
    </motion.div>
  );
}
```

**Action chips — slide in after the message:**

```typescript
// components/chat/ActionChips.tsx

const chipContainerVariants = {
  hidden: {},
  visible: {
    transition: { staggerChildren: 0.06, delayChildren: 0.15 },
  },
};

const chipVariants = {
  hidden:  { opacity: 0, scale: 0.85, y: 4 },
  visible: {
    opacity: 1,
    scale: 1,
    y: 0,
    transition: ease.snappy,
  },
};

<motion.div variants={chipContainerVariants} initial="hidden" animate="visible">
  {chips.map(chip => (
    <motion.button key={chip.id} variants={chipVariants} className="action-chip">
      {chip.label}
    </motion.button>
  ))}
</motion.div>
```

---

#### 14.7.6 IP Pipeline Kanban — Drag and Drop

The prosecution pipeline board (Module 2.7) is the richest animated surface. Cards drag with physics, drop into columns with a spring, and columns rebalance as cards move.

```typescript
// components/dockets/PipelineBoard.tsx
// Uses Motion's drag capabilities + LayoutGroup for cross-column reordering

import { motion, LayoutGroup, Reorder } from 'motion/react';

// The dragging card lifts and casts a deeper shadow
const draggingCardStyle = {
  scale: 1.03,
  boxShadow: '0 12px 32px rgba(0,0,0,0.16)',
  zIndex: 50,
  cursor: 'grabbing',
};

// The column receiving a card glows subtly to indicate it can accept
// (left accent bar brightens, very subtle background tint)

// Cards within a column reorder with layout animation — smooth reflow
<LayoutGroup>
  {stages.map(stage => (
    <Reorder.Group
      key={stage.id}
      axis="y"
      values={stage.matters}
      onReorder={(reordered) => handleReorder(stage.id, reordered)}
    >
      {stage.matters.map(matter => (
        <Reorder.Item key={matter.id} value={matter}>
          <PipelineCard matter={matter} />
        </Reorder.Item>
      ))}
    </Reorder.Group>
  ))}
</LayoutGroup>
```

---

#### 14.7.7 Number and Counter Animations

Anywhere a number changes — deadline counts, billing totals, unread badges — it animates with a spring-driven count-up, not a hard cut.

```typescript
// components/ui/AnimatedNumber.tsx

import { useSpring, useMotionValue, useTransform, motion } from 'motion/react';
import { useEffect } from 'react';

export function AnimatedNumber({ value, format = (n: number) => n.toString() }) {
  const motionValue = useMotionValue(0);
  const spring = useSpring(motionValue, ease.gentle);
  const display = useTransform(spring, (v) => format(Math.round(v)));

  useEffect(() => {
    motionValue.set(value);
  }, [value]);

  return <motion.span>{display}</motion.span>;
}

// Usage examples:
// <AnimatedNumber value={invoiceTotal} format={(n) => `₹${n.toLocaleString('en-IN')}`} />
// <AnimatedNumber value={unreadCount} />
// <AnimatedNumber value={daysUntilDeadline} />
```

**Unread badge pop:**

When a new email arrives or a new comment is posted, the unread count badge doesn't just increment — it bounces.

```typescript
// components/ui/UnreadBadge.tsx

<motion.span
  key={count}                             // Re-triggers animation on count change
  initial={{ scale: 1.4 }}
  animate={{ scale: 1 }}
  transition={ease.snappy}
  className="badge badge--unread"
>
  {count}
</motion.span>
```

---

#### 14.7.8 The `✦` AI Sparkle — Entrance Animation

Every piece of AI-generated content is marked with a `✦` sparkle icon (Module 22.11). When it first appears — thread summary, action chip, smart reply — the sparkle animates in before the text.

```typescript
// components/ui/AISparkle.tsx

export function AISparkle() {
  return (
    <motion.span
      className="ai-sparkle"
      initial={{ scale: 0, rotate: -30, opacity: 0 }}
      animate={{ scale: 1, rotate: 0, opacity: 1 }}
      transition={{ ...ease.snappy, delay: 0.05 }}
    >
      ✦
    </motion.span>
  );
}
```

---

#### 14.7.9 Form and Input Feedback

Forms communicate errors, validation, and success through motion — never just colour change alone.

```typescript
// Shake on validation error — the field tells you it rejected the input
const shakeVariants = {
  shake: {
    x: [0, -6, 6, -4, 4, -2, 2, 0],
    transition: { duration: 0.4, ease: 'easeInOut' },
  },
};

<motion.input
  animate={hasError ? 'shake' : ''}
  variants={shakeVariants}
  className={hasError ? 'input input--error' : 'input'}
/>

// Success state — field border animates to sage green then settles
<motion.div
  animate={isSuccess ? { borderColor: '#4A7C59' } : { borderColor: '#E2DDD8' }}
  transition={transition.normal}
/>
```

**Save confirmation:**

After saving a matter, submitting a form, or marking a deadline complete — a subtle check animation replaces the save button momentarily before returning to normal.

```typescript
// components/ui/SaveButton.tsx

const buttonVariants = {
  idle:    { scale: 1 },
  saving:  { scale: 0.97, opacity: 0.7 },
  saved:   { scale: 1 },
};

// The button label transitions: "Save" → "✓ Saved" → "Save"
// AnimatePresence handles the label swap with vertical clip
```

---

#### 14.7.10 Persist Voice — Recording Waveform

The floating voice recorder (Module 23) has a live waveform visualiser during recording. This is the most visually rich animation in the entire app — it needs to feel alive.

```typescript
// components/voice/VoiceWaveform.tsx
// Real audio amplitude data drives the bar heights via Web Audio API
// Each bar is an independent motion.div with spring physics

export function VoiceWaveform({ audioData }: { audioData: Float32Array }) {
  const bars = Array.from({ length: 32 });

  return (
    <div className="waveform">
      {bars.map((_, i) => (
        <motion.div
          key={i}
          className="waveform__bar"
          animate={{
            scaleY: audioData[i] ?? 0.1,     // Real amplitude value
            opacity: 0.6 + (audioData[i] ?? 0) * 0.4,
          }}
          transition={{
            type: 'spring',
            stiffness: 800,
            damping: 40,
            // Each bar has a slightly different stiffness — organic variation
            stiffness: 600 + i * 8,
          }}
          style={{ transformOrigin: 'center' }}
        />
      ))}
    </div>
  );
}
```

---

#### 14.7.11 Deadline Completion — The Done Moment

When an attorney marks a deadline complete, this is the most important positive moment in the docketing experience. It deserves the most deliberate animation in the app.

```typescript
// components/deadlines/DeadlineCard.tsx

// Step 1: The checkbox fills with a spring snap (0.1s)
// Step 2: The card fades and slides left simultaneously (0.25s)
// Step 3: The remaining cards close the gap with layout animation (0.3s)
// Step 4: If all deadlines in a matter are done — a brief green flash on the matter row

const completionSequence = async (controls: AnimationControls) => {
  await controls.start({                  // Checkbox snap
    scale: [1, 1.15, 1],
    transition: { duration: 0.15, times: [0, 0.4, 1] },
  });
  await controls.start({                  // Card exit
    opacity: 0,
    x: -20,
    transition: { duration: 0.2, ease: ease.exit },
  });
};
```

---

#### 14.7.12 Accessibility — Reduced Motion

All animations in Persist respect the `prefers-reduced-motion` OS setting. When the attorney has reduced motion enabled:

- Transitions drop to opacity-only (no translate, scale, or blur)
- Duration tokens halve
- Spring animations become instant
- The waveform visualiser becomes a static bar chart
- Stagger delays are removed

```typescript
// src/design-system/motion.ts — add to motion tokens

export function useMotionPreference() {
  const shouldReduce = useReducedMotion();   // Motion's built-in hook

  return {
    transition: shouldReduce
      ? { duration: 0.01 }
      : transition.normal,
    cardHover: shouldReduce
      ? {}
      : { y: -2, boxShadow: '0 4px 16px rgba(0,0,0,0.10)' },
    // All other motion values similarly conditioned
  };
}
```

---

#### 14.7.13 Animation Inventory — Where Motion Lives

| Surface | Animation type | Tokens used |
|---|---|---|
| Matter list | Staggered card entrance | `stagger 0.05`, `enter` |
| Matter card hover | Lift + shadow deepen | `fast`, `standard` |
| Matter card archive | Slide left exit + reflow | `exit`, `layout` |
| Status badge change | Vertical clip swap | `snappy` |
| Overdue accent bar | Opacity pulse | `2.4s infinite` |
| Panel open (right) | Slide from right + blur in | `gentle` |
| Panel close | Slide to right | `exit` |
| Modal open | Scale up from centre | `snappy` |
| Module switch | Opacity + Y shift | `enter` / `exit` |
| Chat overlay open | Scale + blur in | `gentle` |
| Chat message stream | Layout growth | `gentle` |
| Action chips | Staggered scale in | `snappy + stagger 0.06` |
| AI sparkle | Rotate + scale in | `snappy` |
| Number counter | Spring count-up | `gentle spring` |
| Unread badge update | Scale bounce | `snappy` |
| Pipeline card drag | Lift + shadow | Direct style |
| Pipeline drop | Spring settle | `gentle` |
| Form error | Horizontal shake | `0.4s easeInOut` |
| Save confirmation | Label swap | `snappy` |
| Deadline completion | Snap → slide → reflow | Sequence |
| Voice waveform | Per-bar spring | `stiffness 600–856` |
| Sidebar nav active | Background slide | `gentle` |
| Tab switch | Underline slide | `spring` |

---

### 15.1 Why a Built-In Mail Module

Attorneys at Persistas spend a significant portion of their day in email — receiving examination reports from TM Registry, court notices, opposing counsel correspondence, client instructions, and registry communications. Today all of this arrives in a general inbox, completely disconnected from the matter it belongs to. Every reply has to be manually linked back. Nothing is searchable by matter. Context is lost.

Persist Mail solves this by bringing email inside the platform where all the matter context already lives. An email arrives, gets auto-linked to its matter, can be replied to with a template, has its attachments saved to the matter document store, and is archived with a single keystroke. The attorney never has to leave Persist to manage professional communication.

---

### 15.2 The Design Hybrid — Three Inspirations, One Interface

Persist Mail draws from three distinct sources and synthesises them into something purpose-built for a law firm:

**From Inbox by Google** — the visual language and interaction philosophy: card-based email rows, matter bundling instead of sender bundling, the Done/Archive paradigm (not Delete), snooze with time and context options, pinning, sweep to clear a bundle in one action, inline highlights of key information directly in the list row, and the calm uncluttered aesthetic of a to-do list applied to email.

**From Outlook** — the power-user functionality underneath: Focused tab with AI-trained routing, full rules engine (sender, subject, keyword, domain → route to tab or bundle), conversation threading, categories and colour coding, flag for follow-up, read receipts (optional), out-of-office handling, delegate access for partners managing an associate's inbox, and the organisational rigour that professional legal email demands.

**From Persist itself** — the legal layer that neither Inbox by Google nor Outlook ever had: every email is matter-aware, sender-matched to a client or court or registry, permanently stored as part of the matter communication record, and linked to deadlines, documents, and billing.

---

### 15.3 The Main Window — Tab Architecture

The heart of Persist Mail is a **fixed horizontal tab bar** at the top of the mail view. This is the primary navigation layer — it does not scroll, it does not collapse, it is always visible. Every tab is a filtered, focused view of the same unified mailbox.

```
┌────────────────────────────────────────────────────────────────────────────┐
│  ✦ Focused  │  Petalveda Scents  │  Zenith Brands  │  Rahul Industries  │ + │
│  ─────────  │                    │                  │                     │   │
│  [active]   │                    │     [3 unread]   │                     │   │
└────────────────────────────────────────────────────────────────────────────┘
```

**Fixed tabs — always present:**

**✦ Focused** — the master inbox. Contains all emails that are not yet assigned to a client tab, plus pinned items from all tabs, plus anything the AI has elevated as high priority. This is the attorney's "first open" every morning. The AI learns which senders and subjects belong here — court notices, client escalations, registry correspondence all surface here automatically.

**Client tabs (one per active client / matter group)** — each tab is a named, persistent view showing only emails related to that client or matter group. An attorney handling Petalveda Scents, Zenith Brands, and Rahul Industries simultaneously has three client tabs in addition to Focused. Every email that arrives and is auto-linked to Petalveda Scents appears in that tab — and in Focused only if the AI flags it as high priority. The client tab shows the unread count as a badge.

**+ (Add tab)** — opens a quick picker to pin a new client or matter as a tab. Tabs can be reordered by drag. Maximum 8 pinned tabs visible simultaneously; remaining accessible via overflow.

**Tab persistence:** tabs are saved per-attorney, per-device. They survive app restarts. A partner's tab set reflects the matters they are currently active on, not a fresh view every time.

---

### 15.4 Visual Design Language — Inbox by Google Applied

The visual aesthetic of Persist Mail is directly modelled on Inbox by Google, adapted to the Persist design system (warm whites, Playfair Display headings, DM Sans body, slate blue accents, terracotta highlights). The core visual principles:

**Card rows, not list rows.** Each email in the inbox is a card — a rounded rectangle with subtle shadow (`box-shadow: 0 1px 3px rgba(0,0,0,0.05)`), not a flat table row. Cards have visible internal padding (16px horizontal, 12px vertical). The slight elevation distinguishes individual emails without being heavy.

**Information hierarchy in each card:**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  ◉  Akash Kumar Srivastava (Advocate)             Today  2:14 PM  [PIN] │
│     RE: Reply to your notice dated March 31, 2026                       │
│     ...received through email to our client on April 06, 2026 and the  │
│     [PDF: Nytarra_reply_final.pdf]  [📎 1 attachment]                   │
│                                                    [DONE]  [SNOOZE]  [⋮] │
└─────────────────────────────────────────────────────────────────────────┘
```

Row anatomy top to bottom:
1. **Sender name** (DM Sans 14px, 500 weight) — full name from contacts if matched, raw email if not — left-aligned. Date/time — right-aligned, tertiary colour. Pin icon — far right, appears on hover.
2. **Subject line** (DM Sans 14px, 400 weight for read, 500 for unread) — full subject, no truncation on desktop.
3. **Preview snippet** (DM Sans 13px, secondary colour) — first 120 characters of email body, enough to identify content without opening.
4. **Inline highlights** — if the email contains an attachment, a court date, a filing deadline, or a monetary amount, these are surfaced directly in the card as small pills below the preview. A PDF attachment shows its filename as a tappable pill. A date like "hearing on April 15, 2026" is extracted and shown as a date chip. An amount like "₹50,000" is extracted and shown as a money chip. These chips are clickable: the PDF chip opens the attachment preview; the date chip prompts to create a deadline.
5. **Action row** (appears on hover, always visible on touch) — Done (✓), Snooze (🕐), More (⋮). Never occupy permanent vertical space; they appear inline on hover to keep the card compact.

**Unread state:** Unread emails have a small terracotta (`#B5604A`) vertical accent bar on the left edge of the card, 3px wide, full card height. Sender name and subject are DM Sans 500 weight. Read emails have no accent bar; sender and subject revert to 400 weight.

**Matter tag pill:** Every email that has been auto-linked to a matter shows a small pill in the bottom-left of the card — `P&P-2026-TM-0042 · Petalveda Scents` in the Persist accent colour. For emails in the client tab view, this pill is hidden (it's redundant — you're already in the Petalveda Scents tab). In the Focused tab, it's always shown.

---

### 15.5 Matter Bundles — The Core Organisational Principle

Within each tab, emails are organised into **matter bundles** — not chronological lists. A bundle is a collapsible accordion row that groups all emails belonging to the same matter.

```
▼  Petalveda Scents — TM Opposition (Nytarra)          4 emails  [SWEEP]
   ┌──────────────────────────────────────────────────────────────────────┐
   │  ◉  Akash Kumar Srivastava    RE: Reply to notice      Today 2:14 PM │
   │  ◎  Sree Lakshmi (Sent)       Notice dated March 31    3 Apr 2026    │
   │  ◎  TM Registry               Notice of Opposition      1 Apr 2026   │
   │  ◎  Petalveda Scents (client) Instructions for reply    28 Mar 2026  │
   └──────────────────────────────────────────────────────────────────────┘

▶  Zenith Brands — TM Application                       2 emails  [SWEEP]

▶  Rahul Industries — NDA Review                        1 email   [SWEEP]
```

**Bundle header shows:**
- Matter name and type (collapsed view)
- Unread count badge (if any emails unread)
- Sweep button — one click archives all read emails in the bundle
- Expand/collapse toggle (▼/▶)

**Bundle behaviour:**
- New email auto-links to a bundle → bundle rises to the top of its tab and opens automatically
- Pinned emails within a bundle are visually elevated — shown with a subtle pin icon and warm amber left accent
- Sweep button archives all non-pinned, non-snoozed emails in the bundle simultaneously — equivalent to Inbox by Google's sweep gesture
- A bundle with all emails Done collapses and moves to the bottom of the tab, dimmed

**Manual bundling:** If an email isn't auto-linked, the attorney right-clicks or long-presses → "Add to matter" → matter picker appears. The email moves to the correct bundle. This teaches the system for future emails from the same sender.

---

### 15.6 The Focused Tab — AI Priority Inbox

The Focused tab is the master view. It combines:

1. **High-priority unread emails** — AI-flagged as important based on: sender is a court/registry/opposing counsel, subject contains deadline keywords ("hearing", "notice", "order", "response required"), email is a reply to something the attorney sent.
2. **All pinned emails** across all matter bundles — regardless of which client tab they live in.
3. **Snoozed emails that have resurfaced** — at the top, with a clock icon and the reason they've returned.
4. **Follow-up nudges** — not emails themselves, but AI-generated reminder cards: "You sent an email to Akash Kumar Srivastava 5 days ago about the Nytarra notice with no reply. Follow up?"

The Focused tab is not a separate inbox — it is a filtered view of the same underlying email store. An email appearing in Focused also appears in its client tab. There is no duplication of data; only duplication of display.

**Training the Focused tab (Outlook-style):**
- Right-click any email in Focused → "Move to [Client Tab] only" — removes it from Focused for this sender going forward
- Right-click any email in a client tab → "Always show in Focused" — elevates all future emails from this sender to Focused
- The AI learns from these choices; within 2 weeks of use, the routing is near-perfect without manual intervention

---

### 15.7 Interaction Vocabulary — Inbox by Google Actions

**Done (✓)** — the primary action. Marks the email as actioned and removes it from the active view. It is NOT deleted. It is permanently stored in the matter's Communication tab (Module 1). Done emails are accessible via the matter record, via search, or via the Done folder. The goal is inbox zero through actioning, not deletion.

**Snooze (🕐)** — hides the email until a chosen time. Options:
- Later today (default: 3 hours from now)
- Tomorrow morning (8:00 AM next day)
- Next week (Monday 8:00 AM)
- Before a hearing (if a hearing date is linked to the matter — "Remind me 2 days before the April 15 hearing")
- Custom date/time
- On specific location (future feature — "When I arrive at court")

Snoozed emails surface at the top of the relevant tab with a clock icon and a one-line note of why they reappeared.

**Pin (📌)** — keeps the email at the top of its matter bundle, regardless of date. Used for emails the attorney needs to return to: an unresolved query from opposing counsel, a registry communication that needs a reply, a client instruction that needs confirmation. Pinned emails are also shown in the Focused tab.

**Sweep** — available on matter bundle headers. Archives all non-pinned emails in a bundle in one action. The most powerful bulk action in the module. After a matter is resolved, the attorney sweeps the bundle — all emails are marked Done and stored in the matter Communication tab.

**Flag for follow-up (🚩)** — Outlook-style flag. Adds the email to a follow-up list visible in the sidebar. Unlike snooze (time-based), flagging is indefinite — the email stays flagged until manually cleared. Used for emails awaiting external action (court reply, client approval, counterparty response).

**Keyboard shortcuts (desktop):**
- `E` — Done (archive)
- `S` — Snooze (opens snooze picker)
- `P` — Pin / Unpin
- `R` — Reply
- `F` — Forward
- `N` — New email
- `1` through `8` — switch to tab 1–8
- `J` / `K` — next / previous email in list
- `Enter` — open selected email
- `Esc` — close open email, return to list
- `Cmd+Enter` — send
- `Cmd+Z` — undo last action (Done, Snooze, etc.)

---

### 15.8 Rules Engine — Outlook-Grade Automation

Behind the visual simplicity is a full Outlook-equivalent rules engine. Attorneys and partners configure rules that run automatically on every incoming email.

**Rule conditions (can be combined with AND/OR):**
- Sender email address or domain (e.g. `@ipindia.gov.in` → always matters, always Focused)
- Sender name contains
- Subject line contains or matches (e.g. "Examination Report" → specific bundle)
- To or CC contains
- Body contains keyword
- Has attachment
- Email size over threshold

**Rule actions (one or more per rule):**
- Route to tab (e.g. always go to "Petalveda Scents" tab)
- Add to matter bundle (auto-link to specific matter ID)
- Pin automatically
- Flag for follow-up
- Apply category / colour label
- Show in Focused
- Mark as read immediately (for automated system emails that don't need review)
- Create a deadline in the docketing engine (e.g. email from TM Registry + "Examination Report" in subject → create a 30-day response deadline)
- Notify via WhatsApp alert (for urgent senders like courts)
- Auto-reply with template (e.g. acknowledgement receipt)

**Pre-built rules for Indian IP practice (loaded by default):**
- `@ipindia.gov.in` → Focused + create deadline + notify via WhatsApp
- `@delhihighcourt.nic.in` → Focused + Flag
- `@cgpdtm.gov.in` (Patent Office) → Focused + create deadline
- Subject contains "Examination Report" → create deadline (30 days TM, 12 months Patent)
- Subject contains "Opposition" → Focused + pin
- Subject contains "Show Cause" → Focused + urgent flag + WhatsApp alert

Partners can create firm-wide rules. Associates can create personal rules. Personal rules layer on top of firm rules; firm rules cannot be disabled by associates.

---

### 15.9 Composition — Drafting Outgoing Mail

**Compose window** — opens as a panel at the bottom-right of the screen (Gmail-style), not a full-screen modal. The rest of the inbox remains visible. Compose panel can be expanded to full-screen or minimised to a title bar while drafting.

**Compose panel anatomy:**
```
┌──────────────────────────────────────────────────────────────────────┐
│  New Email                                          [—] [□] [×]      │
│  To: ___________________________________________  [CC] [BCC]          │
│  Subject: ________________________________________________           │
│  Matter: [Petalveda Scents — TM Opposition ▾]                        │
│  Template: [Select template ▾]   Letterhead: [✓ Persistas & Partners]│
├──────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  [Rich text body — DM Sans 14px, 1.6 line height]                   │
│                                                                       │
│  AI Assist: "Draft a reply to this notice" →                        │
│                                                                       │
├──────────────────────────────────────────────────────────────────────┤
│  📎 Attach from matter   🖊 Signature   📋 Template   🤖 AI Draft    │
│                                          [Send]  [Schedule ▾]        │
└──────────────────────────────────────────────────────────────────────┘
```

**Key composition features:**

**Matter linking at compose time** — every outgoing email is linked to a matter before sending. If the "To" address matches a known client, the matter field auto-fills. If multiple matters match the sender, a dropdown appears. The attorney cannot send a professional email without it being matter-linked (configurable — can be made optional for personal emails).

**Template quick-insert** — the Template dropdown surfaces the firm's document template library (Module 9.1) filtered to email templates. Standard letters (acknowledgement of notice, interim response, submission receipt confirmation) appear here. Selecting a template pre-fills the subject and body, with variable fields highlighted for completion.

**Persist letterhead** — toggle to add the Persistas & Partners email letterhead (logo, partner contact details, footer) to outgoing emails. Applied as an HTML email template, not as an image attachment.

**AI Draft** — "Draft a reply to this notice based on the matter context." Claude reads the incoming email being replied to, reads the matter record, and drafts a contextually appropriate response. The draft appears in the compose body for attorney review and editing. Always marked as AI-generated until the attorney clears it.

**Attach from matter** — opens the matter document store directly. Attorney browses and attaches documents without navigating to the Documents module. Attachments are automatically cleaned of metadata before sending (Module 9.4 — clean-on-send enforced).

**Schedule send** — send at a specific time (8:00 AM next morning, before a deadline, after a weekend). Scheduled emails are stored in a Scheduled folder and can be cancelled before they send.

---

### 15.10 Thread View — Reading an Email

When the attorney opens an email, the inbox list slides left (three-pane layout: tab navigation → email list → email content). The email content pane shows:

**Email header:**
- Sender (full name + avatar initials + email address)
- Date and time (full, not relative)
- To / CC / BCC (expandable)
- Matter tag (linked, clicking opens the matter)
- Linked deadline (if the AI created a deadline from this email, shown as a chip)

**Email body:**
- Rendered HTML email (or plain text if HTML not available)
- Attachments shown as large preview cards below the body — PDF preview on hover, open-in-viewer on click
- Inline images displayed inline (not as attachments)

**Smart highlights (Inbox by Google feature adapted for legal):**
- Dates mentioned in the body are highlighted and shown as a chip above the body: "Hearing date: April 15, 2026 → [Add to docket]"
- Monetary amounts: "₹50,000 → [Log to matter]"
- Document references: "Annexure A" → if found in matter document store, shown as a chip link
- Opposing party names: if matched to matter record, highlighted

**Reply bar** — always pinned at the bottom of the email view:
- Quick reply chips (AI-generated, 2–3 options): "We acknowledge receipt" / "We will respond by [date]" / "Please clarify"
- Full reply: opens the compose panel in reply mode
- Forward: opens compose panel in forward mode

**Action bar** — right edge of email view:
- Done (E)
- Snooze (S)
- Pin (P)
- Flag (F)
- Move to bundle
- Print / Export to PDF
- Create deadline from this email
- View in matter record

---

### 15.11 Notifications and Alerts

**In-app notifications** (bell icon, top-right of Persist Desktop):
- New email in Focused tab
- New email in a pinned matter bundle
- Snoozed email resurfacing
- Follow-up nudge

**System notifications (OS native — Tauri notification plugin):**
- High-priority emails only: court notices, opposing counsel replies to active matters, registry emails
- Shows sender, subject, first line of body
- Action buttons in the notification: "Mark Done" / "Snooze" / "Open"

**WhatsApp alerts (Meta Cloud API — per rule configuration):**
- Configurable per rule — not all emails trigger WhatsApp
- Triggered for: TM Registry examination reports, court orders, urgent opposing counsel notices
- Message format: "📧 New: [Subject] from [Sender] — [Matter name]. Open in Persist."

**Daily Brief integration (Module 11.1):**
- 5 most important unread emails surfaced in the morning Daily Brief
- Follow-up nudges from Mail appear as tasks in the Today view

---

### 15.12 Search

**Global mail search** — `Cmd+F` in mail view opens a full-text search across all emails:
- By sender, subject, body text
- By matter name or matter ID
- By date range
- By attachment type (has:pdf, has:attachment)
- By status (is:unread, is:pinned, is:snoozed, is:flagged)
- By tab (in:focused, in:petalveda)

Search results appear as a temporary view in the email list — the tab structure remains. Closing search returns to the previous tab view.

**Matter-scoped search** — from within a matter record, the Communication tab shows all emails linked to that matter with full-text search scoped to that matter only.

---

### 15.13 Sidebar — Left Navigation

The mail module has its own left sidebar, visible when Mail is the active module:

```
📥 Focused               [12]
── Client Tabs (pinned)
   Petalveda Scents       [3]
   Zenith Brands          [1]
   Rahul Industries
── 
📤 Sent
⏰ Snoozed               [2]
🚩 Flagged               [5]
📋 Scheduled             [1]
✓  Done (Archive)
🗑️ Bin
──
🏷️ Categories
   Court Notices          [4]
   Registry               [2]
   Client                 [8]
   Opposing Counsel       [1]
──
📁 All Matters (bundles)
```

**Categories** — Outlook-style colour-coded category labels, applied manually or via rules. An email can have multiple categories. Used for cross-matter filtering (e.g. "show me all Court Notices across all matters").

---

### 15.14 Email ↔ Matter Integration Points

This is the layer that makes Persist Mail different from every other email client:

- **Auto-link on receipt** — incoming email from a known sender (client, court, opposing counsel, registry) is automatically linked to the most recently active matching matter
- **Deadline creation from email** — examination report received → 30-day deadline automatically created in the docketing engine, linked to the matter
- **Document save from attachment** — PDF attachment in an email → one-click save directly to the matter document store, with metadata (sender, date, email subject) auto-filled
- **Time entry prompt** — after replying to a matter-linked email, Persist prompts: "Log time for this email? [5 min] [10 min] [Custom] [Skip]"
- **Communication tab** — every email (sent and received) linked to a matter is permanently stored in the matter's Communication tab — full thread view, fully searchable, never deleted
- **Billing integration** — emails can be tagged as billable communication; time spent on email (auto-tracked via the time logger) can be added to the matter invoice

---

### 15.15 Shortwave-Inspired Additions

Shortwave (the Inbox-by-Google spiritual successor built by ex-Google engineers) introduces several interaction patterns that sit cleanly on top of the existing Module 15 spec without replacing anything. These are additive — every section 15.1–15.14 remains exactly as written. What follows are the specific Shortwave elements worth adopting, translated into the Persist legal context.

---

#### 15.15.1 AI Thread Summary Line

Every thread in the inbox list shows a one-line AI summary directly below the subject line — always visible, never requiring the thread to be opened.

```
┌─────────────────────────────────────────────────────────────────────────┐
│  ◉  Akash Kumar Srivastava (Advocate)             Today  2:14 PM  [PIN] │
│     RE: Reply to your notice dated March 31, 2026                       │
│     ✦ Counsel disputes passing off; requests 4-week extension to file   │
│     [PDF: Nytarra_reply_final.pdf]  [📎 1 attachment]                   │
│                                                    [DONE]  [SNOOZE]  [⋮] │
└─────────────────────────────────────────────────────────────────────────┘
```

The `✦` sparkle icon marks the line as AI-generated. The summary is one sentence — the actual legal substance of the email, not a vague paraphrase. Claude reads the full email and generates: "Counsel disputes passing off; requests 4-week extension to file counter-statement." For a court notice: "IPAB lists matter for hearing on 28 April 2026 at 2:30 PM." For a client email: "Client confirms TM renewal instruction for ALOVERA mark."

This is generated on email arrival and cached. The attorney never has to open an email to know whether it demands immediate action.

**In thread view** — the AI summary appears as a full sentence directly below the thread title, before the message chain begins:

```
Closing the deal with Rahul Industries                         [3 participants ▾]
✦ Client confirms instructions; requests draft NDA by Friday

── Thread ──────────────────────────────────────────
```

---

#### 15.15.2 Chat-Bubble Thread Rendering

When a thread has more than two participants or is an active back-and-forth exchange, the thread renders in **chat-bubble style** rather than as stacked email paragraphs.

- Outgoing messages (sent by the logged-in attorney) appear on the right, in a soft slate blue bubble
- Incoming messages appear on the left, with the sender's avatar and name above
- Each message shows sender name + time, not just the time
- Multi-participant threads show `[Name] & [Name] & You` as the thread group label below the subject

This applies only to active multi-party threads. Formal one-directional correspondence (TM Registry notices, court orders) renders as standard email paragraphs — the traditional format is more appropriate for legal correspondence that will be archived and cited.

**Toggle:** the attorney can switch any thread between chat-bubble view and traditional email view with one click. Default is determined by thread type (registry/court → traditional; client/counsel → chat-bubble).

---

#### 15.15.3 Live Typing Indicator

When another member of the firm (associate, paralegal, partner) is composing a reply to the same thread simultaneously, a typing indicator appears in the thread view:

```
[Sree Lakshmi is drafting a reply...  ●●●]
```

This prevents two attorneys from sending duplicate replies to the same opposing counsel email — a real problem in active matters handled by a partner-associate pair. The indicator is live, updates in real time via WebSocket, and disappears when they send or abandon the draft.

---

#### 15.15.4 `@mention` Highlighting in Thread View

When a message body contains `@Name` (e.g. `@Kajal — please review this before we respond`), the mention renders as a highlighted blue pill inline in the message text, not as plain text. The mentioned attorney receives a specific notification: "You were mentioned in an email about [Matter name]."

This is particularly useful in threads where multiple attorneys are CC'd — `@Kajal` makes the routing of responsibility explicit and visible in the thread, not buried in the email body.

---

#### 15.15.5 Contextual AI Right Panel

When a thread is open, a **contextual AI panel** appears on the right side of the thread view — not as a separate chat window, but as a smart sidebar that reads the current email and proactively surfaces relevant actions.

The panel is triggered by AI analysis of the open thread's content. It does not require the attorney to ask anything. Examples of what it surfaces:

**If the email contains a meeting request:**
```
┌──────────────────────────────────────────────────┐
│  ✦ Schedule a hearing prep call                   │
│                                                   │
│  [Oct]  Delhi HC Hearing Prep                     │
│  [28]   Mon, 28 Apr 2026                          │
│         2:00 PM – 3:00 PM                         │
│         kajal@persistas.com                       │
│         akashsrivastava753@gmail.com              │
│         Google Meet / Teams                       │
│                               [Create event]      │
└──────────────────────────────────────────────────┘
```

**If the email contains a deadline or a filing reference:**
```
┌──────────────────────────────────────────────────┐
│  ✦ Create a docket entry                          │
│                                                   │
│  Counter-statement due — 2 months from            │
│  opposition date (28 Apr 2026)                    │
│  → Deadline: 28 Jun 2026                          │
│  Matter: Petalveda Scents TM Opposition           │
│                               [Create deadline]   │
└──────────────────────────────────────────────────┘
```

**If the email is related to other threads in the matter:**
```
┌──────────────────────────────────────────────────┐
│  ✦ Related threads in this matter                 │
│                                                   │
│  I found 3 related emails. Add to a todo?         │
│  ○ Petalveda TM Opposition                        │
│                                                   │
│  Threads it would include:                        │
│  · Notice of Opposition (Akash, 28 Mar)           │
│  · Registry Acknowledgement (1 Apr)               │
│  · This email (RE: Reply, 10 Apr)                 │
│                                                   │
│  [Go to threads]          [Create todo]           │
└──────────────────────────────────────────────────┘
```

**If the email is from a new sender not in Persist contacts:**
```
┌──────────────────────────────────────────────────┐
│  ✦ New contact                                    │
│                                                   │
│  Akash Kumar Srivastava                           │
│  akashsrivastava753@gmail.com                     │
│  Mentioned role: Advocate                         │
│                                                   │
│  [Add to Petalveda Scents matter]                 │
│  [Add to contacts only]                           │
└──────────────────────────────────────────────────┘
```

The panel auto-collapses when no contextual suggestions are available. It never shows generic suggestions — only actions directly derived from the current thread's content. The attorney can dismiss any suggestion with one click.

---

#### 15.15.6 Thread-Bound Todos (Shortwave Pattern)

In Shortwave, a "todo" is not a separate task — it is a named collection of email threads grouped under a single action item. This pattern is directly applicable to legal matters.

**In Persist Mail**, a Todo is a named action item that bundles one or more email threads together:

```
TODOS
  ◎  Draft counter-statement for Nytarra opposition   [3 threads]
  ◎  Follow up: Rahul Industries NDA approval          [1 thread]
```

When an attorney creates a Todo, they name it (the action, not a thread subject), then link one or more threads to it. The Todo appears at the top of the Focused tab and in the Daily Brief.

This is distinct from the main Task system (Module 11.2). Thread-bound Todos are email-native tasks — they live in the mail module and are linked to specific email evidence. Module 11 tasks are matter-level actions that may or may not involve email.

The AI can also proactively suggest Todos from the contextual panel (15.15.5): "I found 3 related emails about the Nytarra opposition. Create a todo?" The attorney approves with one click.

**Todo completion:** marking a Todo done marks all its linked threads as Done (archive) in one action — the same as Sweep but with a named action item attached.

---

#### 15.15.7 Bundle Icons (Shortwave Pattern)

Each matter bundle in the thread list has a **custom icon** that makes it instantly recognisable without reading the name — borrowed from Shortwave's Finance ($), Feedback (💬), Travel (✈) bundle icons.

In Persist, bundle icons are mapped to matter type:

| Matter Type | Icon | Colour |
|---|---|---|
| Trademark | ® | Slate blue |
| Patent | ⚙ | Warm amber |
| Copyright | © | Sage green |
| Design | ◈ | Terracotta |
| Corporate / NDA | 📋 | Charcoal |
| Litigation | ⚖ | Muted red |
| Court / Registry | 🏛 | Warm grey |

The icon appears on the left of the bundle row in the thread list, replacing the generic folder icon. Bundles are scannable by icon + name — the attorney's eye finds "Trademark + Petalveda" before reading the full text.

---

#### 15.15.8 Stacked Participant Avatars in Thread Header

When an email thread is open, the top-right of the thread header shows stacked overlapping avatar circles for all participants — exactly as Shortwave renders them.

```
RE: Reply to your notice...                  [👤 👤 👤]  ☆  🏷  ⏰  🗑  ✓
```

Hovering the avatar stack expands it to show each participant's full name and email. Clicking a participant opens their contact record in Persist. This replaces the need to scan the To/CC fields to understand who is in the thread.

---

#### 15.15.9 AI Smart Reply Chips

At the bottom of every open thread, Persist shows 2–3 AI-generated quick reply options as pill buttons — directly below the message chain, above the compose bar:

```
[✦ We acknowledge receipt and will respond by 10 May]   [✦ Please clarify the examination objections]   [✦ We request a 30-day extension]
```

These are not generic suggestions. Claude reads the email content and the matter context (what type of matter, what stage, what the email contains) and generates legally appropriate reply options. Clicking a chip opens the compose window with that text pre-filled — the attorney reviews, edits, and sends.

For a TM examination report: chips suggest "We will file a detailed response within 30 days", "We request a hearing date", "We wish to amend the specification of goods."

For a court notice: chips suggest "We acknowledge receipt", "We will file our reply by [date]", "We are seeking instructions from our client."

For a client email: chips suggest appropriate client-specific responses based on the matter context.

The sparkle `✦` icon on each chip marks it as AI-generated. An attorney who sends a chip-selected reply does so knowing it is AI-suggested — the chip is never sent without a deliberate click.

---

*Note: Module 15 mail design specification updated v2.3 — April 2026. Sections 15.1–15.14 from v2.0 are unchanged. Section 15.15 adds Shortwave-inspired patterns.*

---

## 15A. Persist Editor — Markdown + Smart Tags (Global Writing Layer)

This module defines the **universal writing layer** embedded in every text input across Persist — email compose, document drafts, matter notes, meeting notes, and Persist Chat. It is not a standalone module with its own navigation entry; it is infrastructure that every other module sits on top of. Wherever an attorney types in Persist, this system is active.

It has three interconnected components:

1. **Live Markdown rendering** — the editor is a rich text surface where Markdown syntax renders instantly as the attorney types
2. **Project-specific expansion vocabulary** — per-matter shorthand definitions that expand inline
3. **Global smart tags** — `@` and `#` triggered data lookups that fetch live structured data from Persist's own records and from the legal intelligence layer (Module 18)

---

### 15A.1 The Editor — Live Markdown, No Raw Syntax

Every text field across Persist uses the same underlying editor: a **WYSIWYG Markdown editor** built on ProseMirror (the same foundation as Notion, Linear, and Coda). The attorney never sees raw Markdown syntax in the rendered view. They type naturally; formatting appears immediately.

**How it works:**

The attorney types `**` and immediately the cursor is inside a bold span — the asterisks disappear. They type `## ` at the start of a line and it renders as a heading, not as hash characters. The source is Markdown; the experience is a clean, formatted document.

Markdown is never forced. All formatting is also accessible via a floating toolbar that appears on text selection — the same Bold/Italic/Heading/Quote/List buttons found in any rich text editor. Attorneys who have never heard of Markdown use the toolbar; attorneys who know Markdown type faster with shortcuts. Both paths produce identical output.

**Supported Markdown syntax across all surfaces:**

| Syntax | Renders as | Use in legal writing |
|---|---|---|
| `**text**` or `__text__` | **Bold** | Emphasis on party names, key dates, operative clauses |
| `*text*` or `_text_` | *Italic* | Case names, Latin phrases, defined terms |
| `# Heading 1` | Large section heading | Document section titles |
| `## Heading 2` | Sub-section heading | Clause groupings |
| `### Heading 3` | Paragraph heading | Sub-clauses |
| `> quoted text` | Block quote | Quoting judgments, statutory text, opposing notice |
| `` `code` `` | Inline code | TM application numbers, case IDs, file references |
| `---` | Horizontal rule | Section dividers in notes |
| `- item` or `* item` | Bullet list | Arguments, facts, evidence list |
| `1. item` | Numbered list | Grounds of opposition, steps, clauses |
| `- [ ] task` | Checkbox (unchecked) | Action items in meeting notes |
| `- [x] task` | Checkbox (checked) | Completed actions |
| `[text](url)` | Hyperlink | Indian Kanoon links, court portals |
| `~~text~~` | ~~Strikethrough~~ | Deleted/superseded clauses in draft review |
| `==text==` | ==Highlighted== | Flagged text for partner review |

**Surface-specific Markdown behaviour:**

- **Email compose:** Markdown is rendered in the compose panel but exported as clean HTML when sent. The recipient sees formatted bold, headings, and bullet lists in their email client — not raw `**` characters.
- **Document drafts (Module 9):** Markdown drives the document body. When the document is compiled to PDF via LaTeX (Module 9.8), headings map to `\section{}`, bold to `\textbf{}`, blockquotes to the `quote` environment. The attorney writes Markdown; LaTeX handles the typesetting.
- **Matter notes and meeting notes:** Rendered Markdown stored as structured text. Exported as PDF or copied as formatted text with full fidelity.
- **Persist Chat (Module 21):** Chat inputs support Markdown. AI responses from Claude are rendered with Markdown formatting — the AI's use of `**bold**`, `> quotes`, and numbered lists renders correctly in the chat thread.

---

### 15A.2 Project-Specific Expansion Vocabulary

Every matter/case in Persist has its own **expansion dictionary** — a set of short user-defined abbreviations that expand to full text when typed. These are completely private to the matter and are defined by the attorneys working on it.

**How it works:**

The attorney opens any matter, goes to the Matter Settings → Expansions tab, and defines shorthand entries:

```
Shorthand         →  Expansion
──────────────────────────────────────────────────────
/p                →  Petalveda Scents Private Limited
/opp              →  M/s Nytarra (represented by Akash Kumar Srivastava, Advocate)
/tm1              →  Trade Mark Application No. 1234567 in Class 03
/court            →  The Trade Marks Registry, Delhi
/reg              →  The Registrar of Trade Marks
/date1            →  March 31, 2026
/notice           →  Legal Notice dated March 31, 2026 issued by Akash Kumar Srivastava, Advocate
/filing           →  April 09, 2026
```

When the attorney types `/p` followed by a space or punctuation in any editor within this matter's context, the text instantly expands to the full expansion. The slash prefix is the trigger. If the expansion doesn't fire (wrong context, mid-word), the text stays as typed.

**Matter context awareness:**

Expansions are scoped to the matter currently open. When composing an email linked to the Petalveda Scents matter, `/p` expands. When composing an unlinked email or a different matter's document, `/p` does nothing. The system knows which matter the editor is operating within.

**Expansion types:**

- **Text expansions** — expand to a plain or formatted string (as above)
- **Date expansions** — `/today` always expands to the current date in `DD Month YYYY` format; `/deadline:next` expands to the next deadline date for the current matter
- **Block expansions** — `/without` expands to the full "Without Prejudice" header block, formatted and ready; `/omnibus` expands to the standard omnibus denial paragraph; `/reserve` expands to the standard rights-reserved closing
- **Citation expansions** — `/s11` expands to "Section 11(1) of the Trade Marks Act, 1999" (defined per-matter based on the applicable statutes)

**Global expansions vs matter-specific expansions:**

In addition to per-matter expansions, the firm admin can define **global expansions** available in every matter:

```
/firm      →  Persistas & Partners, Advocates
/address   →  80-A, Pocket-A, Mayuri Enclave, Mayur Vihar Phase-III, Delhi - 110096
/wp        →  Without Prejudice and Without Admission
/tma       →  The Trade Marks Act, 1999
/pa        →  The Patents Act, 1970
/ca        →  The Copyright Act, 1957
/ica       →  The Indian Contract Act, 1872
```

Matter-specific expansions override global ones if the same shorthand is defined in both.

---

### 15A.3 Smart Tags — Live Data Fetching Inline

Smart tags are the most powerful layer. They are triggered by `@` (for Persist internal data) and `#` (for legal references from the Indian Kanoon corpus). When the attorney types either trigger character, an **inline autocomplete dropdown** appears immediately — no menu navigation, no tab-switching. The attorney picks from the dropdown or types to narrow it, presses Enter, and a **rendered data chip** replaces the typed tag inline.

Data chips are not plain text — they are live, interactive inline elements that display structured data and remain linked to their source.

---

**`@` Tags — Persist Internal Data**

These fetch data from within Persist's own records.

**`@matter:`** — fetch matter details

```
@matter:petalveda
```
Renders inline as:
```
[Petalveda Scents — TM Opposition (Nytarra) · P&P-2026-TM-0042 · Active]
```
The chip is clickable — opens the matter. Hovering shows a mini-card: client, status, responsible attorney, next deadline.

**`@client:`** — fetch client details

```
@client:petalveda
```
Renders: `[M/s Petalveda Scents Private Limited · petalveda@gmail.com · +91 XXXXX]`

**`@deadline:`** — fetch deadline details

```
@deadline:next           → next deadline in the current matter
@deadline:P&P-2026-TM-0042   → specific matter's next deadline
@deadline:all            → list of all upcoming deadlines (renders as a formatted table)
```

**`@doc:`** — fetch a document from the matter store

```
@doc:examination-report
```
Renders as a linked document chip: `[📄 Examination Report — TM 1234567 · Received 01 Apr 2026]`
Clicking opens the document in the PDF viewer.

**`@contact:`** — fetch a contact's details

```
@contact:akash
```
Renders: `[Akash Kumar Srivastava, Advocate · C-70, New Delhi South Extn. · akashsrivastava753@gmail.com]`
Used in letters and emails to auto-insert opposing counsel details without retyping.

**`@clause:`** — insert a clause from the firm's clause library (Module 9.1)

```
@clause:omnibus-denial
@clause:without-prejudice
@clause:rights-reserved
@clause:no-likelihood-confusion
```
When selected, the full clause text is inserted at the cursor position as a Markdown block quote, formatted and ready. This is the inline equivalent of the clause library picker — faster because the attorney never leaves the editor.

**`@time:`** — insert a time-related value

```
@time:today          → 10 April 2026
@time:now            → 10 April 2026, 11:45 AM
@time:+30d           → 10 May 2026  (30 days from today)
@time:+7d            → 17 April 2026
```
Useful when drafting deadlines into letters: "Please respond within 30 days, i.e., by @time:+30d."

**`@party:`** — insert a party designation from the matter record

```
@party:plaintiff     → Petalveda Scents Private Limited (Plaintiff/Applicant)
@party:defendant     → M/s Nytarra (Defendant/Opponent)
@party:court         → The Trade Marks Registry, Delhi
```

**`@rule:`** — insert an expansion from the matter's expansion dictionary (same as `/shorthand` but tag-triggered)

```
@rule:notice         → Legal Notice dated March 31, 2026...
```

---

**`#` Tags — Legal Reference Data (Indian Kanoon + Corpus)**

These fetch data from the legal intelligence layer (Module 18) — the Indian Kanoon corpus, the constitutional corpus, and the firm's own reference library (Module 17).

**`#case:`** — fetch a case citation

This is the richest tag. It operates in three stages, selectable by the attorney:

```
#case:AIR2019SC123
```

**Stage 1 — Format + link (always happens):**

The raw citation is validated, normalised to the correct neutral / SCC / AIR format, and rendered as a formatted chip:
```
[AIR 2019 SC 123 · Tata Sons Ltd. v. Manu Kosuri & Ors. · Supreme Court · 2019]
```
The chip links to the full judgment on Indian Kanoon. Citation format is auto-selected based on context — SCC format for Supreme Court matters, AIR for High Court, neutral citation if available.

**Stage 2 — Headnote (expandable):**

A small `+` button on the chip expands it to show the case headnote or key holding:
```
[AIR 2019 SC 123 · Tata Sons Ltd. v. Manu Kosuri & Ors. · 2019]
  ▸ Held: A domain name serves as a trademark and its registration
    constituting passing off is actionable. The goodwill in a trademark
    extends to its use as a domain name.
```
The headnote is fetched from the Indian Kanoon API (Module 18) — the AI-generated holding summary for the judgment.

**Stage 3 — Passage pull (on demand):**

The attorney can click "Pull passage" → a panel opens on the right showing the full judgment text. The attorney selects the specific passage they want to quote, presses "Insert quote" → the selected text is inserted at the cursor position as a Markdown block quote with the citation appended:

```markdown
> "A domain name serves as a badge of origin and its misuse constitutes
> passing off in the same manner as trademark infringement."
>
> — *AIR 2019 SC 123 · Tata Sons Ltd. v. Manu Kosuri & Ors.*
```

The block quote is fully formatted — ready to paste into a pleading, letter, or examination reply without any further editing.

**`#section:`** — fetch a statutory provision

```
#section:TMA-11(1)     → Section 11(1) of the Trade Marks Act, 1999
#section:ICA-73         → Section 73 of the Indian Contract Act, 1872
#section:COI-19(1)(g)   → Article 19(1)(g) of the Constitution of India
```

Renders as:
```
[§11(1) Trade Marks Act, 1999 — Relative grounds for refusal of registration]
```

The chip expands to show the full statutory text on click. The attorney can select and insert any portion as a block quote.

**`#gazette:`** — fetch a gazette notification

```
#gazette:TM-2025-1234
```
Fetches the notification from the Gazette corpus (Module 18) and renders as a cited chip.

**`#ref:`** — insert from the matter's reference library (Module 17)

```
#ref:
```
Typing `#ref:` followed by any keyword opens an autocomplete dropdown searching the matter's saved reference library — cases, statutes, articles, and precedents. The attorney picks the reference and it is inserted as a formatted citation chip.

---

### 15A.4 The Autocomplete Dropdown — Interaction Design

When the attorney types `@` or `#`, an autocomplete dropdown appears immediately below the cursor. It is styled as a small floating card (max 320px wide, max 6 items visible, scrollable):

```
┌─────────────────────────────────────────┐
│  @ matter                               │
│  ─────────────────────────────────────  │
│  📁 Petalveda Scents — TM Opposition    │
│  📁 Zenith Brands — TM Application     │
│  📁 Rahul Industries — NDA Review      │
│  ─────────────────────────────────────  │
│  ⏰ @deadline:next  · due in 3 days     │
│  📎 @doc:examination-report            │
└─────────────────────────────────────────┘
```

**Interaction:**
- Typing narrows the dropdown instantly (fuzzy search)
- Arrow keys navigate; `Enter` selects; `Esc` dismisses
- The dropdown is aware of the current matter context — results are sorted by relevance to the open matter first
- For `#case:`, typing the citation directly (e.g. `#case:AIR 2019`) triggers a live search against the Indian Kanoon API — results appear as the attorney types, typically within 300ms

**Keyboard shortcut:** `Cmd+K` in any editor opens a command palette that surfaces all smart tags, expansions, and clauses in one unified search — for attorneys who prefer command palette style over inline triggers.

---

### 15A.5 Rendered vs Export Behaviour

The data chips embedded in a document are **live when inside Persist**. When a document is exported or compiled:

**Email (HTML):** Chips render as formatted inline text with hyperlinks. `@client:petalveda` becomes the client's name as plain text with their email linked. `#case:AIR2019SC123` becomes the formatted citation with a hyperlink to Indian Kanoon. The recipient sees clean, formatted content.

**PDF via LaTeX (Module 9.8):** Chips are serialised to their text values before LaTeX compilation. `@client:petalveda` becomes `M/s Petalveda Scents Private Limited`. `#case:AIR2019SC123` becomes the full formatted citation in the configured citation style. Block quotes from `#case:` passage pulls render as the LaTeX `quote` environment with the citation as a footnote.

**Copy to clipboard:** Chips copy as their text value — not as interactive elements. Pasting into Word or another application produces clean formatted text.

**Matter notes / internal records:** Chips remain live — data updates propagate. If the client's email address changes in the client record, `@client:petalveda` chips in all notes automatically reflect the new value without any manual editing.

---

### 15A.6 Per-Matter Expansion Management — UI

Each matter has an **Expansions** tab in the Matter Settings panel (accessible via the matter detail page → ⚙️ Settings → Expansions):

```
┌──────────────────────────────────────────────────────────────┐
│  Matter Expansions — Petalveda Scents TM Opposition          │
│                                               [+ Add]        │
│  Shorthand      Expansion                     Type    Action │
│  ─────────────────────────────────────────────────────────── │
│  /p             Petalveda Scents Pvt Ltd      Text    [Edit] │
│  /opp           M/s Nytarra (Akash Kumar...)  Text    [Edit] │
│  /tm1           TM App. No. 1234567 Cl. 03    Text    [Edit] │
│  /notice        Legal Notice dated March...   Text    [Edit] │
│  /s11           Section 11(1) TMA 1999        Text    [Edit] │
│  ─────────────────────────────────────────────────────────── │
│  Global expansions also active in this matter (view/edit)    │
└──────────────────────────────────────────────────────────────┘
```

Adding or editing an expansion takes under 10 seconds — shorthand field + expansion field + save. No configuration complexity. The expansions are immediately active in all editors for this matter.

Expansions are **shareable within the matter team** — when an associate defines `/opp`, the same expansion is available to the partner working the same matter. They are matter-scoped, not user-scoped.

---

### 15A.7 Technical Implementation Notes (Keel + Deck)

**Deck (React) implementation:**
- ProseMirror as the editor core — the same engine used by Notion, Linear, GitLab's rich text editor
- Custom ProseMirror plugin for smart tag rendering — handles `@` and `#` trigger detection, dropdown rendering, and chip node type
- Custom ProseMirror plugin for `/expansion` — watches for slash-prefixed text sequences and fires expansion lookup
- Chips are ProseMirror `NodeView` — inline, non-editable, with click handlers
- Markdown parsing via `prosemirror-markdown` — bidirectional: Markdown → ProseMirror doc and ProseMirror doc → Markdown for storage

**Keel (Rust) implementation:**
- `expansions` table in SQLite: `(matter_id, shorthand, expansion_text, expansion_type, is_global, created_by, updated_at)`
- Smart tag data fetched via existing Keel commands: `get_matter()`, `get_client()`, `get_deadlines()`, `get_document()` — no new backend endpoints needed for `@` tags
- `#case:` tags make a Keel → Indian Kanoon API call (Module 18 infrastructure); result cached in SQLite for 24 hours per citation
- `#section:` statutory text loaded from the local constitutional + statute corpus bundled in the app
- Export serialisation: a `serialise_chips()` Keel command walks the ProseMirror document tree and replaces chip nodes with their text representations before LaTeX compilation or email HTML generation

---

### 15A.8 Where It Applies — Surface Summary

| Surface | Markdown | `/expansions` | `@` tags | `#` tags |
|---|---|---|---|---|
| Email compose (Module 15) | ✓ | ✓ | ✓ | ✓ |
| Document draft body (Module 9) | ✓ | ✓ | ✓ | ✓ |
| Smart Form free-text fields (Module 9.8) | ✓ | ✓ | ✓ | ✓ |
| Matter notes | ✓ | ✓ | ✓ | ✓ |
| Internal comments on matters/docs | ✓ | ✓ | ✓ limited | ✗ |
| Meeting notes (Module 11.3) | ✓ | ✓ | ✓ | ✓ |
| Persist Chat input (Module 21) | ✓ | ✓ | ✓ | ✓ |
| Task descriptions (Module 11.2) | ✓ | ✓ | `@matter` only | ✗ |
| Contract review notes (Module 10) | ✓ | ✓ | ✓ | ✓ |

`@` tags limited in comments = only `@matter:`, `@deadline:`, `@contact:` — no full data expansion
`#` tags not in task descriptions — task fields are single-line, not rich text

---

## 16. Advanced PDF Engine

Module 9.5 covered basic PDF workflows. This section upgrades it to a full professional-grade PDF editing suite embedded directly in Persist — no Adobe Acrobat, no external tool required.

### 16.1 Page Manipulation Tools

- **Extract pages:** select one or more pages → extract to a new PDF document
- **Rotate pages:** rotate any page 90°/180°/270° — individually or in bulk
- **Split document:** split a PDF at defined page boundaries into separate files (e.g. split a 200-page court bundle into individual orders)
- **Delete pages:** remove selected pages permanently
- **Reorder pages:** drag and drop pages in a thumbnail view to reorder
- **Replace pages:** swap a specific page with an updated version (useful for correcting a single page in a filed document)
- **Crop pages:** trim margins or crop to a specific area
- **Insert pages:** insert blank pages or pages from another PDF at any position

### 16.2 Page Numbering & Bookmarks

- **Number pages:** add page numbers in any position (header/footer, left/centre/right), with configurable start number, font, and format (1, i, A, etc.)
- **Bates stamping:** auto-number pages across a document set with firm-configured format (`P&P/2026/0001`)
- **Create bookmarks:** add named bookmarks to specific pages — renders as a clickable table of contents in any PDF viewer
- **Auto-bookmarks:** when generating a court bundle PDF, automatically creates bookmarks from the bundle index (one bookmark per document)
- **Exhibit stamping:** overlay "Exhibit A", "Exhibit B" etc. on designated pages for court submissions

### 16.3 OCR — Optical Character Recognition

- Convert any scanned PDF into a fully searchable, text-selectable document
- Multi-language OCR: English + Devanagari script support (for Hindi-language documents, government orders, state gazette notifications)
- OCR quality detection: system flags PDFs where text extraction quality is below threshold, suggesting re-scan
- Post-OCR, documents become eligible for full-text search within the matter document store
- All court-uploaded scanned orders / examination reports processed automatically via OCR on upload

### 16.4 Markup & Annotation Tools

A complete professional annotation toolkit — collaborative, persistent, and matter-linked:

**Text tools:**
- Textbox: add a floating text box anywhere on the page
- Typewriter: type directly onto the page in a fixed-position style (like typing on paper)
- Callout: text box with a directional leader line pointing to a specific area
- Note: small sticky-note icon, click to expand and read

**Drawing tools:**
- Pen: freehand drawing in any colour and thickness
- Highlight: colour-coded text highlighting (yellow, green, blue, pink, orange)
- Eraser: remove any markup
- Line: straight line connector
- Arrow: line with arrowhead — direction configurable
- Arc: curved line
- Polyline: multi-segment open line (connect multiple points)
- Rectangle: filled or outlined rectangle
- Circle: perfect circle / oval
- Ellipse: free ellipse
- Polygon: multi-sided closed shape
- Cloud: cloud-shaped border (standard markup annotation for indicating areas requiring attention in legal documents)
- Flag: attention marker on a specific area

**Review tools:**
- Stamps: apply pre-defined or custom stamps — `FILED`, `EXECUTED`, `CONFIDENTIAL`, `DRAFT — NOT FOR CIRCULATION`, `WITHOUT PREJUDICE`, `APPROVED`, `REJECTED`, custom firm stamps
- Flatten markups: burn all annotations permanently into the PDF (removes editability — used before filing)
- Unflatten markups: where annotations remain as separate layers, extract them back to editable form

### 16.5 Layers

- PDF layer support: view, show/hide, lock/unlock, and delete individual layers in multi-layer PDFs
- Create new layers: add content to specific layers (e.g. "Attorney Comments" layer, "Client Review" layer)
- Export with layers: export with all layers, selected layers only, or flattened

### 16.6 Document Comparison in PDF

- Compare two PDFs side by side — word-level difference detection
- Visual overlay mode: changes highlighted directly on the document
- Change summary panel: list of all additions, deletions, and formatting changes
- Supports comparison of scanned PDFs (OCR applied to both before comparison)

### 16.7 Shared Markup Sessions (Collaborative PDF Review)

This is the standout feature — multiple attorneys or a mixed attorney/client group can review and annotate the same PDF simultaneously in a live shared session.

**How it works:**
- Attorney creates a shared session from any PDF in the matter document store
- Invites participants by name (internal staff or client portal users)
- All participants see each other's cursors and annotations appear in real time
- Annotations are attributed to each participant (colour-coded by user)
- Chat panel on the right: in-session text discussion without leaving the document
- Session history: all annotations from the session saved permanently to the matter
- Session recording: full session replay — who annotated what, in what order, with timestamps
- Resolve/unresolve: participants can flag a markup as resolved (hidden from default view) or unresolved (still requires attention)

This replaces the workflow of emailing PDFs back and forth with comments, or using external tools like Zoom annotation or Adobe Review.

### 16.8 Smart Text — Direct PDF Field Editing with LaTeX Backend

This is the implementation of point 4 from the feature request — the ability to click on any text field or line in a generated PDF and edit it directly, with LaTeX automatically recompiling the document in the background.

**How it works:**
- All PDFs generated via the Smart Form Compiler (Module 9.8) carry a linked `.tex` source file
- When opened in the Persist PDF viewer, editable fields are shown with a subtle underline or highlight
- Clicking an editable field opens an inline text editor — the attorney types the new content directly on the document
- On confirmation, the backend injects the change into the `.tex` source and recompiles
- The PDF updates in place — no reformatting, no layout breaking, no need to go back to the form
- Non-generated PDFs (uploaded externally) do not have Smart Text — only Persist-generated documents
- Smart Text edits are tracked: every change is versioned with the editor's name and timestamp

This gives attorneys the "edit directly on the document" experience while preserving the LaTeX guarantee of perfect formatting.

---

## 17. Legal Reference Manager — Per Matter (Mendeley Equivalent)

### 17.1 What It Is

A citation and reference management system embedded in Persist, scoped **per matter** — not a global library. Every matter has its own reference library: the cases cited, statutes referenced, articles consulted, and precedents used in that matter. This is the legal equivalent of Mendeley, adapted for litigation and IP practice.

### 17.2 Reference Types Supported

- **Case law:** Supreme Court judgments, High Court orders, Tribunal decisions — linked to Indian Kanoon or uploaded manually
- **Statutes:** specific sections/articles of Indian legislation (Trade Marks Act 1999, Patents Act 1970, Copyright Act 1957, Indian Contract Act, IPC, CPC, etc.)
- **Constitutional provisions:** articles, clauses, schedules — linked to the Constitution
- **Gazette notifications:** official Government of India or State government notifications relevant to the matter
- **Academic/journal articles:** uploaded PDFs of legal articles, commentary, or research
- **Foreign case law:** cited foreign precedents (UK Privy Council, US cases for IP matters, EU decisions)
- **Books / treatises:** cited textbooks (e.g. P. Narayanan on IP Law, Mulla on Contract)
- **Custom / miscellaneous:** any other reference document

### 17.3 Core Features

**Adding references:**
- Search Indian Kanoon directly from within Persist — find a judgment, click "Add to Matter References" → saved instantly with full metadata (citation, court, date, coram, parties)
- Paste a citation string — AI extracts the metadata and creates the reference entry
- Upload a PDF — OCR extracts the citation details; attorney confirms
- Browser web clipper (future Phase): save any web-accessible legal resource to a matter reference library

**Reference record contains:**
- Citation (full neutral / SCC / AIR format)
- Court, bench, date
- Parties (petitioner / respondent)
- Key propositions for which it is cited in this matter (attorney-added notes)
- PDF of the full judgment (stored in matter document store)
- Tags: which argument/pleading this case supports
- Usage log: which documents in this matter cite this reference

**Organising references:**
- Group by: proposition argued, statute / article cited, document in which cited
- Tag references: "For — No Likelihood of Confusion", "Against — Prior Use", "On — Doctrine of Honest Concurrent Use"
- Notes and highlights: read, highlight, and annotate PDFs of judgments within the reference manager
- Shared reference library: within a matter, all assigned attorneys share the same reference library

**Citation generation:**
- Generate a formatted citation list from all references in a matter — SCC style, AIR style, or custom
- Insert citation footnotes directly into documents being drafted in Module 9
- Auto-formatted bibliography appended to any document with one click

**Notebook (Mendeley Notebook equivalent):**
- A per-matter research notebook — all highlights and notes from all referenced PDFs in one consolidated view
- Searchable: "Show me every note I made across all references about 'prior use'"

---

## 18. Legal Intelligence Layer — Indian Law Database Integration

### 18.1 What This Is

A curated, searchable, AI-indexed corpus of Indian public legal data — constitutional articles, statutes, Supreme Court and High Court judgments, gazette notifications, law commission reports, and constituent assembly debates — embedded directly into Persist. Attorneys search this corpus from within the platform without switching to Indian Kanoon, SCC Online, or Manupatra.

### 18.2 Data Sources (All Public Record)

| Source | Content | Access Method |
|---|---|---|
| Indian Kanoon API | 4.5M+ legal documents: SC, 24 HCs, 17 Tribunals, laws | Paid API (indiankanoon.org API) |
| Kanoon.dev API | Court structure, case data, judgments | REST API |
| Constitution of India | All 395 articles, 12 schedules, amendments | Public domain — ingested directly |
| Gazette of India | Central govt notifications, statutory orders | data.gov.in OGD API + scraping |
| Ministry of Law and Justice | Acts, rules, policy documents | Public website + OGD platform |
| Constituent Assembly Debates | Full debate records | Public domain |
| Law Commission Reports | Reports 1–280+ | Public domain PDFs |
| IP Office (TM / Patent Registry) | TM Journal, examination guidelines, practice notes | Public portal scraping |
| Supreme Court website | Cause lists, orders, recent judgments | Public portal scraping |

*All sources are public domain or openly licensed. Government legislative and judicial documents carry no copyright restrictions under Indian law.*

### 18.3 Search & Query Features

**Unified search bar:**
- Single search across all data sources simultaneously
- Results ranked by relevance to the matter type and practice area (if a matter is open, context-aware ranking boosts IP-related results for an IP matter)
- Filter by: source type (judgment / statute / gazette / constitution), court, date range, practice area

**Judgment search (Indian Kanoon integration):**
- Full structural analysis: every judgment segmented into Facts / Issues / Arguments / Precedent Analysis / Analysis of Law / Conclusion
- Precedent analyser: each citation in a judgment classified as Party / Neutral / Positive / Negative
- AI tags: what each judgment is about
- Click "Add to Matter References" → instantly added to the matter's reference library (Module 17)

**Statute search:**
- Search by section number, keyword, or plain-language description
- Cross-reference: every section linked to cases that have interpreted it
- Amendment history: see how a section has changed over time

**Gazette search:**
- Search notifications by ministry, date, subject, or keyword
- Save relevant notifications to a matter

**Constitutional search:**
- Browse or search by article number or keyword
- Every article linked to the judgments that have interpreted it

### 18.4 AI-Powered Legal Research Assistant

Built on Claude API, context-aware to the open matter:

- **Research query:** "Find all Supreme Court judgments where a trademark opposition was dismissed on grounds of no likelihood of confusion between word marks and device marks" → AI searches the corpus and returns a curated, ranked list with summaries
- **Statute interpretation:** "What does Section 11(1) of the Trade Marks Act say about relative grounds for refusal, and how has the Delhi High Court interpreted it in the last 5 years?" → AI synthesises the statutory text with relevant judgments
- **Constitutional research:** "Which articles of the Constitution protect freedom of trade and profession, and how has Article 19(1)(g) been applied in IP matters?" → AI answers with citations
- **Argument builder:** "I am arguing that my client's mark is descriptive and lacks distinctiveness under Section 9 of the TMA — find me supporting precedents"
- All AI research responses include: cited sources with links, confidence level, and a note that attorney must independently verify citations

### 18.5 ML / AI Training Pipeline

The legal corpus described in 18.2, combined with Persistas' own template library (Module 9.8) and document library (Module 3), forms the foundation for training and fine-tuning a **Persist-specific legal AI model** over time.

**Training data categories:**

1. **Constitutional corpus:** All articles, schedules, amendments, Constituent Assembly debates — provides deep constitutional understanding
2. **Statutory corpus:** All central legislation available in public domain — structured by act, section, clause
3. **Judgments corpus:** 4.5M+ judgments from Indian Kanoon — the largest available Indian legal corpus; structured with Facts / Issues / Arguments / Analysis / Conclusion segments
4. **Gazette corpus:** Notifications, statutory orders, TM Journal — for regulatory and IP context
5. **Template corpus:** Persistas' own approved document templates (once the template library is built under Module 9.8) — trains the model on the firm's specific drafting style and language
6. **Precedent corpus:** All documents in the matter reference library (Module 17) over time — creates a firm-specific legal knowledge base

**What this enables (future AI capabilities):**

- **Style transfer:** AI drafts that sound exactly like Persistas' own documents, not generic legal language — because it has been trained on the firm's actual templates
- **Matter-specific case prediction:** based on the facts of a matter, AI suggests likely judicial outcomes based on similar historical cases
- **Examination report auto-reply:** AI trained on hundreds of TM examination report replies can draft a near-complete reply given only the examination report as input — with accuracy improving over time
- **Judgment outcome prediction:** given a set of facts and the applicable law, AI estimates the likelihood of success based on historical outcomes

**Data governance:**
- Client documents are never used for training without explicit per-client opt-in consent
- Firm templates and internal documents are used for firm-specific training only — never shared with any external model provider
- All training is done on Anthropic's API with fine-tuning capabilities or via a separate self-hosted model pipeline (to be determined in Phase 7)
- The training pipeline is a long-term initiative — the data collection and tagging begins from Day 1 (every document generated or reviewed in Persist is tagged for potential training use), but active model fine-tuning begins in Phase 7

---

## 19. Microsoft 365 Integration

### 19.1 Scope and Approach

Microsoft 365 integration is feasible, well-documented, and manageable in scope. Microsoft provides the **Microsoft Graph API** — a unified REST API that gives access to all M365 services: Outlook (email + calendar), Teams, OneDrive, SharePoint, and Word. Integration does not require a Word add-in ribbon (which is excluded from scope) — it operates at the data layer via API.

The integration is additive — Persist remains the primary platform; M365 provides data feeds and sync.

### 19.2 Outlook Integration (Email + Calendar)

**Email sync via Microsoft Graph:**
- Attorneys who use Outlook as their primary email can connect their M365 account to Persist via OAuth 2.0
- Incoming emails flow into the Persist Mail Module (Module 15) — synced from Outlook in real time
- Emails sent from Persist can optionally be sent via the attorney's Outlook/Exchange account (so they appear in Outlook Sent folder and carry the firm's Exchange domain)
- Two-way sync: emails read or marked Done in Persist are reflected in Outlook, and vice versa

**Calendar sync via Microsoft Graph:**
- Outlook calendar events (court dates, client meetings, internal reviews) sync into Module 11's meeting intelligence
- Deadlines created in Persist's docketing engine (Module 2) can be pushed to the attorney's Outlook calendar as calendar events — with matter name, description, and reminder
- Two-way: if an attorney creates a court date in Outlook, it appears in their Persist Today view

### 19.3 Microsoft Teams Integration

#### Teams as the Scale Messaging Layer

Persist's native collaboration is handled by the Matter Discussion tab (Module 1.4) — a matter-scoped comment thread that covers the real-time communication needs of a small firm without requiring a general messaging infrastructure.

When Persist scales beyond Persistas & Partners — either to more attorneys internally or to other firms — the messaging architecture has two paths:

**Path A — Extend Matter Discussion internally.** The Discussion tab's WebSocket infrastructure and `matter_comments` table are designed as the foundation of a general messaging system. At 5+ attorneys, firm-wide channels and direct messages are added on top of the same server infrastructure (see Module 1.4.8).

**Path B — Microsoft Teams as the external messaging layer.** Firms already living in Teams keep their communication there. Persist integrates deeply with Teams so that matter context, deadlines, and action items flow between the two systems without duplication. This is the path for enterprise clients or firms with existing M365 mandates.

Both paths are supported. The choice is made per-firm at configuration time.

---

**Matter-linked Teams channels:**

When a new matter is opened in Persist, the attorney can optionally auto-create a linked Teams channel named after the matter: `#petalveda-scents-tm-opposition`. The channel is created in a designated "Matters" Teams team, provisioned via the Microsoft Graph API (`POST /teams/{team-id}/channels`).

What this enables:
- All matter-related team communication in Teams is linked from the matter record in Persist — an attorney can open the matter in Persist and jump to the Teams channel in one click
- `@mention` in Teams → if the mentioned attorney's name matches a Persist user, creates a linked task in Module 11.2 with the Teams message as the task description
- The Teams channel becomes the external collaboration space (for firms using Teams with clients or outside counsel), while the Persist Discussion tab remains the internal matter-native record

**Channel naming convention:** `{matter-type}-{client-short-name}-{matter-short-desc}` — e.g. `tm-petalveda-opposition`, `pat-rahul-formula-x`. Configurable per firm.

**Channel archive on matter close:** when a matter is closed or archived in Persist, the linked Teams channel is automatically archived in Teams (read-only, not deleted). The channel history is preserved for the record.

---

**Meeting notes sync:**

Teams meeting transcripts and recordings (stored in SharePoint/OneDrive after the meeting) can be imported into Persist's meeting intelligence (Module 11.3). Keel polls the linked SharePoint folder via Graph API, retrieves the transcript `.vtt` file, and runs it through the same AI action-item extraction pipeline used for in-app meeting notes. Extracted action items are linked to the relevant matter and appear in the Task module.

This means an attorney who runs a client call on Teams gets the same AI meeting notes experience as one using Persist's native calendar integration — without changing how they run their meetings.

---

**Persist → Teams notifications:**

Persist can post to a designated Teams channel when key matter events occur — useful for partners who prefer Teams over email for monitoring:

- Deadline created or escalated to Critical
- Examination Report received (auto-docketed)
- Invoice sent to client
- Matter status changed

Configurable per matter and per event type. Posts are formatted as Teams Adaptive Cards — showing matter name, event, and a deep link back to the matter in Persist.

---

**The architecture decision:** Teams handles broad team communication; Persist handles matter intelligence. The integration makes them complementary, not competing. An attorney's workflow becomes: open Persist for matter work → jump to Teams for quick team chat → return to Persist to log action items, deadlines, and documents. No data lives in two places — Persist is always the source of truth; Teams is the communication layer.

### 19.4 OneDrive / SharePoint Integration

**Document sync:**
- Attorneys can access OneDrive / SharePoint files directly from within Persist's document picker — without downloading and re-uploading
- Documents generated in Persist can be saved to a linked SharePoint document library (e.g. a matter-specific SharePoint folder) in addition to Persist's own storage
- SharePoint serves as a secondary document backup layer for firms that require M365 as their document of record

### 19.5 Word Integration (Light-touch, via Graph API)

Rather than a full Word add-in ribbon (out of scope), Persist offers light-touch Word integration:
- **Open in Word:** any DOCX document in Persist can be opened directly in Word Online (browser) or Word desktop via a deep link — edits saved back to Persist automatically via OneDrive sync
- **Import from Word:** attorneys can import a DOCX from their OneDrive into a matter's document store in one click
- This covers 95% of the "Word integration" use case without the complexity of a full Office add-in

### 19.6 Authentication

- All M365 integration uses OAuth 2.0 via Microsoft Entra ID (formerly Azure Active Directory)
- Each attorney connects their personal M365 account individually — Persist never stores M365 credentials
- Permissions requested are granular: Mail.ReadWrite, Calendars.ReadWrite, Files.ReadWrite, Teams.ReadBasic — scoped to minimum necessary access
- Attorneys can disconnect their M365 account from Persist at any time

### 19.7 Phase Assignment

M365 integration ships in **Phase 3** alongside the calendar sync and notification work — it is not significantly more complex than the Google Calendar integration already planned, since both use the same Microsoft Graph API approach. The Outlook email sync extends naturally into the Mail Module (Module 15).

---

## 20. Template Library & ML Training Roadmap

### 20.1 Template Library Build Strategy

Templates for the Smart Form Compiler (Module 9.8) are built in two parallel tracks:

**Track A — Persistas' Own Documents:**
As Pratik and the partners share actual firm documents (beginning with the Reply to Legal Notice already analysed), each document is processed to extract its structure, variable fields, and boilerplate language, then converted into a `.tex` template with a corresponding React intake form definition. Every shared document accelerates the template library.

**Track B — Court/Registry Format Templates:**
For documents with prescribed formats (vakalatnama, affidavit, power of attorney), templates are built from the official format published by the relevant court or registry. These require no firm documents — only the official format specification.

**Template versioning:**
Every template carries: version number, date created, approved by (partner name), last revised date, applicable court/registry, and format source reference. Associates always see which version they are using.

### 20.2 ML Training Data Tagging Protocol

Every document that enters Persist is eligible for inclusion in the training corpus — subject to governance rules:

| Document type | Tagged automatically? | Requires consent? |
|---|---|---|
| Firm-generated templates | Yes | No (firm property) |
| Public judgments (Indian Kanoon) | Yes | No (public domain) |
| Constitutition / statutes / gazette | Yes | No (public domain) |
| Client-uploaded documents | No | Yes — explicit per-client opt-in |
| Matter documents (firm work product) | No | Partner approval required per matter |
| AI-generated drafts | Yes (with attorney corrections) | No |

**Tagging schema for training data:**
Each document tagged with: document type, practice area, Indian statute(s) referenced, court/registry, outcome (for judgments: allowed/dismissed/settled), language (English/Hindi/mixed), and quality score (attorney rating of the document quality on a 1–5 scale).

---

## 21. Persist Chat — Context-Aware AI Assistant (Littlebird Chat Equivalent)

### 21.1 The Core Idea

Littlebird Chat's defining insight is this: <b>a chat interface is only as useful as the context behind it.</b> Generic AI assistants make you explain everything from scratch — copy-pasting documents, describing the situation, catching the model up. Littlebird eliminates this by reading your screen constantly, so the AI already knows what you're working on.

Persist Chat does the same thing, but with a critical legal upgrade. Littlebird reads your screen across all apps. Persist Chat reads your **entire legal practice** — every matter, every document, every deadline, every email, every contract clause, every precedent, every billing record, every draft — all structured, all searchable, all matter-linked. No screen reading needed. The context is already inside the platform.

When an attorney opens Persist Chat, the AI already knows:

- Which matter they are currently viewing
- Every document in that matter — drafts, filed copies, correspondence, court orders
- Every deadline in the docket — past, present, future
- Every email in the matter communication thread
- Every precedent saved to the matter reference library
- Every contract intelligence extraction for the matter
- The client's full history across all matters
- Every time entry and billing record
- Every note made by any attorney on the matter
- The full firm-wide template and clause library

This is not a chatbot bolted onto the side of the platform. It is the **single unified interface that connects every module** — the one place where an attorney can ask anything about anything, and get an answer grounded in the actual facts of their practice.

---

### 21.2 Access — How Persist Chat Opens

**Web platform — Hummingbird trigger (named after Littlebird's equivalent feature):**
- Keyboard shortcut (`Cmd + /` or `Cmd + Space` within Persist) opens the chat pane as a floating overlay on top of whatever the attorney is currently working on
- Does not navigate away from the current screen — the attorney continues to see their matter, document, or task beneath the chat
- Chat pane is resizable: compact side-panel, half-screen, or full-screen mode
- Dismissed instantly with `Esc` — the underlying screen is exactly as left

**iOS app:**
- Dedicated Chat tab in the firm app
- Also accessible via a floating chat button on every screen — tap to open inline without leaving the current view
- Continues the conversation from the web session — full cross-device context continuity

**Context awareness by screen:**
When Persist Chat opens, it reads the current state of the platform — which matter is open, which document is active, which deadline is selected — and primes the AI with that context automatically. The attorney does not need to explain what they are working on.

Examples of how context changes the behaviour:

| Current screen | What Persist Chat already knows when opened |
|---|---|
| Matter detail — Zenith Brands TM Opposition | Full matter record: client, filing date, all deadlines, all documents, all emails, all precedents |
| Document editor — TM Examination Reply draft | The current draft content, the matter it belongs to, and the examination report it is responding to |
| Contract review — Prism Technologies NDA | All extracted smart field values, flagged clauses, and the review project state |
| Daily Brief / Today tab | All open matters, all deadlines this week, all pending tasks |
| Invoice — Rahul Industries | Invoice line items, matter history, payment status |

---

### 21.3 What You Can Ask — Capabilities by Category

**Matter research:**
- "What is the current status of the Zenith Brands matter?"
- "When is the next deadline for Petalveda Scents and what needs to be done?"
- "List all matters where the opposition window closes in the next 60 days."
- "Which matters have had no activity in the past 14 days?"
- "What documents are pending partner approval across all matters?"

**Document work:**
- "Summarise the examination report in the current matter."
- "What are the grounds of objection raised in this examination report?"
- "Draft a reply to point 3 of the examination report based on our standard distinctiveness argument."
- "Compare the current draft with the previous version — what changed?"
- "Find all clauses in this NDA that differ from our standard template."
- "Which of our precedents support the argument that the mark is inherently distinctive?"

**Legal research:**
- "Find Supreme Court judgments on honest concurrent use of trademarks in the FMCG sector."
- "What does Section 11(1) of the Trade Marks Act say and how has it been interpreted?"
- "Show me Delhi High Court judgments from the last 3 years on trade dress infringement."
- "Is there a constitutional provision protecting the right to carry on a trade or business?"
- "Summarise the key holdings in the top 5 cases on passing off in India."

**Client and billing:**
- "What is the total outstanding amount for Rahul Industries across all matters?"
- "When was the last time we sent a status update to Petalveda Scents?"
- "How many hours have been logged on the Zenith Brands matter this month?"
- "Which clients have unpaid invoices older than 30 days?"

**Task and workflow:**
- "What do I need to do today?"
- "Create a task: Draft TM examination reply for Zenith Brands, due in 10 days, assign to Meera."
- "What tasks are overdue across all matters?"
- "Set a reminder to follow up with Rahul Industries on the NDA next Monday."
- "Move the Petalveda Scents matter to 'Awaiting Client Response' status."

**Drafting assistance:**
- "Write a without-prejudice opening paragraph for a reply to a cease and desist letter."
- "Give me three alternative formulations for the distinctiveness argument in this examination reply."
- "Draft an email to the client summarising what happened at today's hearing."
- "Improve the language in paragraph 4 of the current draft — make it more formal."
- "Generate a meeting prep brief for tomorrow's call with Rahul Industries."

**Memory and recall:**
- "What did we discuss in the last meeting with Petalveda Scents?"
- "Find the email where the client sent us the evidence of prior use."
- "When was the TM application for Zenith Brands filed and who handled it?"
- "What was the outcome of the last matter we handled for this client?"

---

### 21.4 How It Works — Technical Architecture

Persist Chat is powered by the **Claude API (claude-sonnet-4-6)** with a carefully structured context injection layer. Every time the chat is opened:

**Step 1 — Context assembly:**
The backend assembles a structured context payload from the current platform state:
```
Current matter: [matter record, status, parties, key dates]
Open documents: [titles, summaries, last-modified]
Active deadlines: [all deadlines, urgency flags]
Recent emails: [last 10 emails in the matter thread, summarised]
References: [list of cases and statutes saved to matter]
Active draft: [if a document is open in editor, its current content]
User profile: [attorney name, role, active matters]
```

**Step 2 — Query routing:**
Before sending to Claude, the query is classified:
- Legal research query → also searches the Indian Kanoon corpus (Module 18) and matter reference library (Module 17)
- Document query → retrieves the full document content if needed
- Action query (create task, change status) → routed to execute the action directly, then confirmed with the attorney
- General matter query → answered from the assembled context

**Step 3 — Response with citations:**
Every response includes source attribution — which document, email, judgment, or record the answer is drawn from. Attorneys can click any citation to jump directly to the source.

**Step 4 — Action execution:**
When the attorney confirms an action suggested in chat ("Yes, create that task"), it is executed immediately within the platform — no copy-pasting, no navigating to another screen.

---

### 21.5 Conversation Memory — Within and Across Sessions

**Within a session:**
The chat maintains full conversation history for the duration of the session. The attorney can build on previous questions — "Now draft that reply using the third argument we just discussed."

**Across sessions — matter memory:**
Every chat conversation linked to a matter is saved in that matter's record. The attorney can recall past chat sessions: "Show me the research I did on this matter last week." This is the legal equivalent of Littlebird's ability to recall anything you've seen on your screen — but scoped to the matter.

**Cross-matter memory:**
The attorney can ask questions that span matters: "Have we ever handled a case involving camphor trade dress before?" — Persist Chat searches across all matters for relevant history.

---

### 21.6 Chat in the iOS App

On mobile, Persist Chat is the **most-used feature of the iOS firm app** for anything that needs a quick answer on the go:

- "Am I free Thursday afternoon?" → checks calendar and matter deadlines
- "What's the status of Zenith Brands?" → summarises from matter record
- "Quick — what were the grounds of the examination report for Matter P&P-2026-TM-0042?" → retrieves from document
- "Draft a quick WhatsApp message to the client about the hearing date"

The iOS chat interface is conversational and minimal — a clean chat thread, no clutter, full context from the web session.

---

### 21.7 Privacy and Governance

- Chat conversations are stored encrypted, scoped to each attorney's account
- Attorneys can delete any chat session at any time
- Partner-level accounts can see whether chat has been used on a matter (audit log) but cannot read individual chat conversations of associates — privacy preserved within the firm
- All data processed by the Claude API is under Anthropic's data processing agreement — no training on firm data
- Sensitive matter data (client-privileged documents) is never sent to the API without the attorney's active session — context is assembled per-request, not stored permanently with the model provider

---

### 21.8 Phase Assignment

Persist Chat ships in **Phase 3** (Weeks 19–26) alongside the AI drafting assistant. The underlying infrastructure — Claude API integration, matter context assembly, document retrieval — is already being built for Module 7. Persist Chat is the natural evolution: instead of a structured intake form triggering Claude, it is a freeform conversational interface over the same context layer.

The difference from Module 7 (AI Drafting Assistant):
- Module 7 is a **structured workflow** — select a template, fill a form, generate a document
- Persist Chat is a **freeform conversation** — ask anything, get answers, take actions, all from one place

Both use the same Claude API backend. Persist Chat is the conversational shell around every AI capability in the platform.

---

### 21.9 UX Design Principles — Informed by Houston (gethouston.ai)

Houston (`gethouston.ai`) was evaluated as a potential foundation for Persist Desktop and ruled out — it is explicitly designed for single-user, personal-scale tools, incompatible with a multi-attorney firm platform. However, its UX philosophy for AI-native interfaces is the sharpest available thinking on how humans and AI should interact inside an application. The following four principles from Houston are directly adopted into Persist Chat's interaction design:

**1. Teaching, not configuring.** Attorneys never adjust AI behaviour through a settings panel. They teach Persist Chat by talking to it. *"Don't suggest WhatsApp messages for TM Registry deadlines — always use formal email."* Persist Chat writes this as a persistent rule against the relevant module context. The attorney sees the rule in plain English with an undo button. It survives session restarts automatically.

**2. Showing, not driving.** Persist Chat never navigates the attorney to another screen without being asked. When it creates a deadline, drafts a document, or finds a record, it emits an **action chip** — a small button inside the reply: *"I created the deadline. →Show me the matter."* The attorney clicks it if they want to navigate. The AI offers; the human decides. It never takes the wheel.

**3. Rules as data, not code branches.** The Smart Form Compiler (Module 9.8) and the contract intelligence extraction rules (Module 10.1) store per-matter and per-attorney customisations in structured data files, not in branching application code. When an attorney teaches a variation — *"for Rahul Industries matters, always show amounts in INR"* — the AI writes to the rules data, not to application logic. Edge cases accumulate cleanly in readable files rather than spreading through the codebase.

**4. Symmetric access.** The AI and the attorney operate on the same data with the same visibility. When Persist Chat creates a task, it appears in the Tasks module exactly as if the attorney created it manually. When the attorney updates a matter status, Persist Chat sees it immediately. There is no separate "AI view" of the data — one source of truth, read and written by both.

---

## 22. AI Infrastructure & Model Routing Strategy

### 22.1 Overview

Every AI-powered feature in Persist runs through a single unified routing layer in Keel. The Deck never selects a model — it calls one command (`ai_request`) with a task type, context, and prompt. Keel classifies the request, routes it to the appropriate Claude model, handles escalation if needed, and returns the result. The model selection is entirely invisible to the attorney.

This architecture implements Anthropic's **Advisor–Executor pattern** (announced April 2026): Opus 4.6 acts as the strategic advisor for complex reasoning; Sonnet 4.6 is the daily workhorse for most AI tasks; Haiku 4.5 handles high-volume lightweight operations. The result is near-Opus quality across the platform at roughly 15× lower cost than running everything on Opus.

---

### 22.2 The Three-Tier Model Map

```
┌──────────────────────────────────────────────────────────────────────────┐
│  TIER 1 — HAIKU 4.5                                                       │
│  claude-haiku-4-5-20251001                                                │
│  $1 input / $5 output per MTok  ·  Fastest  ·  200K context              │
│  Role: The Worker — high-volume, sub-second, no deep reasoning required   │
├──────────────────────────────────────────────────────────────────────────┤
│  • AI thread summary on email arrival (Module 15.15.1)                    │
│  • Smart reply chip generation (Module 15.15.9)                           │
│  • Email auto-routing and client tab assignment                            │
│  • Sentinel rule condition evaluation (Module 8.7)                        │
│  • Document metadata extraction on upload                                 │
│  • Form field auto-fill from matter record (Module 9.8)                   │
│  • Docket event date extraction from registry emails                      │
│  • OCR post-processing — clean raw Tesseract output (Module 16.3)         │
│  • Search query expansion and result re-ranking                            │
│  • Task priority scoring in Daily Brief (Module 11.1)                     │
│  • Citation format normalisation (Module 15A.3 — lookup phase only)       │
└──────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────────┐
│  TIER 2 — SONNET 4.6                                                      │
│  claude-sonnet-4-6                                                        │
│  $3 input / $15 output per MTok  ·  Fast  ·  1M context                  │
│  Role: The Doer — the platform's daily intelligence engine                │
├──────────────────────────────────────────────────────────────────────────┤
│  • Persist Chat — all conversational queries (Module 21)                  │
│  • AI Drafting — TM replies, C&D letters, client advisories (Module 7)   │
│  • Smart Form Compiler — template generation (Module 9.8)                 │
│  • Document comparison + AI summary (Module 9.3)                          │
│  • AI proofreading and document repair (Module 9.2)                       │
│  • Contract clause extraction — standard contracts (Module 10.1)          │
│  • Smart field extraction — risk scoring (Module 10.3)                    │
│  • Daily Brief assembly (Module 11.1)                                     │
│  • Meeting notes + action item extraction (Module 11.3)                   │
│  • Contextual AI right panel in mail (Module 15.15.5)                     │
│  • Case headnote and holding summary (Module 15A.3 — headnote phase)      │
│  • Routine intelligence reports (Module 11.4)                             │
│  • AI chat with contracts — document and project level (Module 10.4)      │
└──────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────────┐
│  TIER 3 — OPUS 4.6  — The Advisor                                         │
│  claude-opus-4-6                                                          │
│  $5 input / $25 output per MTok  ·  Moderate  ·  1M context              │
│  Role: Deep reasoning, called only when complexity demands it             │
├──────────────────────────────────────────────────────────────────────────┤
│  • Patent claim drafting from first principles (Module 7)                 │
│  • Complex IP strategy reasoning ("file divisional?", "abandon?")         │
│  • Multi-document due diligence report generation (Module 10.5)           │
│  • Legal research — cross-case synthesis, constitutional questions (18)   │
│  • Contract risk scoring on large/complex agreements (Module 10.3)        │
│  • Sonnet escalation: when Sonnet signals it is out of its depth          │
│  • Explicit attorney request: "Use deep analysis" toggle in Persist Chat  │
└──────────────────────────────────────────────────────────────────────────┘
```

---

### 22.3 Current Model Specifications (April 2026)

| | Opus 4.6 | Sonnet 4.6 | Haiku 4.5 |
|---|---|---|---|
| API ID | `claude-opus-4-6` | `claude-sonnet-4-6` | `claude-haiku-4-5-20251001` |
| Input price | $5 / MTok | $3 / MTok | $1 / MTok |
| Output price | $25 / MTok | $15 / MTok | $5 / MTok |
| Context window | 1M tokens | 1M tokens | 200K tokens |
| Max output | 128K tokens | 64K tokens | 64K tokens |
| Speed | Moderate | Fast | Fastest |
| SWE-bench score | 80.8% | 79.6% | 73.3% |
| Extended thinking | ✓ | ✓ | ✓ |
| Adaptive thinking | ✓ | ✓ | ✗ |

**Key insight:** Sonnet 4.6 scores within 1.2% of Opus 4.6 at 40% lower cost. For the vast majority of legal drafting and analysis tasks in Persist, Sonnet is the right default. Opus is reserved for the narrow set of tasks where that 1.2% genuinely matters — complex patent strategy, cross-case legal synthesis, large-scale due diligence.

**Haiku's 200K context ceiling** is a hard constraint. Any task requiring more than 200K tokens of context (large matter records, multi-document analysis, long contract portfolios) must be routed to Sonnet or Opus. The router enforces this automatically based on assembled context size.

---

### 22.4 Escalation Logic — How Routing Decisions Are Made

The routing brain lives entirely in `keel/services/ai_router.rs`. Three mechanisms determine which model runs:

**Mechanism 1 — Pre-classification by task type**

Before any model is called, the router classifies the request by its registered task type. Each task type has a designated tier:

```rust
// keel/config/ai_thresholds.rs

pub enum TaskType {
    // Always Haiku
    ThreadSummary, SmartReplyChips, EmailAutoTag,
    SentinelEval, MetadataExtract, FormAutoFill,
    DateExtraction, OcrCleanup, SearchExpansion,
    CitationLookup,

    // Always Sonnet
    PersistChat, AIDrafting, SmartForm,
    DocComparison, Proofreading, ContractExtraction,
    DailyBrief, MeetingNotes, MailContextPanel,
    CaseHeadnote, RoutineReport, ContractChat,

    // Always Opus
    PatentClaims, IPStrategyReasoning, DiligenceReport,

    // Sonnet with possible Opus escalation
    LegalResearch, RiskScoring, ComplexDrafting,
}
```

**Mechanism 2 — Context size gate**

Before calling Haiku, the router checks assembled context size. If it exceeds 180K tokens (leaving a safety margin below Haiku's 200K limit), it automatically upgrades to Sonnet:

```rust
pub fn route(task: &TaskRequest) -> Model {
    let base_model = task_type_to_base_model(&task.task_type);

    // Hard context ceiling — Haiku cannot handle > 180K
    if base_model == Model::Haiku
        && task.context_tokens > 180_000 {
        return Model::Sonnet;
    }

    // Large context forces Opus for quality-critical tasks
    if task.context_tokens > 600_000
        && task.quality_critical {
        return Model::Opus;
    }

    base_model
}
```

**Mechanism 3 — Sonnet confidence escalation**

For tasks in the `SonnetWithEscalation` class, Sonnet runs first. Its response is evaluated for confidence signals. If below threshold, the same request is re-run on Opus:

```rust
pub async fn run_with_escalation(task: &TaskRequest) -> AIResponse {
    let sonnet_response = call_claude(Model::Sonnet, task).await;

    let should_escalate =
        sonnet_response.confidence_score < CONFIDENCE_THRESHOLD  // < 0.72
        || sonnet_response.contains_uncertainty_markers()         // "I'm not certain..."
        || sonnet_response.legal_complexity_score > COMPLEXITY_THRESHOLD
        || task.attorney_requested_deep_analysis;

    if should_escalate {
        // Re-run on Opus — Sonnet response discarded
        return call_claude(Model::Opus, task).await;
    }

    sonnet_response
}
```

**Mechanism 4 — Explicit attorney override**

In Persist Chat (Module 21) and the AI Drafting panel (Module 7), the attorney can toggle a `⚙ Deep Analysis` switch. This bypasses all routing logic and forces Opus for that request only. The switch is off by default — the attorney activates it for tasks they know are complex: patent strategy discussions, multi-jurisdiction analysis, large due diligence.

The `⚙ Deep Analysis` toggle is visually subtle — a small icon in the Chat input bar and the Drafting panel, not a prominent button. It should be used intentionally, not habitually.

---

### 22.5 Why Sonnet, Not Haiku, Is the Escalation Judge

A critical design decision: **Haiku does not escalate to Opus**. Haiku's job is tasks that don't require judgment. Tasks that might need Opus always start at Sonnet.

This is intentional. Haiku's 73.3% benchmark vs Sonnet's 79.6% means it is materially less capable of assessing its own limitations — it may under-escalate on legal analysis because it cannot accurately judge the complexity of what it doesn't know. Sonnet is the appropriate escalation gate because it is close enough to Opus capability to make a reliable judgment about when Opus is needed.

```
Haiku  →  (never escalates)  →  Haiku output
Sonnet →  (evaluates confidence)  →  Sonnet output
                                  →  Opus output (if escalation triggered)
Opus   →  (always final)  →  Opus output
```

---

### 22.6 Full Module-by-Module Routing Table

| Module | Feature | Default | Escalates to | Escalation trigger |
|---|---|---|---|---|
| 7 | TM/IP letter drafting | Sonnet | Opus | Patent claims, complex opposition |
| 7 | Patent claim drafting | Opus | — | Always Opus |
| 8.7 | Sentinel rule eval | Haiku | — | Never |
| 9.2 | AI proofreading | Sonnet | — | Never |
| 9.3 | Doc comparison + summary | Sonnet | Opus | >20 pages or >3 documents |
| 9.8 | Smart Form generation | Sonnet | — | Never |
| 10.1 | Contract clause extraction | Sonnet | Opus | High-risk flag or complex clause |
| 10.3 | Contract risk scoring | Sonnet | Opus | Contract >50 pages |
| 10.4 | AI chat with contracts | Sonnet | Opus | Attorney requests deep analysis |
| 10.5 | Diligence report gen | Opus | — | Always Opus — high-stakes output |
| 11.1 | Daily Brief assembly | Sonnet | — | Never |
| 11.3 | Meeting notes + actions | Sonnet | — | Never |
| 11.4 | Routine intelligence | Sonnet | — | Never |
| 15.15.1 | Email thread summary | Haiku | — | Never |
| 15.15.5 | Contextual AI mail panel | Sonnet | — | Never |
| 15.15.9 | Smart reply chips | Haiku | — | Never |
| 15A.3 | Citation lookup | Haiku | — | Never |
| 15A.3 | Case headnote | Sonnet | Opus | Passage pull + analysis |
| 18 | Legal research | Sonnet | Opus | Cross-case synthesis, >5 cases |
| 21 | Persist Chat | Sonnet | Opus | `⚙ Deep Analysis` or confidence < 0.72 |

---

### 22.7 Keel Architecture — The Routing Layer

The routing layer is a self-contained service in Keel. Deck has no knowledge of model selection. Every AI call from the frontend goes through one Tauri command:

```
keel/
├── commands/
│   └── ai.rs                    # Single entry point: invoke('ai_request', ...)
│
├── services/
│   ├── ai_router.rs             # Routing brain — classifies, gates, escalates
│   ├── claude_client.rs         # Anthropic API calls — Haiku, Sonnet, Opus
│   ├── context_assembler.rs     # Builds the context payload for each request
│   └── confidence_evaluator.rs  # Parses Sonnet response for escalation signals
│
└── config/
    └── ai_thresholds.rs         # Tunable constants — no redeploy to adjust
```

**`ai_thresholds.rs` — tunable without redeployment:**

```rust
pub const CONFIDENCE_THRESHOLD: f32 = 0.72;
pub const COMPLEXITY_THRESHOLD: f32 = 0.80;
pub const HAIKU_CONTEXT_CEILING: usize = 180_000;
pub const LARGE_CONTEXT_OPUS_GATE: usize = 600_000;
pub const DEEP_ANALYSIS_DEFAULT: bool = false;
```

These constants are the knobs. If Opus is being triggered too often (cost spike), raise `CONFIDENCE_THRESHOLD`. If quality is suffering on edge cases, lower it. No code change, no redeploy — just a config update.

**The Deck side is clean:**

```typescript
// deck/lib/ai.ts — the only AI function Deck ever calls

export async function aiRequest(params: AIRequestParams): Promise<AIResponse> {
    return invoke<AIResponse>('ai_request', {
        taskType: params.taskType,
        context: params.context,
        prompt: params.prompt,
        deepAnalysis: params.deepAnalysis ?? false,
    });
    // Model selection happens entirely in Keel — Deck never knows which model ran
}
```

---

### 22.8 Prompt Caching — Cost Reduction on Top of Routing

Anthropic's prompt caching reduces costs by 90% on repeated input tokens. Persist uses this aggressively for the context that appears in every request:

- **System prompt caching:** the base system prompt for Persist Chat (matter context, firm profile, attorney role) is cached — the same ~5K token block is not re-billed on every message
- **Matter context caching:** when an attorney has been working on the same matter for several messages, the assembled matter context (documents, deadlines, communications) is cached as a prefix
- **Statute corpus caching:** the Indian statutory text loaded for legal research (Trade Marks Act, Patents Act, ICA, CPC) is a large fixed block — cached once per session, not re-billed per query

Implemented via the `cache_control: {"type": "ephemeral"}` blocks in the Anthropic API messages structure. Cache lifetime is 5 minutes per session; Persist Chat refreshes the cache on matter context changes.

Combined with three-tier routing, prompt caching reduces effective AI spend by an estimated **additional 25–40%** on top of the routing savings.

---

### 22.9 Cost Model — Production Estimate

Estimate for 10 attorneys, 500 active matters, moderate daily usage:

```
HAIKU  (70% of all calls by volume — tiny tasks)
  Thread summaries: ~200/day × 300 tokens avg       = 60K tokens/day
  Smart replies, auto-tag, extraction: ~500 × 200   = 100K tokens/day
  Total: ~160K output tokens/day × $5/MTok           = ~$0.80/day

SONNET (28% of calls — the daily workhorse)
  Persist Chat: ~100 queries × 2K tokens avg         = 200K tokens/day
  Drafting, analysis, daily brief: ~50 × 5K          = 250K tokens/day
  Total: ~450K output tokens/day × $15/MTok           = ~$6.75/day

OPUS   (2% of calls — deep work only)
  Complex analysis, claim drafting: ~10 × 8K         = 80K tokens/day
  Total: ~80K output tokens/day × $25/MTok            = ~$2.00/day

BASE TOTAL:                                          ~$9.55/day
Prompt caching savings (~30%):                       -$2.87/day
Batch API savings where applicable (50%):            -$0.50/day

ESTIMATED NET COST:                                  ~$6.18/day
                                                     ~$185/month
                                                     ~$22/month per attorney
```

**Comparison if everything ran on Opus:** ~$145/day → $4,350/month
**Routing + caching savings:** ~24× cost reduction

---

### 22.10 Phase Assignment

**Phase 0 (Tauri scaffold, Weeks 1–2):**
The routing infrastructure is built as part of the scaffold — the `ai_router.rs`, `claude_client.rs`, and `context_assembler.rs` stubs are created even before any features use them. This ensures every feature built in Phases 1–7 plugs into the routing layer from day one rather than calling the API directly.

**Phase 3 (Weeks 19–26):**
First production use — Persist Chat (Module 21) and AI Drafting (Module 7) go live. The full three-tier routing is active. Escalation thresholds are monitored and tuned during this phase.

**Phase 4+ :**
Every subsequent AI feature (Smart Form Compiler, contract extraction, daily brief, mail summaries) plugs in to the same routing layer — no new API plumbing per feature.

---

### 22.11 Attorney-Visible AI Indicators

Attorneys should know when AI is involved in what they're reading, without being overwhelmed by disclaimers. Three visual signals throughout the platform:

- **`✦` sparkle icon** — marks AI-generated content inline (thread summaries, smart reply chips, action panel suggestions). Small, non-intrusive, always present on AI output.
- **`DRAFT — AI GENERATED`** banner — appears on any AI-drafted document in the drafting suite (Module 7, 9.8). Stays until the attorney explicitly removes it by confirming the draft. Required for all documents that could be shared with clients or courts.
- **`⚙ Deep Analysis`** indicator — when a Persist Chat response was generated by Opus (either via escalation or attorney request), a small `⚙` chip appears on the response bubble indicating deeper analysis was used. Allows attorneys to trust the response level appropriately.

No other model attribution is shown. Attorneys do not need to know whether a response came from Sonnet or Haiku — only whether deep analysis (Opus) was applied.

---

## 23. Persist Voice — Global Voice Capture Layer

### 23.1 Overview

Persist Voice is a global voice capture layer embedded throughout the entire desktop application — the Persist equivalent of Wisprflow, built natively into Keel and Deck rather than as a separate tool. Wherever an attorney is in Persist, a single global hotkey opens the voice recorder. What they say is captured, transcribed, AI-corrected, and then routed as one of four output types: a Note, an Instruction, a Mail Reply, or a Task. All three artifacts of the capture — the raw audio, the raw transcript, and the AI-corrected text — are stored permanently and linked to the current context (matter, email thread, document, meeting).

Over time, Persist Voice learns each attorney's vocabulary, preferred phrasing, speaking patterns, and correction habits. It becomes a personalised dictation layer that requires progressively less correction.

This module is not a standalone module with its own page — it is infrastructure that sits on top of every surface in Persist, exactly as Module 15A (Persist Editor) does for text input.

---

### 23.2 The Trigger — Global Hotkey

**Desktop (macOS + Windows):**

```
Cmd+Shift+V (macOS)   /   Ctrl+Shift+V (Windows)
```

This hotkey is registered globally via the Tauri global shortcut plugin — it fires even when Persist is not the active window, and even when Persist is minimised to the system tray. The attorney can be in Outlook, a browser, or a PDF reader — pressing the hotkey opens the Persist Voice recorder floating over whatever is on screen.

A second trigger exists within Persist itself: a microphone icon `🎙` is present in every text input surface (email compose, notes fields, matter notes, Persist Chat input, meeting notes). Clicking it opens voice capture scoped to that input.

**The floating recorder UI:**

```
┌──────────────────────────────────────────────────────────┐
│  🎙 Persist Voice                           [×]           │
│  ─────────────────────────────────────────────────────── │
│  [Petalveda Scents — TM Opposition]  ← auto-detected ctx │
│                                                          │
│       ████░░░░░░░░░░░░░░░░░░░░░░░  Recording...         │
│       0:07                                               │
│                                                          │
│  ○ Note  ○ Instruction  ○ Mail Reply  ● Task             │
│                                                          │
│  [Stop]                                     [Cancel]     │
└──────────────────────────────────────────────────────────┘
```

The recorder is a small floating window, always on top, draggable. It shows:
- The auto-detected matter context (which matter the attorney is currently working on)
- A waveform visualiser during recording
- Elapsed time
- The output type selector (Note / Instruction / Mail Reply / Task) — defaulting to the most likely type based on context
- Stop and Cancel buttons. Escape = Cancel. Space = Stop.

---

### 23.3 The Three Artifacts — What Gets Saved

Every voice capture produces and permanently stores three distinct artifacts:

**Artifact 1 — Raw Audio Clip**

The original `.webm` audio recording, stored in the encrypted vault (`storage/vault.rs`) alongside documents. Never discarded. The attorney can play it back from any capture record. Useful for:
- Reviewing exactly what was said before the transcript was corrected
- Disputes about instructions given ("I said to file by Friday, not Monday")
- Meeting recordings where the verbatim is important

Storage format: `.webm` (Opus codec), compressed. Average 1-minute clip ≈ 300KB in the vault.

**Artifact 2 — Raw Transcript**

The verbatim speech-to-text output before any AI correction — filler words, false starts, repetitions, and all. Produced by the transcription engine immediately after recording stops. Stored as plain text.

```
raw: "umm so we need to reply to the the examination report for petalveda
and I think the main ground is uh confusion with nytarra mark so
let me make a note to draft the response uh by next tuesday maybe"
```

**Artifact 3 — AI-Corrected Text**

The clean, formatted, legally-appropriate version produced by Claude from the raw transcript plus the matter context. Filler words removed, grammar corrected, structure applied, legal terminology preserved.

```
corrected: "Draft response to examination report for Petalveda Scents
(TM App. No. 1234567). Primary ground: likelihood of confusion with
Nytarra mark. Target completion: Tuesday, 14 April 2026."
```

All three artifacts are stored together in a single `voice_captures` record in SQLite. The attorney always sees the corrected text by default but can expand to see the raw transcript or play the audio clip.

---

### 23.4 The Four Output Types

After recording stops and the corrected text is ready (typically 2–4 seconds), the attorney sees the output panel. They choose how to route the capture:

```
┌────────────────────────────────────────────────────────────────┐
│  🎙 Persist Voice — Capture Complete                           │
│  ─────────────────────────────────────────────────────────── │
│  ✦ Draft response to examination report for Petalveda Scents   │
│    (TM App. No. 1234567). Primary ground: likelihood of        │
│    confusion with Nytarra mark. Target: 14 Apr 2026.           │
│                                                                │
│  [▶ Play clip]  [Show raw transcript]  [Edit corrected text]  │
│                                                                │
│  Save as:                                                      │
│  ┌──────────┐  ┌─────────────┐  ┌──────────────┐  ┌──────┐   │
│  │  📝 Note │  │ 📋 Instruc- │  │ 📧 Mail Reply│  │ ✓ Task│  │
│  │          │  │    tion     │  │              │  │      │   │
│  └──────────┘  └─────────────┘  └──────────────┘  └──────┘   │
│                                                                │
│  Matter: [Petalveda Scents — TM Opposition ▾]                 │
│  Tags:   [+ Add tag]                                          │
│                                                                │
│  [Save]                                          [Discard]    │
└────────────────────────────────────────────────────────────────┘
```

**📝 Note** — saves to the matter's Notes tab. Appears in the matter timeline. Accessible from the Communication tab. Searchable. The corrected text becomes the note body; the audio clip and raw transcript are attached.

**📋 Instruction** — saves as a structured instruction to a specific person or team. The attorney can add an assignee. Appears in the Tasks module (Module 11.2) as an instruction item, tagged with the voice source. Example: "Sree Lakshmi — please review and file the counter-statement by 28 April."

**📧 Mail Reply** — the corrected text is inserted directly into the compose window for the email thread the attorney was viewing when they triggered the hotkey. If no email was open, it opens a new compose window with the text pre-filled. The attorney reviews and sends.

**✓ Task** — creates a new task in Module 11.2 (Smart Task Management) with the corrected text as the task title and description, linked to the current matter. Due date extracted from the text if present ("by next Tuesday" → parsed to the actual date).

The four types are not mutually exclusive — a single capture can be saved as both a Note and a Task simultaneously.

---

### 23.5 AI Correction Pipeline — How the Text Is Produced

The correction runs entirely in Keel via the AI router (Module 22). Model: **Haiku 4.5** for standard captures (fast, cheap — this fires on every capture); escalates to **Sonnet 4.6** for complex multi-sentence captures or when Haiku's confidence is low.

The correction prompt is context-aware. The matter context assembled at correction time:

```
System:
You are a legal dictation assistant for an Indian IP law firm.
The attorney is currently working on: {matter_title} ({matter_type}).
Client: {client_name}.
Relevant parties: {party_names}.
Recent context: {last_3_matter_notes}.
Attorney's correction history: {personal_corrections_summary}.
Firm vocabulary: {firm_dictionary}.

Task:
Clean the raw transcript into clear, professional legal text.
- Remove filler words (um, uh, like, you know, basically)
- Fix grammar and punctuation
- Preserve all legal terminology, party names, and case references exactly
- If a date phrase is present (next Tuesday, by Friday), resolve it to DD Month YYYY
- If a number or deadline is mentioned, preserve it precisely
- Output ONLY the corrected text — no preamble, no explanation

Raw transcript: {raw_transcript}
```

The matter context is what makes this different from generic dictation tools. Wisprflow doesn't know what "Petalveda" is, doesn't know who "Nytarra" is, doesn't know that "examination report" has a specific 30-day statutory response window. Persist Voice does — because it has the full matter record available to Keel at correction time.

**Correction speed target:** raw transcript ready in < 1 second after recording stops; AI-corrected text ready in < 3 seconds. Total latency from Stop to seeing the corrected output: under 4 seconds.

---

### 23.6 Personal Voice Profile — Learning Over Time

Every attorney in Persist has a `voice_profile` record that accumulates learning across all their captures. This is what enables the "evolves with the user" behaviour.

**What gets learned:**

**1. Personal dictionary** — every time an attorney edits the AI-corrected text after a capture, Persist Voice diffs the correction and extracts new vocabulary. If "Petalveda" was corrected to "Petalveda Scents Private Limited", that expansion is added to the dictionary. Next capture, Haiku receives it in context and produces the full form automatically.

```
voice_dictionary (per attorney):
  "petalveda"     → "Petalveda Scents Private Limited"
  "nytarra"       → "M/s Nytarra"
  "akash"         → "Akash Kumar Srivastava, Advocate"
  "tm act"        → "Trade Marks Act, 1999"
  "fer"           → "First Examination Report"
  "poa"           → "Power of Attorney"
  "ipab"          → "Intellectual Property Appellate Board"
```

These are stored per-attorney in SQLite and injected into every correction prompt.

**2. Matter-specific snippets** — short voice triggers that expand to full legal boilerplate. Defined by the attorney, or auto-suggested by Persist when the same phrase is corrected more than three times.

```
voice_snippets (per matter):
  "notice"       → "Legal Notice dated March 31, 2026 issued by Akash Kumar Srivastava"
  "our client"   → "M/s Petalveda Scents Private Limited, the Applicant"
  "the mark"     → "Trade Mark Application No. 1234567 in Class 03"
  "the deadline" → "Response to Examination Report due 12 April 2026"
```

Global firm snippets (defined by partners in Settings) apply across all attorneys.

**3. Correction pattern learning** — if an attorney consistently changes a particular correction pattern (e.g. always changes "file a response" to "file a detailed written response"), Persist Voice learns this preference and applies it automatically in future corrections.

**4. Output type preferences by context** — if an attorney's captures in the mail module are almost always routed as Mail Reply, the default output type selector pre-selects Mail Reply when the hotkey is triggered from within an email thread. Pattern observed and applied silently.

**5. Speaking style calibration** — over time, the correction prompt includes a compressed summary of the attorney's writing style: sentence length preference, use of legal Latin phrases, tendency to use formal vs informal address in different document types. This is assembled by Keel weekly from accepted corrections, not from rejected ones.

All learning data is stored locally in SQLite (the `voice_profile`, `voice_dictionary`, `voice_snippets`, `correction_patterns` tables). It is synced to the attorney's other devices (if they use Persist on multiple machines) via the sync server — but never to the client portal or any external service.

---

### 23.7 Where Voice Capture Is Available — All Surfaces

The global hotkey fires from anywhere. The `🎙` inline trigger appears in:

| Surface | Module | Inline trigger | Context auto-detected |
|---|---|---|---|
| Matter notes | 1 | ✓ | Current matter |
| Deadline notes field | 2 | ✓ | Current matter + deadline |
| Email compose body | 15 | ✓ | Matter linked to email |
| Email thread view | 15 | ✓ | Thread matter |
| Meeting notes | 11.3 | ✓ | Calendar event + matter |
| Persist Chat input | 21 | ✓ | Current matter |
| Document draft body | 9 | ✓ | Document + matter |
| Smart Form free-text fields | 9.8 | ✓ | Matter + template |
| Contract review notes | 10 | ✓ | Review project + matter |
| Task description | 11.2 | ✓ | Current matter |
| Internal comments | Any | ✓ | Current matter |

When the global hotkey is used without any Persist surface in focus, the context defaults to "no matter" and the output type defaults to Note. The attorney can manually assign a matter from the output panel before saving.

---

### 23.8 Voice Capture History

Every capture is stored permanently and accessible from two places:

**From the matter record** — the matter's Notes tab shows all voice captures linked to that matter, with the corrected text visible inline, an audio play button, and a "Show raw transcript" toggle. Captures appear in the matter timeline alongside emails, documents, and deadlines.

**From the global Voice Inbox** — accessible from the left sidebar under a `🎙 Voice` label. Shows all captures across all matters, sorted by date, filterable by output type, matter, and attorney. Useful for reviewing what was dictated in a day, or finding a specific instruction.

```
🎙 Voice Inbox — All Captures

Today
  ─────────────────────────────────────────────────────────────────
  11:32 AM  📝 Note      Petalveda TM Opposition
  ▶ 0:14   "Draft response to examination report... by 14 Apr"
  ─────────────────────────────────────────────────────────────────
  10:15 AM  ✓ Task       Zenith Brands TM Application
  ▶ 0:08   "Follow up with Zenith re: proof of use affidavit"
  ─────────────────────────────────────────────────────────────────

Yesterday
  ─────────────────────────────────────────────────────────────────
  6:22 PM   📧 Mail Reply  Rahul Industries NDA
  ▶ 0:21   "Thank you for sending the revised NDA. We have reviewed..."
  ─────────────────────────────────────────────────────────────────
```

---

### 23.9 Data Model (SQLite)

```sql
CREATE TABLE voice_captures (
    id TEXT PRIMARY KEY,
    attorney_id TEXT NOT NULL REFERENCES users(id),
    matter_id TEXT REFERENCES matters(id),      -- nullable: may be unlinked
    surface TEXT,                                -- 'matter-notes', 'email-compose', 'chat', etc.
    output_type TEXT NOT NULL
        CHECK (output_type IN ('Note','Instruction','MailReply','Task')),

    -- Three artifacts
    audio_vault_path TEXT NOT NULL,              -- path to .webm in encrypted vault
    audio_duration_seconds INTEGER NOT NULL,
    raw_transcript TEXT NOT NULL,                -- verbatim speech-to-text
    corrected_text TEXT NOT NULL,                -- AI-corrected output

    -- Routing
    linked_entity_id TEXT,   -- note_id / task_id / email_thread_id after save
    linked_entity_type TEXT, -- 'Note' / 'Task' / 'MailReply' / 'Instruction'

    -- Metadata
    tags TEXT DEFAULT '[]',
    is_discarded INTEGER NOT NULL DEFAULT 0,     -- soft-delete
    correction_applied INTEGER NOT NULL DEFAULT 0, -- 1 if attorney edited after AI correction
    attorney_edit TEXT,                           -- the attorney's manual edits (for learning)

    created_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_voice_matter ON voice_captures(matter_id);
CREATE INDEX idx_voice_attorney ON voice_captures(attorney_id, created_at DESC);

CREATE TABLE voice_profiles (
    attorney_id TEXT PRIMARY KEY REFERENCES users(id),
    preferred_output_type TEXT DEFAULT 'Note', -- most used output type
    style_summary TEXT,                         -- AI-assembled writing style summary (updated weekly)
    total_captures INTEGER NOT NULL DEFAULT 0,
    total_duration_seconds INTEGER NOT NULL DEFAULT 0,
    updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE voice_dictionary (
    id TEXT PRIMARY KEY,
    attorney_id TEXT NOT NULL REFERENCES users(id),
    trigger_word TEXT NOT NULL,         -- what the attorney says
    expansion TEXT NOT NULL,            -- what it expands to
    scope TEXT NOT NULL DEFAULT 'global'
        CHECK (scope IN ('global', 'matter')),
    matter_id TEXT REFERENCES matters(id),  -- if scope = 'matter'
    use_count INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
CREATE UNIQUE INDEX idx_voice_dict ON voice_dictionary(attorney_id, trigger_word, matter_id);

CREATE TABLE voice_snippets (
    id TEXT PRIMARY KEY,
    attorney_id TEXT REFERENCES users(id),   -- null = firm-wide snippet
    trigger_phrase TEXT NOT NULL,             -- voice cue
    expansion_text TEXT NOT NULL,             -- full text output
    scope TEXT NOT NULL DEFAULT 'global'
        CHECK (scope IN ('global', 'matter', 'firm')),
    matter_id TEXT REFERENCES matters(id),
    use_count INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE correction_patterns (
    id TEXT PRIMARY KEY,
    attorney_id TEXT NOT NULL REFERENCES users(id),
    original_phrase TEXT NOT NULL,    -- what AI produced
    corrected_phrase TEXT NOT NULL,   -- what attorney changed it to
    frequency INTEGER NOT NULL DEFAULT 1,
    last_seen DATETIME NOT NULL DEFAULT (datetime('now'))
);
```

---

### 23.10 Keel Implementation

```
src-tauri/
├── commands/
│   └── voice.rs          ← Tauri commands: start_recording, stop_recording,
│                            get_correction, save_capture, get_voice_history,
│                            update_dictionary, get_snippets
├── services/
│   ├── voice_recorder.rs  ← OS microphone access via Tauri audio plugin
│   ├── transcriber.rs     ← Speech-to-text (Whisper model, local or API)
│   ├── voice_corrector.rs ← AI correction pipeline (Haiku → Sonnet escalation)
│   └── voice_learner.rs   ← Background learning: diff corrections, update profile
├── storage/
│   └── vault.rs           ← audio clip stored here alongside documents (existing)
```

**Transcription engine decision:** Two options with a clear recommendation:

Option A — **Local Whisper (whisper.cpp):** runs entirely on-device, no network call, zero latency cost, complete privacy. Bundled as a sidecar in the Keel build (adds ~150MB to installer for the medium model, which gives good accuracy for legal English and Indian accents). Recommended for Phase 1 of this feature.

Option B — **OpenAI Whisper API:** faster for long recordings, no installer size cost, but requires internet and sends audio to OpenAI servers. Not appropriate for a law firm platform without explicit data processing agreements.

**Recommendation: Option A (local Whisper).** Audio clips contain client-privileged communications. They must not leave the machine. The installer size increase is acceptable — TeX Live already adds ~80MB.

The transcription runs in `tokio::task::spawn_blocking` — a CPU-intensive operation that must not block the main thread.

---

### 23.11 Deck Implementation

```
src/
├── components/
│   └── voice/
│       ├── VoiceRecorder.tsx       ← floating recorder window (global hotkey listener)
│       ├── VoiceOutputPanel.tsx    ← three artifacts + output type selector
│       ├── VoiceMicButton.tsx      ← inline 🎙 trigger for text inputs
│       ├── VoiceInbox.tsx          ← full capture history view
│       └── VoiceDictionaryEditor.tsx ← manage personal dictionary + snippets
```

The `VoiceRecorder` component is mounted once at the app root level — it listens for the global hotkey event from Tauri and renders the floating window anywhere. It is not re-mounted per page.

The `VoiceMicButton` is a small icon component dropped into every text input where voice capture is relevant. Clicking it calls `invoke('start_recording')` with the current surface and matter context pre-filled.

---

### 23.12 AI Model Routing for Voice

| Operation | Model | Reason |
|---|---|---|
| AI text correction (standard) | Haiku 4.5 | High-volume, fires on every capture — must be fast and cheap |
| AI text correction (complex, long) | Sonnet 4.6 | Escalation when Haiku confidence < threshold |
| Style summary generation (weekly) | Sonnet 4.6 | Runs once a week per attorney in background |
| Snippet suggestion (from patterns) | Haiku 4.5 | Pattern matching — lightweight |
| Transcription | Local Whisper | Never goes through Claude API |

---

### 23.13 Phase Assignment

**Phase 3 (Weeks 19–26) — Core voice capture:**
- Global hotkey, floating recorder, three-artifact storage
- Haiku-based correction with matter context
- Four output types (Note, Instruction, Mail Reply, Task)
- Personal dictionary — manual entry
- Voice Inbox

**Phase 4 (Weeks 27–38) — Learning layer:**
- Automatic dictionary learning from correction diffs
- Matter-scoped snippets
- Correction pattern tracking
- Output type preference learning by context
- `voice_learner.rs` background service

**Phase 6 (Weeks 55–62) — Style evolution:**
- Weekly style summary generation via Sonnet
- Style summary injected into correction prompts
- Per-surface tone adaptation (formal in documents, direct in tasks)
- Attorney-facing profile view: "Here is what Persist Voice has learned about you"

---

*Note: Module 23 added v2.6 — April 2026. Persist Voice is the native voice layer for Persist Desktop — modelled on Wisprflow's dictation philosophy but extended with three-artifact storage, matter context awareness, legal vocabulary learning, and four output type routing.*

---

## 24. HPAS — Hierarchical Parallel Agent System (Multi-Agent Orchestration Engine)

### 24.1 Overview

HPAS is the execution engine that powers all intelligent operations in Persist. It replaces direct model calls with a structured three-level agent hierarchy — GrandParents (L0), Parents (L1), and Workers (L2) — that operate in parallel, maintain persistent context via delta activation, communicate laterally via a typed message bus, and apply layered semantic compression to achieve token consumption proportional to information novelty rather than total work volume.

Every AI call in Persist (Section 22) flows through HPAS once it is introduced in Phase 3A. The existing `ai_router.rs` three-tier routing (Haiku/Sonnet/Opus) continues to govern model selection — HPAS sits above the router as the orchestration layer that decides what work needs to be done, decomposes it into parallel tasks, and reassembles results. The router decides which model runs each task.

**Core efficiency claim:** At N=200 parallel tasks, HPAS consumes ~127x fewer tokens than single-context approaches and ~7.5x fewer than naive parallel approaches. For Persist's typical workloads (contract review, due diligence, multi-document analysis), this translates to 10-35x cheaper change propagation versus full re-runs.

**Full specification:** `HPAS_PRD_Standalone.md` (platform-agnostic, 9 components, full token economics model).
**Persist binding:** `HPAS_Persists_Integration.md` (Rust implementation, Tauri integration, macOS constraints, implementation sequence).

---

### 24.2 Why HPAS Exists in Persist

Three Persist features expose the limitations of direct model calls:

**Contract Intelligence (Module 10):** A due diligence project with 50 contracts requires extracting clauses from each, cross-referencing terms, scoring risk, and generating a unified report. Sequential processing is O(n^2) in token cost because each step's context grows with all prior results. HPAS decomposes this into 50 parallel Worker agents (L2) for extraction, a Parent (L1) for cross-referencing within contract types, and a GrandParent (L0) for the final synthesis — each agent sees only its scoped slice.

**Daily Brief (Module 11.1):** Assembling a morning brief pulls from matters, deadlines, emails, tasks, billing, and calendar — six domains. Without HPAS, the context window fills with raw data from all six. With HPAS, six Parents each handle one domain in parallel, compress their outputs to ~500 tokens each, and the GrandParent synthesises from ~3,000 tokens of structured facts rather than ~60,000 tokens of raw data.

**Persist Chat multi-step queries (Module 21):** "Compare the indemnity clauses across all NDAs for this client and flag any that deviate from our standard" requires document retrieval, clause extraction per document, cross-document comparison, and anomaly flagging — four distinct stages with dependencies. HPAS's Parent-Worker hierarchy handles this as a structured workflow, not a monolithic prompt.

---

### 24.3 Architecture Summary

```
┌────────────────────────────────────────────────────────┐
│                 GRANDPARENT (L0)                        │
│  Project-level orchestration. Full job context.         │
│  Dispatches domain slices to Parents.                   │
│  Communicates with other GPs via OrchestratorBus.       │
│  ~20K tokens initial, ~2K per delta activation.         │
├────────────────────────────────────────────────────────┤
│                  PARENT (L1)                             │
│  Domain-level coordination. Dispatches Workers.         │
│  Integrates compressed outputs. Lateral comms via       │
│  MessageBus. Writes to RegistryAgent.                   │
│  ~15K tokens initial, ~1.5K per delta activation.       │
├────────────────────────────────────────────────────────┤
│                  WORKER (L2)                             │
│  Single scoped task. No hierarchy awareness.            │
│  Receives context slice, produces structured output.    │
│  ~4K tokens per activation. Fully parallel.             │
└────────────────────────────────────────────────────────┘
```

**Nine components:** SessionAgent, SemanticCompressor, MessageBus, OrchestratorBus, VectorMemory, RegistryAgent, SubAgent, ContextCompressor, ApprovalGate. All specified in `HPAS_PRD_Standalone.md` Section 6.

---

### 24.4 Persist-Specific Implementation

HPAS is implemented as a native Rust module at `src-tauri/src/hpas/`. It compiles directly into the Tauri binary — no separate process, no Docker, no external runtime.

**Module layout:**
```
src-tauri/src/hpas/
├── mod.rs               — dispatch router, Job/StructuredResult types
├── agent.rs             — SubAgent (tokio::spawn per worker)
├── compressor.rs        — SemanticCompressor (pure function)
├── store.rs             — WorkflowStore trait + SqliteStore impl
├── session.rs           — SessionAgent with SQLite history
├── memory.rs            — VectorMemory with sqlite-vss
├── registry.rs          — RegistryAgent (SQLite table)
├── bus.rs               — MessageBus + OrchestratorBus via Tauri events
└── compressor_ctx.rs    — ContextCompressor (periodic distillation)
```

**Key bindings:**
- `WorkflowStore` backed by SQLite (local) — swappable to Restate for cloud scale
- `MessageBus` and `OrchestratorBus` via Tauri event system (~0.01ms latency)
- `VectorMemory` via sqlite-vss extension bundled in `Contents/Resources/`
- Batch API SubAgent dispatch via Anthropic Batch API + Tauri local HTTP webhook
- React calls HPAS exclusively through `useHpas.ts` hook → `invoke("hpas_dispatch", ...)`

**Bypass rule:** Intelligence (interpretation, assembly, compression, multi-step coordination) → HPAS. Pure data transport (cursor moves, file opens, text edits, real-time sync) → direct Tauri IPC. Same rule as Section 22.

---

### 24.5 Relationship to Section 22 AI Router

HPAS and the AI router (Section 22) are complementary, not competing:

```
React UI
  ↓
useHpas.ts → invoke("hpas_dispatch", job)
  ↓
HPAS dispatch (mod.rs) — decomposes job into agent graph
  ↓
Each agent (SessionAgent/SubAgent) makes AI calls via:
  ↓
ai_router.rs — selects Haiku/Sonnet/Opus per task type
  ↓
claude_client.rs — Anthropic API call
```

The router's task-type classification (Section 22.4), context-size gating, Sonnet confidence escalation, and attorney Deep Analysis override all continue to operate. HPAS adds the orchestration layer above: deciding what work to decompose, how to parallelise it, and how to compress and reassemble results.

---

### 24.6 Phase Assignment

HPAS is introduced incrementally, exactly when Persist features first need each capability:

**Phase 3A — Dispatch Foundation (alongside first Phase 3 AI feature):**
- `Job`, `StructuredResult`, `Priority`, `JobStatus` types in `hpas/mod.rs`
- `WorkflowStore` trait + `SqliteStore` in `store.rs`
- `hpas_dispatch` Tauri command
- `useHpas.ts` React hook
- `SubAgent` in `agent.rs` — single tokio task, API call, typed result
- `SemanticCompressor` in `compressor.rs`
- First real feature wired end-to-end through HPAS

**Phase 3B — Parallel Dispatch (when first multi-fetch feature needed):**
- `FuturesUnordered` parallel dispatch in `agent.rs`
- `SessionAgent` in `session.rs` — SQLite-persisted conversation history
- `ContextCompressor` in `compressor_ctx.rs`
- Verify parallel wall-time <= slowest agent, not sum

**Phase 3C — Communication Layer (when cross-domain coordination needed):**
- `RegistryAgent` in `registry.rs`
- `MessageBus` + `OrchestratorBus` in `bus.rs` via Tauri events
- `VectorMemory` in `memory.rs` via sqlite-vss
- Lateral communication tests

**Phase 4 — Full Hierarchy + ApprovalGate:**
- `ApprovalGate` — Tauri notification + SQLite pending state
- Full GrandParent → Parent → Agent graph for real jobs
- Batch API dispatch (`batchMode: true`) + webhook handler
- Optional `AgentGraph.tsx` real-time visualisation

**Phase 5 — Cloud Scale (when multi-user/distributed):**
- `RestateStore` behind existing `WorkflowStore` trait
- Feature-flag: `HPAS_BACKEND=restate` at compile time
- All tests pass with Restate backend

---

### 24.7 Success Metrics

| Metric | Target |
|---|---|
| Token cost vs single-context at N=50 | >= 8x reduction |
| Token cost vs single-context at N=200 | >= 20x reduction |
| Parent delta activation cost | <= 2,000 tokens |
| Agent execution cost | <= 4,500 tokens avg |
| SemanticCompressor ratio | >= 8:1 avg |
| Change propagation vs full re-run | >= 10x cheaper |
| Cold launch time increase | <= 200ms |
| Interactive dispatch latency (P95) | <= 200ms |
| MessageBus event delivery | <= 5ms |
| Registry consistency under concurrent writes | 100% |
| Schema conformance at all boundaries | 100% |
| .app bundle size increase | <= 5MB |

---

### 24.8 Cargo Dependencies Added by HPAS

```toml
tiktoken-rs  = "0.5"     # token counting for context ceiling management
```

All other dependencies (`tokio`, `rusqlite`, `serde`, `serde_json`, `reqwest`, `uuid`) are already present in Persist's Cargo.toml. sqlite-vss is bundled as a loadable extension in the app Resources folder, not a Cargo dependency.

---

*Note: Section 24 added v2.9 — April 2026. HPAS is the multi-agent orchestration engine for Persist. Full platform-agnostic specification in HPAS_PRD_Standalone.md; Persist-specific binding in HPAS_Persists_Integration.md.*

---

*This document is confidential and intended for internal use at Persistas & Partners only.*
