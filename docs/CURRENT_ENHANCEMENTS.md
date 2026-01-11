# Current Enhancements — ProofAudio-cli

Best Day Labs

This document tracks approved enhancements and their implementation status.

---

## Completed Enhancements

### 1. Display End Location in CLI

**Task:** Add ending location to CLI output - it previously only displayed the starting location

**Purpose:** Leverage available trust vectors

**Status:** Completed

**Changes:**
- Text output: `Location: 37.775, -122.418 → 37.775, -122.418 (+/- 65m)`
- JSON output: Added `endLat`, `endLon`, `endAccuracy` fields

---

### 2. Audio Extraction from Sealed Bundles

**Task:** Add `--extract` flag to save audio from sealed bundles after verification

**Purpose:** Allow users to listen to verified recordings without keeping them only in encrypted form

**Status:** Completed

**Changes:**
- Added `--extract <DIR>` CLI argument
- Created `verify_and_extract_sealed_bundle()` function in verify.rs
- Audio is only written after successful verification
- Security note added to documentation

**Usage:**
```bash
proofaudio-cli evidence.proofaudio --password "secret" --extract ./output/
```

---


## Open Enhancements



## Enhancement Request Template

When adding new enhancements, use this format:

```
### [Number]. [Enhancement Title]

**Task:** [Brief description of what needs to change]

**Purpose:** [Why this change improves the product]

**Status:** Pending | In Progress | Completed

**Files to Update:**
- [List of affected files]

**Cross-Agent Review Required:**
- [ ] PRODUCT
- [ ] ARCHITECTURE
- [ ] SECURITY (if crypto-related)
- [ ] PRIVACY (if data-related)
- [ ] COPY-REVIEWER (if user-facing text)
- [ ] ACCESSIBILITY (if UI-related)
- [ ] iOS-DEV
```
