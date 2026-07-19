// Deadline watcher background service.
// Polls every 15 minutes, recalculates urgency, fires OS notifications for
// newly-overdue and newly-critical deadlines.

use std::time::Duration;
use sqlx::SqlitePool;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Start the deadline watcher.
/// Called once from lib.rs setup() after the DB pool is ready.
/// Spawns a Tokio task — returns immediately.
pub async fn start(pool: SqlitePool, app: AppHandle) {
    tokio::spawn(async move {
        // Run once at startup, then every 15 minutes.
        let mut interval = tokio::time::interval(Duration::from_secs(15 * 60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            run_cycle(&pool, &app).await;
        }
    });
}

/// One polling cycle: refresh urgency and notify on newly-urgent deadlines.
async fn run_cycle(pool: &SqlitePool, app: &AppHandle) {
    // Fetch pending deadlines before refresh so we can detect transitions
    let before = match snapshot_urgency(pool).await {
        Ok(v) => v,
        Err(e) => {
            log::warn!("deadline_watcher: snapshot failed: {e}");
            return;
        }
    };

    match crate::db::queries::deadlines::refresh_all_urgency(pool).await {
        Ok(n) if n > 0 => log::info!("deadline_watcher: refreshed urgency for {n} deadlines"),
        Ok(_) => {}
        Err(e) => {
            log::warn!("deadline_watcher: urgency refresh failed: {e}");
            return;
        }
    }

    let after = match snapshot_urgency(pool).await {
        Ok(v) => v,
        Err(e) => {
            log::warn!("deadline_watcher: post-refresh snapshot failed: {e}");
            return;
        }
    };

    // Fire notifications for deadlines that transitioned into Overdue or Critical
    for (id, new_urgency) in &after {
        let old_urgency = before.iter()
            .find(|(eid, _)| eid == id)
            .map(|(_, u)| u.as_str())
            .unwrap_or("Normal");

        if old_urgency == new_urgency {
            continue; // no change
        }

        match new_urgency.as_str() {
            "Overdue" => {
                if let Ok(Some(d)) = get_deadline_context(pool, id).await {
                    notify(app, "Deadline Overdue", &format!(
                        "{} — {} is now overdue", d.matter_title, d.event
                    ));
                }
            }
            "Critical" if old_urgency != "Overdue" => {
                if let Ok(Some(d)) = get_deadline_context(pool, id).await {
                    notify(app, "Deadline Critical — 3 Days", &format!(
                        "{} — {} due in ≤3 days", d.matter_title, d.event
                    ));
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn snapshot_urgency(pool: &SqlitePool) -> anyhow::Result<Vec<(String, String)>> {
    Ok(sqlx::query_as::<_, (String, String)>(
        "SELECT id, urgency FROM deadlines WHERE status = 'Pending'"
    )
    .fetch_all(pool)
    .await?)
}

struct DeadlineCtx {
    matter_title: String,
    event:        String,
}

async fn get_deadline_context(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<DeadlineCtx>> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT m.title, d.docketing_event
         FROM deadlines d JOIN matters m ON d.matter_id = m.id
         WHERE d.id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(matter_title, event)| DeadlineCtx { matter_title, event }))
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show();
}
