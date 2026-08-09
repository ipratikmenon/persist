// Wire types — the payloads Keel's projection layer produces.
//
// These MUST match src-tauri/src/services/sync_engine/projection.rs. They are
// duplicated rather than shared because the two crates ship independently: the
// desktop may be a version ahead of the server, and a shared struct would make
// that a compile error instead of a handled mismatch.
//
// Unknown fields are rejected (`deny_unknown_fields`) so a desktop that starts
// sending something new fails loudly here rather than silently dropping it.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatterPublic {
    pub id: String,
    pub client_id: String,
    pub title: String,
    pub matter_type: String,
    pub status: String,
    pub opened_date: String,
    pub forum: Option<String>,
    pub jurisdiction: String,
    pub client_notes: Option<String>,
    pub responsible_attorney: Option<String>,
    pub next_deadline_date: Option<String>,
    pub next_deadline_event: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeadlinePublic {
    pub id: String,
    pub client_id: String,
    pub matter_id: String,
    pub docketing_event: String,
    pub due_date: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IpAssetPublic {
    pub id: String,
    pub client_id: String,
    pub matter_id: String,
    pub asset_type: String,
    pub title: String,
    pub application_number: Option<String>,
    pub registration_number: Option<String>,
    pub filing_date: Option<String>,
    pub registration_date: Option<String>,
    pub expiry_date: Option<String>,
    pub status: String,
    pub classes: Vec<i64>,
    pub jurisdiction: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvoicePublic {
    pub id: String,
    pub client_id: String,
    pub status: String,
    pub invoice_date: String,
    pub due_date: Option<String>,
    pub subtotal: f64,
    pub cgst_amount: f64,
    pub sgst_amount: f64,
    pub igst_amount: f64,
    pub total_with_tax: f64,
    pub amount_paid: f64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClientPublic {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortalUserPublic {
    pub id: String,
    pub client_id: String,
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub status: String,
}

// ---------------------------------------------------------------------------
// Envelope
// ---------------------------------------------------------------------------

/// One outbox entry as it arrives. `payload` is absent for a Delete — a
/// tombstone only needs to say which row to remove.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PushEntry {
    /// The desktop's sync_outbox row id. Echoed back so it can be cleared.
    pub outbox_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub op: String,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushRequest {
    pub entries: Vec<PushEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rejection {
    pub outbox_id: String,
    pub reason: String,
}

/// The desktop clears only what was accepted. A rejected entry stays queued and
/// is retried, so a bad payload never silently disappears.
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PushResponse {
    pub accepted: Vec<String>,
    pub rejected: Vec<Rejection>,
}

// ---------------------------------------------------------------------------
// Inbound (client-authored, pulled by the desktop)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PendingUpload {
    pub id: String,
    pub client_id: String,
    pub portal_user_id: String,
    pub matter_id: Option<String>,
    pub filename: String,
    pub mime_type: String,
    pub file_size_bytes: i64,
    pub object_key: String,
    pub sha256: String,
    pub scan_status: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PendingDispute {
    pub id: String,
    pub client_id: String,
    pub portal_user_id: String,
    pub invoice_id: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullResponse {
    pub uploads: Vec<PendingUpload>,
    pub disputes: Vec<PendingDispute>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AckRequest {
    /// Ids the desktop has ingested into the vault / matter file.
    #[serde(default)]
    pub ingested_uploads: Vec<String>,
    #[serde(default)]
    pub rejected_uploads: Vec<String>,
    #[serde(default)]
    pub ingested_disputes: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AckResponse {
    pub marked: u64,
}
