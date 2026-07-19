# specs/module-02-docketing.md
# SPEC for Module 2: Docketing & Deadline Engine
# Written before implementation. This is the contract Claude implements against.

---

## Overview

The Docketing Engine is the highest-criticality module in Persist. In Indian IP prosecution,
a missed statutory deadline results in immediate and irreversible abandonment of the application
— no cure, no revival, significant professional liability. This module is the firm's first line
of legal defence, not a calendar feature.

Refer to PRD Sections 2.1–2.18 for full design rationale. This spec covers the data model,
Keel command signatures, business rules, and edge cases needed for implementation.

---

## Data Models

### `ip_assets` table
```sql
CREATE TABLE ip_assets (
    id TEXT PRIMARY KEY,                      -- UUID
    matter_id TEXT NOT NULL REFERENCES matters(id) ON DELETE CASCADE,
    asset_type TEXT NOT NULL
        CHECK (asset_type IN ('Trademark','Patent','Design','Copyright','PlantVariety')),
    title TEXT NOT NULL,                      -- mark name or invention title
    application_number TEXT,                  -- official registry number
    registration_number TEXT,                 -- once registered/granted
    filing_date DATE,
    priority_date DATE,                       -- Paris Convention / PCT priority
    grant_date DATE,
    registration_date DATE,
    expiry_date DATE,                         -- calculated, auto-updated
    applicant_entity_type TEXT NOT NULL DEFAULT 'Company'
        CHECK (applicant_entity_type IN ('Individual','Startup','SmallEntity','Company','Government')),
    jurisdiction TEXT NOT NULL DEFAULT 'India',
    classes TEXT DEFAULT '[]',               -- JSON array of Nice/Locarno class numbers
    status TEXT NOT NULL DEFAULT 'Pending'
        CHECK (status IN ('Pending','Examination','Accepted','Advertised','Opposed',
                          'Registered','Granted','Lapsed','Abandoned','Cancelled')),
    notes TEXT,
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_ip_assets_matter ON ip_assets(matter_id);
CREATE INDEX idx_ip_assets_type ON ip_assets(asset_type, status);
```

### `deadlines` table
```sql
CREATE TABLE deadlines (
    id TEXT PRIMARY KEY,
    matter_id TEXT NOT NULL REFERENCES matters(id) ON DELETE CASCADE,
    ip_asset_id TEXT REFERENCES ip_assets(id),
    docketing_event TEXT NOT NULL,            -- human-readable: "Response to Examination Report"
    reference_number TEXT,                    -- P&P-DD-NNNN format
    deadline_type TEXT NOT NULL
        CHECK (deadline_type IN ('Statutory','Internal','Court','Renewal','Annuity','Custom')),
    ip_type TEXT
        CHECK (ip_type IN ('Trademark','Patent','Design','Copyright','PCT','Madrid','General')),
    due_date DATE NOT NULL,
    responsible_user_id TEXT REFERENCES users(id),
    status TEXT NOT NULL DEFAULT 'Pending'
        CHECK (status IN ('Pending','InProgress','Completed','Overdue','Waived','Missed')),
    priority TEXT NOT NULL DEFAULT 'Normal'
        CHECK (priority IN ('Normal','High','Critical')),
    urgency TEXT NOT NULL DEFAULT 'Normal'    -- calculated, not stored permanently
        CHECK (urgency IN ('Normal','Warning','Critical','Overdue')),
    country TEXT NOT NULL DEFAULT 'India',
    completion_date DATE,
    completion_notes TEXT,
    evidence_doc_id TEXT REFERENCES documents(id),
    -- Triggering event documentation (Module 2.10)
    triggering_document_id TEXT REFERENCES documents(id),
    triggering_event_date DATE,
    triggering_event_type TEXT,               -- 'ExaminationReport'|'NOA'|'RegistrationCert'|...
    -- Dual verification (Module 2.12)
    is_verified INTEGER NOT NULL DEFAULT 0,
    verified_by TEXT REFERENCES users(id),
    verified_at DATETIME,
    -- Cascade linkage (Module 2.9)
    cascade_root_id TEXT REFERENCES deadlines(id),  -- the anchor deadline that spawned this one
    cascade_template_id TEXT REFERENCES cascade_templates(id),
    -- Client instruction tracking
    client_instruction_received INTEGER NOT NULL DEFAULT 0,
    client_instruction_notes TEXT,
    created_by TEXT REFERENCES users(id),
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_deadlines_matter ON deadlines(matter_id, due_date ASC);
CREATE INDEX idx_deadlines_due ON deadlines(due_date, status);
CREATE INDEX idx_deadlines_unverified ON deadlines(is_verified, deadline_type);
```

### `cascade_templates` table
```sql
CREATE TABLE cascade_templates (
    id TEXT PRIMARY KEY,
    anchor_event_type TEXT NOT NULL,          -- 'TMApplication'|'PCTFiling'|'PatentGrant'|...
    ip_type TEXT NOT NULL,
    jurisdiction TEXT NOT NULL DEFAULT 'India',
    template_json TEXT NOT NULL,              -- JSON array of DeadlineRule objects
    last_verified DATE NOT NULL,              -- when statutory rules last confirmed
    notes TEXT,
    created_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
```

Template JSON structure (each rule in the array):
```json
{
  "event_name": "Response to Examination Report",
  "deadline_type": "Statutory",
  "offset_days": 30,
  "offset_from": "triggering_date",
  "internal_buffer_days": 7,
  "requires_client_instruction": false,
  "requires_verification": true,
  "reminder_days": [30, 14, 7, 3, 1]
}
```

### `deadline_escalations` table (Module 2.11)
```sql
CREATE TABLE deadline_escalations (
    id TEXT PRIMARY KEY,
    deadline_id TEXT NOT NULL REFERENCES deadlines(id) ON DELETE CASCADE,
    escalation_level INTEGER NOT NULL,        -- 1=14day, 2=7day, 3=3day, 4=missed
    triggered_at DATETIME NOT NULL,
    notified_user_ids TEXT NOT NULL,          -- JSON array of user IDs
    notification_channels TEXT NOT NULL,      -- JSON: ["in_app","whatsapp"]
    resolution_action TEXT,
    resolved_at DATETIME,
    resolved_by TEXT REFERENCES users(id)
);
```

### `document_intake_events` table (Module 2.16)
```sql
CREATE TABLE document_intake_events (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id),
    matter_id TEXT REFERENCES matters(id),
    received_at DATETIME NOT NULL,
    document_type TEXT,
    sla_target_hours INTEGER NOT NULL DEFAULT 24,
    docketed_at DATETIME,
    docketed_by TEXT REFERENCES users(id),
    sla_met INTEGER,
    reviewed_no_action INTEGER NOT NULL DEFAULT 0,
    reviewed_by TEXT REFERENCES users(id),
    reviewed_at DATETIME
);
```

### `docket_errors` table (Module 2.17)
```sql
CREATE TABLE docket_errors (
    id TEXT PRIMARY KEY,
    deadline_id TEXT NOT NULL REFERENCES deadlines(id),
    matter_id TEXT NOT NULL REFERENCES matters(id),
    error_type TEXT NOT NULL
        CHECK (error_type IN ('NearMiss','Missed','IntakeDelay')),
    triggered_at DATETIME NOT NULL,
    root_cause TEXT
        CHECK (root_cause IN ('DelayedIntake','WrongDateCalculated','ClientInstructionDelayed',
                              'AssociateFailure','SystemGap','StaffingIssue',
                              'IntentionalWaiver','Other')),
    root_cause_notes TEXT,
    remediation_action TEXT,
    reviewed_by TEXT REFERENCES users(id),
    reviewed_at DATETIME,
    is_resolved INTEGER NOT NULL DEFAULT 0
);
```

### `ip_fee_schedule` table (Module 2.18)
```sql
CREATE TABLE ip_fee_schedule (
    id TEXT PRIMARY KEY,
    ip_type TEXT NOT NULL,                    -- 'Patent'|'Trademark'|'Design'
    fee_event TEXT NOT NULL,                  -- 'Annuity_Year2'|'TMRenewal_PerClass'|...
    entity_type TEXT NOT NULL,                -- 'Individual'|'Startup'|'SmallEntity'|'Company'
    base_fee_inr INTEGER NOT NULL,
    surcharge_per_month_inr INTEGER DEFAULT 0,
    grace_period_months INTEGER DEFAULT 0,
    effective_from DATE NOT NULL,
    last_verified DATE NOT NULL,
    notes TEXT
);
```

### `foreign_associates` table (Module 2.14)
```sql
CREATE TABLE foreign_associates (
    id TEXT PRIMARY KEY,
    firm_name TEXT NOT NULL,
    country_code TEXT NOT NULL,               -- ISO alpha-2
    practice_areas TEXT NOT NULL,             -- JSON: ['Patent','Trademark','Design']
    contact_name TEXT,
    contact_email TEXT,
    contact_phone TEXT,
    standard_instruction_template TEXT,
    notes TEXT,
    is_preferred INTEGER NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
```

### `pct_national_phase_rules` table (Module 2.13)
```sql
CREATE TABLE pct_national_phase_rules (
    country_code TEXT PRIMARY KEY,
    country_name TEXT NOT NULL,
    deadline_months INTEGER NOT NULL DEFAULT 30,
    deadline_reference TEXT NOT NULL DEFAULT 'priority_date',
    requires_translation INTEGER NOT NULL DEFAULT 0,
    translation_languages TEXT,               -- JSON array
    formality_requirements TEXT,              -- JSON
    additional_notes TEXT,
    last_verified DATE NOT NULL
);
```

---

## Keel Commands (`commands/deadlines.rs`)

```rust
// All return Result<T, String> for Tauri IPC

// Core CRUD
get_deadline(id: String) -> Deadline
list_deadlines(filter: DeadlineFilter) -> Vec<DeadlineSummary>
create_deadline(input: CreateDeadlineInput) -> Deadline
update_deadline(id: String, input: UpdateDeadlineInput) -> Deadline
mark_complete(id: String, notes: String, evidence_doc_id: Option<String>) -> Deadline
waive_deadline(id: String, reason: String, client_instruction: String) -> Deadline

// Cascade generator (Module 2.9)
generate_cascade(anchor: CascadeAnchor) -> Vec<Deadline>
// anchor: { matter_id, ip_asset_id, event_type, anchor_date, target_countries: Option<Vec<String>> }

// Verification (Module 2.12)
verify_deadline(id: String) -> Deadline  // Keel rejects if verifier == creator
dispute_deadline(id: String, dispute_notes: String) -> Deadline

// Intake SLA (Module 2.16)
record_document_intake(document_id: String, matter_id: String, document_type: String) -> IntakeEvent
mark_intake_reviewed_no_action(event_id: String) -> IntakeEvent
get_pending_intakes(matter_id: Option<String>) -> Vec<IntakeEvent>

// Error log (Module 2.17)
get_docket_errors(filter: DocketErrorFilter) -> Vec<DocketError>
resolve_docket_error(id: String, root_cause: String, notes: String, remediation: String) -> DocketError

// Fee schedule (Module 2.18)
get_fee_estimate(ip_type: String, fee_event: String, entity_type: String) -> FeeEstimate
get_fee_forecast(matter_ids: Vec<String>, months_ahead: u32) -> Vec<FeeScheduleItem>

// IP Assets
get_ip_asset(id: String) -> IpAsset
list_ip_assets(matter_id: String) -> Vec<IpAsset>
create_ip_asset(input: CreateIpAssetInput) -> IpAsset
update_ip_asset(id: String, input: UpdateIpAssetInput) -> IpAsset
```

---

## Services (`services/`)

### `cascade_engine.rs`
Reads a `CascadeAnchor` input, looks up the matching `cascade_templates` record, generates
a `Vec<CreateDeadlineInput>` for the entire downstream chain, and batch-inserts them.

All cascade templates stored in the DB as JSON — not hardcoded in Rust. Updating a template
(e.g. when India Patent Act fee schedule changes) is a DB update, not a code change.

**Phase 1 cascade templates to seed:**
- `TMApplication_India` — TM examination report window, opposition period, registration, first renewal
- `PatentApplication_India` — FER deadline (12 months), hearing, grant, annuities from year 2
- `DesignApplication_India` — registration, first renewal (5yr), max term (15yr)
- `CopyrightRegistration_India` — minimal (no prosecution chain)

**Phase 3 cascade templates to add:**
- `PCTFiling` — all international phase + national phase per-country
- `MadridIRRegistration` — WIPO renewal + per-country refusal periods
- `ParisConvention_TM` — priority claim deadline (6 months from foreign filing)

### `abandonment_watcher.rs`
Background Tokio task. Runs every 30 minutes. For each statutory deadline in `Pending` or
`InProgress` status:
- If `due_date - today == 14` and no attorney action logged → create escalation L1
- If `due_date - today == 7` → create escalation L2 + WhatsApp to responsible partner
- If `due_date - today == 3` → create escalation L3 + mandatory check-in task
- If `due_date < today` and status != 'Completed' && != 'Waived' → update status to 'Missed',
  create escalation L4, create `docket_errors` record with type 'Missed'

"Attorney action logged" = any update to `deadlines.updated_at` or `completion_notes` within
the last 48 hours by any user assigned to the matter.

### `deadline_watcher.rs`
Background Tokio task. Runs every 15 minutes. Recalculates urgency for all active deadlines:
- `due_date < today` → `Overdue`, `status = Missed` (if not Completed/Waived)
- `due_date <= today + 3 days` → `Critical`
- `due_date <= today + 7 days` → `Warning`
- else → `Normal`
Fires OS notifications via Tauri notification plugin for Critical/Overdue changes.

### `intake_sla_tracker.rs`
Called by `commands/documents.rs` when a document is uploaded or arrives via mail sync.
Creates a `document_intake_events` record. The background task checks every 15 minutes for
events where `docketed_at IS NULL AND received_at < (now - sla_target_hours)` and creates
an alert visible in the docket list and daily brief.

---

## TypeScript Types (`src/lib/ipc-types.ts` additions)

```typescript
export interface Deadline {
    id: string;
    matterId: string;
    ipAssetId: string | null;
    docketingEvent: string;
    referenceNumber: string | null;
    deadlineType: 'Statutory' | 'Internal' | 'Court' | 'Renewal' | 'Annuity' | 'Custom';
    ipType: 'Trademark' | 'Patent' | 'Design' | 'Copyright' | 'PCT' | 'Madrid' | 'General' | null;
    dueDate: string;           // ISO date
    responsibleUserId: string | null;
    status: 'Pending' | 'InProgress' | 'Completed' | 'Overdue' | 'Waived' | 'Missed';
    priority: 'Normal' | 'High' | 'Critical';
    urgency: 'Normal' | 'Warning' | 'Critical' | 'Overdue';
    country: string;
    completionDate: string | null;
    completionNotes: string | null;
    // Triggering event
    triggeringDocumentId: string | null;
    triggeringEventDate: string | null;
    triggeringEventType: string | null;
    // Verification
    isVerified: boolean;
    verifiedBy: string | null;
    verifiedAt: string | null;
    // Client instruction
    clientInstructionReceived: boolean;
    // Cascade
    cascadeRootId: string | null;
    createdAt: string;
    updatedAt: string;
}

export interface IpAsset {
    id: string;
    matterId: string;
    assetType: 'Trademark' | 'Patent' | 'Design' | 'Copyright' | 'PlantVariety';
    title: string;
    applicationNumber: string | null;
    registrationNumber: string | null;
    filingDate: string | null;
    priorityDate: string | null;
    grantDate: string | null;
    registrationDate: string | null;
    expiryDate: string | null;
    applicantEntityType: 'Individual' | 'Startup' | 'SmallEntity' | 'Company' | 'Government';
    jurisdiction: string;
    classes: number[];
    status: string;
}

export interface CascadeAnchor {
    matterId: string;
    ipAssetId: string;
    eventType: string;
    anchorDate: string;
    targetCountries?: string[];   // ISO alpha-2 codes, for PCT/Madrid
}

export interface FeeEstimate {
    ipType: string;
    feeEvent: string;
    entityType: string;
    baseFeeInr: number;
    surchargePerMonthInr: number;
    gracePeriodMonths: number;
    lastVerified: string;
}
```

---

## Urgency Calculation Rules

Calculated in `deadline_watcher.rs` and on every `list_deadlines` call. Not stored permanently
— recalculated on read to ensure freshness.

```
Urgency::Overdue   → due_date <  today
Urgency::Critical  → due_date <= today + 3 days
Urgency::Warning   → due_date <= today + 7 days
Urgency::Normal    → due_date >  today + 7 days
```

Status vs Urgency are independent:
- A deadline can be `status=InProgress, urgency=Critical` (being worked on but running out of time)
- A deadline can be `status=Pending, urgency=Normal` (not started, plenty of time)
- Urgency is visual signal only; Status is the business state

---

## Indian IP Statutory Deadlines (Phase 1 seed data)

These go into `cascade_templates` table at DB seeding time.

### TM Application (Trade Marks Act 1999)
| Event | Rule |
|---|---|
| Response to Examination Report | 30 days from date of notice (Rule 45) |
| Response to Show Cause Notice | 30 days from date of notice |
| Filing Evidence in Opposition | 2 months from service of counter-statement |
| Counter-statement to Opposition | 2 months from service of notice of opposition |
| Opposition period (after advertisement) | 4 months from date of advertisement |
| First renewal | Due before 10-year expiry from filing date |
| Grace period for renewal (with surcharge) | 6 months after expiry |

### Patent Application (Patents Act 1970)
| Event | Rule |
|---|---|
| Response to First Examination Report (FER) | 12 months from date of FER issuance |
| Filing of Request for Examination (RFE) | 48 months from priority date (Rule 24B) |
| National Phase Entry — India (PCT) | 31 months from international filing/priority date |
| Annual annuity fees | From 2nd anniversary of filing date each year |
| Grace period for annuity (with surcharge) | 6 months from due date |

### Design Application (Designs Act 2000)
| Event | Rule |
|---|---|
| First renewal | Due at year 10 (Form 6) |
| Second renewal | Due at year 15 (Form 6) |
| Maximum term | 15 years from registration |

---

## Indian Patent Annuity Fee Schedule (to seed `ip_fee_schedule`)

Source: Patents Act 1970, Schedule I (current as of April 2026 — verify on build)

| Year | Individual (₹) | Startup/SmallEntity (₹) | Company (₹) |
|---|---|---|---|
| 2 | 800 | 2,000 | 4,000 |
| 3 | 800 | 2,500 | 5,000 |
| 4 | 1,200 | 3,000 | 6,000 |
| 5 | 1,600 | 4,000 | 8,000 |
| 6 | 2,000 | 5,000 | 10,000 |
| 7 | 2,400 | 6,000 | 12,000 |
| 8 | 2,800 | 7,000 | 14,000 |
| 9 | 3,200 | 8,000 | 16,000 |
| 10 | 4,000 | 10,000 | 20,000 |
| 11–20 | 4,800/yr | 12,000/yr | 24,000/yr |
| Surcharge (per month late) | 10% of annuity | 10% | 10% |

---

## Business Rules

### Deadline reference number format
`P&P-DD-NNNN` — sequential per firm (not per matter), padded to 4 digits minimum.
Generated by Keel in `commands/deadlines.rs`. Never by Deck. Sequence tracked in `sequences` table.

### Dual verification enforcement (Module 2.12)
Keel enforces at the command level: if `verify_deadline` is called by the same user who
`created_by` the deadline, return `Err("Cannot verify your own deadline")`. Never bypass this.

### Cascade recalculation
When an IP asset's `filing_date`, `priority_date`, or `grant_date` is updated, Keel queries
all cascade-linked deadlines (`cascade_root_id IS NOT NULL` or is the root itself) and prompts
recalculation. Does NOT auto-recalculate silently — presents a confirmation to the attorney
showing old dates vs new dates before committing.

### Client instruction requirement
For `deadline_type = 'Renewal'` or `'Annuity'`: the deadline cannot move from `Pending` to
`InProgress` unless `client_instruction_received = 1`. Keel enforces at the status transition
command. Deck shows a modal: "Confirm client instruction received before proceeding."

### What requires verification (Module 2.12)
Only `deadline_type = 'Statutory'` or `'Court'` requires `is_verified = 1` to become active.
`Internal`, `Renewal`, `Annuity`, `Custom` deadlines do NOT require verification.

---

## Edge Cases

- `due_date` on a weekend or Indian public holiday: statutory deadlines do NOT automatically
  shift — the statutory date is what it is. Internal warnings fire earlier but the deadline
  is recorded as-is. Never auto-adjust statutory dates.
- Multiple IP assets per matter: the docket list shows all deadlines regardless of IP asset,
  filterable by asset. The pipeline board shows per-asset prosecution stages.
- Matter closed with open deadlines: closing a matter does NOT delete or waive deadlines.
  The attorney must explicitly waive or complete each open deadline before the matter can be
  moved to `Closed` status. Keel enforces this check in `close_matter`.
- Cascade generated for a matter with no IP asset yet: not allowed. `generate_cascade` requires
  a valid `ip_asset_id`. If no asset exists, the attorney must create the IP asset first.
