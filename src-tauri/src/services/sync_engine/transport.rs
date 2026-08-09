// Push/pull transport — the only code that sends firm data off the machine.
// specs/module-05-portal.md §5, §12.
//
// Everything it sends has already been through projection.rs. This module does
// no filtering of its own: if it could decide what to send, there would be two
// places to get that decision wrong.
//
// FAILURE POSTURE
//
// A failed push never drops an outbox entry. The server acknowledges entries
// individually, and only acknowledged ids are cleared — a payload the server
// rejects stays queued and is retried, so a bug in one row cannot silently lose
// a change the firm made.

use super::projection::{self, Withheld};
use super::OutboxEntry;
use crate::db::queries::{billing, deadlines, ip_assets, matters};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// How many entries go in one push. Small enough that a failure retries
/// quickly, large enough that a first sync is not hundreds of round trips.
const BATCH_SIZE: i64 = 100;

const TOKEN_HEADER: &str = "x-persist-sync-token";

// ---------------------------------------------------------------------------
// Wire envelope — must match server/src/types.rs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PushEntry {
    outbox_id:   String,
    entity_type: String,
    entity_id:   String,
    op:          String,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload:     Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PushRequest {
    entries: Vec<PushEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Rejection {
    outbox_id: String,
    reason:    String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PushResponse {
    accepted: Vec<String>,
    #[serde(default)]
    rejected: Vec<Rejection>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingUpload {
    pub id:              String,
    pub client_id:       String,
    pub portal_user_id:  String,
    pub matter_id:       Option<String>,
    pub filename:        String,
    pub mime_type:       String,
    pub file_size_bytes: i64,
    pub object_key:      String,
    pub sha256:          String,
    pub scan_status:     String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingDispute {
    pub id:             String,
    pub client_id:      String,
    pub portal_user_id: String,
    pub invoice_id:     String,
    pub reason:         String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullResponse {
    pub uploads:  Vec<PendingUpload>,
    pub disputes: Vec<PendingDispute>,
}

/// Outcome of one push cycle, for the UI and the log.
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PushOutcome {
    pub sent:     usize,
    pub accepted: usize,
    pub rejected: usize,
    /// Kept short — the full reasons go to the log.
    pub first_error: Option<String>,
}

// ---------------------------------------------------------------------------
// Building payloads
// ---------------------------------------------------------------------------

/// Turn one outbox entry into a wire entry, reading current state from SQLite.
///
/// Returns Ok(None) when the row should not be sent — either it has been
/// deleted locally since it was queued, or the projection withheld it (a
/// deadline the attorney has since made private, a draft invoice). Both are
/// normal, and both clear the entry rather than retrying for ever.
async fn build_entry(pool: &SqlitePool, e: &OutboxEntry) -> Result<Option<PushEntry>> {
    let op = e.op.as_str();

    // A tombstone needs no payload: the server only needs to know what to drop.
    if op == "Delete" {
        return Ok(Some(PushEntry {
            outbox_id:   e.id.clone(),
            entity_type: e.entity_type.clone(),
            entity_id:   e.entity_id.clone(),
            op:          op.to_string(),
            payload:     None,
        }));
    }

    let payload: Option<serde_json::Value> = match e.entity_type.as_str() {
        "Client" => {
            let Some(row) = matters::get_client_row(pool, &e.entity_id).await? else {
                return Ok(None);
            };
            Some(serde_json::to_value(projection::project_client(&row))?)
        }
        "Matter" => {
            let Some(row) = matters::get_row(pool, &e.entity_id).await? else {
                return Ok(None);
            };
            // Resolve the attorney to a display name here; the projection
            // signature refuses a user id, so this is the only way in.
            let name = matters::responsible_attorney_name(pool, &row).await?;
            Some(serde_json::to_value(projection::project_matter(
                &row,
                name.as_deref(),
                None,
            ))?)
        }
        "Deadline" => {
            let Some(row) = deadlines::get_row(pool, &e.entity_id).await? else {
                return Ok(None);
            };
            let client_id = matters::client_id_for_matter(pool, &row.matter_id).await?;
            match projection::project_deadline(&row, &client_id) {
                Ok(p) => Some(serde_json::to_value(p)?),
                Err(Withheld::DeadlineNotClientVisible) => {
                    // Made private after it was queued. Nothing to send.
                    log::info!("deadline {} withheld: not client-visible", e.entity_id);
                    return Ok(None);
                }
                Err(other) => anyhow::bail!("deadline withheld: {other:?}"),
            }
        }
        "IpAsset" => {
            let Some(row) = ip_assets::get_row(pool, &e.entity_id).await? else {
                return Ok(None);
            };
            let client_id = matters::client_id_for_matter(pool, &row.matter_id).await?;
            Some(serde_json::to_value(projection::project_ip_asset(&row, &client_id))?)
        }
        "Invoice" => {
            let Some(row) = billing::get_invoice_row(pool, &e.entity_id).await? else {
                return Ok(None);
            };
            match projection::project_invoice(&row) {
                Ok(p) => Some(serde_json::to_value(p)?),
                Err(Withheld::DraftInvoice) => {
                    log::info!("invoice {} withheld: still a draft", e.entity_id);
                    return Ok(None);
                }
                Err(other) => anyhow::bail!("invoice withheld: {other:?}"),
            }
        }
        other => {
            // Document and Payment projections land with the document pipeline.
            log::warn!("no projection for entity type '{other}' yet; skipping");
            return Ok(None);
        }
    };

    Ok(Some(PushEntry {
        outbox_id:   e.id.clone(),
        entity_type: e.entity_type.clone(),
        entity_id:   e.entity_id.clone(),
        op:          op.to_string(),
        payload,
    }))
}

// ---------------------------------------------------------------------------
// Push
// ---------------------------------------------------------------------------

/// Put clients ahead of everything else in a batch.
///
/// Every table in the mirror has a foreign key to `mirror.clients`, so a matter
/// sent before its client is refused outright. The outbox is oldest-first and
/// clients are created before their matters, so the order usually holds already
/// — but "usually" is not what a first sync should rest on. The sort is stable,
/// so within each group the outbox order survives.
fn order_for_push(entries: &mut [PushEntry]) {
    entries.sort_by_key(|e| if e.entity_type == "Client" { 0 } else { 1 });
}

/// Drain one batch of the outbox to the server.
pub async fn push_once(
    pool: &SqlitePool,
    server_url: &str,
    token: &str,
) -> Result<PushOutcome> {
    let batch = super::pending(pool, BATCH_SIZE).await?;
    if batch.is_empty() {
        return Ok(PushOutcome::default());
    }

    let mut entries = Vec::new();
    let mut skip_ids = Vec::new();

    for e in &batch {
        match build_entry(pool, e).await {
            Ok(Some(entry)) => entries.push(entry),
            // Nothing to send — clear it rather than retrying for ever.
            Ok(None) => skip_ids.push(e.id.clone()),
            Err(err) => {
                log::warn!("could not build payload for {}: {err:#}", e.entity_id);
                super::record_failure(pool, &e.id, &format!("{err:#}")).await?;
            }
        }
    }

    super::clear(pool, &skip_ids).await?;

    if entries.is_empty() {
        return Ok(PushOutcome::default());
    }

    order_for_push(&mut entries);

    let sent = entries.len();
    let url = format!("{}/sync/push", server_url.trim_end_matches('/'));

    let response = reqwest::Client::new()
        .post(&url)
        .header(TOKEN_HEADER, token)
        .json(&PushRequest { entries })
        .send()
        .await
        .with_context(|| format!("push to {url}"))?;

    if !response.status().is_success() {
        anyhow::bail!("sync server returned {}", response.status());
    }

    let body: PushResponse = response.json().await.context("decode push response")?;

    super::clear(pool, &body.accepted).await?;

    // Rejected entries stay queued. Record why, so a stuck row is visible.
    for r in &body.rejected {
        log::warn!("server rejected {}: {}", r.outbox_id, r.reason);
        super::record_failure(pool, &r.outbox_id, &r.reason).await?;
    }

    Ok(PushOutcome {
        sent,
        accepted: body.accepted.len(),
        rejected: body.rejected.len(),
        first_error: body.rejected.first().map(|r| r.reason.clone()),
    })
}

// ---------------------------------------------------------------------------
// Pull
// ---------------------------------------------------------------------------

/// Fetch client-authored items awaiting ingest. Applying them is a separate,
/// deliberate step — the desktop can refuse.
pub async fn pull_once(server_url: &str, token: &str) -> Result<PullResponse> {
    let url = format!("{}/sync/pull", server_url.trim_end_matches('/'));

    let response = reqwest::Client::new()
        .get(&url)
        .header(TOKEN_HEADER, token)
        .send()
        .await
        .with_context(|| format!("pull from {url}"))?;

    if !response.status().is_success() {
        anyhow::bail!("sync server returned {}", response.status());
    }

    response.json().await.context("decode pull response")
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AckRequest {
    pub ingested_uploads:  Vec<String>,
    pub rejected_uploads:  Vec<String>,
    pub ingested_disputes: Vec<String>,
}

/// Tell the server which inbound items the desktop has handled.
pub async fn ack(server_url: &str, token: &str, req: &AckRequest) -> Result<u64> {
    #[derive(Deserialize)]
    struct AckResponse { marked: u64 }

    let url = format!("{}/sync/ack", server_url.trim_end_matches('/'));

    let response = reqwest::Client::new()
        .post(&url)
        .header(TOKEN_HEADER, token)
        .json(req)
        .send()
        .await
        .with_context(|| format!("ack to {url}"))?;

    if !response.status().is_success() {
        anyhow::bail!("sync server returned {}", response.status());
    }

    Ok(response.json::<AckResponse>().await?.marked)
}

// ---------------------------------------------------------------------------
// Tests
//
// The network paths need a live server (exercised by server/scripts/test-sync.sh).
// What is worth testing here is payload construction: that a withheld row
// produces no entry at all, which is the difference between "we chose not to
// send this" and "we sent it and hoped the server would refuse".
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::sync_engine::{self, EntityType, Op};

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();

        sqlx::query("INSERT INTO clients (id, name) VALUES ('c-1','Acme Corp')")
            .execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO matters (id, client_id, title, matter_type, opened_date)
             VALUES ('M-1','c-1','PETALVEDA','Trademark', date('now'))")
            .execute(&pool).await.unwrap();
        pool
    }

    /// `invoices.created_by` and `matters.responsible_partner_id` are both FKs
    /// into `users`, so tests that touch either need a row to point at.
    async fn seed_user(pool: &SqlitePool, id: &str, name: &str) {
        sqlx::query(
            "INSERT INTO users (id, name, email, role, password_hash)
             VALUES (?1, ?2, ?1 || '@persist.in', 'Partner', 'x')")
            .bind(id).bind(name)
            .execute(pool).await.unwrap();
    }

    async fn only_entry(pool: &SqlitePool) -> Option<PushEntry> {
        let batch = sync_engine::pending(pool, 10).await.unwrap();
        assert_eq!(batch.len(), 1, "expected exactly one queued entry");
        build_entry(pool, &batch[0]).await.unwrap()
    }

    #[tokio::test]
    async fn delete_needs_no_payload() {
        let pool = test_pool().await;
        sync_engine::enqueue(&pool, EntityType::Document, "doc-1", Op::Delete).await.unwrap();

        let entry = only_entry(&pool).await.expect("tombstone should be sent");
        assert_eq!(entry.op, "Delete");
        assert!(entry.payload.is_none(), "a tombstone carries no payload");
    }

    #[tokio::test]
    async fn a_row_deleted_since_queueing_is_skipped() {
        let pool = test_pool().await;
        sync_engine::enqueue(&pool, EntityType::Matter, "GONE", Op::Upsert).await.unwrap();
        assert!(only_entry(&pool).await.is_none());
    }

    #[tokio::test]
    async fn a_deadline_made_private_is_not_sent() {
        // Queued while visible, hidden before the push. It must produce no
        // entry at all — not an entry the server has to refuse.
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO deadlines (id, matter_id, docketing_event, event_type, due_date, is_client_visible)
             VALUES ('d-1','M-1','Response','Statutory','2026-12-01', 0)")
            .execute(&pool).await.unwrap();

        sync_engine::enqueue(&pool, EntityType::Deadline, "d-1", Op::Upsert).await.unwrap();
        assert!(only_entry(&pool).await.is_none(), "private deadline must not be sent");
    }

    #[tokio::test]
    async fn a_visible_deadline_is_sent_without_its_notes() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO deadlines (id, matter_id, docketing_event, event_type, due_date,
                                    is_client_visible, notes)
             VALUES ('d-1','M-1','Response','Statutory','2026-12-01', 1, 'NOTES_LEAK strategy')")
            .execute(&pool).await.unwrap();

        sync_engine::enqueue(&pool, EntityType::Deadline, "d-1", Op::Upsert).await.unwrap();
        let entry = only_entry(&pool).await.expect("visible deadline should be sent");

        let wire = serde_json::to_string(&entry).unwrap();
        assert!(wire.contains("Response"));
        assert!(!wire.contains("NOTES_LEAK"), "notes reached the wire: {wire}");
    }

    #[tokio::test]
    async fn a_draft_invoice_is_not_sent() {
        let pool = test_pool().await;
        seed_user(&pool, "user-slm", "Sree Lakshmi Menon").await;
        sqlx::query(
            "INSERT INTO invoices (id, client_id, status, invoice_date, created_by)
             VALUES ('INV-1','c-1','Draft','2026-08-01','user-slm')")
            .execute(&pool).await.unwrap();

        sync_engine::enqueue(&pool, EntityType::Invoice, "INV-1", Op::Upsert).await.unwrap();
        assert!(only_entry(&pool).await.is_none(), "a draft must never be sent");
    }

    #[tokio::test]
    async fn a_sent_invoice_is_sent_without_internal_fields() {
        let pool = test_pool().await;
        // `created_by` takes a sentinel id, since it is a free-text FK to users.
        // `gst_type` cannot — a CHECK constraint pins it to Intra/Inter — so that
        // one is asserted by key name instead.
        seed_user(&pool, "CREATEDBY_LEAK", "Kajal Thakur").await;
        sqlx::query(
            "INSERT INTO invoices (id, client_id, status, invoice_date, created_by, gst_type)
             VALUES ('INV-2','c-1','Sent','2026-08-01','CREATEDBY_LEAK','Inter')")
            .execute(&pool).await.unwrap();

        sync_engine::enqueue(&pool, EntityType::Invoice, "INV-2", Op::Upsert).await.unwrap();
        let entry = only_entry(&pool).await.expect("sent invoice should be sent");

        let wire = serde_json::to_string(&entry).unwrap();
        assert!(!wire.contains("CREATEDBY_LEAK"), "created_by reached the wire: {wire}");
        assert!(!wire.contains("createdBy"),      "created_by reached the wire: {wire}");
        assert!(!wire.contains("gstType"),        "gst_type reached the wire: {wire}");
        assert!(!wire.contains("matterIds"),      "matter_ids reached the wire: {wire}");
        assert!(!wire.contains("pdfDocId"),       "pdf_doc_id reached the wire: {wire}");
    }

    /// The mirror's foreign keys all point at mirror.clients, so a batch that
    /// sent a matter before its client would be refused on a first sync. Queue
    /// them in the wrong order and check the ordering is corrected.
    #[tokio::test]
    async fn a_client_is_pushed_ahead_of_everything_that_references_it() {
        let pool = test_pool().await;
        sync_engine::enqueue(&pool, EntityType::Matter, "M-1", Op::Upsert).await.unwrap();
        sync_engine::enqueue(&pool, EntityType::Client, "c-1", Op::Upsert).await.unwrap();

        let batch = sync_engine::pending(&pool, 10).await.unwrap();
        assert_eq!(batch[0].entity_type, "Matter", "queued matter-first, as intended");

        let mut entries = Vec::new();
        for e in &batch {
            if let Some(entry) = build_entry(&pool, e).await.unwrap() {
                entries.push(entry);
            }
        }
        order_for_push(&mut entries);

        assert_eq!(
            entries.iter().map(|e| e.entity_type.as_str()).collect::<Vec<_>>(),
            vec!["Client", "Matter"],
            "the client must go first or the matter's foreign key is refused"
        );
    }

    #[tokio::test]
    async fn matter_payload_carries_a_name_not_a_user_id() {
        let pool = test_pool().await;
        seed_user(&pool, "user-slm", "Sree Lakshmi Menon").await;
        sqlx::query(
            "UPDATE matters SET responsible_partner_id = 'user-slm',
                                internal_notes = 'INTERNAL_LEAK weak prior art'
             WHERE id = 'M-1'")
            .execute(&pool).await.unwrap();

        sync_engine::enqueue(&pool, EntityType::Matter, "M-1", Op::Upsert).await.unwrap();
        let entry = only_entry(&pool).await.expect("matter should be sent");

        let wire = serde_json::to_string(&entry).unwrap();
        assert!(wire.contains("Sree Lakshmi Menon"), "attorney name missing: {wire}");
        assert!(!wire.contains("user-slm"),       "user id reached the wire: {wire}");
        assert!(!wire.contains("INTERNAL_LEAK"),  "internal notes reached the wire: {wire}");
    }
}
