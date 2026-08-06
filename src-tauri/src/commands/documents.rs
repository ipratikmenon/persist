// Phase 1 Module 3 — Document Management IPC commands.
//
// Security invariants (enforced here, not at DB level):
//   1. Document bytes ALWAYS go through storage::vault (AES-256-GCM).
//   2. vault_path is NEVER returned to Deck — only DocumentMeta is sent.
//   3. Every byte LEAVING THE FIRM must pass through clean_metadata().
//      get_document() returns raw bytes for internal work (an attorney needs to
//      see a counterparty's tracked changes); export_document() is the
//      client-facing path and is the only one that cleans. Never send the
//      output of get_document() to a client.
//   4. Uploaded files are read via Tauri FS plugin (Deck passes bytes).

use crate::{
    db::queries::documents::{self as doc_queries, CreateDocumentInput},
    storage::{metadata, metadata::CleanReport, vault},
    AppState,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// IPC types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DocumentMeta {
    pub id: String,
    pub matter_id: String,
    pub filename: String,
    pub category: String,
    pub mime_type: String,
    pub file_size_bytes: i64,
    pub version: i64,
    pub uploaded_by: String,
    pub is_shared_with_client: bool,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    // vault_path intentionally ABSENT — never sent to Deck
}

impl From<crate::db::queries::documents::DocumentRow> for DocumentMeta {
    fn from(r: crate::db::queries::documents::DocumentRow) -> Self {
        DocumentMeta {
            id: r.id,
            matter_id: r.matter_id,
            filename: r.filename,
            category: r.category,
            mime_type: r.mime_type,
            file_size_bytes: r.file_size_bytes,
            version: r.version,
            uploaded_by: r.uploaded_by,
            is_shared_with_client: r.is_shared_with_client != 0,
            description: r.description,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// Result of a client-facing export: cleaned bytes plus what was stripped.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedDocument {
    pub filename: String,
    pub bytes:    Vec<u8>,
    pub report:   CleanReport,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadDocumentInput {
    pub matter_id: String,
    pub filename: String,
    pub category: String,
    pub mime_type: Option<String>,
    pub description: Option<String>,
    /// Raw file bytes sent by Deck via Tauri IPC.
    /// Deck reads the file with the FS plugin and passes bytes here.
    pub bytes: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// List all documents for a given matter (metadata only — no bytes).
#[tauri::command]
pub async fn list_documents(
    matter_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<DocumentMeta>, String> {
    let pool = state.db.lock().await;
    let rows = doc_queries::list_for_matter(&pool, &matter_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(DocumentMeta::from).collect())
}

/// Upload a document: encrypt bytes to vault, write metadata to DB.
/// Deck reads the file with the Tauri FS plugin and sends bytes here.
#[tauri::command]
pub async fn upload_document(
    input: UploadDocumentInput,
    state: tauri::State<'_, AppState>,
) -> Result<DocumentMeta, String> {
    let doc_id = Uuid::new_v4().to_string();
    let mime_type = input.mime_type.unwrap_or_else(|| "application/octet-stream".into());

    // Encrypt and write to vault.
    let vault_path = vault::encrypt_to_vault(
        &state.vault_dir,
        &state.vault_key,
        &input.bytes,
        &input.matter_id,
        &doc_id,
    )
    .map_err(|e| format!("vault encrypt error: {e}"))?;

    let file_size = input.bytes.len() as i64;

    let pool = state.db.lock().await;
    let row = doc_queries::create(
        &pool,
        CreateDocumentInput {
            id: &doc_id,
            matter_id: &input.matter_id,
            filename: &input.filename,
            category: &input.category,
            mime_type: &mime_type,
            file_size_bytes: file_size,
            vault_path: &vault_path,
            uploaded_by: "current_user", // replaced by real auth in Phase 1 Auth module
            description: input.description.as_deref(),
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(DocumentMeta::from(row))
}

/// Retrieve and decrypt document bytes for INTERNAL use — viewing in Deck, or
/// saving a working copy to the attorney's own machine.
///
/// Deliberately does NOT strip metadata. An attorney reviewing a counterparty's
/// draft needs to see its tracked changes and comments; stripping them here
/// would destroy exactly the information they opened the file to read.
///
/// These bytes must never be sent to a client. Use `export_document` for that.
#[tauri::command]
pub async fn get_document(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<u8>, String> {
    let pool = state.db.lock().await;
    let row = doc_queries::get_by_id(&pool, &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("document not found: {id}"))?;

    vault::decrypt_from_vault(&state.vault_dir, &state.vault_key, &row.vault_path)
        .map_err(|e| format!("vault decrypt error: {e}"))
}

/// Retrieve document bytes for a CLIENT-FACING export, with metadata stripped.
///
/// This is the only path bytes may take out of the firm. It fails closed: a file
/// type the stripper cannot clean returns an error rather than uncleaned bytes.
/// The returned report names what was removed so the attorney can see it before
/// sending (e.g. "3 tracked change(s)", "Reviewer comments", "GPS location data").
///
/// The sync engine (Module 5 §9.1) uses this same path when sharing to the portal.
#[tauri::command]
pub async fn export_document(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<ExportedDocument, String> {
    let pool = state.db.lock().await;
    let row = doc_queries::get_by_id(&pool, &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("document not found: {id}"))?;

    let bytes = vault::decrypt_from_vault(
        &state.vault_dir,
        &state.vault_key,
        &row.vault_path,
    )
    .map_err(|e| format!("vault decrypt error: {e}"))?;

    let cleaned = metadata::clean_with_report(&bytes, &row.mime_type)
        .map_err(|e| e.to_string())?;

    if cleaned.report.is_empty() {
        log::info!("Exported document {id} ({}): no metadata found", row.filename);
    } else {
        log::info!(
            "Exported document {id} ({}): removed {:?}",
            row.filename,
            cleaned.report.removed
        );
    }

    Ok(ExportedDocument {
        filename: row.filename,
        bytes:    cleaned.bytes,
        report:   cleaned.report,
    })
}

/// Delete a document: remove DB record and vault file.
#[tauri::command]
pub async fn delete_document(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    crate::rbac::require(&state, crate::rbac::Permission::DeleteDocument).await?;

    let pool = state.db.lock().await;
    let vault_path = doc_queries::delete(&pool, &id)
        .await
        .map_err(|e| e.to_string())?;

    // Best-effort vault file deletion — DB record is already gone.
    if let Err(e) = vault::delete_from_vault(&state.vault_dir, &vault_path) {
        log::warn!("vault file delete failed (document {id}): {e}");
    }

    Ok(())
}
