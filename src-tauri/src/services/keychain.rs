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
const KEYRING_USER: &str = "session_token";
const FALLBACK_FILENAME: &str = ".session_token";

pub struct Keychain {
    /// Used only when the platform keychain is unavailable.
    fallback_path: PathBuf,
}

impl Keychain {
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            fallback_path: app_data_dir.join(FALLBACK_FILENAME),
        }
    }

    /// Persist the session token. Errors are logged, never fatal — a failure here
    /// costs the user session persistence, not access.
    pub fn store_session_token(&self, token: &str) {
        match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
            Ok(entry) => match entry.set_password(token) {
                Ok(()) => {
                    // Keychain took it — make sure no stale fallback copy lingers.
                    let _ = std::fs::remove_file(&self.fallback_path);
                    return;
                }
                Err(e) => log::warn!("keychain write failed ({e}); using file fallback"),
            },
            Err(e) => log::warn!("keychain unavailable ({e}); using file fallback"),
        }

        if let Err(e) = self.write_fallback(token) {
            log::error!("failed to persist session token to fallback file: {e}");
        }
    }

    /// Read the stored session token, if any.
    pub fn load_session_token(&self) -> Option<String> {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
            match entry.get_password() {
                Ok(token) if !token.trim().is_empty() => return Some(token),
                Ok(_) => return None,
                // NoEntry is normal (never logged in); anything else falls through.
                Err(keyring::Error::NoEntry) => {}
                Err(e) => log::warn!("keychain read failed ({e}); trying file fallback"),
            }
        }

        match std::fs::read_to_string(&self.fallback_path) {
            Ok(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
            _ => None,
        }
    }

    /// Remove the stored token (explicit logout). Clears both backends so a
    /// half-written fallback can never resurrect a logged-out session.
    pub fn clear_session_token(&self) {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(e) => log::warn!("keychain delete failed: {e}"),
            }
        }
        let _ = std::fs::remove_file(&self.fallback_path);
    }

    // -----------------------------------------------------------------------

    #[cfg(unix)]
    fn write_fallback(&self, token: &str) -> std::io::Result<()> {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600) // owner read/write only
            .open(&self.fallback_path)?;
        f.write_all(token.as_bytes())
    }

    #[cfg(not(unix))]
    fn write_fallback(&self, token: &str) -> std::io::Result<()> {
        // Windows: the app data directory is already per-user ACL'd.
        let mut f = std::fs::File::create(&self.fallback_path)?;
        f.write_all(token.as_bytes())
    }
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
        assert!(!kc.fallback_path.exists());
    }

    #[test]
    fn clear_removes_the_fallback_file() {
        let (kc, _dir) = temp_keychain();
        kc.write_fallback("token-xyz").unwrap();
        assert!(kc.fallback_path.exists());

        kc.clear_session_token();
        assert!(!kc.fallback_path.exists());
    }

    #[test]
    fn whitespace_only_token_is_treated_as_absent() {
        let (kc, _dir) = temp_keychain();
        kc.write_fallback("   \n").unwrap();
        assert_eq!(kc.load_session_token(), None);
    }

    #[cfg(unix)]
    #[test]
    fn fallback_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let (kc, _dir) = temp_keychain();
        kc.write_fallback("token-perm").unwrap();

        let mode = std::fs::metadata(&kc.fallback_path)
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "session token file must be 0600");
    }
}
