# Technical Specification: ProofCapture CLI Verifier

> **IMPLEMENTED** — This specification has been fully implemented in the `proofcapture-cli` tool. See [CLI_IMPLEMENTATION_PLAN.md](CLI_IMPLEMENTATION_PLAN.md) for implementation details and [CLI_INTEROPERABILITY_SPEC.md](CLI_INTEROPERABILITY_SPEC.md) for byte-level format specifications.

## 1. Purpose & Scope
The ProofCapture CLI Verifier is a "Universal Verifier" that ensures the cryptographic claims of any ProofCapture recording can be audited on desktop systems. It fulfills the requirement for Deterministic Verification, proving that the "Root of Trust" is mathematical (SHA-256 and P-256 ECDSA) rather than platform-dependent.

## 2. Technical Stack
**Language:** Rust (selected for memory safety and static binary compilation).

Cryptographic Primitives:

Hashing: SHA-256.

Signatures: P-256 ECDSA.

Encryption: AES-256-GCM (for Sealed Proofs).

Key Derivation: PBKDF2-HMAC-SHA256 (600,000 iterations).

## 3. Core Functional Requirements
### 3.1 Input Formats
The tool must process two primary artifact types:

Standard Proof Bundle: A directory or ZIP archive containing recording.m4a, manifest.json, and README.txt.

Sealed Proof Bundle (.proofcapture): A single encrypted file containing the audio and manifest.

### 3.2 Decryption Logic (Sealed Proofs Only)
For .proofcapture files, the tool must execute the following:

KDF Execution: Derive a symmetric key from a user-provided password and the plaintext salt found in the bundle header.

Authenticated Decryption: Decrypt the container using AES-256-GCM with the provided nonce.

Integrity Check: The GCM authentication tag must be verified; failure results in an immediate "Bundle Corrupted" error.

### 3.3 Verification Pipeline
Once the manifest and audio are in plaintext:

Canonicalization: Parse the manifest.json ensuring keys are sorted and whitespace is handled consistently to match the signature input.

Audio Hashing: Recompute the SHA-256 hash of the recording.m4a file.

Signature Validation: Verify the ECDSA signature using the publicKey provided in the manifest.

Trust Vector Evaluation: Analyze metadata for location, motion, and continuity to determine the Trust Level (A, B, or C).

## 4. Security Model & Constraints
No Persistence: The tool must never write decrypted audio or passwords to the disk; all operations must occur in RAM.

Auditability: The source code must be open-source to allow verification of the verification logic itself.

Fail-Closed: Any deviation in the hash or signature must result in a binary "FAILED" status.

## 5. Command Line Interface (CLI)

### 5.1 Basic Usage
```bash
# Verify a standard bundle
proofcapture-cli ./MyRecordingBundle/

# Verify and decrypt a sealed proof
proofcapture-cli evidence.proofcapture --password "user-secret-pass"

# Extract audio after verification
proofcapture-cli evidence.proofcapture --password "secret" --extract ./output/

# JSON output for scripting
proofcapture-cli ./bundle/ --format json
```

### 5.2 Error Taxonomy
The CLI must return standard exit codes and error messages defined in the system architecture:

hashMismatch: "Audio has been modified".

signatureInvalid: "Signature verification failed".

decryptionFailed: "Could not decrypt. Check your password.".

## 6. Output Example

```
PROOFAUDIO VERIFICATION SUMMARY
===============================
Status:      VERIFIED
Trust Level: Level A (Verified Continuous Capture)

RECORDING DETAILS
-----------------
Captured:    2024-01-15T10:30:00Z
Duration:    2:15
Format:      AAC (M4A container)
Size:        1,234,567 bytes

CRYPTOGRAPHIC IDENTITY
----------------------
Device Key:  a1b2c3d4e5f6...
App:         com.bestdaylabs.proofcapture v1.0.0

TRUST VECTORS
-------------
Location:    37.775, -122.418 → 37.775, -122.419 (+/- 65m)
Motion:      Stationary (variance: 0.0023)
Continuity:  Uninterrupted
Clock:       America/Los_Angeles

LIMITATIONS
-----------
This verification proves capture integrity, NOT:
- Who is speaking
- That statements are true
- Legal consent to record
- Absence of AI-generated audio
```
