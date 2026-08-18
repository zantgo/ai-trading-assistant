use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::Engine;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::{Mutex, OnceLock};

static MASTER_KEY: Mutex<Option<[u8; 32]>> = Mutex::new(None);

pub fn init_master_key(secret: &str) -> Option<[u8; 32]> {
    if secret.is_empty() {
        return None;
    }
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    match MASTER_KEY.lock() {
        Ok(mut guard) => {
            *guard = Some(key);
            Some(key)
        }
        Err(_) => None,
    }
}

/// Replace the in-process master key (AUDIT-V6-077 rotation). Callers must
/// have already re-encrypted stored secrets under the new key.
pub fn rotate_master_key(new_secret: &str) -> Option<[u8; 32]> {
    let mut hasher = Sha256::new();
    hasher.update(new_secret.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    match MASTER_KEY.lock() {
        Ok(mut guard) => {
            *guard = Some(key);
            Some(key)
        }
        Err(_) => None,
    }
}

pub fn get_master_key() -> Option<[u8; 32]> {
    MASTER_KEY.lock().ok().and_then(|g| *g)
}

#[allow(dead_code)]
pub fn require_master_key() -> [u8; 32] {
    match get_master_key() {
        Some(key) => key,
        None => {
            panic!(
                "EXCHANGE_SECRET_KEY environment variable is not set or is empty. \
                 Set it to encrypt exchange API credentials at rest. \
                 Example: export EXCHANGE_SECRET_KEY=\"your-secure-random-string\""
            );
        }
    }
}

fn derive_key() -> Result<[u8; 32], String> {
    get_master_key().ok_or_else(|| "EXCHANGE_SECRET_KEY not set".to_string())
}

pub fn encrypt_field(plain: &str) -> Result<String, String> {
    let key = derive_key()?;
    encrypt_with_key(plain, &key)
}

pub fn decrypt_field(encoded: &str) -> Result<String, String> {
    let key = derive_key()?;
    decrypt_with_key(encoded, &key)
}

/// Encrypt with an explicit key (rotation + backup export).
pub fn encrypt_with_key(plain: &str, key: &[u8; 32]) -> Result<String, String> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| format!("cipher init failed: {}", e))?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plain.as_bytes())
        .map_err(|e| format!("encryption failed: {}", e))?;
    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    Ok(base64::engine::general_purpose::STANDARD.encode(&combined))
}

/// Decrypt with an explicit key (rotation + backup export).
pub fn decrypt_with_key(encoded: &str, key: &[u8; 32]) -> Result<String, String> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| format!("cipher init failed: {}", e))?;
    let combined = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| format!("base64 decode failed: {}", e))?;
    if combined.len() < 12 {
        return Err("ciphertext too short".to_string());
    }
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plain = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("decryption failed: {}", e))?;
    String::from_utf8(plain).map_err(|e| format!("invalid utf-8 after decryption: {}", e))
}

/// Derive a backup key from an operator passphrase (SHA-256).
pub fn backup_key_from_passphrase(passphrase: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(passphrase.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

pub fn master_key_available() -> bool {
    get_master_key().is_some()
}
