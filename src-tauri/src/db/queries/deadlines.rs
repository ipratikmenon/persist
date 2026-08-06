use sqlx::SqlitePool;
use crate::commands::deadlines::{
    Deadline, DeadlineSummary, CreateDeadlineInput, UpdateDeadlineInput,
};

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
pub struct DeadlineRow {
    pub id:              String,
    pub matter_id:       String,
    pub ip_asset_id:     Option<String>,
    pub reference_number: Option<String>,
    pub docketing_event: String,
    pub event_type:      String,
    pub due_date:        String,
    pub status:          String,
    pub urgency:         String,
    pub notes:           Option<String>,
    pub completed_at:    Option<String>,
    pub completed_by:    Option<String>,
    pub created_by:      Option<String>,
    pub is_verified:     i64,
    pub verified_by:     Option<String>,
    pub verified_at:     Option<String>,
    pub created_at:      String,
    pub updated_at:      String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct DeadlineSummaryRow {
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

impl From<DeadlineRow> for Deadline {
    fn from(r: DeadlineRow) -> Self {
        Deadline {
            id:              r.id,
            matter_id:       r.matter_id,
            ip_asset_id:     r.ip_asset_id,
            reference_number: r.reference_number,
            docketing_event: r.docketing_event,
            event_type:      r.event_type,
            due_date:        r.due_date,
            status:          r.status,
            urgency:         r.urgency,
            notes:           r.notes,
            completed_at:    r.completed_at,
            completed_by:    r.completed_by,
            created_by:      r.created_by,
            is_verified:     r.is_verified != 0,
            verified_by:     r.verified_by,
            verified_at:     r.verified_at,
            created_at:      r.created_at,
            updated_at:      r.updated_at,
        }
    }
}

impl From<DeadlineSummaryRow> for DeadlineSummary {
    fn from(r: DeadlineSummaryRow) -> Self {
        DeadlineSummary {
            id:              r.id,
            matter_id:       r.matter_id,
            matter_title:    r.matter_title,
            matter_type:     r.matter_type,
            client_name:     r.client_name,
            docketing_event: r.docketing_event,
            event_type:      r.event_type,
            due_date:        r.due_date,
            status:          r.status,
            urgency:         r.urgency,
            notes:           r.notes,
            updated_at:      r.updated_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Urgency calculation — single source of truth used by commands + watcher
// ---------------------------------------------------------------------------

/// Recalculate urgency tier from the due date.
/// Only Pending deadlines have meaningful urgency; Complete/Waived always return "Normal".
pub fn urgency_for(due_date: &str, status: &str) -> &'static str {
    if status != "Pending" {
        return "Normal";
    }
    let today = chrono::Utc::now().date_naive();
    let due = chrono::NaiveDate::parse_from_str(due_date, "%Y-%m-%d")
        .unwrap_or(today);
    let days = (due - today).num_days();
    if days < 0       { "Overdue"  }
    else if days <= 3 { "Critical" }
    else if days <= 7 { "Warning"  }
    else              { "Normal"   }
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

pub async fn get_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Deadline>> {
    let row = sqlx::query_as::<_, DeadlineRow>(
        "SELECT id, matter_id, ip_asset_id, reference_number, docketing_event, event_type,
                due_date, status, urgency, notes, completed_at, completed_by,
                created_by, is_verified, verified_by, verified_at, created_at, updated_at
         FROM deadlines WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(Deadline::from))
}

/// All deadlines for a single matter, ordered by due date.
pub async fn list_for_matter(pool: &SqlitePool, matter_id: &str) -> anyhow::Result<Vec<Deadline>> {
    let rows = sqlx::query_as::<_, DeadlineRow>(
        "SELECT id, matter_id, ip_asset_id, reference_number, docketing_event, event_type,
                due_date, status, urgency, notes, completed_at, completed_by,
                created_by, is_verified, verified_by, verified_at, created_at, updated_at
         FROM deadlines
         WHERE matter_id = ?
         ORDER BY due_date ASC, created_at ASC"
    )
    .bind(matter_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Deadline::from).collect())
}

/// All pending deadlines across all matters — for the DocketList view.
/// Joins matters + clients for denormalized display columns.
pub async fn list_all_pending(pool: &SqlitePool) -> anyhow::Result<Vec<DeadlineSummary>> {
    let rows = sqlx::query_as::<_, DeadlineSummaryRow>(
        "SELECT d.id, d.matter_id,
                m.title AS matter_title, m.matter_type, c.name AS client_name,
                d.docketing_event, d.event_type, d.due_date, d.status, d.urgency,
                d.notes, d.updated_at
         FROM deadlines d
         JOIN matters m  ON d.matter_id = m.id
         JOIN clients c  ON m.client_id  = c.id
         WHERE d.status = 'Pending'
         ORDER BY
             CASE d.urgency
                 WHEN 'Overdue'  THEN 1
                 WHEN 'Critical' THEN 2
                 WHEN 'Warning'  THEN 3
                 ELSE                 4
             END,
             d.due_date ASC"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(DeadlineSummary::from).collect())
}

pub async fn create(
    pool: &SqlitePool,
    id: &str,
    input: &CreateDeadlineInput,
) -> anyhow::Result<Deadline> {
    let urgency = urgency_for(&input.due_date, "Pending");
    let event_type = input.event_type.as_deref().unwrap_or("Custom");
    let reference = next_reference_number(pool).await?;

    // Statutory deadlines are client-visible by default: the client is legally
    // affected and must not be surprised. Procedural/Custom are the firm's own
    // internal steps and stay private. Either is overridable per deadline.
    let client_visible = i64::from(event_type == "Statutory");

    sqlx::query(
        "INSERT INTO deadlines (id, matter_id, ip_asset_id, reference_number, docketing_event,
                                event_type, due_date, urgency, notes, is_client_visible, created_by)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(&input.matter_id)
    .bind(&input.ip_asset_id)
    .bind(&reference)
    .bind(&input.docketing_event)
    .bind(event_type)
    .bind(&input.due_date)
    .bind(urgency)
    .bind(&input.notes)
    .bind(client_visible)
    .bind(&input.created_by)
    .execute(pool)
    .await?;

    get_by_id(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Deadline not found after insert"))
}

pub async fn update(
    pool: &SqlitePool,
    id: &str,
    input: &UpdateDeadlineInput,
) -> anyhow::Result<Deadline> {
    // Fetch current to recalculate urgency if due_date changes
    let current = get_by_id(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Deadline not found: {id}"))?;

    let new_due = input.due_date.as_deref().unwrap_or(&current.due_date);
    let new_urgency = urgency_for(new_due, &current.status);

    sqlx::query(
        "UPDATE deadlines
         SET docketing_event = COALESCE(?, docketing_event),
             due_date        = COALESCE(?, due_date),
             notes           = COALESCE(?, notes),
             urgency         = ?,
             updated_at      = datetime('now')
         WHERE id = ?"
    )
    .bind(&input.docketing_event)
    .bind(&input.due_date)
    .bind(&input.notes)
    .bind(new_urgency)
    .bind(id)
    .execute(pool)
    .await?;

    get_by_id(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Deadline not found after update"))
}

pub async fn mark_complete(
    pool: &SqlitePool,
    id: &str,
    notes: &str,
) -> anyhow::Result<Deadline> {
    sqlx::query(
        "UPDATE deadlines
         SET status       = 'Complete',
             urgency      = 'Normal',
             notes        = CASE WHEN ? = '' THEN notes ELSE ? END,
             completed_at = datetime('now'),
             updated_at   = datetime('now')
         WHERE id = ?"
    )
    .bind(notes)
    .bind(notes)
    .bind(id)
    .execute(pool)
    .await?;

    get_by_id(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Deadline not found after completion"))
}

pub async fn delete(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM deadlines WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Recalculate and persist urgency for all pending deadlines.
/// Called by deadline_watcher on its polling cycle.
pub async fn refresh_all_urgency(pool: &SqlitePool) -> anyhow::Result<u64> {
    // Fetch all pending deadlines with their current due_date
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, due_date FROM deadlines WHERE status = 'Pending'"
    )
    .fetch_all(pool)
    .await?;

    let mut updated = 0u64;
    for (id, due_date) in rows {
        let urgency = urgency_for(&due_date, "Pending");
        let result = sqlx::query(
            "UPDATE deadlines SET urgency = ?, updated_at = datetime('now')
             WHERE id = ? AND urgency != ?"
        )
        .bind(urgency)
        .bind(&id)
        .bind(urgency)
        .execute(pool)
        .await?;
        updated += result.rows_affected();
    }
    Ok(updated)
}

/// Next docket reference in `P&P-DD-NNNN` form.
///
/// Sequential per firm, not per matter (spec §Business Rules), so the number is
/// unique in correspondence. Shares the `sequences` table with matter and
/// invoice IDs.
pub async fn next_reference_number(pool: &SqlitePool) -> anyhow::Result<String> {
    sqlx::query(
        "INSERT INTO sequences (key, next_val) VALUES ('DD', 2)
         ON CONFLICT(key) DO UPDATE SET next_val = next_val + 1",
    )
    .execute(pool)
    .await?;

    let next: i64 = sqlx::query_scalar("SELECT next_val FROM sequences WHERE key = 'DD'")
        .fetch_one(pool)
        .await?;

    // next_val points at the *following* number, so the one just claimed is n-1.
    Ok(format!("P&P-DD-{:04}", next - 1))
}

/// Statutory deadlines not yet checked by a second attorney, soonest first.
pub async fn list_unverified(pool: &SqlitePool) -> anyhow::Result<Vec<DeadlineSummary>> {
    let rows = sqlx::query_as::<_, DeadlineSummaryRow>(
        "SELECT d.id, d.matter_id,
                m.title AS matter_title, m.matter_type, c.name AS client_name,
                d.docketing_event, d.event_type, d.due_date, d.status, d.urgency,
                d.notes, d.updated_at
         FROM deadlines d
         JOIN matters m ON d.matter_id = m.id
         JOIN clients c ON m.client_id  = c.id
         WHERE d.is_verified = 0
           AND d.event_type = 'Statutory'
           AND d.status = 'Pending'
         ORDER BY d.due_date ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(DeadlineSummary::from).collect())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::deadlines::CreateDeadlineInput;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        // Minimal schema for deadline tests
        sqlx::query(
            "CREATE TABLE clients (
                id TEXT PRIMARY KEY, name TEXT NOT NULL, type TEXT NOT NULL DEFAULT 'Company',
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL DEFAULT (datetime('now')),
                updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
            )"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE matters (
                id TEXT PRIMARY KEY, client_id TEXT NOT NULL, title TEXT NOT NULL,
                matter_type TEXT NOT NULL DEFAULT 'Trademark',
                status TEXT NOT NULL DEFAULT 'Active',
                priority TEXT NOT NULL DEFAULT 'Normal',
                jurisdiction TEXT NOT NULL DEFAULT 'India',
                opened_date DATE NOT NULL DEFAULT (date('now')),
                tags TEXT NOT NULL DEFAULT '[]',
                linked_matter_ids TEXT NOT NULL DEFAULT '[]',
                created_at DATETIME NOT NULL DEFAULT (datetime('now')),
                updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
            )"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE deadlines (
                id TEXT PRIMARY KEY, matter_id TEXT NOT NULL, docketing_event TEXT NOT NULL,
                ip_asset_id TEXT, reference_number TEXT,
                is_client_visible INTEGER NOT NULL DEFAULT 0,
                created_by TEXT, is_verified INTEGER NOT NULL DEFAULT 0,
                verified_by TEXT, verified_at DATETIME,
                event_type TEXT NOT NULL DEFAULT 'Custom',
                due_date DATE NOT NULL,
                status TEXT NOT NULL DEFAULT 'Pending',
                urgency TEXT NOT NULL DEFAULT 'Normal',
                notes TEXT, completed_at DATETIME, completed_by TEXT,
                created_at DATETIME NOT NULL DEFAULT (datetime('now')),
                updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
            )"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE sequences (key TEXT PRIMARY KEY, next_val INTEGER NOT NULL DEFAULT 1)"
        ).execute(&pool).await.unwrap();

        // Seed client + matter
        sqlx::query("INSERT INTO clients (id, name) VALUES ('c1', 'Acme Corp')")
            .execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO matters (id, client_id, title) VALUES ('M-001', 'c1', 'Test Matter')"
        ).execute(&pool).await.unwrap();

        pool
    }

    #[test]
    fn test_urgency_calculation() {
        let today = chrono::Utc::now().date_naive();
        let fmt = |d: chrono::NaiveDate| d.format("%Y-%m-%d").to_string();

        let past = fmt(today - chrono::Duration::days(1));
        assert_eq!(urgency_for(&past,                               "Pending"), "Overdue");
        assert_eq!(urgency_for(&fmt(today + chrono::Duration::days(1)), "Pending"), "Critical");
        assert_eq!(urgency_for(&fmt(today + chrono::Duration::days(5)), "Pending"), "Warning");
        assert_eq!(urgency_for(&fmt(today + chrono::Duration::days(30)),"Pending"), "Normal");

        // Complete/Waived always Normal regardless of date
        assert_eq!(urgency_for(&past, "Complete"), "Normal");
        assert_eq!(urgency_for(&past, "Waived"),   "Normal");
    }

    #[tokio::test]
    async fn test_create_and_get_deadline() {
        let pool = test_pool().await;
        let due = (chrono::Utc::now() + chrono::Duration::days(30))
            .format("%Y-%m-%d").to_string();
        let input = CreateDeadlineInput {
            matter_id:       "M-001".into(),
            ip_asset_id:     None,
            created_by:      None,
            docketing_event: "Examination Report Response".into(),
            event_type:      Some("Statutory".into()),
            due_date:        due.clone(),
            notes:           Some("File before registry deadline".into()),
        };

        let d = create(&pool, "D-001", &input).await.unwrap();
        assert_eq!(d.docketing_event, "Examination Report Response");
        assert_eq!(d.status, "Pending");
        assert_eq!(d.urgency, "Normal");

        let fetched = get_by_id(&pool, "D-001").await.unwrap().unwrap();
        assert_eq!(fetched.event_type, "Statutory");
    }

    /// Reference numbers are sequential per firm and citable in correspondence.
    #[tokio::test]
    async fn reference_numbers_are_sequential_and_padded() {
        let pool = test_pool().await;
        let due = (chrono::Utc::now() + chrono::Duration::days(30))
            .format("%Y-%m-%d").to_string();

        let mk = |n: &str| CreateDeadlineInput {
            matter_id:       "M-001".into(),
            ip_asset_id:     None,
            created_by:      None,
            docketing_event: n.to_string(),
            event_type:      Some("Statutory".into()),
            due_date:        due.clone(),
            notes:           None,
        };

        let a = create(&pool, "R-1", &mk("First")).await.unwrap();
        let b = create(&pool, "R-2", &mk("Second")).await.unwrap();
        let c = create(&pool, "R-3", &mk("Third")).await.unwrap();

        assert_eq!(a.reference_number.as_deref(), Some("P&P-DD-0001"));
        assert_eq!(b.reference_number.as_deref(), Some("P&P-DD-0002"));
        assert_eq!(c.reference_number.as_deref(), Some("P&P-DD-0003"));
    }

    /// Statutory deadlines are client-visible by default; others are not.
    #[tokio::test]
    async fn statutory_deadlines_default_to_client_visible() {
        let pool = test_pool().await;
        let due = (chrono::Utc::now() + chrono::Duration::days(30))
            .format("%Y-%m-%d").to_string();

        let mk = |t: &str| CreateDeadlineInput {
            matter_id:       "M-001".into(),
            ip_asset_id:     None,
            created_by:      None,
            docketing_event: "Event".into(),
            event_type:      Some(t.to_string()),
            due_date:        due.clone(),
            notes:           None,
        };

        create(&pool, "V-stat", &mk("Statutory")).await.unwrap();
        create(&pool, "V-proc", &mk("Procedural")).await.unwrap();

        let stat: i64 = sqlx::query_scalar(
            "SELECT is_client_visible FROM deadlines WHERE id = 'V-stat'")
            .fetch_one(&pool).await.unwrap();
        let proc_v: i64 = sqlx::query_scalar(
            "SELECT is_client_visible FROM deadlines WHERE id = 'V-proc'")
            .fetch_one(&pool).await.unwrap();

        assert_eq!(stat, 1, "a client must not be surprised by a statutory date");
        assert_eq!(proc_v, 0, "internal steps stay private");
    }

    /// The unverified list is the daily question dual verification exists for.
    #[tokio::test]
    async fn unverified_list_shows_only_open_statutory_deadlines() {
        let pool = test_pool().await;
        let due = (chrono::Utc::now() + chrono::Duration::days(30))
            .format("%Y-%m-%d").to_string();

        let mk = |t: &str| CreateDeadlineInput {
            matter_id:       "M-001".into(),
            ip_asset_id:     None,
            created_by:      Some("user-kt".into()),
            docketing_event: format!("{t} event"),
            event_type:      Some(t.to_string()),
            due_date:        due.clone(),
            notes:           None,
        };

        create(&pool, "U-stat",  &mk("Statutory")).await.unwrap();
        create(&pool, "U-proc",  &mk("Procedural")).await.unwrap();
        create(&pool, "U-done",  &mk("Statutory")).await.unwrap();
        sqlx::query("UPDATE deadlines SET status = 'Complete' WHERE id = 'U-done'")
            .execute(&pool).await.unwrap();
        create(&pool, "U-ok", &mk("Statutory")).await.unwrap();
        sqlx::query("UPDATE deadlines SET is_verified = 1 WHERE id = 'U-ok'")
            .execute(&pool).await.unwrap();

        let unverified = list_unverified(&pool).await.unwrap();
        let ids: Vec<&str> = unverified.iter().map(|d| d.id.as_str()).collect();

        assert_eq!(ids, vec!["U-stat"],
            "only open, unverified, statutory deadlines belong here; got {ids:?}");
    }

    /// B02: a deadline can hang off a specific IP asset, and round-trips as such.
    #[tokio::test]
    async fn test_deadline_links_to_ip_asset() {
        let pool = test_pool().await;
        let due = (chrono::Utc::now() + chrono::Duration::days(60))
            .format("%Y-%m-%d").to_string();

        let linked = CreateDeadlineInput {
            matter_id:       "M-001".into(),
            ip_asset_id:     Some("ip-001".into()),
            created_by:      None,
            docketing_event: "Renewal — 10 year term".into(),
            event_type:      Some("Statutory".into()),
            due_date:        due.clone(),
            notes:           None,
        };
        let d = create(&pool, "D-100", &linked).await.unwrap();
        assert_eq!(d.ip_asset_id.as_deref(), Some("ip-001"));

        // Survives a re-read.
        let fetched = get_by_id(&pool, "D-100").await.unwrap().unwrap();
        assert_eq!(fetched.ip_asset_id.as_deref(), Some("ip-001"));

        // Matter-level deadlines still work with no asset attached.
        let unlinked = CreateDeadlineInput {
            matter_id:       "M-001".into(),
            ip_asset_id:     None,
            created_by:      None,
            docketing_event: "Client call".into(),
            event_type:      None,
            due_date:        due,
            notes:           None,
        };
        let d2 = create(&pool, "D-101", &unlinked).await.unwrap();
        assert!(d2.ip_asset_id.is_none());

        // Both appear in the matter listing.
        let all = list_for_matter(&pool, "M-001").await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_mark_complete() {
        let pool = test_pool().await;
        let due = (chrono::Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d").to_string();
        let input = CreateDeadlineInput {
            matter_id:       "M-001".into(),
            ip_asset_id:     None,
            created_by:      None,
            docketing_event: "Filing".into(),
            event_type:      None,
            due_date:        due,
            notes:           None,
        };
        create(&pool, "D-002", &input).await.unwrap();

        let completed = mark_complete(&pool, "D-002", "Filed on time").await.unwrap();
        assert_eq!(completed.status,  "Complete");
        assert_eq!(completed.urgency, "Normal");
        assert!(completed.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_urgency_refresh() {
        let pool = test_pool().await;
        let overdue_date = (chrono::Utc::now() - chrono::Duration::days(2))
            .format("%Y-%m-%d").to_string();
        let input = CreateDeadlineInput {
            matter_id:       "M-001".into(),
            ip_asset_id:     None,
            created_by:      None,
            docketing_event: "Old filing".into(),
            event_type:      None,
            due_date:        overdue_date,
            notes:           None,
        };
        create(&pool, "D-003", &input).await.unwrap();

        // Force urgency to 'Normal' to simulate a stale cached value
        sqlx::query("UPDATE deadlines SET urgency = 'Normal' WHERE id = 'D-003'")
            .execute(&pool).await.unwrap();

        let refreshed = refresh_all_urgency(&pool).await.unwrap();
        assert_eq!(refreshed, 1); // one row updated

        let d = get_by_id(&pool, "D-003").await.unwrap().unwrap();
        assert_eq!(d.urgency, "Overdue");
    }
}
