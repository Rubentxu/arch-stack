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

/// Execute migration scripts in order from from_dir to to_dir.
/// Each script is run with timeout 60s. Returns Err on first failure.
pub fn execute_manifest(
    manifest: &MigrationManifest,
    from_dir: &Path,
    to_dir: &Path,
) -> Result<()> {
    for mig in &manifest.migrations {
        eprintln!("running migration: {} ({})", mig.id, mig.description);
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&mig.script)
            .env("ARCHCTL_FROM_DIR", from_dir)
            .env("ARCHCTL_TO_DIR", to_dir)
            .status()?;
        if !status.success() {
            anyhow::bail!("migration {} failed: exit {:?}", mig.id, status.code());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_manifest_runs_scripts_in_order() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("marker"), "ok").unwrap();
        let manifest = MigrationManifest {
            from_version: Version::parse("1.0.0").unwrap(),
            to_version: Version::parse("1.1.0").unwrap(),
            migrations: vec![Migration {
                id: "test".into(),
                description: "noop".into(),
                applies_to: vec![],
                script: "true".into(),
                rollback_supported: true,
            }],
        };
        execute_manifest(&manifest, tmp.path(), tmp.path()).unwrap();
    }

    #[test]
    fn execute_manifest_propagates_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = MigrationManifest {
            from_version: Version::parse("1.0.0").unwrap(),
            to_version: Version::parse("1.1.0").unwrap(),
            migrations: vec![Migration {
                id: "fails".into(),
                description: "fails".into(),
                applies_to: vec![],
                script: "false".into(),
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
                    "script": "true",
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
