// Persist sync server — see specs/module-05-portal.md §12 and server/SERVER-RULES.md.
//
// "Its sole job: bridge the desktop SQLite (canonical) and the client-facing
//  surfaces. It does NOT contain business logic. It is a sync relay."
//
// So this crate persists what the desktop sends and hands back what clients
// have queued. Every decision about what may be shared was made by
// projection.rs on the desktop, before anything left the machine.
//
// AUTHENTICATION
//
// SERVER-RULES specifies mTLS with a client certificate pinned at build time.
// That is a deployment concern (TLS termination) and is NOT implemented in this
// binary — see the note on `require_client` below. The shared-secret check here
// is an application-layer second lock, not a replacement for mTLS.

mod mirror;
mod types;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;
use types::*;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    /// Shared secret the desktop presents. Required — the server refuses to
    /// start without one, so there is no silently-insecure default.
    client_token: Arc<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "persist_sync_server=info,tower_http=info".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL is required"))?;

    // Refuse to run without an explicit credential. A default here would be a
    // server that looks configured and is not.
    let client_token = std::env::var("SYNC_CLIENT_TOKEN").map_err(|_| {
        anyhow::anyhow!(
            "SYNC_CLIENT_TOKEN is required. It is the application-layer check; \
             production must ALSO terminate mTLS with the pinned desktop \
             certificate (server/SERVER-RULES.md)."
        )
    })?;
    if client_token.trim().len() < 32 {
        anyhow::bail!("SYNC_CLIENT_TOKEN must be at least 32 characters");
    }

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    let state = AppState { pool, client_token: Arc::new(client_token) };
    let app = router(state);

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("sync server listening on {addr}");

    axum::serve(listener, app).await?;
    Ok(())
}

fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/sync/push", post(push))
        .route("/sync/pull", get(pull))
        .route("/sync/ack", post(ack))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

/// Check the desktop's credential.
///
/// NOTE: this is NOT the mTLS check SERVER-RULES requires. Certificate pinning
/// happens at TLS termination and is a deployment task that this binary does not
/// perform. Treat this as defence in depth.
fn require_client(state: &AppState, headers: &HeaderMap) -> Result<(), StatusCode> {
    let presented = headers
        .get("x-persist-sync-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Length-independent compare so the response time does not leak the prefix.
    let expected = state.client_token.as_bytes();
    let given = presented.as_bytes();
    let equal = expected.len() == given.len()
        && expected.iter().zip(given).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0;

    if equal { Ok(()) } else { Err(StatusCode::UNAUTHORIZED) }
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.pool).await {
        Ok(_) => (StatusCode::OK, "ok"),
        Err(e) => {
            tracing::error!("health check failed: {e}");
            (StatusCode::SERVICE_UNAVAILABLE, "database unavailable")
        }
    }
}

/// Apply a batch of projected changes.
///
/// Each entry is applied independently: one malformed payload rejects that entry
/// and leaves the rest to land. Rejected entries stay queued on the desktop and
/// are retried, so nothing is lost by a partial failure.
async fn push(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<PushRequest>,
) -> Result<Json<PushResponse>, StatusCode> {
    require_client(&state, &headers)?;

    let mut out = PushResponse::default();

    for entry in req.entries {
        match apply(&state.pool, &entry).await {
            Ok(()) => out.accepted.push(entry.outbox_id),
            Err(e) => {
                tracing::warn!(
                    "rejected {} {} ({}): {e:#}",
                    entry.op, entry.entity_type, entry.entity_id
                );
                out.rejected.push(Rejection {
                    outbox_id: entry.outbox_id,
                    reason: format!("{e:#}"),
                });
            }
        }
    }

    tracing::info!("push: {} accepted, {} rejected", out.accepted.len(), out.rejected.len());
    Ok(Json(out))
}

async fn apply(pool: &PgPool, entry: &PushEntry) -> anyhow::Result<()> {
    if entry.op == "Delete" {
        mirror::delete_entity(pool, &entry.entity_type, &entry.entity_id).await?;
        return Ok(());
    }
    if entry.op != "Upsert" {
        anyhow::bail!("unknown op '{}'", entry.op);
    }

    let payload = entry
        .payload
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Upsert requires a payload"))?;

    match entry.entity_type.as_str() {
        "Client"     => mirror::upsert_client(pool, &serde_json::from_value(payload)?).await,
        "PortalUser" => mirror::upsert_portal_user(pool, &serde_json::from_value(payload)?).await,
        "Matter"     => mirror::upsert_matter(pool, &serde_json::from_value(payload)?).await,
        "Deadline"   => mirror::upsert_deadline(pool, &serde_json::from_value(payload)?).await,
        "IpAsset"    => mirror::upsert_ip_asset(pool, &serde_json::from_value(payload)?).await,
        "Invoice"    => mirror::upsert_invoice(pool, &serde_json::from_value(payload)?).await,
        other        => anyhow::bail!("unknown entity type '{other}'"),
    }
}

/// Client-authored items awaiting ingest. Only clean uploads are offered — the
/// desktop refuses anything else anyway, but there is no reason to hand it over.
async fn pull(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PullResponse>, StatusCode> {
    require_client(&state, &headers)?;

    let uploads = sqlx::query_as::<_, PendingUpload>(
        "SELECT id, client_id, portal_user_id, matter_id, filename, mime_type,
                file_size_bytes, object_key, sha256, scan_status
         FROM inbound.client_uploads
         WHERE status = 'Pending' AND scan_status = 'Clean'
         ORDER BY uploaded_at ASC
         LIMIT 100",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("pull uploads failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let disputes = sqlx::query_as::<_, PendingDispute>(
        "SELECT id, client_id, portal_user_id, invoice_id, reason
         FROM inbound.invoice_disputes
         WHERE status = 'Pending'
         ORDER BY raised_at ASC
         LIMIT 100",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("pull disputes failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(PullResponse { uploads, disputes }))
}

/// Mark inbound items handled, once the desktop has actually applied them.
async fn ack(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AckRequest>,
) -> Result<Json<AckResponse>, StatusCode> {
    require_client(&state, &headers)?;

    let mut marked = 0u64;

    for (ids, status) in [
        (&req.ingested_uploads, "Ingested"),
        (&req.rejected_uploads, "Rejected"),
    ] {
        if ids.is_empty() {
            continue;
        }
        let r = sqlx::query(
            "UPDATE inbound.client_uploads SET status = $1
             WHERE id = ANY($2) AND status = 'Pending'",
        )
        .bind(status)
        .bind(ids)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        marked += r.rows_affected();
    }

    if !req.ingested_disputes.is_empty() {
        let r = sqlx::query(
            "UPDATE inbound.invoice_disputes SET status = 'Ingested'
             WHERE id = ANY($1) AND status = 'Pending'",
        )
        .bind(&req.ingested_disputes)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        marked += r.rows_affected();
    }

    Ok(Json(AckResponse { marked }))
}
