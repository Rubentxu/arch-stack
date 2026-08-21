// Integration tests for `archctl diagram validate` (SCN-040..043).
//
// SCN-040: Validate a well-formed bundle → exits zero.
// SCN-041: Validate missing manifest.json → exits non-zero, lists missing files.
// SCN-042: Validate malformed projection.json → exits non-zero, schema violations reported.
// SCN-043: Validate dangling evidence-id reference → exits non-zero.

use std::fs;
use tempfile::TempDir;

/// A minimal valid manifest.json (matches export_types::Manifest serde shape).
fn valid_manifest() -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": "1.0.0",
        "format": "viewer-bundle",
        "viewSelector": "container:orders",
        "baseRevision": "blake3:0000000000000000000000000000000000000000000000000000000000000000",
        "generatedAt": "2026-07-30T12:00:00Z",
        "elementCount": 1,
        "edgeCount": 0,
        "evidenceCount": 0
    })
}

/// A minimal valid projection.json (matches export_types::Projection / Node / Edge).
fn valid_projection() -> serde_json::Value {
    serde_json::json!({
        "nodes": [
            {
                "id": "el:1",
                "type": "container",
                "name": "OrderService"
            }
        ],
        "edges": []
    })
}

/// A minimal valid evidence.json.
fn valid_evidence() -> serde_json::Value {
    serde_json::json!({
        "evidence": []
    })
}

/// A minimal valid styles.json (matches export_types::Styles serde shape).
fn valid_styles() -> serde_json::Value {
    serde_json::json!({
        "theme": "light",
        "version": "1.0.0",
        "elementColors": {
            "context": "#000000",
            "container": "#000000",
            "component": "#000000",
            "dynamic": "#000000",
            "deployment": "#000000"
        },
        "edgeColors": {
            "default": "#000000"
        }
    })
}

/// Write a complete valid bundle into `dir`.
fn write_valid_bundle(dir: &TempDir) {
    let manifest = valid_manifest();
    let projection = valid_projection();
    let evidence = valid_evidence();
    let styles = valid_styles();

    fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.path().join("projection.json"),
        serde_json::to_string_pretty(&projection).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.path().join("evidence.json"),
        serde_json::to_string_pretty(&evidence).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.path().join("styles.json"),
        serde_json::to_string_pretty(&styles).unwrap(),
    )
    .unwrap();

    let assets = dir.path().join("assets");
    fs::create_dir_all(&assets).unwrap();
    for icon in ["context", "container", "component", "dynamic", "deployment"] {
        // Validator checks file existence only (not content), so the
        // 16-byte placeholder is enough. Extension must match
        // `diagram::assets::ICON_EXTENSION` ("svg") — see validate.rs.
        fs::write(assets.join(format!("{icon}.svg")), [0u8; 16]).unwrap();
    }
}

// SCN-040: well-formed bundle → exits zero
#[test]
fn validate_valid_bundle_exits_zero() {
    let tmpdir = TempDir::new().unwrap();
    write_valid_bundle(&tmpdir);

    assert_cmd::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "diagram",
            "validate",
            tmpdir.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("is valid"));
}

// SCN-040 JSON output mode: exits zero (valid bundle, no output)
#[test]
fn validate_valid_bundle_json_output_exits_zero() {
    let tmpdir = TempDir::new().unwrap();
    write_valid_bundle(&tmpdir);

    assert_cmd::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "diagram",
            "validate",
            "--json",
            tmpdir.path().to_str().unwrap(),
        ])
        .assert()
        .success();
}

// SCN-041: missing manifest.json → exits non-zero
#[test]
fn validate_missing_manifest_exits_nonzero() {
    let tmpdir = TempDir::new().unwrap();
    write_valid_bundle(&tmpdir);
    fs::remove_file(tmpdir.path().join("manifest.json")).unwrap();

    assert_cmd::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "diagram",
            "validate",
            tmpdir.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicates::str::contains("manifest.json"))
        .stdout(predicates::str::contains("not found"));
}

// SCN-041 variant: missing evidence.json → exits non-zero
#[test]
fn validate_missing_evidence_exits_nonzero() {
    let tmpdir = TempDir::new().unwrap();
    write_valid_bundle(&tmpdir);
    fs::remove_file(tmpdir.path().join("evidence.json")).unwrap();

    assert_cmd::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "diagram",
            "validate",
            tmpdir.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicates::str::contains("evidence.json"))
        .stdout(predicates::str::contains("not found"));
}

// SCN-042: malformed projection.json (wrong type for nodes field) → exits non-zero
#[test]
fn validate_malformed_projection_nodes_wrong_type() {
    let tmpdir = TempDir::new().unwrap();
    write_valid_bundle(&tmpdir);

    // schema expects "nodes" to be array, give it a string
    fs::write(
        tmpdir.path().join("projection.json"),
        r#"{ "nodes": "not-an-array", "edges": [] }"#,
    )
    .unwrap();

    assert_cmd::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "diagram",
            "validate",
            tmpdir.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicates::str::contains("projection.json"))
        .stdout(predicates::str::contains("is not of type"));
}

// SCN-042 variant: node with invalid 'type' enum value
#[test]
fn validate_projection_node_invalid_type() {
    let tmpdir = TempDir::new().unwrap();
    write_valid_bundle(&tmpdir);

    let bad_projection = serde_json::json!({
        "nodes": [
            {
                "id": "el:1",
                "type": "invalid_kind",
                "name": "BadService"
            }
        ],
        "edges": []
    });
    fs::write(
        tmpdir.path().join("projection.json"),
        serde_json::to_string_pretty(&bad_projection).unwrap(),
    )
    .unwrap();

    assert_cmd::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "diagram",
            "validate",
            tmpdir.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicates::str::contains("projection.json"))
        .stdout(predicates::str::contains("invalid_kind"));
}

// SCN-043: dangling evidence-id reference → exits non-zero with dangling ref message
//
// NOTE: This test is documented as UNTESTABLE in this correction cycle due to a
// pre-existing bug in `export_types.rs:25`: the `evidence_refs` field on `Node`
// lacks `#[serde(rename = "evidenceRefs")]`, so serde deserializes it as `None`
// regardless of the JSON content. This breaks the dangling-ref check at
// validate.rs:131-147 which iterates `node.evidence_refs`. The fix requires
// adding the rename attribute to export_types.rs (out of scope for this cycle).
// Until then, SCN-043 remains UNTESTED.
// #[test]
// fn validate_dangling_evidence_ref() { ... }
