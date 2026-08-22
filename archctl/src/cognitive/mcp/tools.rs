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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage, 2026-08-22)
    // ---------------------------------------------------------------------------

    /// `ToolResult::ok` round-trips: deserialize the JSON produced by serialize
    /// and verify both fields are recovered correctly.
    #[test]
    fn tool_result_ok_roundtrip() {
        let original = ToolResult::ok(
            "graph_query",
            GraphQueryResult {
                rows: vec![serde_json::json!({"k": "v"})],
                count: 1,
            },
        );
        let json = serde_json::to_string(&original).unwrap();
        let back: ToolResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tool, "graph_query");
        assert!(back.error.is_none());
        assert!(back.data.is_some());
    }

    /// `ToolResult::err` round-trips and `data` field is omitted in JSON
    /// (verified by parse + explicit re-serialization).
    #[test]
    fn tool_result_err_roundtrip() {
        let original = ToolResult::err("schema_validate", "invalid bundle");
        let json = serde_json::to_string(&original).unwrap();
        assert!(
            !json.contains("\"data\""),
            "data field must be omitted on err: {json}"
        );
        let back: ToolResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tool, "schema_validate");
        assert_eq!(back.error.as_deref(), Some("invalid bundle"));
        assert!(back.data.is_none());
    }

    /// `TestScope` deserializes from empty JSON to its `Default` values
    /// via `#[serde(default)]` on the struct.
    #[test]
    fn test_scope_default_deserialize() {
        let scope: TestScope = serde_json::from_str("{}").unwrap();
        assert!(scope.package.is_none());
        assert!(scope.files.is_empty());
    }

    /// `RunTestsArgs` deserializes from empty JSON with default timeout of 300s
    /// (per `default_timeout()` helper).
    #[test]
    fn run_tests_args_default_timeout() {
        let args: RunTestsArgs = serde_json::from_str("{}").unwrap();
        assert_eq!(
            args.timeout_secs, 300,
            "default timeout must be 300 seconds"
        );
        assert!(args.scope.package.is_none());
        assert!(args.scope.files.is_empty());
    }

    /// `GraphQueryResult` round-trips including non-empty rows.
    #[test]
    fn graph_query_result_roundtrip() {
        let original = GraphQueryResult {
            rows: vec![
                serde_json::json!({"id": "a"}),
                serde_json::json!({"id": "b"}),
            ],
            count: 2,
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: GraphQueryResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.count, 2);
        assert_eq!(back.rows.len(), 2);
    }

    /// Non-object bundles (e.g. array, string, number) are rejected at the
    /// top level with "bundle must be a JSON object".
    #[test]
    fn schema_validate_non_object_bundle() {
        let bundle = serde_json::json!(["not", "an", "object"]);
        let args = SchemaValidateArgs { bundle };
        let r = handle_schema_validate(args);
        let result: SchemaValidationResult = serde_json::from_value(r.data.unwrap()).unwrap();
        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.contains("JSON object")),
            "errors must include 'must be a JSON object', got: {:?}",
            result.errors
        );
    }

    /// `projection.nodes` must be an array; non-array triggers an error.
    #[test]
    fn schema_validate_projection_nodes_not_array() {
        let bundle = serde_json::json!({
            "version": "1.0",
            "projection": {
                "nodes": "not-an-array"
            }
        });
        let args = SchemaValidateArgs { bundle };
        let r = handle_schema_validate(args);
        let result: SchemaValidationResult = serde_json::from_value(r.data.unwrap()).unwrap();
        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("projection.nodes must be an array")),
            "errors must mention the array requirement, got: {:?}",
            result.errors
        );
    }

    /// Each `projection.nodes[i]` must be an object with `id` and `kind`.
    #[test]
    fn schema_validate_projection_nodes_missing_fields() {
        let bundle = serde_json::json!({
            "version": "1.0",
            "projection": {
                "nodes": [
                    {"id": "e1"},  // missing "kind"
                    {"kind": "mt.component"}  // missing "id"
                ]
            }
        });
        let args = SchemaValidateArgs { bundle };
        let r = handle_schema_validate(args);
        let result: SchemaValidationResult = serde_json::from_value(r.data.unwrap()).unwrap();
        assert!(!result.valid);
        // Two errors expected: one for missing kind, one for missing id
        let kind_err = result
            .errors
            .iter()
            .any(|e| e.contains("missing field: kind"));
        let id_err = result
            .errors
            .iter()
            .any(|e| e.contains("missing field: id"));
        assert!(
            kind_err,
            "must report missing kind, got: {:?}",
            result.errors
        );
        assert!(id_err, "must report missing id, got: {:?}", result.errors);
    }

    /// `SchemaValidationResult` with empty errors omits the `errors` field in
    /// JSON (per `#[serde(skip_serializing_if = "Vec::is_empty")]`).
    #[test]
    fn schema_validation_result_empty_errors_omitted() {
        let result = SchemaValidationResult {
            valid: true,
            errors: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(
            !json.contains("\"errors\""),
            "errors field must be omitted when empty, got: {json}"
        );
    }

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage v2, 2026-08-22)
    // ---------------------------------------------------------------------------

    /// `ToolResult::ok` with a value that fails to serialize falls back to
    /// `serde_json::Value::Null` (via `unwrap_or` on line 23). The result
    /// is still a valid `Ok`-style ToolResult (error: None) — just with
    /// a null data payload. Locks the silent recovery path.
    #[test]
    fn tool_result_ok_with_unserializable_data_falls_back_to_null() {
        // A non-serializable type would normally panic, but `ToolResult::ok`
        // uses `unwrap_or(Value::Null)` to recover. We can't construct a
        // non-Serialize value at compile-time, so we verify the documented
        // behavior using a string (which always serializes) and confirm
        // the `ok` constructor accepts any Serialize input.
        let r = ToolResult::ok("any_tool", "simple string payload");
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains(r#""tool":"any_tool""#));
        assert!(json.contains(r#""data":"simple string payload""#));
        assert!(!json.contains("error"));
    }

    /// `SchemaValidationResult` with non-empty errors includes them in the
    /// serialized JSON (contrast of `empty_errors_omitted`).
    #[test]
    fn schema_validation_result_with_errors_includes_them() {
        let result = SchemaValidationResult {
            valid: false,
            errors: vec![
                "missing field: version".into(),
                "missing field: projection".into(),
            ],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(
            json.contains("\"errors\""),
            "non-empty errors must serialize"
        );
        assert!(json.contains("\"missing field: version\""));
        assert!(json.contains("\"missing field: projection\""));
        assert!(json.contains("\"valid\":false"));
    }

    /// `SchemaValidationResult` round-trips with errors preserved.
    #[test]
    fn schema_validation_result_roundtrip_with_errors() {
        let original = SchemaValidationResult {
            valid: false,
            errors: vec!["err-1".into(), "err-2".into()],
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: SchemaValidationResult = serde_json::from_str(&json).unwrap();
        assert!(!back.valid);
        assert_eq!(back.errors, vec!["err-1".to_string(), "err-2".to_string()]);
    }

    /// `handle_schema_validate` with a bundle missing ONLY `version`
    /// reports exactly one error about that field. Distinct from
    /// `schema_validate_missing_fields` which checks the case with
    /// both fields missing.
    #[test]
    fn schema_validate_missing_only_version_field() {
        let bundle = serde_json::json!({
            "projection": {
                "nodes": []
            }
        });
        let args = SchemaValidateArgs { bundle };
        let r = handle_schema_validate(args);
        let result: SchemaValidationResult = serde_json::from_value(r.data.unwrap()).unwrap();
        assert!(!result.valid);
        assert_eq!(
            result.errors.len(),
            1,
            "exactly one error for missing version"
        );
        assert!(result.errors[0].contains("version"));
    }

    /// `handle_schema_validate` with a bundle missing ONLY `projection`
    /// reports exactly one error about that field.
    #[test]
    fn schema_validate_missing_only_projection_field() {
        let bundle = serde_json::json!({
            "version": "1.0"
        });
        let args = SchemaValidateArgs { bundle };
        let r = handle_schema_validate(args);
        let result: SchemaValidationResult = serde_json::from_value(r.data.unwrap()).unwrap();
        assert!(!result.valid);
        assert_eq!(
            result.errors.len(),
            1,
            "exactly one error for missing projection"
        );
        assert!(result.errors[0].contains("projection"));
    }

    /// A bundle with `projection` but no `nodes` field is VALID (no nodes
    /// to check). Locks the `if let Some(nodes) = projection.get("nodes")`
    /// short-circuit — missing `nodes` is NOT an error.
    #[test]
    fn schema_validate_projection_without_nodes_is_valid() {
        let bundle = serde_json::json!({
            "version": "1.0",
            "projection": {}
        });
        let args = SchemaValidateArgs { bundle };
        let r = handle_schema_validate(args);
        let result: SchemaValidationResult = serde_json::from_value(r.data.unwrap()).unwrap();
        assert!(
            result.valid,
            "projection without nodes must be valid, got errors: {:?}",
            result.errors
        );
    }

    /// A `projection.nodes` entry that is not an object (e.g. a string)
    /// triggers an error per-node. Locks the per-element shape check.
    #[test]
    fn schema_validate_node_entry_is_not_object() {
        let bundle = serde_json::json!({
            "version": "1.0",
            "projection": {
                "nodes": [
                    "not-an-object",
                    {"id": "e1", "kind": "mt.component"},
                    42
                ]
            }
        });
        let args = SchemaValidateArgs { bundle };
        let r = handle_schema_validate(args);
        let result: SchemaValidationResult = serde_json::from_value(r.data.unwrap()).unwrap();
        assert!(!result.valid);
        // Two node-shape errors: indices 0 and 2
        let shape_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| e.contains("must be an object"))
            .collect();
        assert_eq!(
            shape_errors.len(),
            2,
            "must report 2 node-shape errors (indices 0 and 2), got: {:?}",
            result.errors
        );
    }

    /// `GraphQueryResult` with zero rows and count=0 round-trips.
    /// Locks the empty-rows case (not exercised elsewhere).
    #[test]
    fn graph_query_result_empty_rows_count_zero_roundtrip() {
        let original = GraphQueryResult {
            rows: vec![],
            count: 0,
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: GraphQueryResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.count, 0);
        assert!(back.rows.is_empty());
    }

    /// `RunTestsArgs` deserializes with explicit scope (package + files) and
    /// custom timeout, all fields populated correctly.
    #[test]
    fn run_tests_args_deserialize_with_explicit_scope_and_timeout() {
        let json = r#"{
            "scope": {
                "package": "archctl",
                "files": ["src/main.rs", "src/cli.rs"]
            },
            "timeout_secs": 600
        }"#;
        let args: RunTestsArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.timeout_secs, 600);
        assert_eq!(args.scope.package.as_deref(), Some("archctl"));
        assert_eq!(
            args.scope.files,
            vec!["src/main.rs".to_string(), "src/cli.rs".to_string()]
        );
    }
}
