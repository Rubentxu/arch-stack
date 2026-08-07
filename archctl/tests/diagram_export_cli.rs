// Integration tests for `archctl diagram export` --json envelope (M31).
//
// Tests the new envelope {empty, warning, manifest} for the empty-graph path.

use assert_cmd::Command;
use tempfile::TempDir;

/// Extract the first JSON object from stdout, skipping any tracing log lines
/// that happen to be emitted to stdout (`tracing::info!` in this binary
/// currently writes to stdout — a pre-existing bug, follow-up deferred).
fn extract_first_json(stdout: &str) -> serde_json::Value {
    let start = stdout
        .find('{')
        .unwrap_or_else(|| panic!("no JSON object found in stdout: {:?}", stdout));
    let rest = &stdout[start..];
    serde_json::from_str(rest)
        .unwrap_or_else(|e| panic!("could not parse JSON from stdout: {:?}\nerror: {}", rest, e))
}

/// Test: empty graph via --cwd /tmp → exit 0, JSON has "empty": true.
/// This exercises the M31 spec scenario A1 empirically.
///
/// The CLI auto-initializes the lbug store in the project dir on first run
/// (migrations v1-initial → v3-view-nodes), so even /tmp produces a valid
/// empty graph with elementCount: 0.
#[test]
fn export_empty_graph_json_envelope_has_empty_true() {
    let tmpdir = TempDir::new().unwrap();

    // cargo run from the temp dir finds the workspace Cargo.toml going up,
    // so we use --cwd /tmp to target the temp project dir.
    // The --output goes to the temp dir so we don't need writable /tmp on CI.
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--"])
        .args(["diagram", "export"])
        .args(["--cwd", "/tmp"])
        .args(["--json"])
        .args(["--output", tmpdir.path().join("bundle").to_str().unwrap()])
        .args(["container:*"])
        .output()
        .expect("cargo run should succeed for empty graph");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json = extract_first_json(&stdout);

    assert_eq!(
        json["empty"], true,
        "expected empty:true for zero-element graph, got: {:?}",
        json["empty"]
    );
    assert!(
        json["warning"].as_str().unwrap().contains("no graph found"),
        "expected warning to mention 'no graph found', got: {:?}",
        json["warning"]
    );
    assert!(
        json.get("manifest").is_some(),
        "expected 'manifest' key in envelope"
    );
    assert_eq!(
        json["manifest"]["elementCount"], 0,
        "expected elementCount: 0 in manifest"
    );
}
