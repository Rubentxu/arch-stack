//! List installed archctl versions by scanning the `installs/` directory.

use anyhow::Result;
use std::path::Path;
use std::time::SystemTime;

/// Metadata for an installed version.
#[derive(Debug, Clone, serde::Serialize)]
#[allow(dead_code)] // Some fields reserved for T3 update metadata.
pub struct InstalledVersion {
    /// The semver version.
    pub version: semver::Version,
    /// When the version directory was last modified.
    pub installed_at: SystemTime,
    /// True if this version is currently active (pointed to by `current`).
    pub is_active: bool,
}

/// List all installed versions under `install_root/installs/`.
/// Returns them sorted by version descending (newest first).
pub fn list(install_root: &Path) -> Result<Vec<InstalledVersion>> {
    let installs_dir = install_root.join("installs");
    if !installs_dir.is_dir() {
        return Ok(Vec::new());
    }

    // Read the current symlink target (if it exists) to determine active version.
    let current_link = install_root.join("current");
    let active_target = if current_link.is_symlink() {
        std::fs::read_link(&current_link).ok()
    } else {
        None
    };

    let mut versions = Vec::new();
    for entry in std::fs::read_dir(&installs_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip non-directory entries.
        if !entry.file_type()?.is_dir() {
            continue;
        }

        // Expect "v1.2.3" — strip the leading 'v'.
        let version_str = name_str.strip_prefix('v').unwrap_or(&name_str);
        let Ok(version) = semver::Version::parse(version_str) else {
            continue;
        };

        let installed_at = entry.metadata()?.modified()?;

        // Check if this version is the active one.
        let is_active = active_target
            .as_ref()
            .and_then(|t| t.file_name())
            .map(|n| n == name)
            .unwrap_or(false);

        versions.push(InstalledVersion {
            version,
            installed_at,
            is_active,
        });
    }

    // Sort descending (newest first).
    versions.sort_by(|a, b| b.version.cmp(&a.version));
    Ok(versions)
}

// ---------------------------------------------------------------------------
// Tests — Strict TDD RED phase.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn list_empty_when_no_installs() {
        let tmp = tempfile::tempdir().unwrap();
        let versions = list(tmp.path()).unwrap();
        assert!(versions.is_empty());
    }

    #[test]
    fn list_returns_all_installed_versions() {
        let tmp = tempfile::tempdir().unwrap();
        for v in ["1.32.0", "1.33.0", "1.34.0"] {
            let dir = tmp.path().join(format!("installs/v{v}"));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("archctl"), b"mock").unwrap();
        }
        let versions = list(tmp.path()).unwrap();
        assert_eq!(versions.len(), 3);
        assert!(versions.iter().any(|v| v.version.to_string() == "1.32.0"));
        assert!(versions.iter().any(|v| v.version.to_string() == "1.33.0"));
        assert!(versions.iter().any(|v| v.version.to_string() == "1.34.0"));
    }

    #[test]
    fn list_marks_active_version() {
        let tmp = tempfile::tempdir().unwrap();
        let v1_dir = tmp.path().join("installs/v1.34.0");
        std::fs::create_dir_all(&v1_dir).unwrap();
        std::fs::write(v1_dir.join("archctl"), b"mock").unwrap();
        symlink(&v1_dir, tmp.path().join("current")).unwrap();
        let versions = list(tmp.path()).unwrap();
        let active: Vec<_> = versions.iter().filter(|v| v.is_active).collect();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].version.to_string(), "1.34.0");
    }

    #[test]
    fn list_sorted_newest_first() {
        let tmp = tempfile::tempdir().unwrap();
        for v in ["1.32.0", "1.34.0", "1.33.0"] {
            let dir = tmp.path().join(format!("installs/v{v}"));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("archctl"), b"mock").unwrap();
        }
        let versions = list(tmp.path()).unwrap();
        let vers: Vec<_> = versions.iter().map(|v| v.version.to_string()).collect();
        assert_eq!(vers, vec!["1.34.0", "1.33.0", "1.32.0"]);
    }
}
