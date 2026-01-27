//! Cryptographic operations for ProofCapture verification.
//!
//! Implements SHA-256 hashing, P-256 ECDSA verification, AES-256-GCM decryption,
//! and PBKDF2 key derivation to match the iOS app's CryptoKit implementation.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::error::{Result, VerifyError};

/// Computes SHA-256 hash of data and returns base64-encoded string.
pub fn sha256_base64(data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    BASE64.encode(hash)
}

/// Computes SHA-256 hash of data and returns raw bytes.
pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let hash = Sha256::digest(data);
    hash.into()
}

/// Parses a P-256 public key from raw 64-byte format.
///
/// iOS exports public keys as raw x||y coordinates (64 bytes).
/// We need to prepend 0x04 (uncompressed point marker) for SEC1 parsing.
pub fn parse_public_key(raw_64_bytes: &[u8]) -> Result<VerifyingKey> {
    if raw_64_bytes.len() != 64 {
        return Err(VerifyError::SignatureInvalid);
    }

    // Prepend 0x04 uncompressed point marker
    let mut sec1_bytes = vec![0x04];
    sec1_bytes.extend_from_slice(raw_64_bytes);

    VerifyingKey::from_sec1_bytes(&sec1_bytes).map_err(|_| VerifyError::SignatureInvalid)
}

/// Parses an ECDSA signature from raw 64-byte format.
///
/// iOS exports signatures as raw r||s (64 bytes, each 32 bytes).
pub fn parse_signature(raw_64_bytes: &[u8]) -> Result<Signature> {
    if raw_64_bytes.len() != 64 {
        return Err(VerifyError::SignatureInvalid);
    }

    Signature::from_slice(raw_64_bytes).map_err(|_| VerifyError::SignatureInvalid)
}

/// Verifies an ECDSA signature over a message hash.
pub fn verify_signature(
    public_key: &VerifyingKey,
    message_hash: &[u8; 32],
    signature: &Signature,
) -> bool {
    public_key.verify(message_hash, signature).is_ok()
}

/// Derives an AES-256 key from a password using PBKDF2-HMAC-SHA256.
///
/// Parameters match iOS implementation:
/// - 600,000 iterations
/// - 32-byte output (AES-256 key)
pub fn derive_key_pbkdf2(password: &str, salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations, &mut key);
    key
}

/// Decrypts AES-256-GCM combined format (nonce || ciphertext || tag).
///
/// The encrypted payload format from iOS:
/// - First 12 bytes: nonce
/// - Middle: ciphertext
/// - Last 16 bytes: authentication tag
pub fn decrypt_aes_gcm(key: &[u8; 32], combined: &[u8]) -> Result<Vec<u8>> {
    if combined.len() < 28 {
        // Minimum: 12 (nonce) + 0 (ciphertext) + 16 (tag)
        return Err(VerifyError::BundleCorrupted);
    }

    let nonce = Nonce::from_slice(&combined[..12]);
    let ciphertext_with_tag = &combined[12..];

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| VerifyError::DecryptionFailed)?;

    cipher
        .decrypt(nonce, ciphertext_with_tag)
        .map_err(|_| VerifyError::DecryptionFailed)
}

/// Decodes a base64 string to bytes.
pub fn decode_base64(encoded: &str) -> Result<Vec<u8>> {
    BASE64.decode(encoded).map_err(VerifyError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_empty() {
        let hash = sha256_base64(b"");
        assert_eq!(hash, "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=");
    }

    #[test]
    fn test_sha256_hello() {
        let hash = sha256_base64(b"hello");
        assert_eq!(hash, "LPJNul+wow4m6DsqxbninhsWHlwfp0JecwQzYpOLmCQ=");
    }

    #[test]
    fn test_pbkdf2_derivation() {
        // Basic test that PBKDF2 produces deterministic output
        let key1 = derive_key_pbkdf2("password", b"salt", 1000);
        let key2 = derive_key_pbkdf2("password", b"salt", 1000);
        assert_eq!(key1, key2);

        // Different password produces different key
        let key3 = derive_key_pbkdf2("different", b"salt", 1000);
        assert_ne!(key1, key3);
    }
}
