//! ProofCapture CLI Verifier
//!
//! A command-line tool for verifying ProofCapture recordings.
//!
//! This library provides functionality to verify both standard proof bundles
//! and password-protected sealed proof bundles (.proofcapture files).
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//! use proofcapture_cli::verify::{verify_standard_bundle, verify_sealed_bundle};
//!
//! // Verify a standard bundle
//! let result = verify_standard_bundle(Path::new("./recording_bundle/"));
//!
//! // Verify a sealed bundle
//! let result = verify_sealed_bundle(Path::new("evidence.proofcapture"), "password");
//! ```

pub mod crypto;
pub mod error;
pub mod manifest;
pub mod sealed;
pub mod trust;
pub mod verify;

pub use error::{Result, VerifyError};
pub use manifest::SignedAudioManifest;
pub use trust::TrustLevel;
pub use verify::{verify_audio_and_manifest, verify_sealed_bundle, verify_and_extract_sealed_bundle, verify_standard_bundle, VerificationResult, SealedVerificationResult};
