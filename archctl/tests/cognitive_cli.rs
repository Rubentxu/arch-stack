// E2E tests for `archctl agent` and `archctl mcp` subcommands (PR3).
//
// Covers:
// - `agent list` (human and JSON)
// - `agent dispatch` (human and JSON)
// - `mcp list-tools` (human and JSON)
// - `mcp invoke` with valid/invalid tools

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
    let mut cmd = Command::new(&bin);
    // Use a temp directory as cwd so we don't depend on project state
    // Use /tmp directly to avoid TempDir lifetime issues
    cmd.current_dir("/tmp");
    cmd
}

#[test]
fn agent_list_empty_human() {
    archctl()
        .args(["agent", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No agents registered"));
}

#[test]
fn agent_list_json() {
    archctl()
        .args(["agent", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[]"));
}

#[test]
fn agent_dispatch_no_action_human() {
    archctl()
        .args(["agent", "dispatch", "analyze coupling in src/"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No action"));
}

#[test]
fn agent_dispatch_no_action_json() {
    archctl()
        .args(["agent", "dispatch", "--json", "analyze coupling in src/"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""kind": "NoAction""#));
}

#[test]
fn mcp_list_tools_human() {
    archctl()
        .args(["mcp", "list-tools"])
        .assert()
        .success()
        .stdout(predicate::str::contains("graph_query"))
        .stdout(predicate::str::contains("schema_validate"))
        .stdout(predicate::str::contains("run_tests_local"));
}

#[test]
fn mcp_list_tools_json() {
    archctl()
        .args(["mcp", "list-tools", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("graph_query"));
}

#[test]
fn mcp_invoke_unknown_tool() {
    archctl()
        .args(["mcp", "invoke", "delete_everything"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not in allowlist"));
}

#[test]
fn mcp_invoke_schema_validate_empty() {
    let tmp = TempDir::new().unwrap();
    let data_path = tmp.path().join("args.json");
    // SchemaValidateArgs expects { bundle: {...} }
    std::fs::write(&data_path, r#"{"bundle": {}}"#).unwrap();
    archctl()
        .args([
            "mcp",
            "invoke",
            "schema_validate",
            "-d",
            data_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""valid": false"#))
        .stdout(predicate::str::contains("missing required field"));
}

#[test]
fn mcp_invoke_schema_validate_valid() {
    let tmp = TempDir::new().unwrap();
    let data_path = tmp.path().join("args.json");
    // SchemaValidateArgs expects { bundle: {...} }
    let bundle = serde_json::json!({
        "bundle": {
            "version": "1.0",
            "projection": {
                "nodes": [
                    {"id": "el:1", "kind": "container", "name": "Test"}
                ]
            }
        }
    });
    std::fs::write(&data_path, bundle.to_string()).unwrap();
    archctl()
        .args([
            "mcp",
            "invoke",
            "schema_validate",
            "-d",
            data_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""valid": true"#));
}
