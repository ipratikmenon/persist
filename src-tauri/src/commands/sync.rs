// Phase 2 Module 5 — sync commands.
// specs/module-05-portal.md §11
//
// These control sync and manage the client-facing surface from the desktop.
// The transport lives in services/sync_engine/transport.rs; these commands
// decide *what* would be sent, *whether* sending is on, and drive a run.
//
// Sync is off until a server URL is configured. A firm with no server keeps
// working exactly as it does today, and nothing leaves the machine.

use crate::db::queries::documents as doc_queries;
use crate::rbac::{self, Permission};
use crate::services::sync_engine::{self, transport, EntityType, Op};
use crate::AppState;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// IPC types
// ---------------------------------------------------------------------------

/// Shape preserved from the Phase 0 stub so the Deck sync store needs no rework.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub last_synced_at:  Option<String>,
    pub is_syncing:      bool,
    pub pending_changes: u32,
    // Added in M5 — Deck needs to show why nothing is syncing.
    pub is_enabled:      bool,
    pub server_url:      Option<String>,
    pub last_error:      Option<String>,
    /// Whether a sync token is stored. The token itself is never returned —
    /// Deck only needs to know whether one still has to be entered.
    pub has_token:       bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortalUser {
    pub id:            String,
    pub client_id:     String,
    pub full_name:     String,
    pub email:         String,
    pub phone:         Option<String>,
    pub status:        String,
    pub invited_by:    String,
    pub invited_at:    String,
    pub last_login_at: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct PortalUserRow {
    id:            String,
    client_id:     String,
    full_name:     String,
    email:         String,
    phone:         Option<String>,
    status:        String,
    invited_by:    String,
    invited_at:    String,
    last_login_at: Option<String>,
}

impl From<PortalUserRow> for PortalUser {
    fn from(r: PortalUserRow) -> Self {
        PortalUser {
            id: r.id, client_id: r.client_id, full_name: r.full_name,
            email: r.email, phone: r.phone, status: r.status,
            invited_by: r.invited_by, invited_at: r.invited_at,
            last_login_at: r.last_login_at,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitePortalUserInput {
    pub client_id: String,
    pub full_name: String,
    pub email:     String,
    pub phone:     Option<String>,
}

// ---------------------------------------------------------------------------
// Status and configuration
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn sync_status(state: tauri::State<'_, AppState>) -> Result<SyncStatus, String> {
    let pool = { state.db.lock().await.clone() };

    let st = sync_engine::state(&pool).await.map_err(|e| e.to_string())?;
    let pending = sync_engine::pending_count(&pool).await.map_err(|e| e.to_string())?;

    Ok(SyncStatus {
        last_synced_at:  st.last_pushed_at,
        is_syncing:      false,
        pending_changes: pending.max(0) as u32,
        is_enabled:      st.is_enabled != 0,
        server_url:      st.server_url,
        last_error:      st.last_error,
        has_token:       state.keychain.load_sync_token().is_some(),
    })
}

/// Turn sync on or off. Refuses to enable without a server URL rather than
/// silently doing nothing — a firm that believes sync is on and is wrong is
/// worse off than one that sees an error.
#[tauri::command]
pub async fn set_sync_enabled(
    enabled: bool,
    state: tauri::State<'_, AppState>,
) -> Result<SyncStatus, String> {
    rbac::require(&state, Permission::ManagePortalUsers).await?;

    let pool = { state.db.lock().await.clone() };
    sync_engine::set_enabled(&pool, enabled).await.map_err(|e| e.to_string())?;
    sync_status(state).await
}

#[tauri::command]
pub async fn set_sync_server(
    url: String,
    state: tauri::State<'_, AppState>,
) -> Result<SyncStatus, String> {
    rbac::require(&state, Permission::ManagePortalUsers).await?;

    let pool = { state.db.lock().await.clone() };
    sync_engine::set_server_url(&pool, &url).await.map_err(|e| e.to_string())?;
    sync_status(state).await
}

/// Store the shared secret the sync server expects. Kept in the OS keychain,
/// never in SQLite — the database gets backed up and copied, the keychain does not.
#[tauri::command]
pub async fn set_sync_token(
    token: String,
    state: tauri::State<'_, AppState>,
) -> Result<SyncStatus, String> {
    rbac::require(&state, Permission::ManagePortalUsers).await?;

    let token = token.trim().to_string();
    if token.is_empty() {
        state.keychain.clear_sync_token();
    } else {
        // The server refuses to start on anything shorter, so catching it here
        // saves the attorney a round trip and an opaque 401.
        if token.len() < 32 {
            return Err("The sync token must be at least 32 characters".into());
        }
        state.keychain.store_sync_token(&token);
    }

    sync_status(state).await
}

/// Manual sync: drain the outbox, then collect anything the client sent us.
///
/// A push failure does not abort the pull — inbound work (a client's uploaded
/// examination report) should still arrive when the outbound leg is broken.
#[tauri::command]
pub async fn trigger_sync(state: tauri::State<'_, AppState>) -> Result<SyncStatus, String> {
    let pool = { state.db.lock().await.clone() };
    let st = sync_engine::state(&pool).await.map_err(|e| e.to_string())?;

    if st.is_enabled == 0 {
        return Err("Sync is off. Configure a server URL and enable sync first.".into());
    }
    let server_url = st
        .server_url
        .ok_or("Sync is enabled but no server URL is set.")?;
    let token = state
        .keychain
        .load_sync_token()
        .ok_or("No sync token is configured. Set one before syncing.")?;

    let mut problems: Vec<String> = Vec::new();

    match transport::push_once(&pool, &server_url, &token).await {
        Ok(out) => {
            log::info!(
                "sync push: {} sent, {} accepted, {} rejected",
                out.sent, out.accepted, out.rejected
            );
            if let Some(err) = out.first_error {
                problems.push(format!("{} change(s) rejected: {err}", out.rejected));
            }
        }
        Err(e) => {
            log::warn!("sync push failed: {e:#}");
            problems.push(format!("Push failed: {e}"));
        }
    }

    match transport::pull_once(&server_url, &token).await {
        Ok(pulled) => match ingest_pull(&pool, &server_url, &token, pulled).await {
            Ok(n) => log::info!("sync pull: {n} inbound item(s) recorded"),
            Err(e) => {
                log::warn!("sync ingest failed: {e:#}");
                problems.push(format!("Ingest failed: {e}"));
            }
        },
        Err(e) => {
            log::warn!("sync pull failed: {e:#}");
            problems.push(format!("Pull failed: {e}"));
        }
    }

    let error = if problems.is_empty() { None } else { Some(problems.join("; ")) };
    sync_engine::record_sync_run(&pool, error.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    sync_status(state).await
}

/// Record what the client sent, then acknowledge it.
///
/// Only the *metadata* lands here. The bytes stay on the server until an
/// attorney reviews the upload and pulls it into the vault — an unreviewed
/// client file must never be written into the firm's document store
/// automatically, whatever the scanner said (spec §13.4).
async fn ingest_pull(
    pool: &sqlx::SqlitePool,
    server_url: &str,
    token: &str,
    pulled: transport::PullResponse,
) -> Result<usize, String> {
    let mut ack = transport::AckRequest::default();

    for u in &pulled.uploads {
        // A matter the desktop does not know about would break the FK; drop the
        // attribution rather than the upload.
        let known_matter: Option<String> = match &u.matter_id {
            Some(m) => sqlx::query_scalar("SELECT id FROM matters WHERE id = ?")
                .bind(m)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?,
            None => None,
        };

        let result = sqlx::query(
            "INSERT INTO client_uploads (id, client_id, matter_id, filename, status, uploaded_at)
             VALUES (?, ?, ?, ?, 'Pending', datetime('now'))
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(&u.id)
        .bind(&u.client_id)
        .bind(&known_matter)
        .bind(&u.filename)
        .execute(pool)
        .await;

        match result {
            Ok(_) => ack.ingested_uploads.push(u.id.clone()),
            Err(e) => {
                // An unknown client is the likely cause. Rejecting tells the
                // portal to stop offering it, instead of looping for ever.
                log::warn!("rejecting client upload {}: {e}", u.id);
                ack.rejected_uploads.push(u.id.clone());
            }
        }
    }

    // Invoice disputes are advisory — the attorney reads them in the portal tab.
    for d in &pulled.disputes {
        log::info!("client disputed invoice {}: {}", d.invoice_id, d.reason);
        ack.ingested_disputes.push(d.id.clone());
    }

    let total = ack.ingested_uploads.len() + ack.rejected_uploads.len() + ack.ingested_disputes.len();
    if total == 0 {
        return Ok(0);
    }

    transport::ack(server_url, token, &ack)
        .await
        .map_err(|e| e.to_string())?;

    Ok(total)
}

// ---------------------------------------------------------------------------
// Portal users
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn invite_portal_user(
    input: InvitePortalUserInput,
    state: tauri::State<'_, AppState>,
) -> Result<PortalUser, String> {
    let session = rbac::require(&state, Permission::ManagePortalUsers).await?;

    let email = input.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err("A valid email address is required".into());
    }
    if input.full_name.trim().is_empty() {
        return Err("Name is required".into());
    }

    let pool = { state.db.lock().await.clone() };

    // One email, one client — enforced by a unique index too, but a readable
    // message beats a constraint error.
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT client_id FROM portal_users WHERE email = ?",
    )
    .bind(&email)
    .fetch_optional(&pool)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(other) = existing {
        return Err(if other == input.client_id {
            format!("{email} is already invited for this client")
        } else {
            format!("{email} is already a portal user for another client")
        });
    }

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO portal_users (id, client_id, full_name, email, phone, invited_by)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&input.client_id)
    .bind(input.full_name.trim())
    .bind(&email)
    .bind(input.phone.as_deref().map(str::trim))
    .bind(&session.user_id)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    sync_engine::enqueue(&pool, EntityType::PortalUser, &id, Op::Upsert)
        .await
        .map_err(|e| e.to_string())?;

    get_portal_user(&pool, &id).await
}

#[tauri::command]
pub async fn list_portal_users(
    client_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<PortalUser>, String> {
    let pool = { state.db.lock().await.clone() };
    let rows = sqlx::query_as::<_, PortalUserRow>(
        "SELECT id, client_id, full_name, email, phone, status,
                invited_by, invited_at, last_login_at
         FROM portal_users WHERE client_id = ? ORDER BY full_name",
    )
    .bind(&client_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Revoke access. Syncs immediately so revocation takes effect at the portal's
/// next request rather than at token expiry.
#[tauri::command]
pub async fn revoke_portal_user(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PortalUser, String> {
    rbac::require(&state, Permission::ManagePortalUsers).await?;

    let pool = { state.db.lock().await.clone() };
    let result = sqlx::query(
        "UPDATE portal_users SET status = 'Revoked', updated_at = datetime('now')
         WHERE id = ? AND status <> 'Revoked'",
    )
    .bind(&id)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err(format!("Portal user {id} not found, or already revoked"));
    }

    sync_engine::enqueue(&pool, EntityType::PortalUser, &id, Op::Upsert)
        .await
        .map_err(|e| e.to_string())?;

    get_portal_user(&pool, &id).await
}

async fn get_portal_user(pool: &sqlx::SqlitePool, id: &str) -> Result<PortalUser, String> {
    sqlx::query_as::<_, PortalUserRow>(
        "SELECT id, client_id, full_name, email, phone, status,
                invited_by, invited_at, last_login_at
         FROM portal_users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?
    .map(Into::into)
    .ok_or_else(|| format!("Portal user {id} not found"))
}

// ---------------------------------------------------------------------------
// Sharing
// ---------------------------------------------------------------------------

/// Mark a document visible to the client.
///
/// This only sets the flag and queues the change. The bytes are cleaned and
/// uploaded by the transport (Step 3b) using `export_document`, never by this
/// command — sharing a document is an export, and export means metadata
/// stripping (B07).
#[tauri::command]
pub async fn share_document(
    document_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    rbac::require(&state, Permission::ShareDocumentWithClient).await?;

    let pool = { state.db.lock().await.clone() };

    let result = sqlx::query(
        "UPDATE documents SET is_shared_with_client = 1, updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(&document_id)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err(format!("Document {document_id} not found"));
    }

    sync_engine::enqueue(&pool, EntityType::Document, &document_id, Op::Upsert)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Withdraw a shared document. Queues a tombstone, so the mirror row and the
/// stored object are removed — not merely left un-refreshed.
#[tauri::command]
pub async fn unshare_document(
    document_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    rbac::require(&state, Permission::ShareDocumentWithClient).await?;

    let pool = { state.db.lock().await.clone() };

    let result = sqlx::query(
        "UPDATE documents SET is_shared_with_client = 0, updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(&document_id)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err(format!("Document {document_id} not found"));
    }

    sync_engine::enqueue(&pool, EntityType::Document, &document_id, Op::Delete)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Show or hide a deadline in the portal.
#[tauri::command]
pub async fn set_deadline_client_visible(
    id: String,
    visible: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    rbac::require(&state, Permission::EditMatter).await?;

    let pool = { state.db.lock().await.clone() };

    let result = sqlx::query(
        "UPDATE deadlines SET is_client_visible = ?, updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(i64::from(visible))
    .bind(&id)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err(format!("Deadline {id} not found"));
    }

    // Hiding a deadline must remove it from the mirror, not just stop updating it.
    let op = if visible { Op::Upsert } else { Op::Delete };
    sync_engine::enqueue(&pool, EntityType::Deadline, &id, op)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Documents currently visible to a client — what the portal would show.
/// Lets an attorney answer "what can they see?" without opening the portal.
#[tauri::command]
pub async fn list_shared_documents(
    matter_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<crate::commands::documents::DocumentMeta>, String> {
    let pool = { state.db.lock().await.clone() };
    let rows = doc_queries::list_for_matter(&pool, &matter_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .filter(|r| r.is_shared_with_client != 0)
        .map(crate::commands::documents::DocumentMeta::from)
        .collect())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::services::sync_engine::{self, EntityType, Op};
    use sqlx::SqlitePool;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
        pool
    }

    /// Un-sharing must queue a Delete, or the document stays in the mirror and
    /// the client keeps seeing it.
    #[tokio::test]
    async fn unsharing_queues_a_tombstone() {
        let pool = test_pool().await;

        sync_engine::enqueue(&pool, EntityType::Document, "doc-1", Op::Upsert).await.unwrap();
        sync_engine::enqueue(&pool, EntityType::Document, "doc-1", Op::Delete).await.unwrap();

        let ops: Vec<String> = sqlx::query_scalar(
            "SELECT op FROM sync_outbox WHERE entity_id = 'doc-1'")
            .fetch_all(&pool).await.unwrap();
        assert_eq!(ops, vec!["Delete"]);
    }

    /// One email may map to only one client, across the whole firm.
    #[tokio::test]
    async fn portal_user_email_is_unique_firm_wide() {
        let pool = test_pool().await;

        sqlx::query("INSERT INTO clients (id, name) VALUES ('c-1','Alpha'),('c-2','Beta')")
            .execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO users (id, name, email, role, password_hash)
             VALUES ('u-1','Partner','p@f.in','Partner','x')")
            .execute(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO portal_users (id, client_id, full_name, email, invited_by)
             VALUES ('pu-1','c-1','Anita','anita@example.com','u-1')")
            .execute(&pool).await.unwrap();

        // Same address under a different client must be refused by the index.
        let dup = sqlx::query(
            "INSERT INTO portal_users (id, client_id, full_name, email, invited_by)
             VALUES ('pu-2','c-2','Anita Again','anita@example.com','u-1')")
            .execute(&pool).await;

        assert!(dup.is_err(), "one email must not resolve to two clients");
    }
}
