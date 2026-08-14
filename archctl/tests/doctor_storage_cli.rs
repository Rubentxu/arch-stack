// E2E tests for `archctl doctor --scope storage` (PR2/3).
//
// Covers:
// - `doctor --scope storage` (storage compatibility probe)
// - `doctor --scope unknown` (error handling)
// - `doctor --help` (help text includes --scope)

use assert_cmd::assert::OutputAssertExt;
use predicates::prelude::*;
use std::process::Command;
use tempfile::TempDir;

fn archctl() -> Command {
    // Use the debug binary directly
    let bin = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("archctl");
    let cmd = Command::new(&bin);
    cmd
}

#[test]
fn doctor_scope_storage_runs() {
    let tmp = TempDir::new().unwrap();
    archctl()
        .args(["doctor", "--scope", "storage", "--cwd"])
        .arg(tmp.path())
        .assert()
        .success(); // Either OK or FAIL is acceptable - lbug may not be present
}

#[test]
fn doctor_scope_unknown_gives_error() {
    archctl()
        .args(["doctor", "--scope", "nonexistent_scope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown doctor scope"));
}

#[test]
fn doctor_help_includes_scope_flag() {
    archctl()
        .args(["doctor", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--scope"));
}
