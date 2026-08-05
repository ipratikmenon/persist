// Phase 1 Module 2 — Docketing & Deadline Engine
// All Tauri IPC commands for deadlines and statutory templates.
// See PROGRESS.md Phase 1 Module 2 for the full specification.

use crate::AppState;
use crate::db::queries;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// IPC types — must stay in sync with src/lib/ipc-types.ts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deadline {
    pub id:              String,
    pub matter_id:       String,
    /// Set when the deadline belongs to a specific IP asset (B02); None for
    /// matter-level deadlines such as client meetings or internal reviews.
    pub ip_asset_id:     Option<String>,
    pub docketing_event: String,
    pub event_type:      String,  // Statutory | Procedural | Custom
    pub due_date:        String,
    pub status:          String,  // Pending | Complete | Waived
    pub urgency:         String,  // Overdue | Critical | Warning | Normal
    pub notes:           Option<String>,
    pub completed_at:    Option<String>,
    pub completed_by:    Option<String>,
    pub created_at:      String,
    pub updated_at:      String,
}

/// Denormalised view for the DocketList — includes matter + client info.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeadlineSummary {
    pub id:              String,
    pub matter_id:       String,
    pub matter_title:    String,
    pub matter_type:     String,
    pub client_name:     String,
    pub docketing_event: String,
    pub event_type:      String,
    pub due_date:        String,
    pub status:          String,
    pub urgency:         String,
    pub notes:           Option<String>,
    pub updated_at:      String,
}

/// A statutory/procedural template that Deck can use to pre-populate deadlines
/// when a matter is opened.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatutoryTemplate {
    pub event:                    String,
    pub event_type:               String,  // Statutory | Procedural
    pub description:              String,
    pub typical_days_from_filing: Option<i32>,  // None = triggered by government action
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeadlineInput {
    pub matter_id:       String,
    pub ip_asset_id:     Option<String>,
    pub docketing_event: String,
    pub event_type:      Option<String>,
    pub due_date:        String,
    pub notes:           Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDeadlineInput {
    pub docketing_event: Option<String>,
    pub due_date:        Option<String>,
    pub notes:           Option<String>,
}

// ---------------------------------------------------------------------------
// Statutory templates — Indian IP practice
// ---------------------------------------------------------------------------

fn trademark_templates() -> Vec<StatutoryTemplate> {
    vec![
        StatutoryTemplate {
            event:                    "Examination Report Response".into(),
            event_type:               "Statutory".into(),
            description:              "Respond to TM examination report issued by the Registry".into(),
            typical_days_from_filing: None,
        },
        StatutoryTemplate {
            event:                    "Hearing Attendance".into(),
            event_type:               "Statutory".into(),
            description:              "Attend hearing if called by the Registrar".into(),
            typical_days_from_filing: None,
        },
        StatutoryTemplate {
            event:                    "Opposition Response".into(),
            event_type:               "Statutory".into(),
            description:              "File counter-statement to opposition within 2 months of notice".into(),
            typical_days_from_filing: None,
        },
        StatutoryTemplate {
            event:                    "Renewal Filing".into(),
            event_type:               "Statutory".into(),
            description:              "File TM-R renewal (10-year cycle from registration date)".into(),
            typical_days_from_filing: None,
        },
        StatutoryTemplate {
            event:                    "Send Application Copies to Client".into(),
            event_type:               "Procedural".into(),
            description:              "Dispatch TM-A acknowledgment to client after filing".into(),
            typical_days_from_filing: Some(7),
        },
    ]
}

fn patent_templates() -> Vec<StatutoryTemplate> {
    vec![
        StatutoryTemplate {
            event:                    "Request for Examination (RFE)".into(),
            event_type:               "Statutory".into(),
            description:              "File Form 18 / 18A within 48 months of priority date".into(),
            typical_days_from_filing: Some(1460), // ~48 months
        },
        StatutoryTemplate {
            event:                    "First Examination Report (FER) Response".into(),
            event_type:               "Statutory".into(),
            description:              "Respond to FER within 12 months of issuance".into(),
            typical_days_from_filing: None,
        },
        StatutoryTemplate {
            event:                    "Hearing".into(),
            event_type:               "Statutory".into(),
            description:              "Attend patent hearing if called by the Controller".into(),
            typical_days_from_filing: None,
        },
        StatutoryTemplate {
            event:                    "Annuity Payment".into(),
            event_type:               "Statutory".into(),
            description:              "Annual renewal fee from 2nd year after grant".into(),
            typical_days_from_filing: None,
        },
        StatutoryTemplate {
            event:                    "Send Filing Receipt to Client".into(),
            event_type:               "Procedural".into(),
            description:              "Dispatch application number and filing receipt to client".into(),
            typical_days_from_filing: Some(7),
        },
    ]
}

fn design_templates() -> Vec<StatutoryTemplate> {
    vec![
        StatutoryTemplate {
            event:                    "Examination Objections Response".into(),
            event_type:               "Statutory".into(),
            description:              "Respond to design examination objections".into(),
            typical_days_from_filing: None,
        },
        StatutoryTemplate {
            event:                    "Renewal (5-year cycle)".into(),
            event_type:               "Statutory".into(),
            description:              "Renew registered design before expiry (max 15 years total)".into(),
            typical_days_from_filing: None,
        },
    ]
}

fn copyright_templates() -> Vec<StatutoryTemplate> {
    vec![
        StatutoryTemplate {
            event:                    "Diary Number Follow-up".into(),
            event_type:               "Procedural".into(),
            description:              "Follow up with Copyright Office for diary number confirmation".into(),
            typical_days_from_filing: Some(30),
        },
        StatutoryTemplate {
            event:                    "Registration Certificate".into(),
            event_type:               "Statutory".into(),
            description:              "Collect copyright registration certificate from Copyright Office".into(),
            typical_days_from_filing: None,
        },
    ]
}

// ---------------------------------------------------------------------------
// Deadline commands
// ---------------------------------------------------------------------------

/// All pending deadlines across all matters — powers the DocketList view.
#[tauri::command]
pub async fn list_all_deadlines(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<DeadlineSummary>, String> {
    let db = state.db.lock().await;
    queries::deadlines::list_all_pending(&db)
        .await
        .map_err(|e| e.to_string())
}

/// All deadlines for a single matter — powers the MatterDetail → Deadlines tab.
#[tauri::command]
pub async fn list_deadlines(
    matter_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Deadline>, String> {
    let db = state.db.lock().await;
    queries::deadlines::list_for_matter(&db, &matter_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_deadline(
    input: CreateDeadlineInput,
    state: tauri::State<'_, AppState>,
) -> Result<Deadline, String> {
    let db = state.db.lock().await;
    let id = Uuid::new_v4().to_string();
    queries::deadlines::create(&db, &id, &input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_deadline(
    id: String,
    input: UpdateDeadlineInput,
    state: tauri::State<'_, AppState>,
) -> Result<Deadline, String> {
    let db = state.db.lock().await;
    queries::deadlines::update(&db, &id, &input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mark_deadline_complete(
    id: String,
    notes: String,
    state: tauri::State<'_, AppState>,
) -> Result<Deadline, String> {
    let db = state.db.lock().await;
    queries::deadlines::mark_complete(&db, &id, &notes)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_deadline(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    queries::deadlines::delete(&db, &id)
        .await
        .map_err(|e| e.to_string())
}

/// Return the standard statutory templates for a given matter type.
/// Deck uses these to populate a "Generate standard deadlines" flow.
#[tauri::command]
pub async fn get_statutory_templates(
    matter_type: String,
) -> Result<Vec<StatutoryTemplate>, String> {
    let templates = match matter_type.as_str() {
        "Trademark"  => trademark_templates(),
        "Patent"     => patent_templates(),
        "Design"     => design_templates(),
        "Copyright"  => copyright_templates(),
        _            => vec![],
    };
    Ok(templates)
}
