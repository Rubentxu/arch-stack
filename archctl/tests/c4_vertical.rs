//! Integration tests for the C4 export query fix (ADR-024).
//!
//! Verifies that the export pipeline correctly handles C4 selector queries
//! with the two-field filter (category + kind_id CONTAINS).

use std::fs;
use std::path::Path;

use tempfile::TempDir;

// ─── Helpers ───────────────────────────────────────────────────────────────

fn write(project: &Path, rel: &str, content: &str) {
    let path = project.join(rel);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, content).expect("write temp file");
}

// ─── Tests ────────────────────────────────────────────────────────────────

/// Verify `diagram export container:*` returns a valid bundle (non-empty or empty).
/// This confirms the query pipeline is wired and the CONTAINS filter doesn't crash.
#[test]
fn diagram_export_writes_valid_bundle() {
    let tmpdir = TempDir::new().expect("temp dir");
    let project_dir = tmpdir.path();

    // Create a minimal project so archctl has something to work with
    write(
        project_dir,
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );

    let export_dir = tmpdir.path().join("export");

    // Run export — should succeed even with empty graph (produces empty bundle)
    assert_cmd::Command::cargo_bin("archctl")
        .unwrap()
        .args([
            "diagram",
            "export",
            "--output",
            export_dir.to_str().unwrap(),
            "container:*",
        ])
        .current_dir(project_dir)
        .assert()
        .success();

    // manifest.json should exist and be valid JSON
    let manifest_path = export_dir.join("manifest.json");
    assert!(
        manifest_path.exists(),
        "manifest.json should exist at {}",
        manifest_path.display()
    );
    let manifest_json = fs::read_to_string(&manifest_path).expect("read manifest.json");
    let _manifest: serde_json::Value =
        serde_json::from_str(&manifest_json).expect("manifest.json must be valid JSON");

    // projection.json should exist and be valid JSON
    let projection_path = export_dir.join("projection.json");
    assert!(projection_path.exists(), "projection.json should exist");
    let projection_json = fs::read_to_string(projection_path).expect("read projection.json");
    let _projection: serde_json::Value =
        serde_json::from_str(&projection_json).expect("projection.json must be valid JSON");
}

/// Verify the selector syntax is accepted (parsing works for all C4 selectors).
#[test]
fn diagram_export_accepts_selector_syntax() {
    let tmpdir = TempDir::new().expect("temp dir");
    let project_dir = tmpdir.path();

    write(
        project_dir,
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );

    let export_dir = tmpdir.path().join("export");

    // Test various selector forms — all should be parsed without panic
    for selector in &["container:*", "container:orders", "context:*"] {
        let export_subdir = export_dir.join(selector.replace(':', "_"));
        assert_cmd::Command::cargo_bin("archctl")
            .unwrap()
            .args([
                "diagram",
                "export",
                "--output",
                export_subdir.to_str().unwrap(),
                selector,
            ])
            .current_dir(project_dir)
            .assert()
            .success();
    }
}
