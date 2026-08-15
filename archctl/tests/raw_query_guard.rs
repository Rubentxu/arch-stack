//! E2E guard for the raw Cypher admin boundary (P1-04, ADR-059).
//!
//! Exercises the real CLI entry points (`archctl graph query`) through the
//! composition root against an initialized store, proving:
//! 1. Read-only admin queries succeed after `open_raw` initializes the
//!    schema (regression: `open_raw` used to skip init, so every admin
//!    query failed with "called before init").
//! 2. Write-keyword queries are rejected by the `is_read_only_query`
//!    runtime guard.

use std::process::Command;

fn bin() -> Command {
    // Integration tests run with CARGO_BIN_EXE_archctl set by cargo.
    let exe = env!("CARGO_BIN_EXE_archctl");
    let mut cmd = Command::new(exe);
    cmd.env_remove("ARCHCTL_TELEMETRY");
    cmd
}

fn fixture_project(tag: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let manifest = dir.path().join("Cargo.toml");
    std::fs::write(
        manifest,
        format!("[package]\nname = \"raw-guard-{tag}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("write Cargo.toml");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).expect("mkdir src");
    std::fs::write(
        src.join("main.rs"),
        "fn helper() -> u32 { 7 }\nfn main() { println!(\"{}\", helper()); }\n",
    )
    .expect("write main.rs");
    dir
}

#[test]
fn admin_read_query_succeeds_after_open_raw_init() {
    let project = fixture_project("read");
    // Any pipeline that opens_and_inits is not required: the admin path
    // must initialize on its own. A read against a freshly-created store
    // must return exit 0 with an empty-but-valid result.
    let out = bin()
        .arg("graph")
        .arg("query")
        .arg("MATCH (n) RETURN count(n) AS c")
        .arg("--cwd")
        .arg(project.path())
        .arg("--json")
        .output()
        .expect("spawn archctl");
    assert!(
        out.status.success(),
        "admin read query must succeed, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"c\""),
        "expected count column in output: {stdout}"
    );
}

#[test]
fn admin_write_query_rejected_by_guard() {
    let project = fixture_project("write");
    let out = bin()
        .arg("graph")
        .arg("query")
        .arg("MERGE (n:Element {id: 'guard-probe'})")
        .arg("--cwd")
        .arg(project.path())
        .output()
        .expect("spawn archctl");
    assert!(
        !out.status.success(),
        "write-keyword query must exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("read-only") || stderr.contains("write keywords"),
        "guard error must explain the rejection, stderr: {stderr}"
    );
}
