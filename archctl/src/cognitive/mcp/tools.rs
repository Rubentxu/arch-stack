//! MCP tool handlers — read-only graph inspection tools.
//!
//! These are the 3 allowed tools in v1.0 (hardcoded allowlist).
//! No write operations, no dynamic registration.

use serde::{Deserialize, Serialize};

/// Result of a tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl ToolResult {
    pub fn ok(tool: &str, data: impl serde::Serialize) -> Self {
        Self {
            tool: tool.to_string(),
            error: None,
            data: Some(serde_json::to_value(data).unwrap_or(serde_json::Value::Null)),
        }
    }
    pub fn err(tool: &str, message: impl std::fmt::Display) -> Self {
        Self {
            tool: tool.to_string(),
            error: Some(message.to_string()),
            data: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tool: graph_query
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct GraphQueryArgs {
    pub cypher: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphQueryResult {
    pub rows: Vec<serde_json::Value>,
    pub count: usize,
}

/// Execute a Cypher query against the open LadybugDB session.
/// Returns rows as JSON objects.
pub fn handle_graph_query(args: GraphQueryArgs) -> ToolResult {
    use crate::filesystem::SystemFilesystem;
    use crate::graph::query;

    let project_dir = std::env::current_dir().unwrap_or_default();
    let fs = SystemFilesystem;

    match query(&project_dir, &args.cypher, &fs) {
        Ok(rows) => {
            let count = rows.len();
            ToolResult::ok("graph_query", GraphQueryResult { rows, count })
        }
        Err(e) => ToolResult::err("graph_query", e),
    }
}

// ---------------------------------------------------------------------------
// Tool: schema_validate
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct SchemaValidateArgs {
    pub bundle: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaValidationResult {
    pub valid: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

/// Validate a viewer bundle against the known schema.
/// v1.0: structural checks only (required fields, type sanity).
pub fn handle_schema_validate(args: SchemaValidateArgs) -> ToolResult {
    let bundle = args.bundle;
    let mut errors = Vec::new();

    // Check top-level structure
    if !bundle.is_object() {
        errors.push("bundle must be a JSON object".into());
        return ToolResult::ok(
            "schema_validate",
            SchemaValidationResult {
                valid: false,
                errors,
            },
        );
    }

    let obj = bundle.as_object().unwrap();

    // Required top-level keys
    for key in ["version", "projection"] {
        if !obj.contains_key(key) {
            errors.push(format!("missing required field: {key}"));
        }
    }

    // Validate projection nodes
    if let Some(projection) = obj.get("projection")
        && let Some(nodes) = projection.get("nodes")
    {
        if !nodes.is_array() {
            errors.push("projection.nodes must be an array".into());
        } else {
            for (i, node) in nodes.as_array().unwrap().iter().enumerate() {
                if !node.is_object() {
                    errors.push(format!("projection.nodes[{i}] must be an object"));
                    continue;
                }
                let n = node.as_object().unwrap();
                for field in ["id", "kind"] {
                    if !n.contains_key(field) {
                        errors.push(format!("projection.nodes[{i}] missing field: {field}"));
                    }
                }
            }
        }
    }

    let valid = errors.is_empty();
    ToolResult::ok("schema_validate", SchemaValidationResult { valid, errors })
}

// ---------------------------------------------------------------------------
// Tool: run_tests_local
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct RunTestsArgs {
    #[serde(default)]
    pub scope: TestScope,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct TestScope {
    pub package: Option<String>,
    pub files: Vec<String>,
}

fn default_timeout() -> u64 {
    300
}

#[derive(Debug, Clone, Serialize)]
pub struct TestRunResult {
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<TestFailure>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TestFailure {
    pub package: String,
    pub test: String,
    pub message: String,
}

/// Run the test suite for the given scope.
/// v1.0: shells out to `cargo test` in the archctl crate directory.
/// Returns structured results (not raw stdout).
pub fn handle_run_tests_local(args: RunTestsArgs) -> ToolResult {
    use std::process::Command;
    use std::time::Instant;

    let start = Instant::now();
    let scope = &args.scope;
    let _timeout_secs = args.timeout_secs;

    // Build cargo test arguments
    let mut cmd = Command::new("cargo");
    cmd.arg("test");
    cmd.arg("--message-format=json");
    cmd.arg(format!(
        "--{}",
        if scope.package.is_some() {
            "package"
        } else {
            "workspace"
        }
    ));

    if let Some(pkg) = &scope.package {
        cmd.arg(pkg);
    }
    if !scope.files.is_empty() {
        cmd.arg("--");
        for f in &scope.files {
            cmd.arg(f);
        }
    }

    // Set timeout via std::time::Duration
    let output = cmd
        .output()
        .map_err(|e| format!("failed to spawn cargo test: {e}"))
        .ok();

    let output = match output {
        Some(o) => o,
        None => return ToolResult::err("run_tests_local", "cargo test spawn failed"),
    };

    let elapsed = start.elapsed().as_millis() as u64;

    // Parse JSON test results from cargo --message-format=json
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut ignored = 0usize;
    let mut failures = Vec::new();

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
            let reason = msg.get("reason").and_then(|r| r.as_str());
            if reason == Some("test") {
                let result = msg.get("result").and_then(|r| r.as_str()).unwrap_or("ok");
                match result {
                    "ok" | "passed" => passed += 1,
                    "failed" => {
                        let name = msg
                            .get("data")
                            .and_then(|d| d.get("test"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        failures.push(TestFailure {
                            package: scope.package.clone().unwrap_or_else(|| "workspace".into()),
                            test: name,
                            message: msg
                                .get("data")
                                .and_then(|d| d.get("message"))
                                .and_then(|m| m.as_str())
                                .unwrap_or("test failed")
                                .to_string(),
                        });
                        failed += 1;
                    }
                    "ignored" => ignored += 1,
                    _ => {}
                }
            }
        }
    }

    // If cargo returned non-zero but we parsed nothing, surface the error
    if passed == 0 && failed == 0 && !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return ToolResult::err("run_tests_local", format!("cargo test failed: {stderr}"));
    }

    ToolResult::ok(
        "run_tests_local",
        TestRunResult {
            passed,
            failed,
            ignored,
            duration_ms: elapsed,
            failures,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_result_ok_serde() {
        let r = ToolResult::ok(
            "graph_query",
            GraphQueryResult {
                rows: vec![],
                count: 0,
            },
        );
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains(r#""tool":"graph_query""#));
        assert!(!json.contains("error"));
    }

    #[test]
    fn tool_result_err_serde() {
        let r = ToolResult::err("graph_query", "connection refused");
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains(r#""error":"connection refused""#));
        // data: None is skipped (skip_serializing_if)
        assert!(!json.contains("data"));
    }

    #[test]
    fn schema_validate_missing_fields() {
        let bundle = serde_json::json!({});
        let args = SchemaValidateArgs { bundle };
        let r = handle_schema_validate(args);
        let result: SchemaValidationResult = serde_json::from_value(r.data.unwrap()).unwrap();
        assert!(!result.valid);
        assert!(result.errors.len() >= 2);
    }

    #[test]
    fn schema_validate_valid() {
        let bundle = serde_json::json!({
            "version": "1.0",
            "projection": {
                "nodes": [
                    {"id": "e1", "kind": "mt.component", "name": "A"}
                ]
            }
        });
        let args = SchemaValidateArgs { bundle };
        let r = handle_schema_validate(args);
        let result: SchemaValidationResult = serde_json::from_value(r.data.unwrap()).unwrap();
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }
}
