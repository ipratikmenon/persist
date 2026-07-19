// Phase 1 Auth — Login, logout, session management.
//
// Authentication model:
//   - Local-only: no network, no OAuth. Passwords are bcrypt-hashed in the users table.
//   - Session lives in AppState::session (in-memory Arc<Mutex<Option<SessionData>>>).
//   - Restarting the app always clears the session — re-authentication required.
//   - Default password for both attorneys: "persist2026" (set at first-run seed).

use crate::{db::queries::users as user_queries, AppState, SessionData};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// IPC types returned to Deck
// ---------------------------------------------------------------------------

/// What Deck knows about the current user — vault_path, password_hash, etc. never included.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub user_id: String,
    pub name:    String,
    pub role:    String,
    pub email:   String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginInput {
    pub email:    String,
    pub password: String,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Authenticate with email + password.
/// On success: sets AppState::session and returns Session.
/// On failure: returns an error string (generic — never leaks which field was wrong).
#[tauri::command]
pub async fn login(
    input: LoginInput,
    state: tauri::State<'_, AppState>,
) -> Result<Session, String> {
    let email = input.email.trim().to_lowercase();

    let pool = state.db.lock().await;
    let user = user_queries::get_by_email(&pool, &email)
        .await
        .map_err(|e| {
            log::error!("login db error: {e}");
            "Authentication failed".to_string()
        })?
        .ok_or_else(|| "Invalid email or password".to_string())?;

    // bcrypt verify — constant-time comparison
    let valid = bcrypt::verify(&input.password, &user.password_hash)
        .map_err(|_| "Authentication failed".to_string())?;

    if !valid {
        return Err("Invalid email or password".to_string());
    }

    // Stamp last_login_at
    user_queries::touch_login(&pool, &user.id)
        .await
        .map_err(|e| e.to_string())?;

    // Release pool lock before acquiring session lock.
    drop(pool);

    let session_data = SessionData {
        user_id: user.id.clone(),
        name:    user.name.clone(),
        role:    user.role.clone(),
    };

    *state.session.lock().await = Some(session_data);

    log::info!("Login: {} ({})", user.name, user.role);

    Ok(Session {
        user_id: user.id,
        name:    user.name,
        role:    user.role,
        email:   user.email,
    })
}

/// Clear the in-memory session. The DB is untouched — no token to invalidate.
#[tauri::command]
pub async fn logout(state: tauri::State<'_, AppState>) -> Result<(), String> {
    *state.session.lock().await = None;
    log::info!("Session cleared (logout)");
    Ok(())
}

/// Return the current session, or None if not logged in.
/// Called by Deck on app start to determine whether to show the login screen.
#[tauri::command]
pub async fn get_session(state: tauri::State<'_, AppState>) -> Result<Option<Session>, String> {
    let guard = state.session.lock().await;
    let session = guard.as_ref().map(|s| {
        // We don't store email in SessionData — look it up lazily if needed.
        // For now: return what we have; email can be fetched via a separate query if required.
        Session {
            user_id: s.user_id.clone(),
            name:    s.name.clone(),
            role:    s.role.clone(),
            email:   String::new(), // populated on login; empty on restored session check
        }
    });
    Ok(session)
}
