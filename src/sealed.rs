//! Sealed proof bundle handling.
//!
//! Handles decryption of password-protected .proofaudio bundles.

use serde::Deserialize;

use crate::crypto::{decode_base64, decrypt_aes_gcm, derive_key_pbkdf2};
use crate::error::{Result, VerifyError};

/// Current supported bundle version.
pub const CURRENT_BUNDLE_VERSION: i32 = 1;

/// Outer structure of a sealed proof bundle (.proofaudio file).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealedProofBundle {
    pub version: i32,
    pub salt: String,           // Base64-encoded
    pub nonce: String,          // Base64-encoded (not used - nonce is in combined payload)
    pub kdf_algorithm: String,  // "pbkdf2" or "argon2id"
    pub kdf_parameters: KdfParameters,
    pub encrypted_payload: String, // Base64-encoded AES-GCM combined
    pub created_at: String,     // ISO-8601 timestamp
}

/// KDF parameters for key derivation.
#[derive(Debug, Deserialize)]
pub struct KdfParameters {
    pub iterations: u32,
    #[serde(alias = "memoryCostKB")]
    pub memory_cost_kb: u32,
    pub parallelism: u32,
}

/// Decrypted payload containing audio and manifest.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecryptedPayload {
    pub audio_data: String,     // Base64-encoded audio bytes
    pub manifest_data: String,  // Base64-encoded manifest JSON
    pub audio_filename: String,
}

impl SealedProofBundle {
    /// Parse sealed bundle from JSON bytes.
    pub fn from_json(json_bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(json_bytes).map_err(|_| VerifyError::BundleCorrupted)
    }

    /// Check if bundle version is supported.
    pub fn validate_version(&self) -> Result<()> {
        if self.version > CURRENT_BUNDLE_VERSION {
            return Err(VerifyError::UnsupportedBundleVersion {
                version: self.version,
            });
        }
        Ok(())
    }

    /// Decrypt the bundle using the provided password.
    pub fn decrypt(&self, password: &str) -> Result<DecryptedPayload> {
        // Validate version
        self.validate_version()?;

        // Validate KDF algorithm
        if self.kdf_algorithm != "pbkdf2" {
            // Argon2id not yet supported
            return Err(VerifyError::DecryptionFailed);
        }

        // Decode salt
        let salt = decode_base64(&self.salt)?;

        // Derive key using PBKDF2
        let key = derive_key_pbkdf2(password, &salt, self.kdf_parameters.iterations);

        // Decode encrypted payload
        let encrypted = decode_base64(&self.encrypted_payload)?;

        // Decrypt using AES-256-GCM
        let decrypted = decrypt_aes_gcm(&key, &encrypted)?;

        // Parse decrypted payload as JSON
        let payload: DecryptedPayload =
            serde_json::from_slice(&decrypted).map_err(|_| VerifyError::BundleCorrupted)?;

        Ok(payload)
    }
}

impl DecryptedPayload {
    /// Get the audio data as bytes.
    pub fn audio_bytes(&self) -> Result<Vec<u8>> {
        decode_base64(&self.audio_data)
    }

    /// Get the manifest data as bytes.
    pub fn manifest_bytes(&self) -> Result<Vec<u8>> {
        decode_base64(&self.manifest_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bundle_structure() {
        let json = r#"{
            "version": 1,
            "salt": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
            "nonce": "AAAAAAAAAAAAAAAA",
            "kdfAlgorithm": "pbkdf2",
            "kdfParameters": {
                "iterations": 600000,
                "memoryCostKB": 0,
                "parallelism": 1
            },
            "encryptedPayload": "dGVzdA==",
            "createdAt": "2024-01-01T00:00:00Z"
        }"#;

        let bundle = SealedProofBundle::from_json(json.as_bytes()).unwrap();
        assert_eq!(bundle.version, 1);
        assert_eq!(bundle.kdf_algorithm, "pbkdf2");
        assert_eq!(bundle.kdf_parameters.iterations, 600000);
    }

    #[test]
    fn test_unsupported_version() {
        let json = r#"{
            "version": 99,
            "salt": "AA==",
            "nonce": "AA==",
            "kdfAlgorithm": "pbkdf2",
            "kdfParameters": {"iterations": 1, "memoryCostKB": 0, "parallelism": 1},
            "encryptedPayload": "AA==",
            "createdAt": "2024-01-01T00:00:00Z"
        }"#;

        let bundle = SealedProofBundle::from_json(json.as_bytes()).unwrap();
        let result = bundle.validate_version();
        assert!(matches!(
            result,
            Err(VerifyError::UnsupportedBundleVersion { version: 99 })
        ));
    }
}
