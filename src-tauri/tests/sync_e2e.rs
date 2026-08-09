// End-to-end sync test — Keel's transport against a live sync server.
//
// The unit tests in transport.rs prove that a withheld row produces no wire
// entry. They cannot prove that what we do send is something the server will
// take, because they never open a socket. This test does, which is the only way
// to catch the failure that actually matters: a field name that drifted between
// src-tauri/src/services/sync_engine/transport.rs and server/src/types.rs.
//
// It is skipped unless a server is pointed at, so `cargo test` stays green on a
// machine with no PostgreSQL:
//
//   createdb persist_sync_e2e
//   psql persist_sync_e2e -f server/migrations/0001_mirror.sql
//   psql persist_sync_e2e -f server/migrations/0002_rls.sql
//   SYNC_CLIENT_TOKEN=<32+ chars> DATABASE_URL=... cargo run -p persist-sync-server &
//
//   PERSIST_SYNC_E2E_URL=http://127.0.0.1:8787 \
//   PERSIST_SYNC_E2E_TOKEN=<same token> \
//   cargo test --test sync_e2e -- --nocapture

use persist_lib::services::sync_engine::{self, transport, EntityType, Op};
use sqlx::SqlitePool;

/// Returns (url, token), or None when the test should be skipped.
fn server() -> Option<(String, String)> {
    let url = std::env::var("PERSIST_SYNC_E2E_URL").ok()?;
    let token = std::env::var("PERSIST_SYNC_E2E_TOKEN").ok()?;
    Some((url, token))
}

async fn seeded_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();

    sqlx::query("INSERT INTO clients (id, name) VALUES ('e2e-client','Petalveda Botanicals')")
        .execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO users (id, name, email, role, password_hash)
         VALUES ('e2e-user','Sree Lakshmi Menon','slm@persistas.in','Partner','x')")
        .execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO matters (id, client_id, title, matter_type, opened_date,
                              responsible_partner_id, internal_notes)
         VALUES ('P&P-2026-TM-9001','e2e-client','PETALVEDA','Trademark', date('now'),
                 'e2e-user','INTERNAL_LEAK prior art is weak')")
        .execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO deadlines (id, matter_id, docketing_event, event_type, due_date,
                                is_client_visible, notes)
         VALUES ('e2e-dl-1','P&P-2026-TM-9001','Reply to Examination Report',
                 'Statutory','2026-11-30', 1, 'NOTES_LEAK do not disclose')")
        .execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO invoices (id, client_id, status, invoice_date, created_by,
                               subtotal, cgst_amount, sgst_amount, total_with_tax)
         VALUES ('INV-2026-9001','e2e-client','Sent','2026-08-01','e2e-user',
                 70000.01, 6300.0, 6300.0, 82600.01)")
        .execute(&pool).await.unwrap();

    pool
}

#[tokio::test]
async fn the_server_accepts_what_the_desktop_projects() {
    let Some((url, token)) = server() else {
        eprintln!("skipping: set PERSIST_SYNC_E2E_URL and PERSIST_SYNC_E2E_TOKEN to run");
        return;
    };

    let pool = seeded_pool().await;

    // Queued deliberately out of order: the client is enqueued last, but every
    // mirror table has a foreign key to it, so the push must still send it first.
    sync_engine::enqueue(&pool, EntityType::Matter, "P&P-2026-TM-9001", Op::Upsert).await.unwrap();
    sync_engine::enqueue(&pool, EntityType::Deadline, "e2e-dl-1", Op::Upsert).await.unwrap();
    sync_engine::enqueue(&pool, EntityType::Invoice, "INV-2026-9001", Op::Upsert).await.unwrap();
    sync_engine::enqueue(&pool, EntityType::Client, "e2e-client", Op::Upsert).await.unwrap();

    let out = transport::push_once(&pool, &url, &token).await.expect("push");

    assert_eq!(out.sent, 4, "four rows should have been built");
    assert_eq!(
        out.rejected, 0,
        "server rejected a projected row — the wire contract has drifted: {:?}",
        out.first_error
    );
    assert_eq!(out.accepted, 4);

    // Accepted entries are cleared; nothing should be left waiting.
    let left = sync_engine::pending_count(&pool).await.unwrap();
    assert_eq!(left, 0, "accepted entries must not stay queued");
}

#[tokio::test]
async fn a_wrong_token_is_refused_and_loses_nothing() {
    let Some((url, _)) = server() else {
        eprintln!("skipping: set PERSIST_SYNC_E2E_URL and PERSIST_SYNC_E2E_TOKEN to run");
        return;
    };

    let pool = seeded_pool().await;
    sync_engine::enqueue(&pool, EntityType::Matter, "P&P-2026-TM-9001", Op::Upsert).await.unwrap();

    let result = transport::push_once(&pool, &url, "not-the-right-token-not-the-right-token").await;
    assert!(result.is_err(), "a bad token must fail the push");

    // The change is the firm's work. A rejected push must never eat it.
    let left = sync_engine::pending_count(&pool).await.unwrap();
    assert_eq!(left, 1, "a refused push must leave the change queued");
}

#[tokio::test]
async fn a_tombstone_removes_the_mirror_row() {
    let Some((url, token)) = server() else {
        eprintln!("skipping: set PERSIST_SYNC_E2E_URL and PERSIST_SYNC_E2E_TOKEN to run");
        return;
    };

    let pool = seeded_pool().await;

    sync_engine::enqueue(&pool, EntityType::Client, "e2e-client", Op::Upsert).await.unwrap();
    sync_engine::enqueue(&pool, EntityType::Matter, "P&P-2026-TM-9001", Op::Upsert).await.unwrap();
    sync_engine::enqueue(&pool, EntityType::Deadline, "e2e-dl-1", Op::Upsert).await.unwrap();
    transport::push_once(&pool, &url, &token).await.expect("push upsert");

    // Hiding it from the client must withdraw it, not merely stop refreshing it.
    sync_engine::enqueue(&pool, EntityType::Deadline, "e2e-dl-1", Op::Delete).await.unwrap();
    let out = transport::push_once(&pool, &url, &token).await.expect("push tombstone");

    assert_eq!(out.accepted, 1, "tombstone was not accepted: {:?}", out.first_error);
}

#[tokio::test]
async fn pull_returns_a_well_formed_response() {
    let Some((url, token)) = server() else {
        eprintln!("skipping: set PERSIST_SYNC_E2E_URL and PERSIST_SYNC_E2E_TOKEN to run");
        return;
    };

    // Decoding is the assertion: a field the server renamed shows up here as a
    // deserialisation error, not as silently missing client uploads.
    let pulled = transport::pull_once(&url, &token).await.expect("pull");
    eprintln!(
        "pull: {} upload(s), {} dispute(s)",
        pulled.uploads.len(),
        pulled.disputes.len()
    );

    for u in &pulled.uploads {
        assert_ne!(
            u.scan_status, "Infected",
            "the server must never offer an infected upload for ingest"
        );
    }
}
