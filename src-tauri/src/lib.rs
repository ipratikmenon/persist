use std::collections::HashMap;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;
use bcrypt;

mod commands;
mod config;
mod storage;

/// Public so tests/sync_e2e.rs can drive the transport against a live server.
/// Nothing outside the crate links against Keel in production.
pub mod services;

pub mod db;
pub mod rbac;

use services::keychain::Keychain;

// ---------------------------------------------------------------------------
// App state shared across all Tauri commands
// ---------------------------------------------------------------------------

/// Cached view of the active session.
/// The authoritative record is the `sessions` table — this only avoids a DB
/// round trip on hot paths. Restored at startup from the keychain token (B01).
#[derive(Debug, Clone)]
pub struct SessionData {
    /// The session token — primary key of the `sessions` row.
    pub session_id: String,
    pub user_id:    String,
    pub name:       String,
    pub role:       String,
}

pub struct AppState {
    pub db:        Arc<Mutex<sqlx::SqlitePool>>,
    /// vault_key: placeholder [0u8;32] — Phase 1 Auth loads the real key from OS keychain
    pub vault_key: Arc<[u8; 32]>,
    /// vault_dir: directory where AES-256-GCM encrypted document files live
    pub vault_dir: std::path::PathBuf,
    /// session: cache of the active session; None when logged out
    pub session:   Arc<Mutex<Option<SessionData>>>,
    /// keychain: OS-native storage for the session token (survives restart)
    pub keychain:  Arc<Keychain>,
    /// login_attempts: per-email failure counters for rate limiting.
    /// In memory by design — resets on restart (specs/auth-rbac.md §Security Rules).
    pub login_attempts: Arc<Mutex<HashMap<String, commands::auth::AttemptState>>>,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // Resolve the OS-appropriate app data directory
            // macOS: ~/Library/Application Support/com.persist.app
            // Windows: %APPDATA%\com.persist.app
            let app_data_dir = app.path().app_data_dir()
                .expect("failed to resolve app data directory");
            std::fs::create_dir_all(&app_data_dir)
                .expect("failed to create app data directory");

            let db_path = app_data_dir.join("persist.db");
            let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

            // Open pool and run migrations synchronously before the window opens.
            // tauri::async_runtime::block_on drives the Tokio runtime Tauri owns.
            let pool = tauri::async_runtime::block_on(async {
                let pool = sqlx::SqlitePool::connect(&db_url)
                    .await
                    .expect("failed to open SQLite database");

                sqlx::migrate!("src/db/migrations")
                    .run(&pool)
                    .await
                    .expect("failed to apply database migrations");

                pool
            });

            // Vault key placeholder — Phase 1 Auth will load the real key from
            // the OS keychain via src-tauri/services/keychain.rs
            let vault_key = Arc::new([0u8; 32]);

            // Create vault directory: {app_data_dir}/vault/
            let vault_dir = app_data_dir.join("vault");
            std::fs::create_dir_all(&vault_dir)
                .expect("failed to create vault directory");

            // Seed the two firm attorneys on first run (empty users table).
            // Passwords default to "persist2026" — attorneys should change on first login.
            // bcrypt cost 12 — ~300ms, acceptable at startup.
            // Runs inside block_on because .setup() is not an async closure.
            tauri::async_runtime::block_on(async {
                let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
                    .fetch_one(&pool)
                    .await
                    .unwrap_or(0);

                if n == 0 {
                    let default_hash = bcrypt::hash("persist2026", 12)
                        .expect("bcrypt hash failed");

                    sqlx::query(
                        "INSERT INTO users (id, name, email, role, password_hash) VALUES
                         ('user-slm', 'Sree Lakshmi Menon', 'slm@persist.in', 'Partner', ?),
                         ('user-kt',  'Kajal Thakur',       'kt@persist.in',  'Associate', ?)",
                    )
                    .bind(&default_hash)
                    .bind(&default_hash)
                    .execute(&pool)
                    .await
                    .expect("failed to seed users");

                    log::info!("Seeded 2 default users (default password: persist2026)");
                }
            });

            // Clear sessions that expired while the app was closed, so the
            // sessions table does not grow without bound.
            tauri::async_runtime::block_on(async {
                match db::queries::sessions::delete_expired(&pool).await {
                    Ok(n) if n > 0 => log::info!("Cleared {n} expired session(s)"),
                    Ok(_) => {}
                    Err(e) => log::warn!("failed to clear expired sessions: {e}"),
                }
            });

            app.manage(AppState {
                db:        Arc::new(Mutex::new(pool.clone())),
                vault_key,
                vault_dir,
                // Populated lazily by get_session(), which validates the keychain
                // token against the DB on the first call after launch.
                session:   Arc::new(Mutex::new(None)),
                keychain:  Arc::new(Keychain::new(&app_data_dir)),
                login_attempts: Arc::new(Mutex::new(HashMap::new())),
            });

            // Start the deadline watcher background service.
            // Polls every 15 minutes, recalculates urgency, fires OS notifications.
            let watcher_handle = app.handle().clone();
            tauri::async_runtime::spawn(
                services::deadline_watcher::start(pool.clone(), watcher_handle)
            );

            // Start the abandonment watcher. Polls every 30 minutes and raises
            // the L1/L2/L3/L4 escalation ladder on statutory deadlines. A missed
            // statutory IP deadline is usually irreversible, so this must run
            // whether or not anyone is looking at the docket screen.
            let abandonment_handle = app.handle().clone();
            tauri::async_runtime::spawn(
                services::abandonment_watcher::start(pool, abandonment_handle)
            );

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // --- matters ---
            commands::matters::get_matter,
            commands::matters::list_matters,
            commands::matters::create_matter,
            commands::matters::update_matter,
            commands::matters::update_matter_status,
            commands::matters::close_matter,
            commands::matters::archive_matter,
            commands::matters::search_matters,
            commands::matters::assign_party,
            commands::matters::remove_party,
            // --- clients ---
            commands::matters::get_client,
            commands::matters::list_clients,
            commands::matters::create_client,
            commands::matters::update_client,
            // --- deadlines ---
            commands::deadlines::list_all_deadlines,
            commands::deadlines::list_deadlines,
            commands::deadlines::create_deadline,
            commands::deadlines::update_deadline,
            commands::deadlines::mark_deadline_complete,
            commands::deadlines::delete_deadline,
            commands::deadlines::get_statutory_templates,
            commands::deadlines::verify_deadline,
            commands::deadlines::list_unverified_deadlines,
            // --- IP assets (B02) ---
            commands::ip_assets::list_ip_assets,
            commands::ip_assets::get_ip_asset,
            commands::ip_assets::create_ip_asset,
            commands::ip_assets::update_ip_asset,
            commands::ip_assets::delete_ip_asset,
            commands::ip_assets::list_upcoming_renewals,
            // --- cascade + escalations ---
            commands::cascade::preview_cascade,
            commands::cascade::generate_cascade,
            commands::cascade::list_cascade_anchors,
            commands::cascade::list_escalations,
            commands::cascade::resolve_escalation,
            // --- documents ---
            commands::documents::list_documents,
            commands::documents::upload_document,
            commands::documents::get_document,
            commands::documents::export_document,
            commands::documents::delete_document,
            // --- AI ---
            commands::ai::ai_request,
            // --- billing ---
            commands::billing::get_firm_settings,
            commands::billing::update_firm_settings,
            commands::billing::create_time_entry,
            commands::billing::update_time_entry,
            commands::billing::delete_time_entry,
            commands::billing::list_time_entries,
            commands::billing::get_unbilled_summary,
            commands::billing::create_invoice,
            commands::billing::get_invoice,
            commands::billing::list_invoices,
            commands::billing::list_invoice_line_items,
            commands::billing::list_payments,
            commands::billing::update_invoice_status,
            commands::billing::record_payment,
            commands::billing::generate_invoice_pdf,
            // --- auth ---
            commands::auth::login,
            commands::auth::logout,
            commands::auth::get_session,
            commands::auth::refresh_session,
            // --- sync + client portal (M5) ---
            commands::sync::sync_status,
            commands::sync::trigger_sync,
            commands::sync::set_sync_enabled,
            commands::sync::set_sync_server,
            commands::sync::set_sync_token,
            commands::sync::invite_portal_user,
            commands::sync::list_portal_users,
            commands::sync::revoke_portal_user,
            commands::sync::share_document,
            commands::sync::unshare_document,
            commands::sync::set_deadline_client_visible,
            commands::sync::list_shared_documents,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Persist");
}
