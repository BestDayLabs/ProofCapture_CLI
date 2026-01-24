# Releasing proofaudio-cli

This document describes how to release a new version of proofaudio-cli.

## Prerequisites

- Push access to the repository
- SSH key configured for GitHub

## Release Process

### 1. Update Version Number

Edit `Cargo.toml` and bump the version:

```toml
version = "X.Y.Z"
```

Follow semantic versioning:
- **Patch (0.0.X)**: Bug fixes, documentation, minor improvements
- **Minor (0.X.0)**: New features, backward-compatible changes
- **Major (X.0.0)**: Breaking changes

### 2. Commit the Version Bump

```bash
cd /Users/michaeltaylor/AppBuild/proofaudio-cli
git add Cargo.toml Cargo.lock
git commit -m "Bump version to X.Y.Z"
```

### 3. Push Changes

```bash
git push origin main
```

### 4. Create and Push Tag

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

### 5. Monitor the Release

The GitHub Actions workflow will automatically:
1. Build binaries for all platforms:
   - macOS ARM64 (Apple Silicon)
   - macOS Intel (x86_64)
   - Linux x64
   - Windows x64
2. Create zip files for each binary
3. Publish the release at https://github.com/BestDayLabs/proofaudio-cli/releases

Monitor progress at: https://github.com/BestDayLabs/proofaudio-cli/actions

### 6. Verify the Release

Once complete, verify at https://github.com/BestDayLabs/proofaudio-cli/releases that:
- All 4 platform zips are attached
- Release notes are generated
- Version tag matches

## Current Release: v0.2.1

```bash
# Complete commands for v0.2.1 release:
cd /Users/michaeltaylor/AppBuild/proofaudio-cli
git push origin main
git tag v0.2.1
git push origin v0.2.1
```

## Rollback

If a release has issues:

```bash
# Delete the tag locally and remotely
git tag -d vX.Y.Z
git push origin --delete vX.Y.Z

# Delete the GitHub release manually via web UI
# Then fix issues and re-release
```
