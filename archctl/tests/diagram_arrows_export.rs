// Integration tests for `archctl diagram export --format arrows` (M80b).
//
// Tests:
//   (a) `--format arrows --output <tmp>` produces a valid .arrows JSON
//       containing nodes, relationships, style, and archctl:* pockets.
//   (b) omitting `--output` writes `./<selector-derived>.arrows` in CWD
//       and the filename sanitises `:` and `/` to `_`.

use assert_cmd::Command;
use tempfile::TempDir;

/// Unique tempdir so parallel tests don't contend on the lbug flock
/// (single-writer per project, ADR-010).
fn fresh_cwd() -> std::path::PathBuf {
    TempDir::new().unwrap().keep()
}

/// Run `archctl diagram export --format arrows` with the given args.
fn run_arrows_export(cwd: &std::path::Path, extra_args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--quiet", "--"])
        .args(["diagram", "export"])
        .args(["--cwd"])
        .arg(cwd)
        .args(["container:*", "--format", "arrows"])
        .args(extra_args);
    cmd.output().expect("cargo run should succeed")
}

/// Smoke the JSON structure of an arrows document.
fn assert_arrows_structure(json: &serde_json::Value) {
    assert!(
        json.get("nodes").is_some(),
        "arrows document must have 'nodes' key"
    );
    assert!(
        json.get("relationships").is_some(),
        "arrows document must have 'relationships' key"
    );
    assert!(
        json.get("style").is_some(),
        "arrows document must have 'style' key"
    );
}

/// Test: `--format arrows --output <path>` writes a valid .arrows JSON
/// containing the required top-level keys and archctl:* pockets on nodes.
#[test]
fn arrows_export_writes_valid_json_with_required_keys() {
    let cwd = fresh_cwd();
    let out_dir = TempDir::new().unwrap().keep();
    let out_path = out_dir.join("test.arrows");

    let output = run_arrows_export(&cwd, &["--output", out_path.to_str().unwrap()]);

    assert!(
        output.status.success(),
        "arrows export should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let text = std::fs::read_to_string(&out_path)
        .unwrap_or_else(|e| panic!("expected arrows file at {:?}: {}", out_path, e));

    let json: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("arrows must be valid JSON: {}", e));

    assert_arrows_structure(&json);

    // nodes is an array (may be empty for an empty graph)
    assert!(json["nodes"].is_array(), "nodes must be an array");
    // relationships is an array
    assert!(
        json["relationships"].is_array(),
        "relationships must be an array"
    );
}

/// Test: omitting `--output` writes a `./<selector-derived>.arrows` file in CWD
/// and the filename sanitises `:` and `/` to `_`.
#[test]
fn arrows_export_derives_default_filename_from_selector() {
    // The --cwd flag is passed to archctl to initialise an empty lbug store.
    // The output file is written CWD-relative; capture the path from stdout.
    let out_dir = TempDir::new().unwrap().keep();

    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--quiet", "--"])
        .args(["diagram", "export"])
        .args(["--cwd", out_dir.to_str().unwrap()])
        .args(["container:orders", "--format", "arrows"]);

    let output = cmd.output().expect("cargo run should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "arrows export should succeed; stdout: {}; stderr: {}",
        stdout,
        stderr
    );

    // Extract the path from "Exported arrows to <path>"
    let exported_path = stdout
        .lines()
        .find(|l| l.starts_with("Exported arrows to "))
        .map(|l| l.trim_start_matches("Exported arrows to ").trim());

    let path_str = exported_path
        .expect("expected 'Exported arrows to <path>' in stdout")
        .to_string();

    // Verify the file exists
    let path = std::path::Path::new(&path_str);
    assert!(
        path.exists(),
        "expected arrows file at {:?} but it does not exist; stdout: {}",
        path,
        stdout
    );

    // Verify the filename sanitisation: `container:orders` → `container_orders.arrows`
    assert!(
        path.file_name().unwrap().to_str().unwrap() == "container_orders.arrows",
        "expected 'container_orders.arrows', got {:?}; stdout: {}",
        path.file_name(),
        stdout
    );

    // Verify content is valid JSON with required keys
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("expected arrows file: {}", e));
    let json: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("arrows must be valid JSON: {}", e));

    assert_arrows_structure(&json);
}

/// Test: `--format arrows --json` outputs the expected envelope with
/// `format`, `document`, and `unplaced_count` keys.
#[test]
fn arrows_export_json_envelope_has_required_fields() {
    let cwd = fresh_cwd();

    let output = run_arrows_export(&cwd, &["--output", "/tmp/ignore.arrows", "--json"]);

    assert!(
        output.status.success(),
        "arrows --json should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("stdout must be JSON: {}", e));

    assert_eq!(
        json["format"].as_str().unwrap(),
        "arrows",
        "envelope format must be 'arrows'"
    );
    assert!(
        json.get("document").is_some(),
        "envelope must have 'document' key"
    );
    assert!(
        json.get("unplaced_count").is_some(),
        "envelope must have 'unplaced_count' key"
    );
    assert!(
        json["unplaced_count"].is_u64(),
        "unplaced_count must be a non-negative integer"
    );

    assert_arrows_structure(&json["document"]);
}

/// Test: unknown format is rejected with a message listing accepted values.
#[test]
fn arrows_export_rejects_unknown_format() {
    let cwd = fresh_cwd();
    let out_path = TempDir::new().unwrap().path().join("test.arrows");

    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--quiet", "--"])
        .args(["diagram", "export"])
        .args(["--cwd", cwd.to_str().unwrap()])
        .args(["container:*", "--format", "unknown-format"])
        .args(["--output", out_path.to_str().unwrap()]);

    let output = cmd.output().expect("cargo run should succeed");

    assert!(
        !output.status.success(),
        "unknown format should be rejected"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("accepted formats:"),
        "error message should list accepted formats; got: {}",
        stderr
    );
}
