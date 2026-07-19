// AES-256-GCM document vault.
// All document bytes MUST go through these functions — never raw filesystem.
//
// File format on disk:
//   [12-byte nonce][AES-256-GCM ciphertext + 16-byte auth tag]
//
// Vault layout:
//   {vault_dir}/{matter_id}/{doc_id}.enc
//
// Vault directory:
//   macOS:   ~/Library/Application Support/com.persist.app/vault/
//   Windows: %APPDATA%\com.persist.app\vault\

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use anyhow::Context;
use rand::RngCore;
use std::path::Path;

const NONCE_LEN: usize = 12;

/// Encrypt `bytes` and write to vault.
/// Returns the vault-relative path (e.g. `"TM-2026-0001/abc123.enc"`).
pub fn encrypt_to_vault(
    vault_dir: &Path,
    vault_key: &[u8; 32],
    bytes: &[u8],
    matter_id: &str,
    doc_id: &str,
) -> anyhow::Result<String> {
    // Build per-file output path: {vault_dir}/{matter_id}/{doc_id}.enc
    let rel_path = format!("{}/{}.enc", matter_id, doc_id);
    let abs_path = vault_dir.join(&rel_path);

    // Ensure the matter subdirectory exists.
    if let Some(parent) = abs_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create vault directory: {}", parent.display()))?;
    }

    // Generate a fresh random 96-bit nonce.
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);

    let key    = Key::<Aes256Gcm>::from_slice(vault_key);
    let cipher = Aes256Gcm::new(key);
    let nonce  = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, bytes)
        .map_err(|e| anyhow::anyhow!("AES-256-GCM encrypt failed: {e}"))?;

    // Write: [nonce (12 bytes)][ciphertext + GCM tag (plaintext_len + 16 bytes)]
    let mut file_bytes = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    file_bytes.extend_from_slice(&nonce_bytes);
    file_bytes.extend_from_slice(&ciphertext);

    std::fs::write(&abs_path, &file_bytes)
        .with_context(|| format!("failed to write vault file: {}", abs_path.display()))?;

    Ok(rel_path)
}

/// Decrypt and return the original bytes for a vaulted document.
pub fn decrypt_from_vault(
    vault_dir: &Path,
    vault_key: &[u8; 32],
    vault_path: &str,
) -> anyhow::Result<Vec<u8>> {
    let abs_path = vault_dir.join(vault_path);

    let file_bytes = std::fs::read(&abs_path)
        .with_context(|| format!("failed to read vault file: {}", vault_path))?;

    anyhow::ensure!(
        file_bytes.len() > NONCE_LEN,
        "vault file corrupt or empty: {}",
        vault_path
    );

    let (nonce_bytes, ciphertext) = file_bytes.split_at(NONCE_LEN);

    let key    = Key::<Aes256Gcm>::from_slice(vault_key);
    let cipher = Aes256Gcm::new(key);
    let nonce  = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("AES-256-GCM decrypt failed: {e} (file: {vault_path})"))?;

    Ok(plaintext)
}

/// Delete a vaulted file from disk.
/// Call this when a document is deleted from the DB.
pub fn delete_from_vault(vault_dir: &Path, vault_path: &str) -> anyhow::Result<()> {
    let abs_path = vault_dir.join(vault_path);
    if abs_path.exists() {
        std::fs::remove_file(&abs_path)
            .with_context(|| format!("failed to delete vault file: {vault_path}"))?;
    }
    Ok(())
}

/// Strip metadata from document bytes before any export.
/// MUST be called on every document returned to clients.
///
/// Phase 1: pass-through (no runtime PDF lib dependency).
/// Phase 2: integrate lopdf / exiftool for deep stripping.
pub fn clean_metadata(bytes: &[u8], _doc_type: &str) -> anyhow::Result<Vec<u8>> {
    Ok(bytes.to_vec())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_key() -> [u8; 32] {
        let mut k = [0u8; 32];
        for (i, b) in k.iter_mut().enumerate() {
            *b = i as u8;
        }
        k
    }

    #[test]
    fn round_trip_encrypt_decrypt() {
        let dir = TempDir::new().unwrap();
        let key = test_key();
        let plaintext = b"Confidential matter document bytes";

        let vault_path = encrypt_to_vault(dir.path(), &key, plaintext, "TM-2026-0001", "doc-abc")
            .expect("encrypt should succeed");

        let recovered = decrypt_from_vault(dir.path(), &key, &vault_path)
            .expect("decrypt should succeed");

        assert_eq!(recovered.as_slice(), plaintext.as_ref());
    }

    #[test]
    fn different_nonce_each_call() {
        let dir = TempDir::new().unwrap();
        let key = test_key();
        let data = b"same data";

        let p1 = encrypt_to_vault(dir.path(), &key, data, "m1", "d1").unwrap();
        let p2 = encrypt_to_vault(dir.path(), &key, data, "m1", "d2").unwrap();

        let f1 = std::fs::read(dir.path().join(&p1)).unwrap();
        let f2 = std::fs::read(dir.path().join(&p2)).unwrap();

        // Nonces must differ (first 12 bytes).
        assert_ne!(&f1[..12], &f2[..12], "each encrypt should use a fresh nonce");
    }

    #[test]
    fn wrong_key_fails_decrypt() {
        let dir = TempDir::new().unwrap();
        let key1 = test_key();
        let mut key2 = test_key();
        key2[0] ^= 0xff;

        let vault_path = encrypt_to_vault(dir.path(), &key1, b"secret", "m1", "doc1").unwrap();
        let result = decrypt_from_vault(dir.path(), &key2, &vault_path);
        assert!(result.is_err(), "wrong key should fail to decrypt");
    }

    #[test]
    fn delete_removes_file() {
        let dir = TempDir::new().unwrap();
        let key = test_key();

        let vault_path = encrypt_to_vault(dir.path(), &key, b"data", "m1", "del-me").unwrap();
        assert!(dir.path().join(&vault_path).exists());

        delete_from_vault(dir.path(), &vault_path).unwrap();
        assert!(!dir.path().join(&vault_path).exists());
    }
}
