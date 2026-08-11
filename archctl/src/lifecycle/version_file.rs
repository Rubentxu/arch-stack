//! `.arch-version` file walking and version resolution.
//!
//! Per spec, the resolution precedence is:
//! 1. `--archctl-version` flag (highest)
//! 2. `ARCHCTL_VERSION` env var
//! 3. `.arch-version` file walking up from cwd to $HOME (or /)
//! 4. `current` symlink in install_root (lowest)

use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

/// Walk from `cwd` upward looking for `.arch-version`. Returns the first
/// semver found (skipping comments and blank lines), or None.
pub fn find_arch_version(cwd: &Path) -> Option<semver::Version> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut current = cwd.canonicalize().ok()?;
    loop {
        let candidate = current.join(".arch-version");
        if candidate.is_file()
            && let Some(v) = read_version_file(&candidate)
        {
            return Some(v);
        }
        if Some(&current) == home.as_ref() {
            return None;
        }
        current = current.parent()?.to_path_buf();
    }
}

fn read_version_file(path: &Path) -> Option<semver::Version> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Ok(v) = semver::Version::parse(trimmed) {
            return Some(v);
        }
    }
    None
}

/// Resolve which version should be used. Precedence (high to low):
/// 1. `flag_override` (e.g. --archctl-version X.Y.Z)
/// 2. `env_override` (ARCHCTL_VERSION env var)
/// 3. `.arch-version` in cwd (walking up to HOME)
/// 4. `current` symlink in install_root
pub fn resolve_active_version(
    flag_override: Option<&semver::Version>,
    env_override: Option<&str>,
    cwd: &Path,
    install_root: &Path,
) -> Result<semver::Version> {
    if let Some(v) = flag_override {
        return Ok(v.clone());
    }
    if let Some(s) = env_override
        && let Ok(v) = semver::Version::parse(s)
    {
        return Ok(v);
    }
    if let Some(v) = find_arch_version(cwd) {
        return Ok(v);
    }
    let current_link = install_root.join("current");
    if current_link.is_symlink()
        && let Ok(target) = std::fs::read_link(&current_link)
    {
        // target is relative like "installs/v1.34.0"; extract version.
        if let Some(name) = target.file_name().and_then(|n| n.to_str())
            && let Some(v_str) = name.strip_prefix('v')
            && let Ok(v) = semver::Version::parse(v_str)
        {
            return Ok(v);
        }
    }
    Err(anyhow!(
        "no active version installed. Run 'archctl self install' first."
    ))
}

// ---------------------------------------------------------------------------
// Tests — Strict TDD RED phase.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_arch_version_in_cwd() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".arch-version"), "1.34.0\n").unwrap();
        let version = find_arch_version(tmp.path()).unwrap();
        assert_eq!(version.to_string(), "1.34.0");
    }

    #[test]
    fn find_arch_version_walks_up() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".arch-version"), "1.32.0\n").unwrap();
        let child = tmp.path().join("a/b/c");
        std::fs::create_dir_all(&child).unwrap();
        let version = find_arch_version(&child).unwrap();
        assert_eq!(version.to_string(), "1.32.0");
    }

    #[test]
    fn find_arch_version_stops_at_home() {
        let tmp = tempfile::tempdir().unwrap();
        // No .arch-version anywhere; should return None.
        assert!(find_arch_version(tmp.path()).is_none());
    }

    #[test]
    fn find_arch_version_ignores_comments_and_blanks() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(".arch-version"),
            "# pinned version\n1.34.0\n\n",
        )
        .unwrap();
        let version = find_arch_version(tmp.path()).unwrap();
        assert_eq!(version.to_string(), "1.34.0");
    }

    #[test]
    fn resolve_active_version_precedence_flag_over_env_over_file() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".arch-version"), "1.32.0\n").unwrap();
        std::fs::create_dir_all(tmp.path().join("installs/v1.32.0")).unwrap();
        std::fs::create_dir_all(tmp.path().join("installs/v1.34.0")).unwrap();
        let result = resolve_active_version(
            Some(&semver::Version::parse("9.9.9").unwrap()), // flag wins
            None,
            tmp.path(),
            tmp.path(),
        )
        .unwrap();
        assert_eq!(result.to_string(), "9.9.9");
    }

    #[test]
    fn resolve_active_version_falls_back_to_current_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("installs/v1.34.0")).unwrap();
        std::os::unix::fs::symlink(
            tmp.path().join("installs/v1.34.0"),
            tmp.path().join("current"),
        )
        .unwrap();
        let result = resolve_active_version(None, None, tmp.path(), tmp.path()).unwrap();
        assert_eq!(result.to_string(), "1.34.0");
    }
}
