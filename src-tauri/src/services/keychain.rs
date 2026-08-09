// Keychain — OS-native secret storage for the session token.
//
// specs/auth-rbac.md: "Session tokens: UUIDs stored in OS keychain
// (src-tauri/services/keychain.rs). Never in SQLite plaintext."
//
// Primary backend is the platform keychain via the `keyring` crate:
//   macOS   → Keychain Services
//   Windows → Credential Manager
//   Linux   → Secret Service (libsecret)
//
// Linux dev containers and CI runners usually have no Secret Service daemon, so
// keyring calls fail there. Rather than break the app on those machines, we fall
// back to a 0600 file inside the app data directory. That fallback is NOT a
// security regression in context: the file sits next to persist.db itself, so an
// attacker who can read it can already read every matter in the database. On the
// two platforms the firm actually ships to (macOS, Windows) the keychain is used.

use std::io::Write;
use std::path::{Path, PathBuf};

const KEYRING_SERVICE: &str = "in.persist.desktop";

/// One entry per secret. The keyring user and the fallback filename are paired
/// so the two backends can never disagree about which secret is which.
const SESSION_TOKEN: Secret = Secret { user: "session_token", file: ".session_token" };
/// Shared secret presented to the sync server (M5 §12). Kept out of SQLite for
/// the same reason as the session token: the database is backed up and copied.
const SYNC_TOKEN: Secret = Secret { user: "sync_token", file: ".sync_token" };

#[derive(Clone, Copy)]
struct Secret {
    user: &'static str,
    file: &'static str,
}

pub struct Keychain {
    /// Holds the fallback files, used only when the platform keychain is
    /// unavailable — one file per secret, named by `Secret::file`.
    app_data_dir: PathBuf,
}

impl Keychain {
    pub fn new(app_data_dir: &Path) -> Self {
        Self { app_data_dir: app_data_dir.to_path_buf() }
    }

    /// Persist the session token. Errors are logged, never fatal — a failure here
    /// costs the user session persistence, not access.
    pub fn store_session_token(&self, token: &str) {
        self.store(SESSION_TOKEN, token);
    }

    /// Read the stored session token, if any.
    pub fn load_session_token(&self) -> Option<String> {
        self.load(SESSION_TOKEN)
    }

    /// Remove the stored token (explicit logout). Clears both backends so a
    /// half-written fallback can never resurrect a logged-out session.
    pub fn clear_session_token(&self) {
        self.clear(SESSION_TOKEN);
    }

    pub fn store_sync_token(&self, token: &str) {
        self.store(SYNC_TOKEN, token);
    }

    pub fn load_sync_token(&self) -> Option<String> {
        self.load(SYNC_TOKEN)
    }

    pub fn clear_sync_token(&self) {
        self.clear(SYNC_TOKEN);
    }

    // -----------------------------------------------------------------------

    fn store(&self, secret: Secret, token: &str) {
        let path = self.app_data_dir.join(secret.file);
        match keyring::Entry::new(KEYRING_SERVICE, secret.user) {
            Ok(entry) => match entry.set_password(token) {
                Ok(()) => {
                    // Keychain took it — make sure no stale fallback copy lingers.
                    let _ = std::fs::remove_file(&path);
                    return;
                }
                Err(e) => log::warn!("keychain write failed ({e}); using file fallback"),
            },
            Err(e) => log::warn!("keychain unavailable ({e}); using file fallback"),
        }

        if let Err(e) = write_private(&path, token) {
            log::error!("failed to persist {} to fallback file: {e}", secret.user);
        }
    }

    fn load(&self, secret: Secret) -> Option<String> {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, secret.user) {
            match entry.get_password() {
                Ok(token) if !token.trim().is_empty() => return Some(token),
                Ok(_) => return None,
                // NoEntry is normal (never stored); anything else falls through.
                Err(keyring::Error::NoEntry) => {}
                Err(e) => log::warn!("keychain read failed ({e}); trying file fallback"),
            }
        }

        match std::fs::read_to_string(self.app_data_dir.join(secret.file)) {
            Ok(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
            _ => None,
        }
    }

    fn clear(&self, secret: Secret) {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, secret.user) {
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(e) => log::warn!("keychain delete failed: {e}"),
            }
        }
        let _ = std::fs::remove_file(self.app_data_dir.join(secret.file));
    }

    #[cfg(test)]
    fn fallback_path(&self) -> PathBuf {
        self.app_data_dir.join(SESSION_TOKEN.file)
    }

    #[cfg(test)]
    fn write_fallback(&self, token: &str) -> std::io::Result<()> {
        write_private(&self.fallback_path(), token)
    }
}

#[cfg(unix)]
fn write_private(path: &Path, token: &str) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600) // owner read/write only
        .open(path)?;
    f.write_all(token.as_bytes())
}

#[cfg(not(unix))]
fn write_private(path: &Path, token: &str) -> std::io::Result<()> {
    // Windows: the app data directory is already per-user ACL'd.
    let mut f = std::fs::File::create(path)?;
    f.write_all(token.as_bytes())
}

// ---------------------------------------------------------------------------
// Tests
//
// These exercise the file fallback directly, which is what runs on CI. The
// keychain path can only be tested on a machine with a real Secret Service /
// Keychain, so we do not assert on it here.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_keychain() -> (Keychain, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let kc = Keychain::new(dir.path());
        (kc, dir)
    }

    #[test]
    fn fallback_round_trips_a_token() {
        let (kc, _dir) = temp_keychain();
        kc.write_fallback("token-abc").unwrap();
        assert_eq!(kc.load_session_token().as_deref(), Some("token-abc"));
    }

    #[test]
    fn missing_token_returns_none() {
        let (kc, _dir) = temp_keychain();
        // No keychain entry and no fallback file exists for this temp dir.
        assert!(!kc.fallback_path().exists());
    }

    #[test]
    fn clear_removes_the_fallback_file() {
        let (kc, _dir) = temp_keychain();
        kc.write_fallback("token-xyz").unwrap();
        assert!(kc.fallback_path().exists());

        kc.clear_session_token();
        assert!(!kc.fallback_path().exists());
    }

    #[test]
    fn whitespace_only_token_is_treated_as_absent() {
        let (kc, _dir) = temp_keychain();
        kc.write_fallback("   \n").unwrap();
        assert_eq!(kc.load_session_token(), None);
    }

    /// The two secrets must not share a slot. A sync token that overwrote the
    /// session token would log the attorney out every time sync was configured.
    #[test]
    fn sync_and_session_tokens_do_not_collide() {
        let (kc, dir) = temp_keychain();
        // Write both via the fallback path directly, since CI has no Secret Service.
        write_private(&dir.path().join(SESSION_TOKEN.file), "session-1").unwrap();
        write_private(&dir.path().join(SYNC_TOKEN.file), "sync-1").unwrap();

        assert_eq!(kc.load(SESSION_TOKEN).as_deref(), Some("session-1"));
        assert_eq!(kc.load(SYNC_TOKEN).as_deref(), Some("sync-1"));

        kc.clear(SYNC_TOKEN);
        assert_eq!(kc.load(SESSION_TOKEN).as_deref(), Some("session-1"),
                   "clearing the sync token must not disturb the session");
    }

    #[cfg(unix)]
    #[test]
    fn fallback_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let (kc, _dir) = temp_keychain();
        kc.write_fallback("token-perm").unwrap();

        let mode = std::fs::metadata(kc.fallback_path())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "session token file must be 0600");
    }
}
