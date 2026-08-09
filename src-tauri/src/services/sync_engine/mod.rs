// Sync engine — pushes the public projection outward, pulls client-authored
// items back. specs/module-05-portal.md §5.
//
// Desktop SQLite is the source of truth. This module is the only thing that
// sends firm data off the machine, and everything it sends goes through
// `projection.rs` first.
//
// Off by default: with no server URL configured, `sync_state.is_enabled` stays
// 0 and nothing leaves. A firm with no server keeps working exactly as it does
// today.

pub mod projection;
pub mod transport;

use anyhow::Result;
use sqlx::SqlitePool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Outbox
// ---------------------------------------------------------------------------

/// Entities that can be projected outward.
///
/// Notification has no write path yet; it lands with the notification service.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    /// Must reach the mirror before anything that references it — every other
    /// mirror table has a foreign key to mirror.clients.
    Client,
    Matter,
    Deadline,
    IpAsset,
    Document,
    Invoice,
    Payment,
    PortalUser,
    Notification,
}

impl EntityType {
    pub fn as_str(self) -> &'static str {
        match self {
            EntityType::Client       => "Client",
            EntityType::Matter       => "Matter",
            EntityType::Deadline     => "Deadline",
            EntityType::IpAsset      => "IpAsset",
            EntityType::Document     => "Document",
            EntityType::Invoice      => "Invoice",
            EntityType::Payment      => "Payment",
            EntityType::PortalUser   => "PortalUser",
            EntityType::Notification => "Notification",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Upsert,
    /// Tombstone. Un-sharing a document must actually remove it from the
    /// mirror, not merely stop refreshing it.
    Delete,
}

impl Op {
    pub fn as_str(self) -> &'static str {
        match self {
            Op::Upsert => "Upsert",
            Op::Delete => "Delete",
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub struct OutboxEntry {
    pub id:          String,
    pub entity_type: String,
    pub entity_id:   String,
    pub op:          String,
    pub attempts:    i64,
    pub created_at:  String,
}

/// Record a change for the next push.
///
/// Enqueuing is unconditional — cheap, and a queued row for a firm that never
/// enables sync is harmless. Deciding *not* to record here would mean the change
/// is invisible if sync is switched on later.
///
/// Re-queuing the same entity collapses onto the existing pending row rather
/// than stacking: the push sends current state, so five edits before one sync
/// are one push, not five.
pub async fn enqueue(
    pool: &SqlitePool,
    entity: EntityType,
    entity_id: &str,
    op: Op,
) -> Result<()> {
    // A Delete supersedes any pending Upsert for the same entity — sending an
    // upsert after a delete would resurrect the row in the mirror.
    let existing: Option<(String, String)> = sqlx::query_as(
        "SELECT id, op FROM sync_outbox WHERE entity_type = ? AND entity_id = ?",
    )
    .bind(entity.as_str())
    .bind(entity_id)
    .fetch_optional(pool)
    .await?;

    match existing {
        Some((row_id, existing_op)) => {
            if existing_op != op.as_str() {
                sqlx::query(
                    "UPDATE sync_outbox SET op = ?, attempts = 0, last_error = NULL,
                            updated_at = datetime('now')
                     WHERE id = ?",
                )
                .bind(op.as_str())
                .bind(&row_id)
                .execute(pool)
                .await?;
            }
            // Same op already queued — nothing to do.
        }
        None => {
            sqlx::query(
                "INSERT INTO sync_outbox (id, entity_type, entity_id, op)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(entity.as_str())
            .bind(entity_id)
            .bind(op.as_str())
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

/// Oldest pending entries first — a batch for one push.
pub async fn pending(pool: &SqlitePool, limit: i64) -> Result<Vec<OutboxEntry>> {
    let rows = sqlx::query_as::<_, OutboxEntry>(
        "SELECT id, entity_type, entity_id, op, attempts, created_at
         FROM sync_outbox
         ORDER BY created_at ASC, rowid ASC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn pending_count(pool: &SqlitePool) -> Result<i64> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_outbox")
        .fetch_one(pool)
        .await?;
    Ok(n)
}

/// Clear entries the server has acknowledged.
/// Called by the push transport (Step 3b); covered by tests here.
#[allow(dead_code)]
pub async fn clear(pool: &SqlitePool, ids: &[String]) -> Result<u64> {
    if ids.is_empty() {
        return Ok(0);
    }

    let mut cleared = 0;
    for id in ids {
        let r = sqlx::query("DELETE FROM sync_outbox WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        cleared += r.rows_affected();
    }
    Ok(cleared)
}

/// Record a failed attempt. Entries are never dropped on failure — a change the
/// firm made must not vanish because the network was down.
#[allow(dead_code)]
pub async fn record_failure(pool: &SqlitePool, id: &str, error: &str) -> Result<()> {
    sqlx::query(
        "UPDATE sync_outbox
         SET attempts = attempts + 1, last_error = ?, updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(error)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Backoff before retrying an entry that has failed `attempts` times.
/// Capped so a long outage does not push the next try days away.
/// Consumed by the push loop (Step 3b).
#[allow(dead_code)]
pub fn backoff_seconds(attempts: i64) -> u64 {
    const CAP: u64 = 3600; // 1 hour
    if attempts <= 0 {
        return 0;
    }
    let exp = 2u64.saturating_pow(attempts.min(20) as u32);
    (30u64.saturating_mul(exp)).min(CAP)
}

// ---------------------------------------------------------------------------
// Sync state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]  // last_pulled_at is read by the pull transport (Step 3b)
pub struct SyncStateRow {
    pub is_enabled:     i64,
    pub server_url:     Option<String>,
    pub last_pushed_at: Option<String>,
    pub last_pulled_at: Option<String>,
    pub last_error:     Option<String>,
}

pub async fn state(pool: &SqlitePool) -> Result<SyncStateRow> {
    let row = sqlx::query_as::<_, SyncStateRow>(
        "SELECT is_enabled, server_url, last_pushed_at, last_pulled_at, last_error
         FROM sync_state WHERE id = 1",
    )
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Turn sync on or off. Enabling without a server URL is refused rather than
/// silently doing nothing — a firm that thinks sync is on and is wrong is worse
/// off than one that sees an error.
pub async fn set_enabled(pool: &SqlitePool, enabled: bool) -> Result<SyncStateRow> {
    if enabled {
        let current = state(pool).await?;
        let configured = current
            .server_url
            .as_deref()
            .map(|u| !u.trim().is_empty())
            .unwrap_or(false);

        if !configured {
            anyhow::bail!("Set a sync server URL before enabling sync");
        }
    }

    sqlx::query(
        "UPDATE sync_state SET is_enabled = ?, updated_at = datetime('now') WHERE id = 1",
    )
    .bind(i64::from(enabled))
    .execute(pool)
    .await?;

    state(pool).await
}

pub async fn set_server_url(pool: &SqlitePool, url: &str) -> Result<SyncStateRow> {
    sqlx::query(
        "UPDATE sync_state SET server_url = ?, updated_at = datetime('now') WHERE id = 1",
    )
    .bind(url.trim())
    .execute(pool)
    .await?;
    state(pool).await
}

/// Queue a change from a write path, without letting sync bookkeeping fail the
/// write itself.
///
/// The local row is already committed by the time this runs. Returning an error
/// here would show the attorney a failure for a change that did happen, which is
/// the worse of the two failure modes. Instead the miss is logged and recorded
/// on `sync_state.last_error`, so the Sync tab shows that the mirror is behind
/// rather than silently diverging.
pub async fn note_change(pool: &SqlitePool, entity: EntityType, id: &str, op: Op) {
    if let Err(e) = enqueue(pool, entity, id, op).await {
        log::error!("failed to queue {} {id} for sync: {e:#}", entity.as_str());
        let msg = format!("A change to {} {id} could not be queued for sync.", entity.as_str());
        let _ = sqlx::query(
            "UPDATE sync_state SET last_error = ?, updated_at = datetime('now') WHERE id = 1",
        )
        .bind(&msg)
        .execute(pool)
        .await;
    }
}

/// Stamp the outcome of a sync run.
///
/// `last_pushed_at` and `last_pulled_at` advance even when the run reported
/// problems: they answer "when did we last talk to the server", and a run that
/// pushed nine rows and had one rejected did talk to it. `last_error` carries
/// the qualification, and is cleared on a clean run so a stale failure does not
/// sit in the UI for ever.
pub async fn record_sync_run(pool: &SqlitePool, error: Option<&str>) -> Result<SyncStateRow> {
    sqlx::query(
        "UPDATE sync_state
            SET last_pushed_at = datetime('now'),
                last_pulled_at = datetime('now'),
                last_error     = ?,
                updated_at     = datetime('now')
          WHERE id = 1",
    )
    .bind(error)
    .execute(pool)
    .await?;
    state(pool).await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
        pool
    }

    async fn ops_for(pool: &SqlitePool, entity_id: &str) -> Vec<String> {
        sqlx::query_scalar("SELECT op FROM sync_outbox WHERE entity_id = ?")
            .bind(entity_id)
            .fetch_all(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn enqueue_records_a_pending_change() {
        let pool = test_pool().await;
        enqueue(&pool, EntityType::Matter, "M-1", Op::Upsert).await.unwrap();

        assert_eq!(pending_count(&pool).await.unwrap(), 1);
        let batch = pending(&pool, 10).await.unwrap();
        assert_eq!(batch[0].entity_type, "Matter");
        assert_eq!(batch[0].entity_id, "M-1");
        assert_eq!(batch[0].op, "Upsert");
    }

    #[tokio::test]
    async fn repeated_edits_collapse_to_one_pending_row() {
        // Five edits before a sync should be one push, not five.
        let pool = test_pool().await;
        for _ in 0..5 {
            enqueue(&pool, EntityType::Matter, "M-1", Op::Upsert).await.unwrap();
        }
        assert_eq!(pending_count(&pool).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn delete_supersedes_a_pending_upsert() {
        // Otherwise an upsert queued before an un-share would resurrect the row
        // in the mirror.
        let pool = test_pool().await;
        enqueue(&pool, EntityType::Document, "doc-1", Op::Upsert).await.unwrap();
        enqueue(&pool, EntityType::Document, "doc-1", Op::Delete).await.unwrap();

        assert_eq!(ops_for(&pool, "doc-1").await, vec!["Delete"]);
        assert_eq!(pending_count(&pool).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn different_entities_queue_separately() {
        let pool = test_pool().await;
        enqueue(&pool, EntityType::Matter,  "X-1", Op::Upsert).await.unwrap();
        enqueue(&pool, EntityType::IpAsset, "X-1", Op::Upsert).await.unwrap();
        // Same id, different entity type — two distinct rows.
        assert_eq!(pending_count(&pool).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn pending_is_oldest_first() {
        let pool = test_pool().await;
        enqueue(&pool, EntityType::Matter,   "A", Op::Upsert).await.unwrap();
        enqueue(&pool, EntityType::Deadline, "B", Op::Upsert).await.unwrap();
        enqueue(&pool, EntityType::Invoice,  "C", Op::Upsert).await.unwrap();

        let ids: Vec<String> = pending(&pool, 10).await.unwrap()
            .into_iter().map(|e| e.entity_id).collect();
        assert_eq!(ids, vec!["A", "B", "C"]);
    }

    #[tokio::test]
    async fn clear_removes_only_acknowledged_entries() {
        let pool = test_pool().await;
        enqueue(&pool, EntityType::Matter,   "A", Op::Upsert).await.unwrap();
        enqueue(&pool, EntityType::Deadline, "B", Op::Upsert).await.unwrap();

        let batch = pending(&pool, 10).await.unwrap();
        let acked: Vec<String> = batch.iter()
            .filter(|e| e.entity_id == "A")
            .map(|e| e.id.clone())
            .collect();

        assert_eq!(clear(&pool, &acked).await.unwrap(), 1);
        let left: Vec<String> = pending(&pool, 10).await.unwrap()
            .into_iter().map(|e| e.entity_id).collect();
        assert_eq!(left, vec!["B"]);
    }

    #[tokio::test]
    async fn a_failed_push_keeps_the_entry() {
        // A change the firm made must not vanish because the network was down.
        let pool = test_pool().await;
        enqueue(&pool, EntityType::Matter, "M-1", Op::Upsert).await.unwrap();
        let id = pending(&pool, 1).await.unwrap()[0].id.clone();

        record_failure(&pool, &id, "connection refused").await.unwrap();

        let after = pending(&pool, 1).await.unwrap();
        assert_eq!(after.len(), 1, "entry must survive a failure");
        assert_eq!(after[0].attempts, 1);
    }

    #[test]
    fn backoff_grows_then_caps() {
        assert_eq!(backoff_seconds(0), 0);
        assert_eq!(backoff_seconds(1), 60);
        assert_eq!(backoff_seconds(2), 120);
        assert_eq!(backoff_seconds(3), 240);
        // Capped at an hour — a long outage must not push the next try days out.
        assert_eq!(backoff_seconds(20), 3600);
        assert_eq!(backoff_seconds(1000), 3600);
    }

    #[tokio::test]
    async fn sync_is_off_by_default() {
        let pool = test_pool().await;
        let s = state(&pool).await.unwrap();
        assert_eq!(s.is_enabled, 0, "a firm with no server keeps working unchanged");
        assert!(s.server_url.is_none());
    }

    #[tokio::test]
    async fn enabling_without_a_server_url_is_refused() {
        // Silently doing nothing would leave the firm believing sync is on.
        let pool = test_pool().await;
        let err = set_enabled(&pool, true).await.unwrap_err().to_string();
        assert!(err.contains("server URL"), "got: {err}");
        assert_eq!(state(&pool).await.unwrap().is_enabled, 0);
    }

    #[tokio::test]
    async fn enabling_works_once_a_url_is_set() {
        let pool = test_pool().await;
        set_server_url(&pool, "https://sync.persistas.example").await.unwrap();
        let s = set_enabled(&pool, true).await.unwrap();
        assert_eq!(s.is_enabled, 1);

        let off = set_enabled(&pool, false).await.unwrap();
        assert_eq!(off.is_enabled, 0, "disabling never needs a URL");
    }
}
