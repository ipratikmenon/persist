// Phase 1 Module 2 extended — cascade generation and abandonment escalations.
// specs/module-02-docketing.md §2.9, §2.11
//
// Generation is two-step by design: preview, then commit. An attorney sees the
// whole chain — with the template's last_verified date — before any deadline
// exists. Silently creating twenty annuity deadlines would be worse than useless
// if the anchor date were wrong.

use crate::db::queries::ip_assets as ip_asset_queries;
use crate::services::cascade_engine::{self, CascadeAnchor, CascadePreview};
use crate::AppState;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// IPC types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Escalation {
    pub id:                String,
    pub deadline_id:       String,
    pub escalation_level:  i64,
    pub triggered_at:      String,
    pub resolution_action: Option<String>,
    pub resolved_at:       Option<String>,
    pub resolved_by:       Option<String>,
    // Denormalised for the dashboard.
    pub docketing_event:   String,
    pub due_date:          String,
    pub matter_id:         String,
    pub matter_title:      String,
}

#[derive(Debug, sqlx::FromRow)]
struct EscalationRow {
    id:                String,
    deadline_id:       String,
    escalation_level:  i64,
    triggered_at:      String,
    resolution_action: Option<String>,
    resolved_at:       Option<String>,
    resolved_by:       Option<String>,
    docketing_event:   String,
    due_date:          String,
    matter_id:         String,
    matter_title:      String,
}

impl From<EscalationRow> for Escalation {
    fn from(r: EscalationRow) -> Self {
        Escalation {
            id: r.id,
            deadline_id: r.deadline_id,
            escalation_level: r.escalation_level,
            triggered_at: r.triggered_at,
            resolution_action: r.resolution_action,
            resolved_at: r.resolved_at,
            resolved_by: r.resolved_by,
            docketing_event: r.docketing_event,
            due_date: r.due_date,
            matter_id: r.matter_id,
            matter_title: r.matter_title,
        }
    }
}

// ---------------------------------------------------------------------------
// Commands — cascade
// ---------------------------------------------------------------------------

/// Show the chain an anchor would produce, without writing anything.
#[tauri::command]
pub async fn preview_cascade(
    anchor: CascadeAnchor,
    state: tauri::State<'_, AppState>,
) -> Result<CascadePreview, String> {
    let pool = { state.db.lock().await.clone() };

    // A cascade is always anchored to an IP asset: the statutory chain depends
    // on the right's type and jurisdiction, which the matter alone does not fix.
    // (spec §Business Rules: "generate_cascade requires an ip_asset_id".)
    let asset = ip_asset_queries::get(&pool, &anchor.ip_asset_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "IP asset {} not found. A cascade must be anchored to an asset.",
                anchor.ip_asset_id
            )
        })?;

    if asset.matter_id != anchor.matter_id {
        return Err("IP asset does not belong to that matter".to_string());
    }

    cascade_engine::preview(&pool, &anchor, &asset.asset_type, &asset.jurisdiction)
        .await
        .map_err(|e| e.to_string())
}

/// Generate and persist the chain. Returns the number of deadlines created.
///
/// Re-running the same anchor is refused rather than silently duplicating:
/// twenty duplicate annuities would be a genuine mess to unpick.
#[tauri::command]
pub async fn generate_cascade(
    anchor: CascadeAnchor,
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    let preview = preview_cascade(anchor.clone(), state.clone()).await?;
    let pool = { state.db.lock().await.clone() };

    let already: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deadlines
         WHERE ip_asset_id = ? AND cascade_template_id = ?",
    )
    .bind(&anchor.ip_asset_id)
    .bind(&preview.template_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| e.to_string())?;

    if already > 0 {
        return Err(format!(
            "This cascade has already been generated for that asset ({already} deadline(s)). \
             Delete them first if you need to regenerate."
        ));
    }

    // The root ties the chain together so a later date correction can find it.
    let root_id = Uuid::new_v4().to_string();
    let mut created = 0usize;

    for d in &preview.deadlines {
        let urgency = crate::db::queries::deadlines::urgency_for(&d.due_date, "Pending");

        sqlx::query(
            "INSERT INTO deadlines
                 (id, matter_id, ip_asset_id, docketing_event, event_type, due_date,
                  urgency, notes, is_client_visible, cascade_root_id, cascade_template_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&anchor.matter_id)
        .bind(&anchor.ip_asset_id)
        .bind(&d.docketing_event)
        .bind(&d.event_type)
        .bind(&d.due_date)
        .bind(urgency)
        .bind(&d.notes)
        .bind(i64::from(d.is_client_visible))
        .bind(&root_id)
        .bind(&preview.template_id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

        created += 1;
    }

    log::info!(
        "Generated cascade '{}' for asset {}: {created} deadline(s)",
        anchor.event_type, anchor.ip_asset_id
    );

    Ok(created)
}

/// Anchor event types that have a template, for the UI picker.
#[tauri::command]
pub async fn list_cascade_anchors(
    ip_type: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let pool = { state.db.lock().await.clone() };
    sqlx::query_scalar::<_, String>(
        "SELECT anchor_event_type FROM cascade_templates
         WHERE ip_type = ? ORDER BY anchor_event_type",
    )
    .bind(&ip_type)
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Commands — escalations
// ---------------------------------------------------------------------------

/// Open (unresolved) escalations, most severe and most overdue first.
#[tauri::command]
pub async fn list_escalations(
    include_resolved: Option<bool>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Escalation>, String> {
    let pool = { state.db.lock().await.clone() };
    let include_resolved = include_resolved.unwrap_or(false);

    let sql = if include_resolved {
        "SELECT e.id, e.deadline_id, e.escalation_level, e.triggered_at,
                e.resolution_action, e.resolved_at, e.resolved_by,
                d.docketing_event, d.due_date, d.matter_id, m.title AS matter_title
         FROM deadline_escalations e
         JOIN deadlines d ON d.id = e.deadline_id
         JOIN matters   m ON m.id = d.matter_id
         ORDER BY e.escalation_level DESC, d.due_date ASC"
    } else {
        "SELECT e.id, e.deadline_id, e.escalation_level, e.triggered_at,
                e.resolution_action, e.resolved_at, e.resolved_by,
                d.docketing_event, d.due_date, d.matter_id, m.title AS matter_title
         FROM deadline_escalations e
         JOIN deadlines d ON d.id = e.deadline_id
         JOIN matters   m ON m.id = d.matter_id
         WHERE e.resolved_at IS NULL
         ORDER BY e.escalation_level DESC, d.due_date ASC"
    };

    let rows = sqlx::query_as::<_, EscalationRow>(sql)
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Record what was done about an escalation. Does not change the deadline —
/// completing the work is a separate, deliberate action.
#[tauri::command]
pub async fn resolve_escalation(
    id: String,
    action: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    if action.trim().is_empty() {
        return Err("Describe what was done — an empty resolution is not a record".to_string());
    }

    let user_id = {
        let guard = state.session.lock().await;
        guard.as_ref().map(|s| s.user_id.clone())
    }
    .ok_or_else(|| "Not signed in".to_string())?;

    let pool = { state.db.lock().await.clone() };

    let result = sqlx::query(
        "UPDATE deadline_escalations
         SET resolution_action = ?, resolved_at = datetime('now'), resolved_by = ?
         WHERE id = ? AND resolved_at IS NULL",
    )
    .bind(action.trim())
    .bind(&user_id)
    .bind(&id)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err(format!("Escalation {id} not found, or already resolved"));
    }
    Ok(())
}
