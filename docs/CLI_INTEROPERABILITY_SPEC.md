# CLI Interoperability Specification — ProofCapture

Best Day Labs

---

## 1. Purpose

This document specifies the exact byte-level formats required for external tools (CLI verifiers, third-party validators) to verify ProofCapture recordings. It is the canonical reference for implementing cross-platform verification.

**Target Audience:** Developers implementing CLI verifiers in Rust, Go, or other languages.

**Compatibility:** iOS App version 1.0.0+, Schema version 1

---

## 2. Cryptographic Primitives

All implementations MUST use these exact algorithms:

| Purpose | Algorithm | Library Reference |
|---------|-----------|-------------------|
| Audio Hashing | SHA-256 | CryptoKit `SHA256` |
| Manifest Hashing | SHA-256 | CryptoKit `SHA256` |
| Signing | P-256 ECDSA | CryptoKit `P256.Signing` |
| Symmetric Encryption | AES-256-GCM | CryptoKit `AES.GCM` |
| Key Derivation | PBKDF2-HMAC-SHA256 | CommonCrypto `CCKeyDerivationPBKDF` |

---

## 3. Key and Signature Formats

### 3.1 Public Key Format

**Format:** P-256 Raw Representation (64 bytes)

```
┌──────────────────────────────────────────────────────────────────┐
│ X Coordinate (32 bytes, big-endian) │ Y Coordinate (32 bytes)   │
└──────────────────────────────────────────────────────────────────┘
```

**Encoding in Manifest:** Base64 (standard, no line breaks)

**NOT DER/X9.63 format.** The iOS app strips the `0x04` uncompressed point marker.

**CLI Implementation Note:**
```rust
// Rust (p256 crate): Prepend 0x04 before parsing
fn parse_public_key(raw_64_bytes: &[u8]) -> Result<PublicKey> {
    let mut sec1_bytes = vec![0x04]; // Uncompressed point marker
    sec1_bytes.extend_from_slice(raw_64_bytes);
    PublicKey::from_sec1_bytes(&sec1_bytes)
}
```

```go
// Go: Use elliptic.Unmarshal with prepended 0x04
func parsePublicKey(raw64 []byte) (*ecdsa.PublicKey, error) {
    x, y := elliptic.Unmarshal(elliptic.P256(), append([]byte{0x04}, raw64...))
    return &ecdsa.PublicKey{Curve: elliptic.P256(), X: x, Y: y}, nil
}
```

### 3.2 Signature Format

**Format:** P-256 ECDSA Raw Representation (64 bytes)

```
┌──────────────────────────────────────────────────────────────────┐
│ R Value (32 bytes, big-endian, zero-padded) │ S Value (32 bytes) │
└──────────────────────────────────────────────────────────────────┘
```

**Encoding in Manifest:** Base64 (standard, no line breaks)

**NOT DER/ASN.1 format.** The iOS app converts from DER to raw format.

**Padding:** R and S values are left-padded with zeros to exactly 32 bytes each.

### 3.3 Device Key ID Computation

The `deviceKeyId` field is computed as:

```
deviceKeyId = Base64(SHA256(publicKeyRawBytes))
```

Where `publicKeyRawBytes` is the 64-byte raw public key (not base64 encoded).

---

## 4. Manifest Format

### 4.1 Schema Version 1 Structure

```json
{
  "schemaVersion": 1,
  "audioHash": "<base64-sha256-of-audio-bytes>",
  "audioFormat": "aac",
  "audioSizeBytes": 123456,
  "captureStart": "2024-01-15T10:30:00Z",
  "captureEnd": "2024-01-15T10:32:15Z",
  "durationSeconds": 135.0,
  "appVersion": "1.0.0",
  "appBundleId": "com.bestdaylabs.proofcapture",
  "deviceKeyId": "<base64-sha256-of-public-key>",
  "publicKey": "<base64-raw-64-byte-public-key>",
  "trustVectors": {
    "location": { ... } | null,
    "motion": { ... } | null,
    "continuity": { ... } | null,
    "clock": { ... } | null
  },
  "signature": "<base64-raw-64-byte-signature>"
}
```

### 4.2 Trust Vector Structures

**Location Vector:**
```json
{
  "start": {
    "lat": 37.775,
    "lon": -122.418,
    "accuracy": 65.0
  },
  "end": {
    "lat": 37.775,
    "lon": -122.418,
    "accuracy": 65.0
  }
}
```

- `lat`, `lon`: Rounded to 3 decimal places (~500m precision)
- `accuracy`: Horizontal accuracy in meters

**Motion Vector:**
```json
{
  "accelerationVariance": 0.0023,
  "rotationVariance": 0.0011,
  "duration": 135.0,
  "sampleCount": 1350
}
```

**Continuity Vector:**
```json
{
  "uninterrupted": true,
  "interruptionEvents": []
}
```

**Clock Vector:**
```json
{
  "wallClockStart": "2024-01-15T10:30:00Z",
  "wallClockEnd": "2024-01-15T10:32:15Z",
  "monotonicDelta": 135.0,
  "timeZone": "America/Los_Angeles"
}
```

---

## 5. Manifest Canonicalization Algorithm

**CRITICAL:** The signature is computed over a canonical JSON representation of the manifest. CLI implementations MUST replicate this exactly.

### 5.1 Canonicalization Rules

1. **Exclude the `signature` field** from the hash input
2. **Sort keys alphabetically** (recursive, at all nesting levels)
3. **No whitespace** (compact JSON, no spaces after colons or commas)
4. **Date encoding:** ISO-8601 format with fractional seconds
   - Swift format: `yyyy-MM-dd'T'HH:mm:ss.SSSZZZZZ`
   - Example: `2024-01-15T10:30:00.000Z`
5. **Encoding:** UTF-8

### 5.2 Fields Included in Hash (Alphabetical Order)

The manifest-for-hashing includes exactly these fields:

```
appBundleId
appVersion
audioFormat
audioHash
audioSizeBytes
captureEnd
captureStart
deviceKeyId
durationSeconds
publicKey
schemaVersion
trustVectors
```

**Note:** The `signature` field is EXCLUDED.

### 5.3 Canonical JSON Example

Given this manifest:
```json
{
  "schemaVersion": 1,
  "audioHash": "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=",
  "audioFormat": "aac",
  "audioSizeBytes": 0,
  "captureStart": "2024-01-01T00:00:00.000Z",
  "captureEnd": "2024-01-01T00:00:00.000Z",
  "durationSeconds": 0.0,
  "appVersion": "1.0.0",
  "appBundleId": "com.bestdaylabs.proofcapture",
  "deviceKeyId": "abc123",
  "publicKey": "AAAA...",
  "trustVectors": {
    "location": null,
    "motion": null,
    "continuity": null,
    "clock": null
  },
  "signature": "SIGNATURE_EXCLUDED"
}
```

The canonical JSON for hashing (sorted keys, no whitespace, no signature):
```json
{"appBundleId":"com.bestdaylabs.proofcapture","appVersion":"1.0.0","audioFormat":"aac","audioHash":"47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=","audioSizeBytes":0,"captureEnd":"2024-01-01T00:00:00.000Z","captureStart":"2024-01-01T00:00:00.000Z","deviceKeyId":"abc123","durationSeconds":0,"publicKey":"AAAA...","schemaVersion":1,"trustVectors":{"clock":null,"continuity":null,"location":null,"motion":null}}
```

### 5.4 Hash Computation

```
manifestHash = SHA256(canonicalJSONBytes)
```

The result is 32 bytes (256 bits). This hash is the input to ECDSA signing/verification.

---

## 6. Verification Algorithm

### 6.1 Standard Bundle Verification

```
FUNCTION verify_standard_bundle(bundle_path):

    1. READ recording.m4a → audioBytes
    2. READ manifest.json → manifestJSON
    3. PARSE manifestJSON → manifest

    4. VALIDATE manifest.schemaVersion <= 1
       IF unsupported: RETURN FAILED (schemaUnsupported)

    5. COMPUTE SHA256(audioBytes) → computedHashBytes
    6. ENCODE computedHashBytes as Base64 → computedHash
    7. COMPARE computedHash == manifest.audioHash
       IF mismatch: RETURN FAILED (hashMismatch)

    8. DECODE manifest.publicKey from Base64 → publicKeyBytes (64 bytes)
    9. PREPEND 0x04 to publicKeyBytes → uncompressedKey (65 bytes)
    10. CONSTRUCT P-256 public key from uncompressedKey

    11. CREATE manifestForHashing = manifest WITHOUT "signature" field
    12. CANONICALIZE manifestForHashing:
        - Sort all keys alphabetically (recursive)
        - Encode as compact JSON (no whitespace)
        - Encode dates as ISO-8601
    13. ENCODE canonicalJSON as UTF-8 bytes
    14. COMPUTE SHA256(canonicalJSONBytes) → manifestHash (32 bytes)

    15. DECODE manifest.signature from Base64 → signatureBytes (64 bytes)
    16. SPLIT signatureBytes: R = first 32 bytes, S = last 32 bytes
    17. VERIFY ECDSA(publicKey, manifestHash, R, S)
        IF invalid: RETURN FAILED (signatureInvalid)

    18. COMPUTE trust level from manifest.trustVectors
    19. RETURN VERIFIED with trust level
```

### 6.2 Audio Hash Verification

```
audioHash = Base64(SHA256(rawAudioFileBytes))
```

- Hash the complete file bytes, not decoded audio samples
- The audio format is M4A container with AAC codec
- Do NOT parse or decode the audio; hash raw bytes

---

## 7. Sealed Bundle Format

### 7.1 File Structure

**File Extension:** `.proofcapture`

**Content-Type:** `application/json`

**Outer Structure (JSON-encoded):**

```json
{
  "version": 1,
  "salt": "<base64-32-bytes>",
  "nonce": "<base64-12-bytes>",
  "kdfAlgorithm": "pbkdf2",
  "kdfParameters": {
    "iterations": 600000,
    "memoryCostKB": 0,
    "parallelism": 1
  },
  "encryptedPayload": "<base64-aes-gcm-combined>",
  "createdAt": "2024-01-15T10:30:00Z"
}
```

### 7.2 Field Specifications

| Field | Type | Description |
|-------|------|-------------|
| `version` | Integer | Bundle format version (currently 1) |
| `salt` | Base64 String | 32-byte random salt for KDF |
| `nonce` | Base64 String | 12-byte AES-GCM nonce |
| `kdfAlgorithm` | String | `"pbkdf2"` or `"argon2id"` |
| `kdfParameters` | Object | KDF configuration |
| `encryptedPayload` | Base64 String | AES-GCM combined ciphertext |
| `createdAt` | ISO-8601 String | Bundle creation timestamp |

### 7.3 KDF Parameters

**PBKDF2 (Current Implementation):**
```json
{
  "iterations": 600000,
  "memoryCostKB": 0,
  "parallelism": 1
}
```

- Algorithm: PBKDF2-HMAC-SHA256
- Iterations: 600,000 (tuned for ~250-500ms)
- Output: 32-byte symmetric key

**Argon2id (Future/Reserved):**
```json
{
  "iterations": 3,
  "memoryCostKB": 65536,
  "parallelism": 4
}
```

### 7.4 Encrypted Payload Format

The `encryptedPayload` is AES-GCM "combined" format:

```
┌────────────────────────────────────────────────────────────────┐
│ Nonce (12 bytes) │ Ciphertext (variable) │ Auth Tag (16 bytes) │
└────────────────────────────────────────────────────────────────┘
```

**Total overhead:** 28 bytes (12 + 16)

### 7.5 Decrypted Payload Structure

After AES-GCM decryption, the plaintext is JSON-encoded:

```json
{
  "audioData": "<base64-raw-audio-bytes>",
  "manifestData": "<base64-manifest-json-bytes>",
  "audioFilename": "recording.m4a"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `audioData` | Base64 String | Raw audio file bytes |
| `manifestData` | Base64 String | Signed manifest JSON bytes |
| `audioFilename` | String | Original filename |

---

## 8. Sealed Bundle Decryption Algorithm

```
FUNCTION unseal_bundle(bundle_path, password):

    1. READ bundle_path → bundleData
    2. PARSE bundleData as JSON → bundle

    3. VALIDATE bundle.version <= 1
       IF unsupported: RETURN FAILED (unsupportedBundleVersion)

    4. DECODE bundle.salt from Base64 → salt (32 bytes)
    5. DECODE bundle.encryptedPayload from Base64 → encryptedBytes

    6. DERIVE key using PBKDF2:
       key = PBKDF2-HMAC-SHA256(
           password = UTF8(password),
           salt = salt,
           iterations = bundle.kdfParameters.iterations,
           keyLength = 32
       )

    7. PARSE encryptedBytes as AES-GCM combined:
       nonce = encryptedBytes[0:12]
       ciphertext = encryptedBytes[12:-16]
       authTag = encryptedBytes[-16:]

    8. DECRYPT using AES-256-GCM:
       plaintext = AES-GCM-Open(key, nonce, ciphertext, authTag)
       IF auth tag verification fails: RETURN FAILED (decryptionFailed)

    9. PARSE plaintext as JSON → payload
    10. DECODE payload.audioData from Base64 → audioBytes
    11. DECODE payload.manifestData from Base64 → manifestBytes
    12. PARSE manifestBytes as JSON → manifest

    13. CONTINUE with standard verification (Section 6.1, step 4)
```

---

## 9. Trust Level Computation

### 9.1 Trust Level Hierarchy

| Level | Name | Requirements |
|-------|------|--------------|
| **A** | Verified Continuous Capture | location + motion + continuity.uninterrupted |
| **B** | Verified Capture + Context | location + motion |
| **C** | Verified Capture | Valid signature only |

**Level A is highest, Level C is lowest.**

### 9.2 Computation Algorithm

```
FUNCTION compute_trust_level(trustVectors):

    hasLocation = trustVectors.location != null
    hasMotion = trustVectors.motion != null
    hasContinuity = trustVectors.continuity != null
    isUninterrupted = hasContinuity AND trustVectors.continuity.uninterrupted

    IF hasLocation AND hasMotion AND isUninterrupted:
        RETURN Level A

    IF hasLocation AND hasMotion:
        RETURN Level B

    RETURN Level C
```

---

## 10. Error Taxonomy

CLI implementations MUST return these exact error identifiers:

| Error ID | Exit Code | User Message |
|----------|-----------|--------------|
| `hashMismatch` | 1 | "Audio has been modified since capture." |
| `signatureInvalid` | 2 | "Signature verification failed." |
| `manifestMalformed` | 3 | "Invalid proof file." |
| `schemaUnsupported` | 4 | "Proof format version X is not supported." |
| `audioFileMissing` | 5 | "Audio file not found." |
| `audioFileCorrupt` | 6 | "Audio file is corrupted." |
| `decryptionFailed` | 7 | "Could not decrypt. Check your password." |
| `bundleCorrupted` | 8 | "This file has been modified and cannot be opened." |
| `unsupportedBundleVersion` | 9 | "This sealed proof requires a newer app version." |

---

## 11. CLI Output Format

### 11.1 Verification Success

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
Audio Hash:  47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=

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

### 11.2 Verification Failure

```
PROOFAUDIO VERIFICATION SUMMARY
===============================
Status:      FAILED
Error:       Audio has been modified since capture.

The audio file does not match the cryptographic hash
recorded at capture time. This recording cannot be
verified as authentic.
```

### 11.3 Audio Extraction (Sealed Bundles)

The CLI supports extracting the audio file from sealed bundles after successful verification:

```bash
proofcapture-cli evidence.proofcapture --password "secret" --extract ./output/
```

Upon successful verification, this writes the audio file to the specified directory:
```
Verification passed!
Extracted audio to: ./output/recording.m4a
```

**Security Note:** The extracted audio file loses its cryptographic binding to the manifest once written to disk. For evidentiary purposes, always provide the original `.proofcapture` file.

---

## 12. Test Vectors

### 12.1 SHA-256 Test Vector

```
Input:  (empty string, 0 bytes)
Output: 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=
```

### 12.2 PBKDF2 Test Vector

```
Password:   "TestPassword123!"
Salt:       (32 bytes of 0x00)
Iterations: 600000
KeyLength:  32

Expected Key (hex): [To be generated with reference implementation]
```

### 12.3 Golden Bundle Request

A complete golden test bundle with known values should be generated by the iOS app and stored in the CLI repository. The bundle should include:

1. **Test manifest** with fixed values
2. **Test audio** (e.g., 1 second of silence)
3. **Test key pair** (non-Secure Enclave, fixed)
4. **Test sealed bundle** with known password

---

## 13. Implementation Checklist

### 13.1 Required Capabilities

- [ ] Parse JSON manifest
- [ ] Validate schema version
- [ ] Decode Base64 fields
- [ ] Compute SHA-256 hash of audio bytes
- [ ] Construct P-256 public key from raw 64 bytes
- [ ] Verify ECDSA signature with raw 64-byte format
- [ ] Canonicalize JSON (sorted keys, compact, ISO-8601 dates)
- [ ] Compute trust level from trust vectors
- [ ] Parse sealed bundle JSON
- [ ] Derive key with PBKDF2-HMAC-SHA256
- [ ] Decrypt AES-256-GCM combined format
- [ ] Parse decrypted payload JSON

### 13.2 Security Requirements

- [ ] Never write decrypted audio to disk
- [ ] Clear password from memory after use
- [ ] Fail closed on any verification error
- [ ] Validate all input lengths before processing
- [ ] Handle malformed input gracefully (no crashes)

---

## 14. Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-01 | Initial specification for Schema v1, Bundle v1 |

---

## 15. References

- `Core/Crypto/HashingService.swift` — Manifest canonicalization
- `Core/Crypto/KeyManager.swift` — Key and signature formats
- `Core/Encryption/EncryptionService.swift` — PBKDF2 and AES-GCM
- `Models/SignedAudioManifest.swift` — Manifest structure
- `Models/SealedProofBundle.swift` — Bundle structure
- `Docs/ARCHITECTURE.md` — System architecture
- `Docs/SECURITY_MODEL.md` — Security properties
