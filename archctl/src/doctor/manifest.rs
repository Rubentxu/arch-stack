//! Manifest validation for doctor scopes.
//!
//! This module provides functions to read and validate scope manifests
//! under `manifests/<scope>.toml`. It ensures the declared symbols
//! and invariants match the actual code.

use crate::filesystem::Filesystem;
use crate::scope::{MANIFESTS_DIR, ScopeManifest};
use anyhow::Result;
use std::path::Path;

/// Validate manifests for all known doctor scopes.
///
/// Checks that:
/// - The manifest file exists for each scope
/// - The manifest is valid TOML
/// - All `must_hold` patterns are found in the declared files
///
/// Returns 0 if all validations pass, 1 if any fail.
pub fn validate_manifests(project_dir: &Path, fs: &dyn Filesystem) -> Result<i32, anyhow::Error> {
    // Define which scopes we care about for doctor
    let doctor_scope_ids = ["doctor"];

    let manifests_dir = project_dir.join(MANIFESTS_DIR);
    let mut failed = 0;

    for &scope_id in &doctor_scope_ids {
        let manifest_path = manifests_dir.join(format!("{scope_id}.toml"));

        // Check manifest exists
        if !fs.exists(&manifest_path) {
            println!("MANIFEST: FAIL — {} not found", manifest_path.display());
            failed += 1;
            continue;
        }

        // Load and parse manifest
        let manifest = match ScopeManifest::load(project_dir, scope_id, fs) {
            Ok(m) => m,
            Err(e) => {
                println!(
                    "MANIFEST: FAIL — {} parse error: {e}",
                    manifest_path.display()
                );
                failed += 1;
                continue;
            }
        };

        // Validate must_hold patterns
        if let Err(e) = validate_must_hold(project_dir, &manifest, fs) {
            println!("MANIFEST: FAIL — {}: {e}", manifest_path.display());
            failed += 1;
            continue;
        }

        println!("MANIFEST: OK — {scope_id}");
    }

    if failed > 0 { Ok(1) } else { Ok(0) }
}

/// Validate that all `must_hold` patterns exist in the declared editable files.
fn validate_must_hold(
    project_dir: &Path,
    manifest: &ScopeManifest,
    fs: &dyn Filesystem,
) -> Result<()> {
    let cargo_dir = manifest.cargo_dir.as_deref().unwrap_or("archctl");

    for pattern in &manifest.must_hold {
        let mut found = false;
        for editable in &manifest.editable_files {
            let file_path = project_dir.join(cargo_dir).join(editable);
            if !fs.exists(&file_path) {
                continue;
            }
            let content = match fs.read_to_string(&file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if content.contains(pattern) {
                found = true;
                break;
            }
        }
        if !found {
            anyhow::bail!(
                "must_hold pattern '{}' not found in any editable file: {:?}",
                pattern,
                manifest.editable_files
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::MemoryFilesystem;

    #[test]
    fn validate_must_hold_finds_existing_pattern() {
        let fs = MemoryFilesystem::new();
        let tmp = tempfile::TempDir::new().unwrap();

        // Create a minimal manifest with must_hold
        let manifest_content = r#"
id = "test"
version = "0.1.0"
description = "test manifest"

cargo_dir = "."

editable_files = ["src/lib.rs"]

must_hold = [
    "pub fn run(",
]
"#;

        fs.create_dir_all(tmp.path()).unwrap();
        fs.write(
            &tmp.path().join("manifests/test.toml"),
            manifest_content.as_bytes(),
        )
        .unwrap();
        fs.write(&tmp.path().join("src/lib.rs"), b"pub fn run() {}")
            .unwrap();

        let manifest = ScopeManifest::load(tmp.path(), "test", &fs).unwrap();
        validate_must_hold(tmp.path(), &manifest, &fs).expect("pattern should be found");
    }

    #[test]
    fn validate_must_hold_fails_missing_pattern() {
        let fs = MemoryFilesystem::new();
        let tmp = tempfile::TempDir::new().unwrap();

        let manifest_content = r#"
id = "test"
version = "0.1.0"

cargo_dir = "."

editable_files = ["src/lib.rs"]

must_hold = [
    "nonexistent_pattern_xyz",
]
"#;

        fs.create_dir_all(tmp.path()).unwrap();
        fs.write(
            &tmp.path().join("manifests/test.toml"),
            manifest_content.as_bytes(),
        )
        .unwrap();
        fs.write(&tmp.path().join("src/lib.rs"), b"pub fn run() {}")
            .unwrap();

        let manifest = ScopeManifest::load(tmp.path(), "test", &fs).unwrap();
        let result = validate_must_hold(tmp.path(), &manifest, &fs);
        assert!(result.is_err(), "missing pattern should cause error");
    }
}
