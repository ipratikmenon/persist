// STUB — Phase 2 Module 5 implements this file.
// Handles SQLite → PostgreSQL sync with the Hetzner server.

use crate::AppState;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SyncStatus {
    pub last_synced_at: Option<String>,
    pub is_syncing: bool,
    pub pending_changes: u32,
}

#[tauri::command]
pub async fn sync_status(_state: tauri::State<'_, AppState>) -> Result<SyncStatus, String> {
    Ok(SyncStatus {
        last_synced_at: None,
        is_syncing: false,
        pending_changes: 0,
    })
}

#[tauri::command]
pub async fn trigger_sync(_state: tauri::State<'_, AppState>) -> Result<SyncStatus, String> {
    Err("STUB: trigger_sync — implement in Phase 2 M5".into())
}
