# Homebrew Formula Maintenance Guide

This document describes how to update the Homebrew formula after each
`archctl` release.

## Overview

The `Formula/archctl.rb` file is a Homebrew-core formula that allows
`archctl` to be installed via:

```bash
brew install Rubentxu/arch-stack/archctl
```

## When to Update

After every **minor or major release** of `archctl` (e.g., v1.37.0 →
v1.38.0), the formula must be updated to point to the new release artifacts.

Patch releases (e.g., v1.37.0 → v1.37.1) typically do **not** require
a formula bump if the CLI API has not changed; Homebrew will cache the
 bottle indefinitely. When in doubt, update.

## Update Procedure

### Step 1: Identify the New Version

```bash
git tag -l "v*" --sort=-v:refname | head -5
```

### Step 2: Compute the SHA256 of the Release Artifact

Download the release artifact and compute its SHA256:

```bash
# Linux x86_64
curl -sL https://github.com/Rubentxu/arch-stack/releases/download/vNEW_VERSION/archctl-x86_64-unknown-linux-gnu.tar.gz | shasum -a 256

# macOS arm64 (darwin-aarch64) — add when available
curl -sL https://github.com/Rubentxu/arch-stack/releases/download/vNEW_VERSION/archctl-aarch64-apple-darwin.tar.gz | shasum -a 256
```

### Step 3: Update the Formula

Edit `Formula/archctl.rb`:

1. Change `version "OLD_VERSION"` to `version "NEW_VERSION"`.
2. Update the `url` line to point to the new release artifact.
3. Replace `sha256 "TODO_CI_FILLED"` with the computed hash.
4. If a macOS artifact is now available, add a second `url` + `sha256`
   pair under a `on_macos` block.

Example diff:

```diff
- version "1.36.0"
+ version "1.37.0"
- url "https://github.com/Rubentxu/arch-stack/releases/download/v1.36.0/archctl-x86_64-unknown-linux-gnu.tar.gz"
+ url "https://github.com/Rubentxu/arch-stack/releases/download/v1.37.0/archctl-x86_64-unknown-linux-gnu.tar.gz"
- # sha256 "TODO_CI_FILLED"
+ sha256 "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
```

### Step 4: Verify the Formula Syntax

```bash
brew audit Formula/archctl.rb
brew style Formula/archctl.rb
```

### Step 5: Test the Installation (Dry Run)

```bash
brew install --dry-run Formula/archctl.rb
```

### Step 6: Open a PR against Homebrew/homebrew-core

```bash
git checkout -b update-archctl-NEW_VERSION
git commit -m "archctl NEW_VERSION"
gh pr create --repo Homebrew/homebrew-core --title "archctl NEW_VERSION"
```

Note: Homebrew has strict review requirements. Review the
[Homebrew contributing guide](https://docs.brew.sh/Contributing-to-Homebrew)
before opening the PR.

## Release Checklist

Before each release, confirm:

- [ ] Release artifacts exist at `releases/download/vVERSION/`
- [ ] SHA256 hashes computed for all available platforms
- [ ] `Formula/archctl.rb` updated with new version + hash
- [ ] `brew audit` passes with no errors
- [ ] PR opened against Homebrew/homebrew-core (if applicable)

## Troubleshooting

### "Checksum mismatch"

Ensure you downloaded the **exact** artifact that will be served by GitHub
Releases (not a redirect target). Use `curl -L` to follow redirects.

### "SHA256 hash not found"

Homebrew requires an explicit `sha256` for every URL. If the hash is
missing or `TODO_CI_FILLED`, the formula will fail `brew audit`.

### "URL returned 404"

Verify the release tag exists on GitHub and the artifact filename matches
the URL pattern: `archctl-<target>.tar.gz`.
