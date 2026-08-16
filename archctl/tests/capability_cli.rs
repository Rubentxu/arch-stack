//! Integration tests for `archctl capabilities` CLI surface.
//!
//! Covers:
//! - S3: Default JSON shape (schemaVersion "1")
//! - S4: Markdown deterministic (byte-identical across runs)
//! - S5: Invalid --format rejected (clap exit 2)
//! - S9: --check exits zero when fresh
//! - S10: --check exits non-zero on stale docs

use std::fs;
use tempfile::TempDir;

/// Helper: invoke `archctl capabilities` via cargo test helpers.
fn capabilities_stdout(args: &[&str]) -> Result<String, std::process::Output> {
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_archctl"));
    cmd.args(args);
    let output = cmd.output().expect("archctl binary exists");
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(output)
    }
}

#[test]
fn test_capabilities_default_json_shape() {
    // S3: Default JSON shape has schemaVersion "1".
    let out = capabilities_stdout(&["capabilities"]).expect("capabilities JSON runs");
    let json: serde_json::Value = serde_json::from_str(&out).expect("stdout is valid JSON");
    assert_eq!(
        json.get("schemaVersion")
            .expect("schemaVersion field present"),
        "1",
        "schemaVersion must be '1'"
    );
    let caps = json
        .get("capabilities")
        .expect("capabilities field present");
    assert!(caps.is_array(), "capabilities must be an array");
    let arr = caps.as_array().unwrap();
    assert!(
        !arr.is_empty(),
        "capabilities must not be empty (13 categories)"
    );

    // Validate output against embedded schema.
    let schema_str = archctl::capability::CAPABILITY_REGISTRY_SCHEMA;
    let schema: serde_json::Value =
        serde_json::from_str(schema_str).expect("embedded schema is valid JSON");
    let validator =
        jsonschema::validator_for(&schema).expect("capability-registry schema compiles");
    let result = validator.validate(&json);
    assert!(
        result.is_ok(),
        "capabilities JSON must validate against schema: {:?}",
        result.err()
    );
}

#[test]
fn test_capabilities_json_flag() {
    // S3 variant: --format json produces same shape.
    let out = capabilities_stdout(&["capabilities", "--format", "json"])
        .expect("capabilities --format json runs");
    let json: serde_json::Value = serde_json::from_str(&out).expect("--format json is valid JSON");
    assert_eq!(
        json.get("schemaVersion").expect("schemaVersion present"),
        "1"
    );
}

#[test]
fn test_capabilities_markdown_deterministic() {
    // S4: Markdown is byte-identical across two runs.
    let first = capabilities_stdout(&["capabilities", "--format", "markdown"])
        .expect("capabilities markdown runs");
    let second = capabilities_stdout(&["capabilities", "--format", "markdown"])
        .expect("capabilities markdown runs second time");
    assert_eq!(
        first, second,
        "markdown output must be byte-identical across runs"
    );
}

#[test]
fn test_capabilities_invalid_format_rejected() {
    // S5: Invalid --format exits with clap error (exit 2).
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_archctl"));
    cmd.args(["capabilities", "--format", "xml"]);
    let output = cmd.output().expect("archctl binary exists");
    // Clap exits 2 for invalid value.
    assert_eq!(
        output.status.code(),
        Some(2),
        "--format xml must be rejected by clap with exit 2"
    );
}

#[test]
fn test_capabilities_check_exits_zero_when_fresh() {
    // S9: --check exits 0 when docs/CAPABILITIES.md is fresh.
    let tmp = TempDir::new().expect("temp dir created");
    let docs_md = tmp.path().join("docs").join("CAPABILITIES.md");
    fs::create_dir_all(docs_md.parent().unwrap()).expect("docs dir created");

    // Generate fresh markdown.
    let fresh = capabilities_stdout(&["capabilities", "--format", "markdown"])
        .expect("capabilities markdown runs");
    fs::write(&docs_md, fresh.as_bytes()).expect("docs written");

    // Run check with that file as CWD.
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_archctl"));
    cmd.current_dir(tmp.path());
    cmd.args(["capabilities", "--check"]);
    let output = cmd.output().expect("archctl binary exists");
    assert_eq!(
        output.status.code(),
        Some(0),
        "--check must exit 0 when docs/CAPABILITIES.md matches fresh output"
    );
}

#[test]
fn test_capabilities_check_exits_nonzero_on_stale() {
    // S10: --check exits 1 when docs/CAPABILITIES.md is stale.
    let tmp = TempDir::new().expect("temp dir created");
    let docs_md = tmp.path().join("docs").join("CAPABILITIES.md");
    fs::create_dir_all(docs_md.parent().unwrap()).expect("docs dir created");

    // Write stale content (extra row).
    fs::write(
        &docs_md,
        "# Capability Registry\n\n| ID | Category | stale |\n|----|----------|-------|\n",
    )
    .expect("stale docs written");

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_archctl"));
    cmd.current_dir(tmp.path());
    cmd.args(["capabilities", "--check"]);
    let output = cmd.output().expect("archctl binary exists");
    assert_eq!(
        output.status.code(),
        Some(1),
        "--check must exit 1 when docs/CAPABILITIES.md is stale"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("stale") || stderr.contains("difference"),
        "stderr must report staleness: {}",
        stderr
    );
}
