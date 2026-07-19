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
    pub docketing_event: String,
    pub event_type:      String,
    pub due_date:        String,
    pub status:          String,
    pub urgency:         String,
    pub notes:           Option<String>,
    pub completed_at:    Option<String>,
    pub completed_by:    Option<String>,
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
            docketing_event: r.docketing_event,
            event_type:      r.event_type,
            due_date:        r.due_date,
            status:          r.status,
            urgency:         r.urgency,
            notes:           r.notes,
            completed_at:    r.completed_at,
            completed_by:    r.completed_by,
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
        "SELECT id, matter_id, docketing_event, event_type, due_date, status, urgency,
                notes, completed_at, completed_by, created_at, updated_at
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
        "SELECT id, matter_id, docketing_event, event_type, due_date, status, urgency,
                notes, completed_at, completed_by, created_at, updated_at
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

    sqlx::query(
        "INSERT INTO deadlines (id, matter_id, docketing_event, event_type, due_date, urgency, notes)
         VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(&input.matter_id)
    .bind(&input.docketing_event)
    .bind(event_type)
    .bind(&input.due_date)
    .bind(urgency)
    .bind(&input.notes)
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
                event_type TEXT NOT NULL DEFAULT 'Custom',
                due_date DATE NOT NULL,
                status TEXT NOT NULL DEFAULT 'Pending',
                urgency TEXT NOT NULL DEFAULT 'Normal',
                notes TEXT, completed_at DATETIME, completed_by TEXT,
                created_at DATETIME NOT NULL DEFAULT (datetime('now')),
                updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
            )"
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

    #[tokio::test]
    async fn test_mark_complete() {
        let pool = test_pool().await;
        let due = (chrono::Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d").to_string();
        let input = CreateDeadlineInput {
            matter_id:       "M-001".into(),
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
