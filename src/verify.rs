//! Core verification logic.
//!
//! Implements the verification pipeline for both standard and sealed bundles.

use std::fs;
use std::path::Path;

use crate::crypto::{decode_base64, parse_public_key, parse_signature, sha256_base64, verify_signature};
use crate::error::{Result, VerifyError};
use crate::manifest::{compute_canonical_hash_from_bytes, SignedAudioManifest};
use crate::sealed::SealedProofBundle;
use crate::trust::{compute_trust_level, TrustLevel};

/// Result of a successful verification.
#[derive(Debug)]
pub struct VerificationResult {
    pub manifest: SignedAudioManifest,
    pub trust_level: TrustLevel,
}

/// Verify a standard proof bundle (directory or files).
///
/// Expected structure:
/// - recording.m4a (or similar audio file)
/// - manifest.json
pub fn verify_standard_bundle(bundle_path: &Path) -> Result<VerificationResult> {
    // Determine if path is directory or file
    let (audio_path, manifest_path) = if bundle_path.is_dir() {
        // Look for audio and manifest files in directory
        let audio = find_audio_file(bundle_path)?;
        let manifest = bundle_path.join("manifest.json");
        if !manifest.exists() {
            return Err(VerifyError::ManifestMalformed);
        }
        (audio, manifest)
    } else {
        // Single file - could be a zip or the manifest itself
        // For now, treat as manifest and look for sibling audio
        let parent = bundle_path.parent().unwrap_or(Path::new("."));
        let audio = find_audio_file(parent)?;
        (audio, bundle_path.to_path_buf())
    };

    // Read files
    let audio_bytes = fs::read(&audio_path).map_err(|_| VerifyError::AudioFileMissing)?;
    let manifest_bytes = fs::read(&manifest_path).map_err(|_| VerifyError::ManifestMalformed)?;

    // Verify
    verify_audio_and_manifest(&audio_bytes, &manifest_bytes)
}

/// Verify a sealed proof bundle (.proofaudio file).
pub fn verify_sealed_bundle(bundle_path: &Path, password: &str) -> Result<VerificationResult> {
    // Read bundle
    let bundle_bytes = fs::read(bundle_path).map_err(|e| VerifyError::Io(e))?;

    // Parse and decrypt
    let bundle = SealedProofBundle::from_json(&bundle_bytes)?;
    let payload = bundle.decrypt(password)?;

    // Get audio and manifest bytes
    let audio_bytes = payload.audio_bytes()?;
    let manifest_bytes = payload.manifest_bytes()?;

    // Verify
    verify_audio_and_manifest(&audio_bytes, &manifest_bytes)
}

/// Core verification of audio bytes against manifest.
pub fn verify_audio_and_manifest(
    audio_bytes: &[u8],
    manifest_bytes: &[u8],
) -> Result<VerificationResult> {
    // Parse manifest
    let manifest = SignedAudioManifest::from_json(manifest_bytes)?;

    // Validate schema version
    manifest.validate_schema()?;

    // Step 1: Verify audio hash
    let computed_hash = sha256_base64(audio_bytes);
    if computed_hash != manifest.audio_hash {
        return Err(VerifyError::HashMismatch);
    }

    // Step 2: Parse public key
    let public_key_bytes = decode_base64(&manifest.public_key)?;
    let public_key = parse_public_key(&public_key_bytes)?;

    // Step 3: Compute canonical manifest hash (use original bytes to preserve formatting)
    let manifest_hash = compute_canonical_hash_from_bytes(manifest_bytes)?;

    // Step 4: Parse and verify signature
    let signature_bytes = decode_base64(&manifest.signature)?;
    let signature = parse_signature(&signature_bytes)?;

    if !verify_signature(&public_key, &manifest_hash, &signature) {
        return Err(VerifyError::SignatureInvalid);
    }

    // Step 5: Compute trust level
    let trust_level = compute_trust_level(&manifest.trust_vectors);

    Ok(VerificationResult {
        manifest,
        trust_level,
    })
}

/// Find an audio file in a directory.
fn find_audio_file(dir: &Path) -> Result<std::path::PathBuf> {
    let extensions = ["m4a", "aac", "mp4", "wav"];

    for ext in &extensions {
        // Try "recording.{ext}" first
        let recording = dir.join(format!("recording.{}", ext));
        if recording.exists() {
            return Ok(recording);
        }
    }

    // Look for any audio file
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if extensions.contains(&ext.to_str().unwrap_or("")) {
                    return Ok(path);
                }
            }
        }
    }

    Err(VerifyError::AudioFileMissing)
}

#[cfg(test)]
mod tests {
    // Integration tests would go here with real fixtures
}
