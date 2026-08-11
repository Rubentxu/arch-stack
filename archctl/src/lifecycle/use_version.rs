//! Switch the active archctl version by updating the `current` symlink.

use anyhow::{Context, Result};
use std::path::Path;

use super::install_root::{current_symlink, install_dir};

/// Switch the `current` symlink to point to `target` version.
/// Errors if the target version is not installed.
pub fn use_version(target: &semver::Version, install_root: &Path) -> Result<()> {
    let target_dir = install_dir(install_root, target);

    if !target_dir.is_dir() {
        anyhow::bail!(
            "version {} is not installed ({} does not exist)",
            target,
            target_dir.display()
        );
    }

    let current = current_symlink(install_root);

    // Remove existing symlink if present.
    if current.exists() || current.is_symlink() {
        std::fs::remove_file(&current)
            .with_context(|| format!("remove existing {}", current.display()))?;
    }

    // Create new symlink.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target_dir, &current).with_context(|| {
            format!("symlink {} -> {}", current.display(), target_dir.display())
        })?;
    }
    #[cfg(not(unix))]
    {
        let _ = target; // suppress unused warning
        anyhow::bail!("use_version is only supported on Unix platforms");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests — Strict TDD RED phase.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_version_switches_symlink() {
        let tmp = tempfile::tempdir().unwrap();

        // Set up two installed versions.
        let v132_dir = tmp.path().join("installs/v1.32.0");
        std::fs::create_dir_all(&v132_dir).unwrap();
        std::fs::write(v132_dir.join("archctl"), b"v1.32").unwrap();

        let v134_dir = tmp.path().join("installs/v1.34.0");
        std::fs::create_dir_all(&v134_dir).unwrap();
        std::fs::write(v134_dir.join("archctl"), b"v1.34").unwrap();

        // Pre-existing current pointing to v1.32.0.
        std::os::unix::fs::symlink(&v132_dir, tmp.path().join("current")).unwrap();

        use_version(&semver::Version::parse("1.34.0").unwrap(), tmp.path()).unwrap();

        let target = std::fs::read_link(tmp.path().join("current")).unwrap();
        assert!(target.ends_with("v1.34.0"), "got: {}", target.display());
    }

    #[test]
    fn use_version_errors_on_missing_version() {
        let tmp = tempfile::tempdir().unwrap();
        let result = use_version(&semver::Version::parse("9.9.9").unwrap(), tmp.path());
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("9.9.9"));
    }
}
