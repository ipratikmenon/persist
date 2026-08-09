// Phase 1 Module 2 extended — IP Asset records (B02).
// All Tauri IPC commands for the ip_assets table.
// See specs/module-02-docketing.md §2 for the data model.

use crate::db::queries::ip_assets as ip_asset_queries;
use crate::AppState;
use crate::services::sync_engine::{self, EntityType, Op};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// IPC types — must stay in sync with src/lib/ipc-types.ts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpAsset {
    pub id:                    String,
    pub matter_id:             String,
    pub asset_type:            String,  // Trademark | Patent | Design | Copyright | PlantVariety
    pub title:                 String,
    pub application_number:    Option<String>,
    pub registration_number:   Option<String>,
    pub filing_date:           Option<String>,
    pub priority_date:         Option<String>,
    pub grant_date:            Option<String>,
    pub registration_date:     Option<String>,
    pub expiry_date:           Option<String>,
    pub applicant_entity_type: String,
    pub jurisdiction:          String,
    /// Nice/Locarno class numbers, parsed from the stored JSON array.
    pub classes:               Vec<i64>,
    pub status:                String,
    pub notes:                 Option<String>,
    pub created_at:            String,
    pub updated_at:            String,
}

/// Denormalised renewal row for the firm-wide dashboard.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpcomingRenewal {
    pub id:                  String,
    pub matter_id:           String,
    pub asset_type:          String,
    pub title:               String,
    pub registration_number: Option<String>,
    pub expiry_date:         String,
    pub status:              String,
    pub jurisdiction:        String,
    pub matter_title:        String,
    pub client_name:         String,
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIpAssetInput {
    pub matter_id:             String,
    pub asset_type:            String,
    pub title:                 String,
    pub application_number:    Option<String>,
    pub registration_number:   Option<String>,
    pub filing_date:           Option<String>,
    pub priority_date:         Option<String>,
    pub grant_date:            Option<String>,
    pub registration_date:     Option<String>,
    pub expiry_date:           Option<String>,
    pub applicant_entity_type: Option<String>,
    pub jurisdiction:          Option<String>,
    pub classes:               Option<Vec<i64>>,
    pub status:                Option<String>,
    pub notes:                 Option<String>,
}

/// Every field optional — only what is supplied gets written.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateIpAssetInput {
    pub asset_type:            Option<String>,
    pub title:                 Option<String>,
    pub application_number:    Option<String>,
    pub registration_number:   Option<String>,
    pub filing_date:           Option<String>,
    pub priority_date:         Option<String>,
    pub grant_date:            Option<String>,
    pub registration_date:     Option<String>,
    pub expiry_date:           Option<String>,
    pub applicant_entity_type: Option<String>,
    pub jurisdiction:          Option<String>,
    pub classes:               Option<Vec<i64>>,
    pub status:                Option<String>,
    pub notes:                 Option<String>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

const ASSET_TYPES: [&str; 5] = ["Trademark", "Patent", "Design", "Copyright", "PlantVariety"];
const ENTITY_TYPES: [&str; 5] = ["Individual", "Startup", "SmallEntity", "Company", "Government"];
const STATUSES: [&str; 10] = [
    "Pending", "Examination", "Accepted", "Advertised", "Opposed",
    "Registered", "Granted", "Lapsed", "Abandoned", "Cancelled",
];

/// Reject values the CHECK constraints would reject anyway, so Deck gets a
/// readable message instead of a raw SQLite constraint error.
fn validate(
    asset_type: Option<&str>,
    entity_type: Option<&str>,
    status: Option<&str>,
) -> Result<(), String> {
    if let Some(v) = asset_type {
        if !ASSET_TYPES.contains(&v) {
            return Err(format!("Invalid asset type '{v}'. Expected one of: {}", ASSET_TYPES.join(", ")));
        }
    }
    if let Some(v) = entity_type {
        if !ENTITY_TYPES.contains(&v) {
            return Err(format!("Invalid applicant entity type '{v}'. Expected one of: {}", ENTITY_TYPES.join(", ")));
        }
    }
    if let Some(v) = status {
        if !STATUSES.contains(&v) {
            return Err(format!("Invalid status '{v}'. Expected one of: {}", STATUSES.join(", ")));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Firm-wide renewal view: assets whose renewal falls inside `withinDays`
/// (default 365), plus anything already lapsed.
#[tauri::command]
pub async fn list_upcoming_renewals(
    within_days: Option<i64>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<UpcomingRenewal>, String> {
    let pool = { state.db.lock().await.clone() };
    let rows = ip_asset_queries::list_upcoming_renewals(&pool, within_days.unwrap_or(365))
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| UpcomingRenewal {
            id:                  r.id,
            matter_id:           r.matter_id,
            asset_type:          r.asset_type,
            title:               r.title,
            registration_number: r.registration_number,
            expiry_date:         r.expiry_date.unwrap_or_default(),
            status:              r.status,
            jurisdiction:        r.jurisdiction,
            matter_title:        r.matter_title,
            client_name:         r.client_name,
        })
        .collect())
}

/// All IP assets for a matter, newest filing first.
#[tauri::command]
pub async fn list_ip_assets(
    matter_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<IpAsset>, String> {
    let pool = { state.db.lock().await.clone() };
    ip_asset_queries::list_by_matter(&pool, &matter_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_ip_asset(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<IpAsset, String> {
    let pool = { state.db.lock().await.clone() };
    ip_asset_queries::get(&pool, &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("IP asset {id} not found"))
}

#[tauri::command]
pub async fn create_ip_asset(
    input: CreateIpAssetInput,
    state: tauri::State<'_, AppState>,
) -> Result<IpAsset, String> {
    if input.title.trim().is_empty() {
        return Err("Title is required".to_string());
    }
    validate(
        Some(&input.asset_type),
        input.applicant_entity_type.as_deref(),
        input.status.as_deref(),
    )?;

    let id = Uuid::new_v4().to_string();
    let pool = { state.db.lock().await.clone() };

    let asset = ip_asset_queries::create(&pool, &id, input)
        .await
        .map_err(|e| e.to_string())?;

    sync_engine::note_change(&pool, EntityType::IpAsset, &asset.id, Op::Upsert).await;
    Ok(asset)
}

#[tauri::command]
pub async fn update_ip_asset(
    id: String,
    input: UpdateIpAssetInput,
    state: tauri::State<'_, AppState>,
) -> Result<IpAsset, String> {
    if let Some(t) = &input.title {
        if t.trim().is_empty() {
            return Err("Title cannot be empty".to_string());
        }
    }
    validate(
        input.asset_type.as_deref(),
        input.applicant_entity_type.as_deref(),
        input.status.as_deref(),
    )?;

    let pool = { state.db.lock().await.clone() };
    let asset = ip_asset_queries::update(&pool, &id, input)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("IP asset {id} not found"))?;

    sync_engine::note_change(&pool, EntityType::IpAsset, &asset.id, Op::Upsert).await;
    Ok(asset)
}

/// Delete an asset. Refuses while deadlines still reference it — unlinking or
/// removing those is an explicit decision, never a silent side effect.
#[tauri::command]
pub async fn delete_ip_asset(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let pool = { state.db.lock().await.clone() };

    let linked = ip_asset_queries::count_linked_deadlines(&pool, &id)
        .await
        .map_err(|e| e.to_string())?;

    if linked > 0 {
        return Err(format!(
            "Cannot delete: {linked} deadline(s) are linked to this asset. \
             Reassign or delete them first."
        ));
    }

    ip_asset_queries::delete(&pool, &id)
        .await
        .map_err(|e| e.to_string())?;

    sync_engine::note_change(&pool, EntityType::IpAsset, &id, Op::Delete).await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_enum_values() {
        assert!(validate(Some("Trademark"), Some("Startup"), Some("Registered")).is_ok());
        assert!(validate(None, None, None).is_ok());
    }

    #[test]
    fn rejects_unknown_asset_type() {
        let err = validate(Some("Hologram"), None, None).unwrap_err();
        assert!(err.contains("Invalid asset type"));
    }

    #[test]
    fn rejects_unknown_status() {
        let err = validate(None, None, Some("Vibing")).unwrap_err();
        assert!(err.contains("Invalid status"));
    }

    #[test]
    fn rejects_unknown_entity_type() {
        let err = validate(None, Some("Conglomerate"), None).unwrap_err();
        assert!(err.contains("Invalid applicant entity type"));
    }
}
