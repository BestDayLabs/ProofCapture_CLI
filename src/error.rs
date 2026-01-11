//! Error types for ProofAudio CLI verification.

use thiserror::Error;

/// Verification errors with specific exit codes.
#[derive(Error, Debug)]
pub enum VerifyError {
    #[error("Audio has been modified since capture")]
    HashMismatch,

    #[error("Signature verification failed")]
    SignatureInvalid,

    #[error("Invalid proof file")]
    ManifestMalformed,

    #[error("Proof format version {version} is not supported")]
    SchemaUnsupported { version: i32 },

    #[error("Audio file not found")]
    AudioFileMissing,

    #[error("Audio file is corrupted")]
    AudioFileCorrupt,

    #[error("Could not decrypt. Check your password")]
    DecryptionFailed,

    #[error("This file has been modified and cannot be opened")]
    BundleCorrupted,

    #[error("This sealed proof requires a newer app version")]
    UnsupportedBundleVersion { version: i32 },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Base64 decoding error: {0}")]
    Base64(#[from] base64::DecodeError),
}

impl VerifyError {
    /// Returns the exit code for this error.
    pub fn exit_code(&self) -> i32 {
        match self {
            VerifyError::HashMismatch => 1,
            VerifyError::SignatureInvalid => 2,
            VerifyError::ManifestMalformed => 3,
            VerifyError::SchemaUnsupported { .. } => 4,
            VerifyError::AudioFileMissing => 5,
            VerifyError::AudioFileCorrupt => 6,
            VerifyError::DecryptionFailed => 7,
            VerifyError::BundleCorrupted => 8,
            VerifyError::UnsupportedBundleVersion { .. } => 9,
            VerifyError::Io(_) => 10,
            VerifyError::Json(_) => 3, // Treat as manifest malformed
            VerifyError::Base64(_) => 3,
        }
    }
}

pub type Result<T> = std::result::Result<T, VerifyError>;
