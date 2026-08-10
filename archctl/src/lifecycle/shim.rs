use anyhow::{Context, Result};
use std::path::Path;

/// Bash shim that delegates to the active `archctl` binary in the
/// `current` symlink. Per spec/stack-distribution.md §shim.
pub fn generate_shim() -> String {
    r#"#!/usr/bin/env bash
# archctl shim — delegates to the active version installed via `archctl self`.
ARCHCTL_HOME="${ARCHCTL_HOME:-$HOME/.local/share/archctl}"
if [ -L "$ARCHCTL_HOME/current" ]; then
  exec "$ARCHCTL_HOME/current/archctl" "$@"
else
  echo "archctl: no active version installed. Run 'archctl self install' first." >&2
  exit 127
fi
"#
    .to_string()
}

/// Write the shim to `target` (typically `/usr/local/bin/archctl`).
///
/// Tries to write to the target directly; on permission denied, the
/// caller is expected to handle the fallback to `~/.local/bin/archctl`.
pub fn install_shim(target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create dir {}", parent.display()))?;
    }
    std::fs::write(target, generate_shim())
        .with_context(|| format!("write shim {}", target.display()))?;
    // Set executable bit (Unix only; Windows ignores this).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(target)
            .with_context(|| format!("stat shim {}", target.display()))?
            .permissions();
        perms.set_mode(perms.mode() | 0o755);
        std::fs::set_permissions(target, perms)
            .with_context(|| format!("chmod shim {}", target.display()))?;
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
    fn generate_shim_contains_archctl_home_resolution() {
        let shim = generate_shim();
        assert!(shim.contains("ARCHCTL_HOME"), "missing env var");
        assert!(shim.contains("current/archctl"), "missing binary path");
        assert!(shim.starts_with("#!/usr/bin/env bash"), "missing shebang");
    }

    #[test]
    fn install_shim_writes_executable_file() {
        let tmp = tempfile::tempdir().unwrap();
        // Use a sub-dir as target to simulate /usr/local/bin/.
        let target = tmp.path().join("archctl");
        install_shim(&target).unwrap();
        assert!(target.exists());
        let content = std::fs::read_to_string(&target).unwrap();
        assert!(content.starts_with("#!"));
        // Permission check: executable bit set (Unix).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::metadata(&target).unwrap().permissions();
            assert_ne!(perms.mode() & 0o111, 0, "shim not executable");
        }
    }
}
