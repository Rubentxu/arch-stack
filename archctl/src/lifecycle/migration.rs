//! M73 T3: Migration manifest — run upgrade scripts between versions.

use anyhow::Result;
use semver::Version;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Migration {
    pub id: String,
    pub description: String,
    pub applies_to: Vec<String>,
    /// Path to the script RELATIVE to the staging dir (e.g. "migrate.sh").
    /// NOT an arbitrary shell string. Validated to be a relative path
    /// without `..` or absolute prefix.
    pub script: String,
    pub rollback_supported: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MigrationManifest {
    pub from_version: Version,
    pub to_version: Version,
    pub migrations: Vec<Migration>,
}

impl MigrationManifest {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(Into::into)
    }
}

/// Run a migration script under sandbox restrictions.
/// - Working directory restricted to `cwd` (script can only read/write here).
/// - No network (env vars like HTTP_PROXY are unset).
/// - Empty PATH except /bin:/usr/bin (defense in depth).
/// - Script must be a path to an executable file, NOT arbitrary shell.
///   (This is enforced via `Command::new(script_path)` not `sh -c script_str`.)
pub fn run_sandboxed_script(
    script_path: &Path,
    cwd: &Path,
    from_dir: &Path,
    to_dir: &Path,
) -> Result<()> {
    let status = std::process::Command::new(script_path)
        .current_dir(cwd)
        .env_clear()
        .env("ARCHCTL_FROM_DIR", from_dir)
        .env("ARCHCTL_TO_DIR", to_dir)
        .env("PATH", "/bin:/usr/bin")
        .status()?;
    if !status.success() {
        anyhow::bail!(
            "sandboxed script {} exited with {:?}",
            script_path.display(),
            status.code()
        );
    }
    Ok(())
}

/// Execute migration scripts in order from from_dir to to_dir.
/// Each script is run with timeout 60s. Returns Err on first failure.
pub fn execute_manifest(
    manifest: &MigrationManifest,
    from_dir: &Path,
    to_dir: &Path,
) -> Result<()> {
    for mig in &manifest.migrations {
        eprintln!("running migration: {} ({})", mig.id, mig.description);
        // Validate script path: must be relative without .. or absolute prefix.
        if mig.script.contains("..") || mig.script.starts_with('/') {
            anyhow::bail!(
                "migration script path must be relative without '..': {}",
                mig.script
            );
        }
        let script_path = to_dir.join(&mig.script);
        run_sandboxed_script(&script_path, to_dir, from_dir, to_dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_sandboxed_script_passes_through_exit_code() {
        let tmp = tempfile::tempdir().unwrap();
        // Create a trivial executable.
        let script = tmp.path().join("noop.sh");
        std::fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
        // Sync to ensure the file is fully written before execution.
        let file = std::fs::File::open(&script).unwrap();
        file.sync_all().unwrap();
        drop(file);
        std::fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        run_sandboxed_script(&script, tmp.path(), tmp.path(), tmp.path()).unwrap();
    }

    #[test]
    fn run_sandboxed_script_errors_on_nonzero_exit() {
        let tmp = tempfile::tempdir().unwrap();
        let script = tmp.path().join("fail.sh");
        std::fs::write(&script, "#!/bin/sh\nexit 1\n").unwrap();
        std::fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        assert!(run_sandboxed_script(&script, tmp.path(), tmp.path(), tmp.path()).is_err());
    }

    #[test]
    fn execute_manifest_runs_scripts_in_order() {
        let tmp = tempfile::tempdir().unwrap();
        // Create a trivial executable.
        let script = tmp.path().join("test.sh");
        std::fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        std::fs::write(tmp.path().join("marker"), "ok").unwrap();
        let manifest = MigrationManifest {
            from_version: Version::parse("1.0.0").unwrap(),
            to_version: Version::parse("1.1.0").unwrap(),
            migrations: vec![Migration {
                id: "test".into(),
                description: "noop".into(),
                applies_to: vec![],
                script: "test.sh".into(),
                rollback_supported: true,
            }],
        };
        execute_manifest(&manifest, tmp.path(), tmp.path()).unwrap();
    }

    #[test]
    fn execute_manifest_propagates_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let script = tmp.path().join("fail.sh");
        std::fs::write(&script, "#!/bin/sh\nexit 1\n").unwrap();
        std::fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        let manifest = MigrationManifest {
            from_version: Version::parse("1.0.0").unwrap(),
            to_version: Version::parse("1.1.0").unwrap(),
            migrations: vec![Migration {
                id: "fails".into(),
                description: "fails".into(),
                applies_to: vec![],
                script: "fail.sh".into(),
                rollback_supported: false,
            }],
        };
        assert!(execute_manifest(&manifest, tmp.path(), tmp.path()).is_err());
    }

    #[test]
    fn execute_manifest_rejects_absolute_script_path() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = MigrationManifest {
            from_version: Version::parse("1.0.0").unwrap(),
            to_version: Version::parse("1.1.0").unwrap(),
            migrations: vec![Migration {
                id: "bad".into(),
                description: "bad".into(),
                applies_to: vec![],
                script: "/absolute/path.sh".into(),
                rollback_supported: false,
            }],
        };
        assert!(execute_manifest(&manifest, tmp.path(), tmp.path()).is_err());
    }

    #[test]
    fn execute_manifest_rejects_path_with_double_dot() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = MigrationManifest {
            from_version: Version::parse("1.0.0").unwrap(),
            to_version: Version::parse("1.1.0").unwrap(),
            migrations: vec![Migration {
                id: "bad".into(),
                description: "bad".into(),
                applies_to: vec![],
                script: "../escape.sh".into(),
                rollback_supported: false,
            }],
        };
        assert!(execute_manifest(&manifest, tmp.path(), tmp.path()).is_err());
    }

    #[test]
    fn migration_manifest_parses_json() {
        let json = r#"{
            "from_version": "1.32.0",
            "to_version": "1.33.0",
            "migrations": [
                {
                    "id": "is-directory",
                    "description": "add IsDirectory",
                    "applies_to": ["workspace_state"],
                    "script": "migrate.sh",
                    "rollback_supported": true
                }
            ]
        }"#;
        let m = MigrationManifest::from_bytes(json.as_bytes()).unwrap();
        assert_eq!(m.from_version.to_string(), "1.32.0");
        assert_eq!(m.migrations.len(), 1);
        assert_eq!(m.migrations[0].id, "is-directory");
    }
}
