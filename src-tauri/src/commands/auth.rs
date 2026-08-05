// Phase 1 Auth — Login, logout, session management.
//
// Authentication model:
//   - Local-only: no network, no OAuth. Passwords are bcrypt-hashed in the users table.
//   - A session is a row in `sessions` whose id (UUID v4) is the bearer token.
//     The token is held in the OS keychain (services/keychain.rs), never in SQLite.
//   - Sessions last 8 hours (queries::sessions::SESSION_TTL_HOURS) and survive an
//     app restart — resolves B01, which forced a re-login on every launch.
//   - AppState::session is a cache only. The DB is authoritative on every check.
//   - Rate limiting: 5 failed attempts for an email → 60s lockout, in memory
//     (per specs/auth-rbac.md §Security Rules — resets on restart by design).
//   - Default password for both attorneys: "persist2026" (set at first-run seed).

use crate::{db::queries::sessions as session_queries, db::queries::users as user_queries};
use crate::{AppState, SessionData};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Failed logins before the lockout engages.
const MAX_FAILED_ATTEMPTS: u32 = 5;
/// How long an email is locked out after exceeding MAX_FAILED_ATTEMPTS.
const LOCKOUT_DURATION: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------------------
// Rate limiting (in-memory, per email)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct AttemptState {
    pub failures:     u32,
    pub locked_until: Option<Instant>,
}

impl AttemptState {
    /// Remaining lockout, or None if the caller may attempt a login.
    fn lockout_remaining(&self) -> Option<Duration> {
        self.locked_until
            .and_then(|until| until.checked_duration_since(Instant::now()))
    }
}

// ---------------------------------------------------------------------------
// IPC types returned to Deck
// ---------------------------------------------------------------------------

/// What Deck knows about the current session — password_hash and vault paths never included.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub session_id: String,
    pub user_id:    String,
    pub name:       String,
    pub role:       String,
    pub email:      String,
    /// SQLite datetime string, UTC — when this session stops being valid.
    pub expires_at: String,
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
/// On success: creates a session row, stores the token in the OS keychain,
/// caches it in AppState, and returns the Session.
/// On failure: a generic error — never leaks which field was wrong.
#[tauri::command]
pub async fn login(
    input: LoginInput,
    state: tauri::State<'_, AppState>,
) -> Result<Session, String> {
    let email = input.email.trim().to_lowercase();

    // --- rate limit check ---------------------------------------------------
    {
        let attempts = state.login_attempts.lock().await;
        if let Some(entry) = attempts.get(&email) {
            if let Some(remaining) = entry.lockout_remaining() {
                return Err(format!(
                    "Too many failed attempts. Try again in {} seconds.",
                    remaining.as_secs() + 1
                ));
            }
        }
    }

    let pool = { state.db.lock().await.clone() };

    let user = user_queries::get_by_email(&pool, &email)
        .await
        .map_err(|e| {
            log::error!("login db error: {e}");
            "Authentication failed".to_string()
        })?;

    // bcrypt verify — constant-time comparison. Only run when the user exists.
    let valid = match &user {
        Some(u) => bcrypt::verify(&input.password, &u.password_hash).unwrap_or(false),
        None => false,
    };

    if !valid {
        record_failure(&state, &email).await;
        return Err("Invalid email or password".to_string());
    }

    let user = user.expect("checked above");

    // Successful login clears the failure counter for this email.
    state.login_attempts.lock().await.remove(&email);

    user_queries::touch_login(&pool, &user.id)
        .await
        .map_err(|e| e.to_string())?;

    // Drop any prior sessions for this user so a device only ever holds one
    // live token — logging in again elsewhere invalidates the old session.
    session_queries::delete_for_user(&pool, &user.id)
        .await
        .map_err(|e| e.to_string())?;

    let token = uuid::Uuid::new_v4().to_string();
    let row = session_queries::create(&pool, &token, &user.id)
        .await
        .map_err(|e| {
            log::error!("failed to create session: {e}");
            "Authentication failed".to_string()
        })?;

    state.keychain.store_session_token(&token);

    *state.session.lock().await = Some(SessionData {
        session_id: token.clone(),
        user_id:    user.id.clone(),
        name:       user.name.clone(),
        role:       user.role.clone(),
    });

    log::info!("Login: {} ({})", user.name, user.role);

    Ok(Session {
        session_id: token,
        user_id:    user.id,
        name:       user.name,
        role:       user.role,
        email:      user.email,
        expires_at: row.expires_at,
    })
}

/// Delete the session row, clear the keychain token and the in-memory cache.
/// After this, a restart lands on the login screen.
#[tauri::command]
pub async fn logout(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let token = {
        let guard = state.session.lock().await;
        guard.as_ref().map(|s| s.session_id.clone())
    }
    .or_else(|| state.keychain.load_session_token());

    if let Some(token) = token {
        let pool = { state.db.lock().await.clone() };
        if let Err(e) = session_queries::delete(&pool, &token).await {
            log::warn!("failed to delete session row on logout: {e}");
        }
    }

    state.keychain.clear_session_token();
    *state.session.lock().await = None;

    log::info!("Session cleared (logout)");
    Ok(())
}

/// Return the current session, or None if not logged in / expired.
/// Called by Deck on app start to decide whether to show the login screen.
///
/// Resolution order: in-memory cache → keychain token. Either way the session is
/// re-validated against the DB, so an expired or revoked session never restores.
#[tauri::command]
pub async fn get_session(state: tauri::State<'_, AppState>) -> Result<Option<Session>, String> {
    let cached = {
        let guard = state.session.lock().await;
        guard.as_ref().map(|s| s.session_id.clone())
    };

    let token = match cached.or_else(|| state.keychain.load_session_token()) {
        Some(t) => t,
        None => return Ok(None),
    };

    let pool = { state.db.lock().await.clone() };

    let found = session_queries::get_valid_with_user(&pool, &token)
        .await
        .map_err(|e| {
            log::error!("session lookup failed: {e}");
            "Session lookup failed".to_string()
        })?;

    let Some(s) = found else {
        // Expired or revoked — clean up so we don't retry this token forever.
        state.keychain.clear_session_token();
        *state.session.lock().await = None;
        return Ok(None);
    };

    // Record activity; expiry is only extended by refresh_session.
    if let Err(e) = session_queries::touch(&pool, &token).await {
        log::warn!("failed to touch session: {e}");
    }

    *state.session.lock().await = Some(SessionData {
        session_id: s.session_id.clone(),
        user_id:    s.user_id.clone(),
        name:       s.name.clone(),
        role:       s.role.clone(),
    });

    Ok(Some(Session {
        session_id: s.session_id,
        user_id:    s.user_id,
        name:       s.name,
        role:       s.role,
        email:      s.email,
        expires_at: s.expires_at,
    }))
}

/// Extend the current session by another 8 hours. Deck calls this on user
/// activity so a working attorney is never logged out mid-task.
/// Returns None if there is no longer a valid session to extend.
#[tauri::command]
pub async fn refresh_session(state: tauri::State<'_, AppState>) -> Result<Option<Session>, String> {
    let cached = {
        let guard = state.session.lock().await;
        guard.as_ref().map(|s| s.session_id.clone())
    };

    let token = match cached.or_else(|| state.keychain.load_session_token()) {
        Some(t) => t,
        None => return Ok(None),
    };

    let pool = { state.db.lock().await.clone() };

    // refresh() only touches sessions that are still valid.
    if session_queries::refresh(&pool, &token)
        .await
        .map_err(|e| e.to_string())?
        .is_none()
    {
        state.keychain.clear_session_token();
        *state.session.lock().await = None;
        return Ok(None);
    }

    let s = session_queries::get_valid_with_user(&pool, &token)
        .await
        .map_err(|e| e.to_string())?;

    Ok(s.map(|s| Session {
        session_id: s.session_id,
        user_id:    s.user_id,
        name:       s.name,
        role:       s.role,
        email:      s.email,
        expires_at: s.expires_at,
    }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Record a failed login and engage the lockout once the threshold is crossed.
async fn record_failure(state: &tauri::State<'_, AppState>, email: &str) {
    let mut attempts = state.login_attempts.lock().await;
    let entry = attempts.entry(email.to_string()).or_default();

    entry.failures += 1;
    if entry.failures >= MAX_FAILED_ATTEMPTS {
        entry.locked_until = Some(Instant::now() + LOCKOUT_DURATION);
        entry.failures = 0; // restart the count for the next window
        log::warn!("Login lockout engaged for {email} ({LOCKOUT_DURATION:?})");
    }
}

// ---------------------------------------------------------------------------
// Tests
//
// The commands themselves need a tauri::State, which cannot be constructed in a
// unit test, so these cover the rate-limit state machine — the piece with real
// logic that is not already covered by db::queries::sessions tests.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_attempt_state_is_not_locked() {
        let state = AttemptState::default();
        assert!(state.lockout_remaining().is_none());
    }

    #[test]
    fn future_lockout_reports_remaining_time() {
        let state = AttemptState {
            failures:     0,
            locked_until: Some(Instant::now() + Duration::from_secs(60)),
        };
        let remaining = state.lockout_remaining().expect("should be locked");
        assert!(remaining.as_secs() <= 60 && remaining.as_secs() >= 58);
    }

    #[test]
    fn elapsed_lockout_is_no_longer_locked() {
        let state = AttemptState {
            failures:     0,
            locked_until: Some(Instant::now() - Duration::from_secs(1)),
        };
        assert!(
            state.lockout_remaining().is_none(),
            "a lockout in the past must not block login"
        );
    }
}
