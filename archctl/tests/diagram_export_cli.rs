// Integration tests for `archctl diagram export` --json envelope (M31).
//
// Tests the new envelope {empty, warning, manifest} for the empty-graph path.

use assert_cmd::Command;
use tempfile::TempDir;

/// Build a unique tempdir-path for `--cwd` so two parallel tests do not
/// collide on the same xdg archctl project lock (single-writer per
/// project, ADR-010). Each test gets its own tempdir; the project_dir
/// hash derived from the path is unique, so the lbug flock is contended
/// only between sequential invocations on the same path.
fn fresh_cwd() -> std::path::PathBuf {
    TempDir::new().unwrap().keep()
}

/// Run the in-tree `archctl` against `cwd` and parse JSON output.
fn run_archctl_export(cwd: &std::path::Path, output_path: &std::path::Path) -> (String, String) {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--"])
        .args(["diagram", "export"])
        .args(["--cwd"])
        .arg(cwd)
        .args(["--json"])
        .args(["--output"])
        .arg(output_path)
        .args(["container:*"])
        .output()
        .expect("cargo run should succeed for empty graph");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr)
}

/// Test: empty graph via --cwd → exit 0, JSON has "empty": true.
/// This exercises the M31 spec scenario A1 empirically.
///
/// The CLI auto-initializes the lbug store in the project dir on first run
/// (migrations v1-initial → v3-view-nodes), so any directory produces a
/// valid empty graph with elementCount: 0.
///
/// Stdout is pure JSON (no leading log lines): tracing logs go to stderr
/// since the M31-FU1 fix (`telemetry::init()` redirects via
/// `.with_writer(std::io::stderr)`).
#[test]
fn export_empty_graph_json_envelope_has_empty_true() {
    let cwd = fresh_cwd();
    let output_dir = TempDir::new().unwrap();

    let (stdout, _stderr) = run_archctl_export(&cwd, &output_dir.path().join("bundle"));

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "stdout should be pure JSON envelope: {}\nstdout: {}",
            e, stdout
        )
    });

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

/// Test: tracing logs do NOT pollute stdout — they go to stderr.
/// This exercises the M31-FU1 spec scenario — stdout is clean JSON.
#[test]
fn tracing_logs_do_not_pollute_stdout() {
    let cwd = fresh_cwd();
    let output_dir = TempDir::new().unwrap();

    let (stdout, stderr) = run_archctl_export(&cwd, &output_dir.path().join("bundle"));

    // The first character of stdout MUST be '{' (start of JSON envelope).
    // Before the M31-FU1 fix, stdout started with INFO/INFO log lines.
    let first = stdout.chars().next();
    assert_eq!(
        first,
        Some('{'),
        "stdout should start with '{{' (JSON), got first char: {:?}\nstdout: {}",
        first,
        stdout
    );

    // Stderr should contain the INFO log lines (the regression sentinel).
    assert!(
        stderr.contains("INFO") || stderr.contains("schema"),
        "stderr should contain tracing logs, got: {}",
        stderr
    );
}

/// Test: CLI non-JSON mode produces clean text on stdout (no log lines).
/// This exercises the M31-FU1 spec scenario — text output is not polluted
/// by tracing logs.
#[test]
fn non_json_mode_stdout_is_clean_text() {
    let cwd = fresh_cwd();
    let output_dir = TempDir::new().unwrap();

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--"])
        .args(["diagram", "export"])
        .args(["--cwd"])
        .arg(&cwd)
        .args(["--output"])
        .arg(output_dir.path().join("bundle"))
        .args(["container:*"])
        .output()
        .expect("cargo run should succeed for non-JSON empty graph");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Stdout should be exactly the human-readable summary line.
    let line = stdout.trim();
    assert!(
        line.starts_with("Exported "),
        "stdout should start with 'Exported ', got: {:?}",
        line
    );
    assert!(
        line.contains("0 elements"),
        "stdout should contain '0 elements', got: {:?}",
        line
    );
    // No INFO/timestamp lines leaked into stdout.
    assert!(
        !line.contains("INFO") && !line.contains("Z  "),
        "stdout should not contain INFO log lines, got: {:?}",
        stdout
    );
}
