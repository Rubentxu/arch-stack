use std::path::{Path, PathBuf};

/// Default install root: `~/.local/share/archctl`. Override with
/// `ARCHCTL_HOME` env var.
pub fn install_root() -> PathBuf {
    if let Some(home) = std::env::var_os("ARCHCTL_HOME") {
        PathBuf::from(home)
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("archctl")
    } else {
        PathBuf::from(".local/share/archctl")
    }
}

/// Per-version install dir: `<root>/installs/v<version>/`
pub fn install_dir(root: &Path, version: &semver::Version) -> PathBuf {
    root.join("installs").join(format!("v{}", version))
}

/// Symlink to the active version: `<root>/current`
pub fn current_symlink(root: &Path) -> PathBuf {
    root.join("current")
}

// ---------------------------------------------------------------------------
// Tests — Strict TDD RED phase: these define the expected behaviour first.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn clean_env<F: FnOnce() -> R, R>(key: &str, f: F) -> R {
        // Save current value, clear, run, restore.
        // SAFETY: tests in this module are #[test]-annotated so they run
        // serially via cargo test's default harness (--test-threads=1).
        // `set_var`/`remove_var` are `unsafe` in Rust 2024 because they
        // mutate process-global state; the harness isolation makes the
        // call safe in this context.
        let prev = std::env::var_os(key);
        unsafe { std::env::remove_var(key) };
        let result = f();
        if let Some(p) = prev {
            unsafe { std::env::set_var(key, p) };
        }
        result
    }

    #[test]
    fn install_root_default_is_xdg_local_share() {
        clean_env("ARCHCTL_HOME", || {
            let root = install_root();
            assert!(
                root.ends_with(".local/share/archctl"),
                "got: {}",
                root.display()
            );
        });
    }

    #[test]
    fn install_root_respects_archctl_home_env() {
        clean_env("ARCHCTL_HOME", || {
            // Use a temp dir to avoid polluting HOME.
            let tmp = tempfile::tempdir().unwrap();
            // SAFETY: see `clean_env` — test serial via --test-threads=1.
            unsafe { std::env::set_var("ARCHCTL_HOME", tmp.path()) };
            assert_eq!(install_root(), tmp.path());
        });
    }

    #[test]
    fn install_dir_per_version() {
        let tmp = tempfile::tempdir().unwrap();
        let version = semver::Version::parse("1.34.0").unwrap();
        let dir = install_dir(tmp.path(), &version);
        assert!(dir.ends_with("installs/v1.34.0"), "got: {}", dir.display());
    }

    #[test]
    fn current_symlink_path() {
        let tmp = tempfile::tempdir().unwrap();
        let link = current_symlink(tmp.path());
        assert!(link.ends_with("current"), "got: {}", link.display());
    }
}
