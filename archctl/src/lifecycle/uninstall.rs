//! Uninstall archctl versions: specific version removal or purge all.

use anyhow::{Context, Result};
use std::path::Path;

use super::install_root::{current_symlink, install_dir};

/// Uninstall archctl versions.
///
/// - `target = Some(version)`: removes that specific version directory.
/// - `target = None`: removes all installed versions only when `purge = true`.
/// - `purge = true`: removes the entire `installs/` directory and the `current`
///   symlink (full cleanup).
/// - `purge = false` with `target = None`: no-op (safer default).
pub fn uninstall(target: Option<&semver::Version>, install_root: &Path, purge: bool) -> Result<()> {
    let installs_dir = install_root.join("installs");

    if purge {
        // Remove everything: all versions + current symlink.
        if installs_dir.exists() {
            std::fs::remove_dir_all(&installs_dir)
                .with_context(|| format!("remove installs dir {}", installs_dir.display()))?;
        }
        let current = current_symlink(install_root);
        if current.is_symlink() || current.exists() {
            std::fs::remove_file(&current)
                .with_context(|| format!("remove current symlink {}", current.display()))?;
        }
        return Ok(());
    }

    // Selective uninstall: remove a specific version.
    let Some(version) = target else {
        // purge=false + no target = nothing to do.
        return Ok(());
    };

    let version_dir = install_dir(install_root, version);
    if version_dir.is_dir() {
        std::fs::remove_dir_all(&version_dir)
            .with_context(|| format!("remove {}", version_dir.display()))?;
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
    fn uninstall_removes_specific_version() {
        let tmp = tempfile::tempdir().unwrap();
        let v132_dir = tmp.path().join("installs/v1.32.0");
        std::fs::create_dir_all(&v132_dir).unwrap();
        std::fs::write(v132_dir.join("archctl"), b"mock").unwrap();

        uninstall(
            Some(&semver::Version::parse("1.32.0").unwrap()),
            tmp.path(),
            false,
        )
        .unwrap();

        assert!(!v132_dir.exists(), "v1.32.0 should be removed");
    }

    #[test]
    fn uninstall_purge_removes_all() {
        let tmp = tempfile::tempdir().unwrap();
        for v in ["1.32.0", "1.33.0"] {
            let dir = tmp.path().join(format!("installs/v{v}"));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("archctl"), b"mock").unwrap();
        }

        uninstall(None, tmp.path(), true).unwrap();

        assert!(
            !tmp.path().join("installs").exists(),
            "installs dir should be removed"
        );
        assert!(
            !tmp.path().join("current").exists(),
            "current symlink should be removed"
        );
    }

    #[test]
    fn uninstall_non_purge_keeps_other_versions() {
        let tmp = tempfile::tempdir().unwrap();
        for v in ["1.32.0", "1.34.0"] {
            let dir = tmp.path().join(format!("installs/v{v}"));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("archctl"), b"mock").unwrap();
        }

        uninstall(
            Some(&semver::Version::parse("1.32.0").unwrap()),
            tmp.path(),
            false,
        )
        .unwrap();

        assert!(!tmp.path().join("installs/v1.32.0").exists(), "v1.32.0 removed");
        assert!(tmp.path().join("installs/v1.34.0").exists(), "v1.34.0 kept");
    }

    #[test]
    fn uninstall_none_with_no_purge_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let v_dir = tmp.path().join("installs/v1.34.0");
        std::fs::create_dir_all(&v_dir).unwrap();
        std::fs::write(v_dir.join("archctl"), b"mock").unwrap();

        uninstall(None, tmp.path(), false).unwrap();

        // Nothing removed.
        assert!(v_dir.exists());
    }
}
