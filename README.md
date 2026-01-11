# ProofAudio CLI Verifier

Verify ProofAudio recordings from the command line.

## Installation

### Build from Source

```bash
cargo build --release
```

The binary will be at `target/release/proofaudio-cli`.

### Download Binary

Download from [Releases](https://github.com/bestdaylabs/proofaudio-cli/releases).

## Usage

```bash
# Verify a standard proof bundle (directory)
proofaudio-cli ./recording_bundle/

# Verify a sealed proof (will prompt for password)
proofaudio-cli evidence.proofaudio

# Verify with password on command line
proofaudio-cli evidence.proofaudio --password "shared-secret"

# JSON output for scripting
proofaudio-cli ./bundle/ --format json

# Verbose output with audio hash
proofaudio-cli ./bundle/ --verbose
```

## Output

### Successful Verification

```
PROOFAUDIO VERIFICATION SUMMARY
===============================
Status:      VERIFIED
Trust Level: Level A (Verified Continuous Capture)

RECORDING DETAILS
-----------------
Captured:    2024-01-15T10:30:00Z
Duration:    135.0s
Format:      AAC (M4A container)
Size:        1234567 bytes

CRYPTOGRAPHIC IDENTITY
----------------------
Device Key:  a1b2c3d4e5f6...
App:         com.bestdaylabs.proofaudio v1.0.0

TRUST VECTORS
-------------
Location:    37.775, -122.418 (+/- 65m)
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

### Failed Verification

```
PROOFAUDIO VERIFICATION SUMMARY
===============================
Status:      FAILED
Error:       Audio has been modified since capture.

The audio file does not match the cryptographic hash
recorded at capture time. This recording cannot be
verified as authentic.
```

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

## What This Verifies

- Audio file has not been modified since capture
- Recording was made by the ProofAudio iOS app
- Cryptographic signature is valid
- Trust vectors (location, motion, continuity) if present

## What This Does NOT Verify

- Who is speaking
- That statements are true
- Legal consent to record
- Absence of AI-generated audio played into microphone

## Technical Details

See [docs/CLI_INTEROPERABILITY_SPEC.md](docs/CLI_INTEROPERABILITY_SPEC.md) for the complete technical specification.

## License

MIT
