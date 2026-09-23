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
mod object_store;
mod types;

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use object_store::ObjectStore;
use serde::{Deserialize, Serialize};
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
    /// `None` when no S3_* variable is set. The document-storage routes
    /// answer 503 in that case; everything else keeps working (see
    /// object_store::ObjectStore::from_env).
    s3: Option<ObjectStore>,
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

    let s3 = ObjectStore::from_env()?;
    if s3.is_none() {
        tracing::warn!(
            "object storage not configured (S3_* unset) — /sync/documents and \
             /sync/uploads/*/object will answer 503 until it is"
        );
    }

    let state = AppState { pool, client_token: Arc::new(client_token), s3 };
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
        .route("/sync/documents", post(upload_shared_document))
        .route("/sync/documents/*key", delete(delete_shared_document))
        .route("/sync/uploads/:id/object", get(download_pending_upload))
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
        "Document"   => mirror::upsert_document(pool, &serde_json::from_value(payload)?).await,
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

// ---------------------------------------------------------------------------
// Document storage — spec §12, §9. The bytes never touch SQLite; only the
// object key and a hash do, via the Document upsert on /sync/push.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct UploadDocumentQuery {
    /// The desktop computes this itself (documents/{client_id}/{document_id})
    /// so a re-share after an edit overwrites the same object rather than
    /// leaking an orphaned one under a fresh key.
    key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadDocumentResponse {
    object_key: String,
    size_bytes: usize,
}

/// Store the bytes of a document the desktop has decided to share. Returns
/// the key so the desktop can include it in the Document upsert it pushes
/// next — the mirror row and the object it points at are two separate
/// writes, and this one must land first (see DocumentPublic::object_key).
async fn upload_shared_document(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<UploadDocumentQuery>,
    body: Bytes,
) -> Result<Json<UploadDocumentResponse>, StatusCode> {
    require_client(&state, &headers)?;
    let Some(s3) = &state.s3 else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");

    let size_bytes = body.len();
    s3.put(&q.key, body.to_vec(), content_type).await.map_err(|e| {
        tracing::error!("shared document upload failed for {}: {e:#}", q.key);
        StatusCode::BAD_GATEWAY
    })?;

    Ok(Json(UploadDocumentResponse { object_key: q.key, size_bytes }))
}

/// Remove a shared document's object. Called when an attorney un-shares it —
/// the mirror row is removed by the matching Document Delete on /sync/push;
/// this removes the bytes. Deleting an already-gone key is not an error
/// (ObjectStore::delete treats 404 as success), so a retried unshare is safe.
async fn delete_shared_document(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Result<StatusCode, StatusCode> {
    require_client(&state, &headers)?;
    let Some(s3) = &state.s3 else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    s3.delete(&key).await.map_err(|e| {
        tracing::error!("shared document delete failed for {key}: {e:#}");
        StatusCode::BAD_GATEWAY
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// The desktop downloads a quarantined client upload to verify its hash,
/// clean its metadata, and ingest it into the vault — or refuse it. Only
/// `Pending` uploads are servable; once ingested or rejected there is
/// nothing left for the desktop to fetch.
async fn download_pending_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<(HeaderMap, Bytes), StatusCode> {
    require_client(&state, &headers)?;
    let Some(s3) = &state.s3 else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT object_key, mime_type FROM inbound.client_uploads
         WHERE id = $1 AND status = 'Pending'",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("looking up pending upload {id} failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let Some((object_key, mime_type)) = row else {
        return Err(StatusCode::NOT_FOUND);
    };

    let bytes = s3
        .get(&object_key)
        .await
        .map_err(|e| {
            tracing::error!("fetching quarantined upload {id} failed: {e:#}");
            StatusCode::BAD_GATEWAY
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut response_headers = HeaderMap::new();
    if let Ok(value) = mime_type.parse() {
        response_headers.insert(header::CONTENT_TYPE, value);
    }
    Ok((response_headers, Bytes::from(bytes)))
}

// ---------------------------------------------------------------------------
// Route-level tests
//
// Skip (rather than fail) without SYNC_SERVER_TEST_DATABASE_URL, the same
// convention portal/backend/tests/conftest.py uses for its PostgreSQL-backed
// suite — RLS and the mirror schema are the thing under test, and no mock
// would agree with them the way a real database does.
//
//     createdb persist_sync_server_test
//     for m in server/migrations/*.sql; do psql persist_sync_server_test -f "$m"; done
//     export SYNC_SERVER_TEST_DATABASE_URL=postgres://.../persist_sync_server_test
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::put as axum_put;
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn test_database_url() -> Option<String> {
        std::env::var("SYNC_SERVER_TEST_DATABASE_URL").ok()
    }

    fn unique_id(prefix: &str) -> String {
        let nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        format!("{prefix}-{nanos}")
    }

    // A bucket with no signature check at all — object_store.rs's own tests
    // already prove SigV4 is correct; these tests only need something that
    // actually stores bytes so upload_shared_document / delete_shared_document
    // / download_pending_upload have something real underneath them.
    #[derive(Clone, Default)]
    struct FakeBucket(Arc<Mutex<HashMap<String, Vec<u8>>>>);

    async fn fake_put(
        State(b): State<FakeBucket>,
        Path((_bucket, key)): Path<(String, String)>,
        body: Bytes,
    ) -> StatusCode {
        b.0.lock().unwrap().insert(key, body.to_vec());
        StatusCode::OK
    }

    async fn fake_get(
        State(b): State<FakeBucket>,
        Path((_bucket, key)): Path<(String, String)>,
    ) -> Result<Vec<u8>, StatusCode> {
        b.0.lock().unwrap().get(&key).cloned().ok_or(StatusCode::NOT_FOUND)
    }

    async fn fake_delete(
        State(b): State<FakeBucket>,
        Path((_bucket, key)): Path<(String, String)>,
    ) -> StatusCode {
        b.0.lock().unwrap().remove(&key);
        StatusCode::NO_CONTENT
    }

    async fn spawn_fake_bucket() -> String {
        let app = Router::new()
            .route(
                "/:bucket/*key",
                axum_put(fake_put).get(fake_get).delete(fake_delete),
            )
            .with_state(FakeBucket::default());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    const TEST_TOKEN: &str = "test-sync-client-token-0123456789abcdef";

    async fn spawn_server(pool: PgPool, s3: Option<ObjectStore>) -> String {
        let state = AppState { pool, client_token: Arc::new(TEST_TOKEN.to_string()), s3 };
        let app = router(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    macro_rules! skip_without_test_db {
        () => {
            match test_database_url() {
                Some(url) => url,
                None => {
                    eprintln!("skipping: set SYNC_SERVER_TEST_DATABASE_URL to run");
                    return;
                }
            }
        };
    }

    #[tokio::test]
    async fn a_shared_document_can_be_uploaded_downloaded_and_deleted() {
        let database_url = skip_without_test_db!();
        let pool = PgPoolOptions::new().max_connections(5).connect(&database_url).await.unwrap();
        let bucket_url = spawn_fake_bucket().await;
        let s3 = ObjectStore::new_for_test(&bucket_url, "test-bucket");
        let base = spawn_server(pool, Some(s3)).await;
        let http = reqwest::Client::new();

        let key = unique_id("documents/test-client/doc");
        let uploaded = http
            .post(format!("{base}/sync/documents"))
            .query(&[("key", key.as_str())])
            .header("x-persist-sync-token", TEST_TOKEN)
            .header("content-type", "application/pdf")
            .body(b"%PDF-1.4 shared document bytes".to_vec())
            .send()
            .await
            .unwrap();
        assert_eq!(uploaded.status(), StatusCode::OK);
        let body: serde_json::Value = uploaded.json().await.unwrap();
        assert_eq!(body["objectKey"], key.as_str());
        assert_eq!(body["sizeBytes"], b"%PDF-1.4 shared document bytes".len());

        let deleted = http
            .delete(format!("{base}/sync/documents/{key}"))
            .header("x-persist-sync-token", TEST_TOKEN)
            .send()
            .await
            .unwrap();
        assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn uploading_a_shared_document_without_the_sync_token_is_refused() {
        let database_url = skip_without_test_db!();
        let pool = PgPoolOptions::new().max_connections(5).connect(&database_url).await.unwrap();
        let bucket_url = spawn_fake_bucket().await;
        let s3 = ObjectStore::new_for_test(&bucket_url, "test-bucket");
        let base = spawn_server(pool, Some(s3)).await;
        let http = reqwest::Client::new();

        let resp = http
            .post(format!("{base}/sync/documents"))
            .query(&[("key", "documents/test-client/should-not-land")])
            .body(b"x".to_vec())
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn document_routes_answer_503_when_object_storage_is_not_configured() {
        let database_url = skip_without_test_db!();
        let pool = PgPoolOptions::new().max_connections(5).connect(&database_url).await.unwrap();
        let base = spawn_server(pool, None).await;
        let http = reqwest::Client::new();

        let resp = http
            .post(format!("{base}/sync/documents"))
            .query(&[("key", "documents/test-client/whatever")])
            .header("x-persist-sync-token", TEST_TOKEN)
            .body(b"x".to_vec())
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    async fn insert_test_client(pool: &PgPool, client_id: &str) {
        sqlx::query("INSERT INTO mirror.clients (id, name) VALUES ($1, 'Test Client')")
            .bind(client_id)
            .execute(pool)
            .await
            .unwrap();
    }

    /// Client uploads carry FKs to both mirror.clients and mirror.portal_users.
    async fn insert_test_portal_user(pool: &PgPool, client_id: &str, portal_user_id: &str) {
        sqlx::query(
            "INSERT INTO mirror.portal_users (id, client_id, full_name, email, status)
             VALUES ($1, $2, 'Test User', $3, 'Active')",
        )
        .bind(portal_user_id)
        .bind(client_id)
        .bind(format!("{portal_user_id}@client.test"))
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn a_pending_uploads_bytes_can_be_downloaded_by_the_desktop() {
        let database_url = skip_without_test_db!();
        let pool = PgPoolOptions::new().max_connections(5).connect(&database_url).await.unwrap();
        let bucket_url = spawn_fake_bucket().await;
        let s3 = ObjectStore::new_for_test(&bucket_url, "test-bucket");

        let client_id = unique_id("client");
        insert_test_client(&pool, &client_id).await;
        let portal_user_id = unique_id("portal-user");
        insert_test_portal_user(&pool, &client_id, &portal_user_id).await;

        let upload_id = unique_id("upload");
        let object_key = unique_id("quarantine/test-client/upload");
        s3.put(&object_key, b"%PDF-1.4 client evidence".to_vec(), "application/pdf")
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO inbound.client_uploads
                 (id, client_id, portal_user_id, filename, mime_type,
                  file_size_bytes, object_key, sha256, scan_status, status)
             VALUES ($1, $2, $3, 'evidence.pdf',
                     'application/pdf', 24, $4, 'deadbeef', 'Clean', 'Pending')",
        )
        .bind(&upload_id)
        .bind(&client_id)
        .bind(&portal_user_id)
        .bind(&object_key)
        .execute(&pool)
        .await
        .unwrap();

        let base = spawn_server(pool, Some(s3)).await;
        let http = reqwest::Client::new();

        let resp = http
            .get(format!("{base}/sync/uploads/{upload_id}/object"))
            .header("x-persist-sync-token", TEST_TOKEN)
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.bytes().await.unwrap().as_ref(), b"%PDF-1.4 client evidence");
    }

    #[tokio::test]
    async fn an_already_ingested_uploads_object_route_404s() {
        // Only Pending uploads are servable — an Ingested or Rejected upload
        // has nothing left for the desktop to fetch, and must not appear to
        // succeed with stale bytes.
        let database_url = skip_without_test_db!();
        let pool = PgPoolOptions::new().max_connections(5).connect(&database_url).await.unwrap();

        let client_id = unique_id("client");
        insert_test_client(&pool, &client_id).await;
        let portal_user_id = unique_id("portal-user");
        insert_test_portal_user(&pool, &client_id, &portal_user_id).await;

        let upload_id = unique_id("upload-ingested");
        sqlx::query(
            "INSERT INTO inbound.client_uploads
                 (id, client_id, portal_user_id, filename, mime_type,
                  file_size_bytes, object_key, sha256, scan_status, status)
             VALUES ($1, $2, $3, 'evidence.pdf',
                     'application/pdf', 24, 'irrelevant-key', 'deadbeef', 'Clean', 'Ingested')",
        )
        .bind(&upload_id)
        .bind(&client_id)
        .bind(&portal_user_id)
        .execute(&pool)
        .await
        .unwrap();

        // A configured store, so this exercises the "row exists but isn't
        // Pending" 404 specifically — not the separate "no bucket at all"
        // 503 the previous test already covers.
        let bucket_url = spawn_fake_bucket().await;
        let s3 = ObjectStore::new_for_test(&bucket_url, "test-bucket");
        let base = spawn_server(pool, Some(s3)).await;
        let http = reqwest::Client::new();

        let resp = http
            .get(format!("{base}/sync/uploads/{upload_id}/object"))
            .header("x-persist-sync-token", TEST_TOKEN)
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn pushing_a_document_upsert_lands_in_the_mirror_with_its_object_key() {
        // Proves the other half of the loop: once bytes are in the bucket
        // (previous test), the metadata upsert this route accepts is what
        // makes the document visible to the portal at all.
        let database_url = skip_without_test_db!();
        let pool = PgPoolOptions::new().max_connections(5).connect(&database_url).await.unwrap();
        let base = spawn_server(pool.clone(), None).await;
        let http = reqwest::Client::new();

        let client_id = unique_id("client");
        let matter_id = unique_id("matter");
        let doc_id = unique_id("doc");
        let object_key = format!("documents/{client_id}/{doc_id}");

        insert_test_client(&pool, &client_id).await;
        sqlx::query(
            "INSERT INTO mirror.matters_public
                 (id, client_id, title, matter_type, status, opened_date)
             VALUES ($1, $2, 'Test Matter', 'Trademark', 'Active', CURRENT_DATE)",
        )
        .bind(&matter_id)
        .bind(&client_id)
        .execute(&pool)
        .await
        .unwrap();

        let push_body = serde_json::json!({
            "entries": [{
                "outboxId": "outbox-1",
                "entityType": "Document",
                "entityId": doc_id,
                "op": "Upsert",
                "payload": {
                    "id": doc_id,
                    "clientId": client_id,
                    "matterId": matter_id,
                    "filename": "Examination-Report.pdf",
                    "category": "Official",
                    "mimeType": "application/pdf",
                    "fileSizeBytes": 1024,
                    "version": 1,
                    "description": null,
                    "objectKey": object_key,
                    "sha256": "abc123",
                }
            }]
        });

        let resp = http
            .post(format!("{base}/sync/push"))
            .header("x-persist-sync-token", TEST_TOKEN)
            .json(&push_body)
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["rejected"].as_array().unwrap().len(), 0, "{body:?}");
        assert_eq!(body["accepted"].as_array().unwrap(), &["outbox-1"]);

        let row: (String,) = sqlx::query_as(
            "SELECT object_key FROM mirror.documents_shared WHERE id = $1",
        )
        .bind(&doc_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, object_key);
    }
}
