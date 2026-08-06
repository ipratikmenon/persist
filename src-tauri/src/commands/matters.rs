// Phase 1 Module 1 — Matter Management
// All Tauri IPC commands for matters and clients.
// See specs/module-01-matters.md for the full specification.

use crate::AppState;
use crate::db::queries;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// IPC types — must stay in sync with src/lib/ipc-types.ts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Matter {
    pub id:                     String,
    pub client_id:              String,
    pub title:                  String,
    pub matter_type:            String,
    pub sub_type:               Option<String>,
    pub status:                 String,
    pub priority:               String,
    pub responsible_partner_id: Option<String>,
    pub forum:                  Option<String>,
    pub jurisdiction:           String,
    pub opened_date:            String,
    pub target_close_date:      Option<String>,
    pub internal_notes:         Option<String>,  // NEVER expose to portal
    pub client_notes:           Option<String>,
    pub tags:                   Vec<String>,
    pub linked_matter_ids:      Vec<String>,
    pub parties:                Vec<MatterParty>,
    pub created_at:             String,
    pub updated_at:             String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterSummary {
    pub id:                   String,
    pub title:                String,
    pub client_name:          String,
    pub matter_type:          String,
    pub status:               String,
    pub priority:             String,
    pub responsible_attorney: Option<String>,
    pub next_deadline_date:   Option<String>,
    pub next_deadline_event:  Option<String>,
    pub updated_at:           String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterParty {
    pub user_id:    String,
    pub role:       String,
    pub is_primary: bool,
    pub name:       String,  // denormalised; populated once users table exists
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    pub id:          String,
    pub name:        String,
    pub client_type: String,
    pub email:       Option<String>,
    pub phone:       Option<String>,
    pub address:     Option<String>,
    pub gstin:       Option<String>,
    pub pan:         Option<String>,
    pub is_active:   bool,
    pub created_at:  String,
    pub updated_at:  String,
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterFilter {
    pub status:              Option<Vec<String>>,
    pub matter_type:         Option<Vec<String>>,
    pub responsible_user_id: Option<String>,
    pub client_id:           Option<String>,
    pub priority:            Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMatterInput {
    pub client_id:         String,
    pub title:             String,
    pub matter_type:       String,
    pub sub_type:          Option<String>,
    pub priority:          Option<String>,
    pub forum:             Option<String>,
    pub jurisdiction:      Option<String>,
    pub opened_date:       String,
    pub target_close_date: Option<String>,
    pub internal_notes:    Option<String>,
    pub client_notes:      Option<String>,
    pub tags:              Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMatterInput {
    pub title:              Option<String>,
    pub sub_type:           Option<String>,
    pub priority:           Option<String>,
    pub forum:              Option<String>,
    pub target_close_date:  Option<String>,
    pub internal_notes:     Option<String>,
    pub client_notes:       Option<String>,
    pub tags:               Option<Vec<String>>,
    pub linked_matter_ids:  Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateClientInput {
    pub name:        String,
    pub client_type: String,
    pub email:       Option<String>,
    pub phone:       Option<String>,
    pub address:     Option<String>,
    pub gstin:       Option<String>,
    pub pan:         Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateClientInput {
    pub name:        Option<String>,
    pub client_type: Option<String>,
    pub email:       Option<String>,
    pub phone:       Option<String>,
    pub address:     Option<String>,
    pub gstin:       Option<String>,
    pub pan:         Option<String>,
    pub is_active:   Option<bool>,
}

// ---------------------------------------------------------------------------
// Matter ID generation
// Format: P&P-{YYYY}-{TYPE_CODE}-{NNNN}
// ---------------------------------------------------------------------------

fn type_code(matter_type: &str) -> &'static str {
    match matter_type {
        "Trademark"  => "TM",
        "Patent"     => "PAT",
        "Design"     => "DES",
        "Copyright"  => "CPY",
        "Corporate"  => "CORP",
        "Litigation" => "LIT",
        "Paralegal"  => "PARA",
        _            => "GEN",
    }
}

async fn generate_matter_id(
    pool: &sqlx::SqlitePool,
    matter_type: &str,
) -> anyhow::Result<String> {
    let year = chrono::Utc::now().format("%Y").to_string();
    let code = type_code(matter_type);
    let seq = queries::matters::next_seq(pool, code, year.parse()?).await?;
    Ok(format!("P&P-{year}-{code}-{seq:04}"))
}

// ---------------------------------------------------------------------------
// Status transition validation
// ---------------------------------------------------------------------------

fn validate_status_transition(from: &str, to: &str) -> Result<(), String> {
    let allowed: &[&str] = match from {
        "Active"                 => &["OnHold", "PendingClientResponse", "Closed", "Archived"],
        "OnHold"                 => &["Active", "Closed", "Archived"],
        "PendingClientResponse"  => &["Active", "Closed", "Archived"],
        "Closed"                 => &["Archived"],
        "Archived"               => &[],
        _                        => return Err(format!("Unknown status: {from}")),
    };
    if allowed.contains(&to) {
        Ok(())
    } else {
        Err(format!("Cannot transition from {from} to {to}"))
    }
}

// ---------------------------------------------------------------------------
// Matter commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_matter(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Matter, String> {
    let db = state.db.lock().await;
    queries::matters::get_by_id(&db, &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Matter not found: {id}"))
}

#[tauri::command]
pub async fn list_matters(
    filter: MatterFilter,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<MatterSummary>, String> {
    let db = state.db.lock().await;
    queries::matters::list(&db, &filter)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_matter(
    input: CreateMatterInput,
    state: tauri::State<'_, AppState>,
) -> Result<Matter, String> {
    let db = state.db.lock().await;
    let id = generate_matter_id(&db, &input.matter_type)
        .await
        .map_err(|e| e.to_string())?;
    queries::matters::create(&db, &id, &input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_matter(
    id: String,
    input: UpdateMatterInput,
    state: tauri::State<'_, AppState>,
) -> Result<Matter, String> {
    let db = state.db.lock().await;
    queries::matters::update(&db, &id, &input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_matter_status(
    id: String,
    status: String,
    _note: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Matter, String> {
    let db = state.db.lock().await;
    let current = queries::matters::get_by_id(&db, &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Matter not found: {id}"))?;

    validate_status_transition(&current.status, &status)?;
    queries::matters::update_status(&db, &id, &status)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn close_matter(
    id: String,
    _reason: String,
    state: tauri::State<'_, AppState>,
) -> Result<Matter, String> {
    crate::rbac::require(&state, crate::rbac::Permission::CloseMatter).await?;

    let db = state.db.lock().await;
    let current = queries::matters::get_by_id(&db, &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Matter not found: {id}"))?;

    validate_status_transition(&current.status, "Closed")?;
    queries::matters::update_status(&db, &id, "Closed")
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn archive_matter(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Matter, String> {
    crate::rbac::require(&state, crate::rbac::Permission::ArchiveMatter).await?;

    let db = state.db.lock().await;
    let current = queries::matters::get_by_id(&db, &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Matter not found: {id}"))?;

    validate_status_transition(&current.status, "Archived")?;
    queries::matters::update_status(&db, &id, "Archived")
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_matters(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<MatterSummary>, String> {
    let db = state.db.lock().await;
    queries::matters::search(&db, &query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn assign_party(
    matter_id: String,
    user_id: String,
    role: String,
    state: tauri::State<'_, AppState>,
) -> Result<MatterParty, String> {
    let db = state.db.lock().await;
    let party_id = Uuid::new_v4().to_string();
    let is_primary = role == "Partner";

    queries::matters::add_party(&db, &party_id, &matter_id, &user_id, &role, is_primary)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_party(
    matter_id: String,
    user_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().await;

    // Prevent removing the last primary party from an Active matter
    let matter = queries::matters::get_by_id(&db, &matter_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Matter not found: {matter_id}"))?;

    if matter.status == "Active" {
        let party = matter.parties.iter().find(|p| p.user_id == user_id);
        if let Some(p) = party {
            if p.is_primary {
                let primary_count = matter.parties.iter().filter(|p| p.is_primary).count();
                if primary_count <= 1 {
                    return Err("Cannot remove the last primary party from an Active matter".into());
                }
            }
        }
    }

    queries::matters::remove_party(&db, &matter_id, &user_id)
        .await
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Client commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_client(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Client, String> {
    let db = state.db.lock().await;
    queries::clients::get_by_id(&db, &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Client not found: {id}"))
}

#[tauri::command]
pub async fn list_clients(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Client>, String> {
    let db = state.db.lock().await;
    queries::clients::list_active(&db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_client(
    input: CreateClientInput,
    state: tauri::State<'_, AppState>,
) -> Result<Client, String> {
    let db = state.db.lock().await;
    let id = Uuid::new_v4().to_string();
    queries::clients::create(&db, &id, &input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_client(
    id: String,
    input: UpdateClientInput,
    state: tauri::State<'_, AppState>,
) -> Result<Client, String> {
    let db = state.db.lock().await;
    queries::clients::update(&db, &id, &input)
        .await
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matter_type_codes() {
        assert_eq!(type_code("Trademark"),  "TM");
        assert_eq!(type_code("Patent"),     "PAT");
        assert_eq!(type_code("Design"),     "DES");
        assert_eq!(type_code("Copyright"),  "CPY");
        assert_eq!(type_code("Corporate"),  "CORP");
        assert_eq!(type_code("Litigation"), "LIT");
        assert_eq!(type_code("Paralegal"),  "PARA");
    }

    #[test]
    fn test_valid_status_transitions() {
        assert!(validate_status_transition("Active", "OnHold").is_ok());
        assert!(validate_status_transition("Active", "PendingClientResponse").is_ok());
        assert!(validate_status_transition("Active", "Closed").is_ok());
        assert!(validate_status_transition("Active", "Archived").is_ok());
        assert!(validate_status_transition("OnHold", "Active").is_ok());
        assert!(validate_status_transition("Closed", "Archived").is_ok());
    }

    #[test]
    fn test_invalid_status_transitions() {
        assert!(validate_status_transition("Archived", "Active").is_err());
        assert!(validate_status_transition("Archived", "Closed").is_err());
        assert!(validate_status_transition("Closed", "Active").is_err());
        assert!(validate_status_transition("Active", "Active").is_err());
    }

    #[test]
    fn test_matter_id_format() {
        // ID format is P&P-YYYY-TYPE-NNNN
        let id = format!("P&P-2026-{}-{:04}", "TM", 42);
        assert_eq!(id, "P&P-2026-TM-0042");
        assert!(id.starts_with("P&P-2026-TM-"));
    }
}
