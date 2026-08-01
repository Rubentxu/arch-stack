//! Integration tests for `archctl code sequence`.
//!
//! These tests exercise the full CLI flow: setup a TempDir repo, run
//! `archctl code call-graph --apply` to seed data, then `archctl code sequence`
//! to project. Each test creates its own TempDir (RAII cleanup).

use std::process::Command;

#[test]
fn test_cli_sequence_after_call_graph() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"smoke\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn caller() { helper(); }\npub fn helper() {}\n",
    )
    .unwrap();

    let bin = assert_cmd::cargo::cargo_bin("archctl");

    // Step 1: apply call-graph
    let output = Command::new(&bin)
        .args([
            "code",
            "call-graph",
            "--apply",
            "--lang",
            "rust",
            "--cwd",
        ])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "call-graph --apply failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Step 2: sequence projection
    let output = Command::new(&bin)
        .args([
            "code",
            "sequence",
            "--from",
            "caller",
            "--depth",
            "3",
            "--json",
            "--cwd",
        ])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "sequence failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Extract JSON object from stdout (skip any preceding log lines)
    let json_start = stdout.find('{').unwrap_or(0);
    let json_str = stdout[json_start..].trim();
    let json: serde_json::Value =
        serde_json::from_str(json_str).expect("valid JSON");
    assert_eq!(json["schemaVersion"], "1.0");
    assert!(
        json["interactions"].as_array().unwrap().len() >= 1,
        "expected ≥1 interaction"
    );
}

#[test]
fn test_cli_sequence_apply_emits_warning() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"smoke\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn a() { b(); }\npub fn b() {}\n",
    )
    .unwrap();

    let bin = assert_cmd::cargo::cargo_bin("archctl");

    // Apply call-graph first
    Command::new(&bin)
        .args(["code", "call-graph", "--apply", "--lang", "rust", "--cwd"])
        .arg(dir.path())
        .output()
        .unwrap();

    // Sequence --apply should warn on stderr
    let output = Command::new(&bin)
        .args(["code", "sequence", "--from", "a", "--apply", "--cwd"])
        .arg(dir.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("read-only") || stderr.contains("SCN-217"),
        "expected warning, got: {}",
        stderr
    );
}

#[test]
fn test_cli_sequence_symbol_not_found() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"smoke\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn a() {}\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("archctl");
    Command::new(&bin)
        .args(["code", "call-graph", "--apply", "--lang", "rust", "--cwd"])
        .arg(dir.path())
        .output()
        .unwrap();

    // Sequence with non-existent symbol
    let output = Command::new(&bin)
        .args(["code", "sequence", "--from", "nonexistent", "--cwd"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "expected non-zero exit"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("SymbolNotFound"),
        "expected error, got: {}",
        stderr
    );
}
