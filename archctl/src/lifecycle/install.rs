//! Self-install implementation: copies the current binary to a versioned directory
//! and creates the `current` symlink on first install.

use anyhow::{Context, Result};
use std::path::Path;

use super::install_root::{current_symlink, install_dir};

/// Install archctl `version` from `source_bin` into `install_root`.
/// Copies the binary to `<install_root>/installs/v<version>/archctl` and creates
/// a `current` symlink pointing to it (only on first install).
///
/// # Errors
/// Fails if the source binary cannot be read or the destination directory
/// cannot be created.
pub fn install(version: &semver::Version, install_root: &Path, source_bin: &Path) -> Result<()> {
    let dest_dir = install_dir(install_root, version);
    let dest_bin = dest_dir.join("archctl");

    // Create the versioned directory.
    std::fs::create_dir_all(&dest_dir)
        .with_context(|| format!("create install dir {}", dest_dir.display()))?;

    // Copy the binary.
    std::fs::copy(source_bin, &dest_bin)
        .with_context(|| format!("copy {} to {}", source_bin.display(), dest_bin.display()))?;

    // Create `current` symlink only if it does not already exist.
    let current = current_symlink(install_root);
    if !current.exists() {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&dest_dir, &current).with_context(|| {
                format!("symlink {} -> {}", current.display(), dest_dir.display())
            })?;
        }
        #[cfg(not(unix))]
        {
            // On non-Unix we fall back to copying — less efficient but functional.
            std::fs::copy(&dest_bin, install_root.join("archctl"))
                .with_context(|| "copy binary for current")?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests — Strict TDD RED phase.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn install_copies_binary_to_versioned_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let src_bin = tmp.path().join("source_archctl");
        std::fs::write(&src_bin, b"#!/bin/sh\necho mock\n").unwrap();
        let version = semver::Version::parse("1.34.0").unwrap();
        install(&version, tmp.path(), &src_bin).unwrap();
        let installed = tmp.path().join("installs/v1.34.0/archctl");
        assert!(
            installed.exists(),
            "binary not installed at {}",
            installed.display()
        );
        let content = std::fs::read_to_string(&installed).unwrap();
        assert!(content.contains("mock"));
    }

    #[test]
    fn install_creates_current_symlink_on_first_install() {
        let tmp = tempfile::tempdir().unwrap();
        let src_bin = tmp.path().join("src");
        std::fs::write(&src_bin, b"mock").unwrap();
        let version = semver::Version::parse("1.34.0").unwrap();
        install(&version, tmp.path(), &src_bin).unwrap();
        let current = tmp.path().join("current");
        assert!(current.exists(), "current symlink not created");
    }

    #[test]
    fn install_does_not_overwrite_current_on_existing() {
        // If current already points to another version, don't move it.
        let tmp = tempfile::tempdir().unwrap();
        let prev_dir = tmp.path().join("installs/v1.32.0");
        std::fs::create_dir_all(&prev_dir).unwrap();
        let prev = prev_dir.join("archctl");
        std::fs::write(&prev, b"old").unwrap();
        symlink(&prev_dir, tmp.path().join("current")).unwrap();
        let src_bin = tmp.path().join("src");
        std::fs::write(&src_bin, b"new").unwrap();
        let new_version = semver::Version::parse("1.34.0").unwrap();
        install(&new_version, tmp.path(), &src_bin).unwrap();
        // current should still point to v1.32.0
        let target = std::fs::read_link(tmp.path().join("current")).unwrap();
        assert!(
            target.ends_with("v1.32.0"),
            "current should stay at v1.32.0, got: {}",
            target.display()
        );
    }
}
