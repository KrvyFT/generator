use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use base64::Engine;

use crate::constants::SCRIPT_ENCRYPT_KEY_HEX;

/// Generate a random 12-byte nonce using getrandom (backed by Web Crypto in wasm).
fn random_nonce() -> [u8; 12] {
    let mut buf = [0u8; 12];
    getrandom::getrandom(&mut buf).expect("getrandom failed");
    buf
}

/// Encrypt plaintext JS with AES-256-GCM using the compile-time key.
/// Returns base64( nonce[12] || ciphertext_with_tag ).
pub fn encrypt_script(plaintext: &str) -> Result<String, String> {
    let key_bytes = hex::decode(SCRIPT_ENCRYPT_KEY_HEX).map_err(|e| format!("key decode: {e}"))?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let nonce_bytes = random_nonce();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("encrypt: {e}"))?;

    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(base64::engine::general_purpose::STANDARD.encode(&combined))
}

/// Return the encryption key as base64 (for injection into HTML bootstrap).
pub fn key_as_base64() -> String {
    let key_bytes =
        hex::decode(SCRIPT_ENCRYPT_KEY_HEX).expect("SCRIPT_ENCRYPT_KEY_HEX is invalid hex");
    base64::engine::general_purpose::STANDARD.encode(&key_bytes)
}
