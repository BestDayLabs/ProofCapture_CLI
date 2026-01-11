# ProofAudio CLI Verifier — Implementation Plan

## Overview

A cross-platform command-line tool for verifying ProofAudio recordings. Enables third-party verification without requiring the iOS app.

**Repository:** `proofaudio-cli`
**Language:** Rust (recommended) or Go
**License:** MIT or Apache 2.0

---

## Quick Start (For New Repo)

```bash
# Create repo
mkdir proofaudio-cli && cd proofaudio-cli
git init

# If Rust:
cargo init --name proofaudio-cli

# If Go:
go mod init github.com/bestdaylabs/proofaudio-cli

# Copy these files from iOS repo:
cp /path/to/ProofAudio/Docs/CLI_INTEROPERABILITY_SPEC.md docs/
cp /path/to/ProofAudio/Docs/CLI_VERIFIER_REQUIREMENTS.md docs/
```

---

## Project Structure

### Rust
```
proofaudio-cli/
├── Cargo.toml
├── README.md
├── LICENSE
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library exports
│   ├── verify.rs            # Verification logic
│   ├── manifest.rs          # Manifest parsing & canonicalization
│   ├── crypto.rs            # SHA-256, ECDSA, AES-GCM
│   ├── sealed.rs            # Sealed bundle handling
│   ├── trust.rs             # Trust level computation
│   └── error.rs             # Error types
├── docs/
│   ├── CLI_INTEROPERABILITY_SPEC.md
│   └── CLI_VERIFIER_REQUIREMENTS.md
├── fixtures/                # Golden test vectors
│   ├── standard_bundle/
│   ├── sealed_bundle/
│   └── test_vectors.json
└── tests/
    ├── verification_tests.rs
    ├── crypto_tests.rs
    └── integration_tests.rs
```

### Go
```
proofaudio-cli/
├── go.mod
├── go.sum
├── README.md
├── LICENSE
├── cmd/
│   └── proofaudio-cli/
│       └── main.go          # CLI entry point
├── pkg/
│   ├── verify/              # Verification logic
│   ├── manifest/            # Manifest parsing
│   ├── crypto/              # Cryptographic operations
│   ├── sealed/              # Sealed bundle handling
│   └── trust/               # Trust level computation
├── docs/
├── fixtures/
└── tests/
```

---

## Recommended Dependencies

### Rust
```toml
[dependencies]
# Crypto
sha2 = "0.10"           # SHA-256
p256 = "0.13"           # P-256 ECDSA
aes-gcm = "0.10"        # AES-256-GCM
pbkdf2 = "0.12"         # Key derivation
hmac = "0.12"           # For PBKDF2

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
base64 = "0.21"

# CLI
clap = { version = "4.0", features = ["derive"] }

# Utilities
thiserror = "1.0"       # Error handling
anyhow = "1.0"          # Result handling
```

### Go
```go
// Standard library covers most needs:
// - crypto/sha256
// - crypto/ecdsa + crypto/elliptic
// - crypto/aes + crypto/cipher
// - encoding/json
// - encoding/base64

// External:
// golang.org/x/crypto/pbkdf2
// github.com/spf13/cobra (CLI)
```

---

## Implementation Phases

### Phase 1: Core Cryptography (Week 1)

**Goal:** Verify a standard bundle with known test data.

| Task | Description | Test |
|------|-------------|------|
| SHA-256 hashing | Hash audio bytes | Compare with known hash |
| P-256 key parsing | Parse raw 64-byte public key | Reconstruct and verify |
| ECDSA verification | Verify signature over hash | Use test vector |
| JSON canonicalization | Sort keys, compact output | Byte-exact comparison |

**Deliverable:** `verify_standard_bundle()` works with test fixture.

### Phase 2: Manifest Handling (Week 1-2)

**Goal:** Parse and validate real manifests.

| Task | Description |
|------|-------------|
| Schema validation | Check version, required fields |
| Date parsing | ISO-8601 with fractional seconds |
| Trust vector parsing | Location, motion, continuity, clock |
| Manifest-for-hashing | Exclude signature, sort keys |

**Deliverable:** Parse manifest from iOS-generated bundle.

### Phase 3: Sealed Bundle Support (Week 2)

**Goal:** Decrypt and verify sealed bundles.

| Task | Description |
|------|-------------|
| Bundle JSON parsing | Parse outer wrapper |
| PBKDF2 key derivation | 600K iterations, HMAC-SHA256 |
| AES-GCM decryption | Combined format (nonce+ct+tag) |
| Payload extraction | Decode audio and manifest |

**Deliverable:** `unseal_and_verify()` works with test password.

### Phase 4: CLI Interface (Week 2-3)

**Goal:** User-friendly command-line interface.

| Task | Description |
|------|-------------|
| Argument parsing | `verify <path>` with options |
| Password input | `--password` or secure prompt |
| Output formatting | Human-readable summary |
| Exit codes | Per error taxonomy |

**Deliverable:** Complete CLI matching spec output format.

### Phase 5: Testing & Release (Week 3)

**Goal:** Production-ready release.

| Task | Description |
|------|-------------|
| Unit tests | All crypto operations |
| Integration tests | Real iOS-generated bundles |
| Edge case tests | Corrupted, truncated, malformed |
| Binary builds | macOS, Windows, Linux |
| Documentation | README, installation guide |

---

## Critical Implementation Notes

### 1. Public Key Format

iOS exports raw 64 bytes. Prepend `0x04` for SEC1 parsing:

```rust
// Rust
fn parse_public_key(raw: &[u8; 64]) -> Result<PublicKey> {
    let mut sec1 = vec![0x04];
    sec1.extend_from_slice(raw);
    PublicKey::from_sec1_bytes(&sec1)
}
```

```go
// Go
func parsePublicKey(raw []byte) (*ecdsa.PublicKey, error) {
    uncompressed := append([]byte{0x04}, raw...)
    x, y := elliptic.Unmarshal(elliptic.P256(), uncompressed)
    return &ecdsa.PublicKey{Curve: elliptic.P256(), X: x, Y: y}, nil
}
```

### 2. Signature Format

iOS exports raw 64 bytes (r || s). Convert for verification:

```rust
// Rust (p256 crate uses raw format directly)
let signature = Signature::from_slice(signature_bytes)?;
```

```go
// Go
r := new(big.Int).SetBytes(sig[:32])
s := new(big.Int).SetBytes(sig[32:])
valid := ecdsa.Verify(pubKey, hash, r, s)
```

### 3. JSON Canonicalization

Must match Swift's `JSONEncoder` with `.sortedKeys`:

```rust
// Rust - use serde_json with sorted maps
let mut map = serde_json::Map::new();
// Insert fields alphabetically OR sort after
let canonical = serde_json::to_string(&map)?; // No pretty print
```

```go
// Go - json.Marshal sorts keys by default for maps
// But structs maintain field order, so use map[string]interface{}
```

### 4. AES-GCM Combined Format

The encrypted payload is: `nonce (12) || ciphertext || tag (16)`

```rust
// Rust (aes-gcm)
let nonce = Nonce::from_slice(&encrypted[..12]);
let ciphertext_with_tag = &encrypted[12..];
cipher.decrypt(nonce, ciphertext_with_tag)?
```

---

## Test Fixtures Needed

Generate from iOS app:

### 1. Minimal Test Bundle (Level C)
```
fixtures/minimal/
├── recording.m4a      # 1 second silence
├── manifest.json      # No trust vectors
└── expected.json      # Expected verification result
```

### 2. Full Test Bundle (Level A)
```
fixtures/full/
├── recording.m4a      # Short audio
├── manifest.json      # All trust vectors
└── expected.json
```

### 3. Sealed Bundle
```
fixtures/sealed/
├── test.proofaudio    # Password: "TestPassword123!"
└── expected.json
```

### 4. Test Vectors JSON
```json
{
  "sha256": {
    "input_hex": "",
    "output_base64": "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU="
  },
  "pbkdf2": {
    "password": "TestPassword123!",
    "salt_hex": "00000000...",
    "iterations": 600000,
    "key_hex": "..."
  },
  "ecdsa": {
    "public_key_base64": "...",
    "message_hash_hex": "...",
    "signature_base64": "...",
    "valid": true
  }
}
```

---

## CLI Usage Examples

```bash
# Verify standard bundle
proofaudio-cli verify ./MyRecording/
proofaudio-cli verify ./bundle.zip

# Verify sealed bundle (password prompt)
proofaudio-cli verify evidence.proofaudio

# Verify sealed bundle (password argument)
proofaudio-cli verify evidence.proofaudio --password "secret"

# JSON output for scripting
proofaudio-cli verify ./bundle/ --format json

# Verbose output
proofaudio-cli verify ./bundle/ --verbose
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Verification succeeded |
| 1 | Hash mismatch (audio modified) |
| 2 | Signature invalid |
| 3 | Manifest malformed |
| 4 | Schema unsupported |
| 5 | Audio file missing |
| 6 | Audio file corrupt |
| 7 | Decryption failed (wrong password) |
| 8 | Bundle corrupted |
| 9 | Bundle version unsupported |

---

## README Template

```markdown
# ProofAudio CLI Verifier

Verify ProofAudio recordings from the command line.

## Installation

### macOS (Homebrew)
\`\`\`bash
brew install bestdaylabs/tap/proofaudio-cli
\`\`\`

### Download Binary
Download from [Releases](https://github.com/bestdaylabs/proofaudio-cli/releases).

### Build from Source
\`\`\`bash
cargo install proofaudio-cli
# or
go install github.com/bestdaylabs/proofaudio-cli@latest
\`\`\`

## Usage

\`\`\`bash
# Verify a recording
proofaudio-cli verify recording_bundle/

# Verify a sealed proof
proofaudio-cli verify evidence.proofaudio --password "shared-secret"
\`\`\`

## What This Verifies

- Audio file has not been modified since capture
- Recording was made by the ProofAudio iOS app
- Cryptographic signature is valid

## What This Does NOT Verify

- Who is speaking
- That statements are true
- Legal consent to record
- Absence of AI-generated audio played into microphone

## License

MIT
```

---

## Checklist Before First Release

- [ ] All crypto operations match iOS implementation
- [ ] Verifies iOS-generated standard bundles
- [ ] Verifies iOS-generated sealed bundles
- [ ] All exit codes implemented
- [ ] Human-readable output format
- [ ] JSON output option
- [ ] Unit tests pass
- [ ] Integration tests with real bundles
- [ ] Binaries built for macOS (Intel + ARM), Windows, Linux
- [ ] README complete
- [ ] LICENSE file present
- [ ] GitHub Actions CI configured

---

## References

- `CLI_INTEROPERABILITY_SPEC.md` — Exact byte formats and algorithms
- `CLI_VERIFIER_REQUIREMENTS.md` — Original requirements
- iOS Source: `Core/Crypto/`, `Core/Encryption/`, `Models/`
