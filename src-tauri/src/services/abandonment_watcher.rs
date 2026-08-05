// Abandonment watcher background service.
// specs/module-02-docketing.md §2.11
//
// A missed statutory IP deadline is usually irreversible — the application is
// abandoned and the client's right is gone. This service exists so that never
// happens quietly, and so the firm can prove afterwards which warnings were
// raised and when.
//
// Escalation ladder, for Statutory deadlines only:
//   L1  14 days out   in-app notification
//   L2   7 days out   in-app, and (Phase 3) WhatsApp to the responsible partner
//   L3   3 days out   in-app, marked as requiring a check-in
//   L4  overdue       status -> 'Missed', highest-priority notification
//
// Each level fires at most once per deadline, enforced by a UNIQUE index on
// (deadline_id, escalation_level) rather than by application logic — the service
// runs every 30 minutes and must not raise the same warning 48 times a day.
//
// Procedural and Custom deadlines are deliberately excluded. They are the firm's
// own working dates; escalating them would train attorneys to ignore the alerts
// that actually matter.

use sqlx::SqlitePool;
use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;
use uuid::Uuid;

/// How often the ladder is evaluated.
const POLL_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// Days-remaining thresholds for levels 1..3.
const L1_DAYS: i64 = 14;
const L2_DAYS: i64 = 7;
const L3_DAYS: i64 = 3;

#[derive(Debug, sqlx::FromRow)]
struct DueRow {
    id:              String,
    docketing_event: String,
    due_date:        String,
    status:          String,
    matter_title:    String,
}

/// Start the abandonment watcher. Spawns a Tokio task and returns immediately.
pub async fn start(pool: SqlitePool, app: AppHandle) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            if let Err(e) = run_cycle(&pool, Some(&app)).await {
                log::warn!("abandonment_watcher: cycle failed: {e}");
            }
        }
    });
}

/// One evaluation pass. `app` is None in tests, where notifications are skipped.
pub async fn run_cycle(pool: &SqlitePool, app: Option<&AppHandle>) -> anyhow::Result<usize> {
    let today = chrono::Utc::now().date_naive();

    // Only Statutory deadlines that are still open.
    let rows = sqlx::query_as::<_, DueRow>(
        "SELECT d.id, d.docketing_event, d.due_date, d.status, m.title AS matter_title
         FROM deadlines d
         JOIN matters m ON m.id = d.matter_id
         WHERE d.event_type = 'Statutory'
           AND d.status IN ('Pending', 'Missed')",
    )
    .fetch_all(pool)
    .await?;

    let mut raised = 0usize;

    for row in rows {
        let Ok(due) = chrono::NaiveDate::parse_from_str(&row.due_date, "%Y-%m-%d") else {
            log::warn!("abandonment_watcher: unparseable due_date on {}", row.id);
            continue;
        };

        let days_left = (due - today).num_days();

        // Highest level the deadline currently qualifies for. Levels are
        // cumulative: a deadline first seen 5 days out should raise L1 and L2
        // as well, so the audit trail is not misleading about what was known.
        let level = if days_left < 0 {
            4
        } else if days_left <= L3_DAYS {
            3
        } else if days_left <= L2_DAYS {
            2
        } else if days_left <= L1_DAYS {
            1
        } else {
            continue;
        };

        // Overdue and still open means abandoned unless someone acts.
        if level == 4 && row.status == "Pending" {
            sqlx::query(
                "UPDATE deadlines
                 SET status = 'Missed', urgency = 'Overdue', updated_at = datetime('now')
                 WHERE id = ? AND status = 'Pending'",
            )
            .bind(&row.id)
            .execute(pool)
            .await?;

            log::error!(
                "MISSED STATUTORY DEADLINE: {} ({}) due {}",
                row.docketing_event, row.matter_title, row.due_date
            );
        }

        for l in 1..=level {
            if record_escalation(pool, &row.id, l).await? {
                raised += 1;
                if let Some(app) = app {
                    notify(app, l, &row);
                }
            }
        }
    }

    if raised > 0 {
        log::info!("abandonment_watcher: raised {raised} new escalation(s)");
    }
    Ok(raised)
}

/// Insert an escalation if this level has not already fired for this deadline.
/// Returns true when a new row was created.
///
/// Relies on the UNIQUE index rather than a SELECT-then-INSERT, so two
/// overlapping cycles cannot both raise the same level.
async fn record_escalation(pool: &SqlitePool, deadline_id: &str, level: i64) -> anyhow::Result<bool> {
    // Phase 3 will add "whatsapp" at L2+ (Module 6 notifications).
    let channels = r#"["in_app"]"#;

    let result = sqlx::query(
        "INSERT OR IGNORE INTO deadline_escalations
             (id, deadline_id, escalation_level, notification_channels)
         VALUES (?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(deadline_id)
    .bind(level)
    .bind(channels)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

fn notify(app: &AppHandle, level: i64, row: &DueRow) {
    let (title, urgency_word) = match level {
        1 => ("Deadline in 14 days", "approaching"),
        2 => ("Deadline in 7 days", "close"),
        3 => ("Deadline in 3 days", "imminent"),
        _ => ("STATUTORY DEADLINE MISSED", "missed"),
    };

    let body = if level == 4 {
        format!(
            "{} — {} was due {}. The application may be abandoned. Act now.",
            row.matter_title, row.docketing_event, row.due_date
        )
    } else {
        format!(
            "{} — {} is {} (due {}).",
            row.matter_title, row.docketing_event, urgency_word, row.due_date
        )
    };

    if let Err(e) = app.notification().builder().title(title).body(&body).show() {
        log::warn!("abandonment_watcher: notification failed: {e}");
    }
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

        sqlx::query("INSERT INTO clients (id, name) VALUES ('c1', 'Acme Corp')")
            .execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO matters (id, client_id, title, matter_type, opened_date)
             VALUES ('M-1', 'c1', 'PETALVEDA trademark', 'Trademark', date('now'))",
        )
        .execute(&pool).await.unwrap();
        pool
    }

    /// Insert a statutory deadline `days` from today (negative = overdue).
    async fn seed_deadline(pool: &SqlitePool, id: &str, days: i64, event_type: &str) {
        let due = (chrono::Utc::now().date_naive() + chrono::Duration::days(days))
            .format("%Y-%m-%d").to_string();
        sqlx::query(
            "INSERT INTO deadlines (id, matter_id, docketing_event, event_type, due_date)
             VALUES (?, 'M-1', 'Response to Examination Report', ?, ?)",
        )
        .bind(id).bind(event_type).bind(due)
        .execute(pool).await.unwrap();
    }

    async fn levels_for(pool: &SqlitePool, deadline_id: &str) -> Vec<i64> {
        sqlx::query_scalar(
            "SELECT escalation_level FROM deadline_escalations
             WHERE deadline_id = ? ORDER BY escalation_level",
        )
        .bind(deadline_id)
        .fetch_all(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn far_future_deadline_raises_nothing() {
        let pool = test_pool().await;
        seed_deadline(&pool, "d1", 90, "Statutory").await;

        let raised = run_cycle(&pool, None).await.unwrap();
        assert_eq!(raised, 0);
        assert!(levels_for(&pool, "d1").await.is_empty());
    }

    #[tokio::test]
    async fn fourteen_days_out_raises_l1_only() {
        let pool = test_pool().await;
        seed_deadline(&pool, "d1", 14, "Statutory").await;

        run_cycle(&pool, None).await.unwrap();
        assert_eq!(levels_for(&pool, "d1").await, vec![1]);
    }

    #[tokio::test]
    async fn five_days_out_backfills_l1_and_l2() {
        // A deadline first seen inside the window must not have a misleading
        // audit trail that says only L2 was known.
        let pool = test_pool().await;
        seed_deadline(&pool, "d1", 5, "Statutory").await;

        run_cycle(&pool, None).await.unwrap();
        assert_eq!(levels_for(&pool, "d1").await, vec![1, 2]);
    }

    #[tokio::test]
    async fn three_days_out_raises_through_l3() {
        let pool = test_pool().await;
        seed_deadline(&pool, "d1", 3, "Statutory").await;

        run_cycle(&pool, None).await.unwrap();
        assert_eq!(levels_for(&pool, "d1").await, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn overdue_deadline_is_marked_missed_and_raises_l4() {
        let pool = test_pool().await;
        seed_deadline(&pool, "d1", -1, "Statutory").await;

        run_cycle(&pool, None).await.unwrap();
        assert_eq!(levels_for(&pool, "d1").await, vec![1, 2, 3, 4]);

        let (status, urgency): (String, String) =
            sqlx::query_as("SELECT status, urgency FROM deadlines WHERE id = 'd1'")
                .fetch_one(&pool).await.unwrap();
        assert_eq!(status, "Missed", "overdue statutory deadline must be marked Missed");
        assert_eq!(urgency, "Overdue");
    }

    #[tokio::test]
    async fn escalations_are_not_repeated_across_cycles() {
        let pool = test_pool().await;
        seed_deadline(&pool, "d1", 3, "Statutory").await;

        let first = run_cycle(&pool, None).await.unwrap();
        assert_eq!(first, 3);

        // The watcher runs every 30 minutes; a second pass must be silent.
        let second = run_cycle(&pool, None).await.unwrap();
        assert_eq!(second, 0, "same levels must not fire twice");
        assert_eq!(levels_for(&pool, "d1").await, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn escalation_advances_as_the_date_approaches() {
        let pool = test_pool().await;
        seed_deadline(&pool, "d1", 14, "Statutory").await;
        run_cycle(&pool, None).await.unwrap();
        assert_eq!(levels_for(&pool, "d1").await, vec![1]);

        // Move the deadline closer, as time passing would.
        let closer = (chrono::Utc::now().date_naive() + chrono::Duration::days(2))
            .format("%Y-%m-%d").to_string();
        sqlx::query("UPDATE deadlines SET due_date = ? WHERE id = 'd1'")
            .bind(closer).execute(&pool).await.unwrap();

        let raised = run_cycle(&pool, None).await.unwrap();
        assert_eq!(raised, 2, "L2 and L3 should now fire");
        assert_eq!(levels_for(&pool, "d1").await, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn procedural_deadlines_are_ignored() {
        // Internal working dates must not escalate, or attorneys learn to
        // ignore the alerts that matter.
        let pool = test_pool().await;
        seed_deadline(&pool, "d-proc", -5, "Procedural").await;
        seed_deadline(&pool, "d-cust", -5, "Custom").await;

        let raised = run_cycle(&pool, None).await.unwrap();
        assert_eq!(raised, 0);
        assert!(levels_for(&pool, "d-proc").await.is_empty());

        let status: String =
            sqlx::query_scalar("SELECT status FROM deadlines WHERE id = 'd-proc'")
                .fetch_one(&pool).await.unwrap();
        assert_eq!(status, "Pending", "procedural deadlines are never auto-Missed");
    }

    #[tokio::test]
    async fn completed_and_waived_deadlines_are_ignored() {
        let pool = test_pool().await;
        seed_deadline(&pool, "d-done", -10, "Statutory").await;
        seed_deadline(&pool, "d-waived", -10, "Statutory").await;
        sqlx::query("UPDATE deadlines SET status = 'Complete' WHERE id = 'd-done'")
            .execute(&pool).await.unwrap();
        sqlx::query("UPDATE deadlines SET status = 'Waived' WHERE id = 'd-waived'")
            .execute(&pool).await.unwrap();

        let raised = run_cycle(&pool, None).await.unwrap();
        assert_eq!(raised, 0, "settled deadlines must not escalate");
    }

    #[tokio::test]
    async fn already_missed_deadline_is_not_restatused_but_keeps_its_trail() {
        let pool = test_pool().await;
        seed_deadline(&pool, "d1", -2, "Statutory").await;

        run_cycle(&pool, None).await.unwrap();
        let raised = run_cycle(&pool, None).await.unwrap();
        assert_eq!(raised, 0);

        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM deadline_escalations WHERE deadline_id = 'd1'",
        )
        .fetch_one(&pool).await.unwrap();
        assert_eq!(n, 4, "exactly one row per level");
    }
}
