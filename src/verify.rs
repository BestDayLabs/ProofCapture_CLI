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

/// Result of sealed bundle verification with extracted audio.
#[derive(Debug)]
pub struct SealedVerificationResult {
    pub manifest: SignedAudioManifest,
    pub trust_level: TrustLevel,
    pub audio_data: Vec<u8>,
    pub audio_filename: String,
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
    let result = verify_and_extract_sealed_bundle(bundle_path, password)?;
    Ok(VerificationResult {
        manifest: result.manifest,
        trust_level: result.trust_level,
    })
}

/// Verify a sealed proof bundle and return the decrypted audio data.
pub fn verify_and_extract_sealed_bundle(bundle_path: &Path, password: &str) -> Result<SealedVerificationResult> {
    // Read bundle
    let bundle_bytes = fs::read(bundle_path).map_err(|e| VerifyError::Io(e))?;

    // Parse and decrypt
    let bundle = SealedProofBundle::from_json(&bundle_bytes)?;
    let payload = bundle.decrypt(password)?;

    // Get audio and manifest bytes
    let audio_bytes = payload.audio_bytes()?;
    let manifest_bytes = payload.manifest_bytes()?;

    // Verify
    let verification = verify_audio_and_manifest(&audio_bytes, &manifest_bytes)?;

    Ok(SealedVerificationResult {
        manifest: verification.manifest,
        trust_level: verification.trust_level,
        audio_data: audio_bytes,
        audio_filename: payload.audio_filename.clone(),
    })
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
    use super::*;
    use std::path::PathBuf;

    /// Get the fixtures directory path
    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
    }

    // ==================== Standard Bundle Tests ====================

    #[test]
    fn test_verify_minimal_bundle_succeeds() {
        let bundle_path = fixtures_dir().join("minimal_bundle");
        let result = verify_standard_bundle(&bundle_path);

        assert!(result.is_ok(), "Minimal bundle should verify: {:?}", result.err());

        let verification = result.unwrap();
        assert_eq!(verification.trust_level, TrustLevel::C);
        assert_eq!(verification.manifest.schema_version, 1);
        assert_eq!(verification.manifest.app_bundle_id, "com.bestdaylabs.proofaudio");
    }

    #[test]
    fn test_verify_full_bundle_succeeds() {
        let bundle_path = fixtures_dir().join("full_bundle");
        let result = verify_standard_bundle(&bundle_path);

        assert!(result.is_ok(), "Full bundle should verify: {:?}", result.err());

        let verification = result.unwrap();
        assert_eq!(verification.trust_level, TrustLevel::A);
        assert!(verification.manifest.trust_vectors.location.is_some());
        assert!(verification.manifest.trust_vectors.motion.is_some());
        assert!(verification.manifest.trust_vectors.continuity.is_some());
        assert!(verification.manifest.trust_vectors.clock.is_some());
    }

    #[test]
    fn test_verify_minimal_bundle_has_correct_metadata() {
        let bundle_path = fixtures_dir().join("minimal_bundle");
        let result = verify_standard_bundle(&bundle_path).unwrap();

        assert_eq!(result.manifest.audio_format, "aac");
        assert_eq!(result.manifest.app_version, "1.0.0");
        assert!(result.manifest.duration_seconds > 0.0);
        assert!(result.manifest.audio_size_bytes > 0);
    }

    #[test]
    fn test_verify_full_bundle_location_data() {
        let bundle_path = fixtures_dir().join("full_bundle");
        let result = verify_standard_bundle(&bundle_path).unwrap();

        let location = result.manifest.trust_vectors.location.as_ref().unwrap();
        assert!((location.start.lat - 37.775).abs() < 0.001);
        assert!((location.start.lon - (-122.418)).abs() < 0.001);
        assert!(location.start.accuracy > 0.0);
    }

    #[test]
    fn test_verify_full_bundle_continuity_uninterrupted() {
        let bundle_path = fixtures_dir().join("full_bundle");
        let result = verify_standard_bundle(&bundle_path).unwrap();

        let continuity = result.manifest.trust_vectors.continuity.as_ref().unwrap();
        assert!(continuity.uninterrupted);
        assert!(continuity.interruption_events.is_empty());
    }

    // ==================== Sealed Bundle Tests ====================

    #[test]
    fn test_verify_sealed_bundle_with_correct_password() {
        let bundle_path = fixtures_dir().join("sealed_test.proofaudio");
        let result = verify_sealed_bundle(&bundle_path, "test-password-123");

        assert!(result.is_ok(), "Sealed bundle should verify with correct password: {:?}", result.err());

        let verification = result.unwrap();
        assert_eq!(verification.manifest.app_bundle_id, "com.bestdaylabs.proofaudio");
    }

    #[test]
    fn test_verify_sealed_bundle_with_wrong_password_fails() {
        let bundle_path = fixtures_dir().join("sealed_test.proofaudio");
        let result = verify_sealed_bundle(&bundle_path, "wrong-password");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerifyError::DecryptionFailed));
    }

    #[test]
    fn test_verify_sealed_bundle_with_empty_password_fails() {
        let bundle_path = fixtures_dir().join("sealed_test.proofaudio");
        let result = verify_sealed_bundle(&bundle_path, "");

        assert!(result.is_err());
    }

    #[test]
    fn test_sealed_bundle_has_trust_vectors() {
        let bundle_path = fixtures_dir().join("sealed_test.proofaudio");
        let result = verify_sealed_bundle(&bundle_path, "test-password-123").unwrap();

        // Sealed test bundle has continuity and clock vectors
        assert!(result.manifest.trust_vectors.continuity.is_some());
        assert!(result.manifest.trust_vectors.clock.is_some());
    }

    // ==================== Error Case Tests ====================

    #[test]
    fn test_verify_nonexistent_bundle_fails() {
        let bundle_path = fixtures_dir().join("nonexistent_bundle");
        let result = verify_standard_bundle(&bundle_path);

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_modified_audio_fails() {
        // Create a temporary copy of minimal bundle with modified audio
        let temp_dir = std::env::temp_dir().join("proofaudio_test_modified");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up any previous run
        fs::create_dir_all(&temp_dir).unwrap();

        // Copy manifest
        let source_manifest = fixtures_dir().join("minimal_bundle").join("manifest.json");
        let dest_manifest = temp_dir.join("manifest.json");
        fs::copy(&source_manifest, &dest_manifest).unwrap();

        // Create modified audio (different content than original)
        let dest_audio = temp_dir.join("recording.m4a");
        fs::write(&dest_audio, b"modified audio content that doesn't match hash").unwrap();

        let result = verify_standard_bundle(&temp_dir);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerifyError::HashMismatch));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_verify_tampered_manifest_fails() {
        // Create a temporary copy with tampered manifest
        let temp_dir = std::env::temp_dir().join("proofaudio_test_tampered");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Copy audio
        let source_audio = fixtures_dir().join("minimal_bundle").join("recording.m4a");
        let dest_audio = temp_dir.join("recording.m4a");
        fs::copy(&source_audio, &dest_audio).unwrap();

        // Read and modify manifest
        let source_manifest = fixtures_dir().join("minimal_bundle").join("manifest.json");
        let manifest_content = fs::read_to_string(&source_manifest).unwrap();
        // Change the app version to tamper with the manifest
        let tampered = manifest_content.replace("1.0.0", "2.0.0");
        let dest_manifest = temp_dir.join("manifest.json");
        fs::write(&dest_manifest, tampered).unwrap();

        let result = verify_standard_bundle(&temp_dir);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerifyError::SignatureInvalid));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    // ==================== Trust Level Tests ====================

    #[test]
    fn test_trust_level_c_for_no_vectors() {
        let bundle_path = fixtures_dir().join("minimal_bundle");
        let result = verify_standard_bundle(&bundle_path).unwrap();

        // Minimal bundle has no trust vectors = Level C
        assert_eq!(result.trust_level, TrustLevel::C);
    }

    #[test]
    fn test_trust_level_a_for_all_vectors_continuous() {
        let bundle_path = fixtures_dir().join("full_bundle");
        let result = verify_standard_bundle(&bundle_path).unwrap();

        // Full bundle has all vectors + uninterrupted = Level A
        assert_eq!(result.trust_level, TrustLevel::A);
    }
}
