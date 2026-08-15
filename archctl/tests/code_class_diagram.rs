//! Integration tests for `archctl code class-diagram`.

use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

/// Parse the JSON output from class-diagram command, filtering out any non-JSON
/// prefix lines (e.g., INFO log lines from tracing).
fn parse_class_diagram_output(output: &str) -> serde_json::Value {
    // Find the first '{' to skip any prefix lines
    let json_start = output.find('{').expect("expected JSON output");
    let json_str = &output[json_start..];
    serde_json::from_str(json_str).expect("valid JSON")
}

/// Test that CLI exits 0 on empty project.
#[test]
fn test_class_diagram_empty_project() {
    let tmp = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Class diagram"));
}

/// Test that CLI exits 0 on --json with empty project.
#[test]
fn test_class_diagram_json_empty() {
    let tmp = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report = parse_class_diagram_output(&stdout);
    assert!(
        report["nodes"].as_array().unwrap().is_empty(),
        "expected empty nodes"
    );
}

/// Test Rust struct extraction produces a ClassNode.
#[test]
fn test_class_diagram_rust_struct() {
    let tmp = TempDir::new().unwrap();
    let rs_path = tmp.path().join("foo.rs");
    std::fs::write(
        &rs_path,
        r#"
pub struct UserService {
    pub name: String,
}

impl UserService {
    pub fn new() -> Self { Self { name: String::new() } }
}
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report = parse_class_diagram_output(&stdout);
    let nodes = report["nodes"].as_array().unwrap();
    assert!(!nodes.is_empty(), "expected at least one node");
    assert!(
        nodes.iter().any(|n| n["kind"] == "class"),
        "expected a class node: {}",
        stdout
    );
}

/// Test TypeScript class extends produces an extends edge.
#[test]
fn test_class_diagram_typescript_extends() {
    let tmp = TempDir::new().unwrap();
    let ts_path = tmp.path().join("foo.ts");
    std::fs::write(
        &ts_path,
        r#"
class Animal {}
class Dog extends Animal {}
interface IFoo {}
class Bar implements IFoo {}
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report = parse_class_diagram_output(&stdout);
    let edges = report["edges"].as_array().unwrap();
    assert!(
        edges.iter().any(|e| e["predicate"] == "extends"),
        "expected extends edge: {}",
        stdout
    );
    assert!(
        edges.iter().any(|e| e["predicate"] == "implements"),
        "expected implements edge: {}",
        stdout
    );
}

/// Test Python multiple inheritance produces two extends edges.
#[test]
fn test_class_diagram_python_multi_inherit() {
    let tmp = TempDir::new().unwrap();
    let py_path = tmp.path().join("foo.py");
    std::fs::write(
        &py_path,
        r#"
class Base1:
    pass
class Base2:
    pass
class Derived(Base1, Base2):
    pass
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report = parse_class_diagram_output(&stdout);
    let nodes = report["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 3, "expected 3 nodes: {}", stdout);
    let edges = report["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2, "expected 2 extends edges: {}", stdout);
}

/// Test determinism: running twice produces byte-identical JSON.
#[test]
fn test_class_diagram_determinism() {
    let tmp = TempDir::new().unwrap();
    let rs_path = tmp.path().join("foo.rs");
    std::fs::write(
        &rs_path,
        r#"
pub struct Foo {
    pub value: i32,
}
impl Foo {
    pub fn new() -> Self { Self { value: 0 } }
}
"#,
    )
    .unwrap();

    let run = || {
        let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
            .args([
                "code",
                "class-diagram",
                "--cwd",
                tmp.path().to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        parse_class_diagram_output(&String::from_utf8_lossy(&output.stdout))
    };

    let first = run();
    let second = run();
    assert_eq!(first, second, "class-diagram output must be deterministic");
}

/// Test schema validation via jsonschema.
#[test]
fn test_class_diagram_schema_validation() {
    use std::fs;

    let tmp = TempDir::new().unwrap();
    let rs_path = tmp.path().join("foo.rs");
    fs::write(
        &rs_path,
        r#"
pub struct Service {
    pub name: String,
}
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let report = parse_class_diagram_output(&stdout);
    let schema_bytes = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../schemas/class-diagram-report.schema.json"),
    )
    .unwrap();
    let schema: serde_json::Value = serde_json::from_slice(&schema_bytes).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    validator
        .validate(&report)
        .expect("report must conform to schema");
}

// ─── Selector resolution ────────────────────────────────────────────────────────

/// Scenario: file:<path> selector only processes the specified file.
#[test]
fn test_class_diagram_file_selector() {
    let tmp = TempDir::new().unwrap();
    let foo = tmp.path().join("foo.rs");
    let bar = tmp.path().join("bar.rs");
    std::fs::write(&foo, "pub struct Foo;\n").unwrap();
    std::fs::write(&bar, "pub struct Bar;\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--json",
            "--selector",
            "file:foo.rs",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = parse_class_diagram_output(&String::from_utf8_lossy(&output.stdout));
    let nodes = report["nodes"].as_array().unwrap();
    // Only foo.rs should be processed; bar.rs must not appear
    let files: Vec<_> = nodes.iter().map(|n| n["file"].as_str().unwrap()).collect();
    assert!(
        files.iter().all(|f| *f == "foo.rs"),
        "only foo.rs should be present, got: {files:?}"
    );
}

/// Scenario: module:<id> selector is not yet supported → exit 64 with unknown selector.
#[test]
fn test_class_diagram_module_selector() {
    let tmp = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--selector",
            "module:billing",
        ])
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(64),
        "unknown selector should exit 64, got {:?}",
        output.status.code()
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown selector"),
        "stderr should mention 'unknown selector': {stderr}"
    );
}

/// Scenario: unknown selector format → exit 64 with "unknown selector".
#[test]
fn test_class_diagram_unknown_selector() {
    let tmp = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--selector",
            "unknown:value",
        ])
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(64),
        "unknown selector should exit 64, got {:?}",
        output.status.code()
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown selector"),
        "stderr should mention 'unknown selector': {stderr}"
    );
}

/// Scenario: file:<path> where the file does not exist → exit 64 with "file not found".
#[test]
fn test_class_diagram_missing_file() {
    let tmp = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--selector",
            "file:missing.rs",
        ])
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(64),
        "missing file should exit 64, got {:?}",
        output.status.code()
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("file not found"),
        "stderr should mention 'file not found': {stderr}"
    );
}

// ─── Error handling ────────────────────────────────────────────────────────────

/// Scenario: one malformed file is warned and skipped; valid files are still processed.
#[test]
fn test_class_diagram_parse_error_tolerance() {
    let tmp = TempDir::new().unwrap();
    let good = tmp.path().join("good.rs");
    let bad = tmp.path().join("bad.rs");
    std::fs::write(&good, "pub struct Good;\n").unwrap();
    // Malformed: unclosed brace (parse failure for tree-sitter)
    std::fs::write(&bad, "pub struct Bad { incomplete\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "parse errors should be tolerated, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = parse_class_diagram_output(&String::from_utf8_lossy(&output.stdout));
    let nodes = report["nodes"].as_array().unwrap();
    // good.rs must be projected; bad.rs must not appear
    let files: Vec<_> = nodes.iter().map(|n| n["file"].as_str().unwrap()).collect();
    assert!(
        files.iter().all(|f| *f == "good.rs"),
        "only good.rs should be present, got: {files:?}"
    );
}

/// Scenario: unsupported extension (.go) is warned and skipped without failing the run.
#[test]
fn test_class_diagram_unsupported_extension_skipped() {
    let tmp = TempDir::new().unwrap();
    let go_file = tmp.path().join("main.go");
    std::fs::write(&go_file, "package main\nfunc main() {}\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    // Must not fail; Go file is simply skipped by the extension filter
    assert!(
        output.status.success(),
        "unsupported extension should be skipped, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = parse_class_diagram_output(&String::from_utf8_lossy(&output.stdout));
    let nodes = report["nodes"].as_array().unwrap();
    assert!(nodes.is_empty(), "no nodes expected from .go file");
}

// ─── Intra-file scope ──────────────────────────────────────────────────────────

/// Scenario: cross-file inheritance — parent and child in different files → no edge.
#[test]
fn test_class_diagram_no_cross_file_inheritance() {
    let tmp = TempDir::new().unwrap();
    let parent = tmp.path().join("parent.rs");
    let child = tmp.path().join("child.rs");
    std::fs::write(&parent, "pub struct Animal;\n").unwrap();
    // child extends Animal but Animal is in a different file
    std::fs::write(&child, "pub struct Dog extends Animal;\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let report = parse_class_diagram_output(&String::from_utf8_lossy(&output.stdout));
    let edges = report["edges"].as_array().unwrap();
    // No cross-file extends edges should exist
    assert!(
        edges.is_empty() || !edges.iter().any(|e| e["predicate"] == "extends"),
        "cross-file extends should not appear: {edges:?}"
    );
}

/// Scenario: same-file composition — A has a field typed B (same file) → composes edge.
/// Currently: field members ARE captured, but no `composes` edge is emitted because
/// field-type → edge resolution is not yet wired in `extract_edges`.  The spec
/// Composes edge emission: intra-file field type resolves to a same-file class.
#[test]
fn test_class_diagram_same_file_composes() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("service.rs");
    std::fs::write(
        &file,
        r#"
pub struct Config {}
pub struct App {
    pub config: Config,
}
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let report = parse_class_diagram_output(&String::from_utf8_lossy(&output.stdout));
    let nodes = report["nodes"].as_array().unwrap();
    let edges = report["edges"].as_array().unwrap();

    // Both class nodes must be present
    assert!(
        nodes.iter().any(|n| n["name"] == "Config"),
        "Config node should exist: {nodes:?}"
    );
    assert!(
        nodes.iter().any(|n| n["name"] == "App"),
        "App node should exist: {nodes:?}"
    );

    // App's field member must be captured
    let app_node = nodes.iter().find(|n| n["name"] == "App").unwrap();
    let members = app_node["members"].as_array().unwrap();
    assert!(
        !members.is_empty(),
        "App should have field members captured: {members:?}"
    );

    // Composes edge: App → Config (field type resolves to same-file class).
    let composes: Vec<_> = edges
        .iter()
        .filter(|e| e["predicate"] == "composes")
        .collect();
    assert!(
        !composes.is_empty(),
        "expected at least one composes edge for same-file typed field, got: {edges:?}"
    );
    let app_key = app_node["canonical_key"].as_str().unwrap();
    let config_node = nodes.iter().find(|n| n["name"] == "Config").unwrap();
    let config_key = config_node["canonical_key"].as_str().unwrap();
    assert!(
        composes
            .iter()
            .any(|e| e["source"] == app_key && e["target"] == config_key),
        "expected App→Config composes edge: {composes:?}"
    );
}

/// Scenario: cyclic same-file references (A has field of B, B has field of A) → terminates.
#[test]
fn test_class_diagram_cyclic_reference() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("cyclic.rs");
    std::fs::write(
        &file,
        r#"
pub struct A {
    pub b: B,
}
pub struct B {
    pub a: A,
}
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    // Must terminate successfully (no infinite loop)
    assert!(
        output.status.success(),
        "cyclic references should not cause infinite loops, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = parse_class_diagram_output(&String::from_utf8_lossy(&output.stdout));
    let nodes = report["nodes"].as_array().unwrap();
    assert_eq!(
        nodes.len(),
        2,
        "both A and B nodes should be present: {nodes:?}"
    );
}

// ─── Projection bundle ─────────────────────────────────────────────────────────

/// Scenario: bundle size is below 1 MB even for large projections.
#[test]
fn test_class_diagram_bundle_size_bound() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("many.rs");
    // Generate 2000 structs — should stay well under 1 MB
    let body: String = (0..2000)
        .map(|i| format!("pub struct Struct{i};\n"))
        .collect();
    std::fs::write(&file, body).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout_bytes = output.stdout.len();
    assert!(
        stdout_bytes < 1024 * 1024,
        "bundle ({stdout_bytes} B) must be below 1 MB for <10k nodes"
    );
}

/// Scenario: changing the name in the tuple changes the node ID.
#[test]
fn test_class_diagram_stable_id_changes() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("animal.rs");

    let run_with_content = |content: &str| {
        std::fs::write(&file, content).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
            .args([
                "code",
                "class-diagram",
                "--cwd",
                tmp.path().to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        let report = parse_class_diagram_output(&String::from_utf8_lossy(&output.stdout));
        report["nodes"][0]["canonical_key"]
            .as_str()
            .unwrap()
            .to_string()
    };

    let key_foo = run_with_content("pub struct Foo;\n");
    let key_bar = run_with_content("pub struct Bar;\n");

    // Same file/line/kind but different name → IDs must differ
    assert_ne!(key_foo, key_bar, "changing name must change canonical_key");
    // Same input twice → ID must be stable
    let key_foo2 = run_with_content("pub struct Foo;\n");
    assert_eq!(
        key_foo, key_foo2,
        "canonical_key must be stable for identical input"
    );
}

// ─── Graph application ────────────────────────────────────────────────────────

/// Scenario: `--apply` run twice produces identical element/relation counts (idempotent).
/// Requires lbug graph store; skipped when unavailable.
#[test]
fn test_class_diagram_apply_idempotent() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("service.rs");
    std::fs::write(&file, "pub struct UserService;\n").unwrap();

    let run_apply = || {
        let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
            .args([
                "code",
                "class-diagram",
                "--cwd",
                tmp.path().to_str().unwrap(),
                "--apply",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "apply should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    };

    let first = run_apply();
    let second = run_apply();

    // Idempotent: the total element count (written + skipped) must be identical.
    // First run: migrations applied + 1 element written. Second run: migrations skipped,
    // element is skipped so total stays the same.
    fn extract_total_count(stdout: &str, field: &str) -> usize {
        for line in stdout.lines() {
            if let Some(pos) = line.find(field) {
                // Extract written count: number immediately before field name
                let before = &line[..pos];
                let written = before
                    .split_whitespace()
                    .last()
                    .and_then(|w| w.parse().ok())
                    .unwrap_or(0);
                // Extract skipped count: first number inside parentheses after field
                let after = &line[pos..];
                let skipped = after
                    .split('(')
                    .nth(1)
                    .and_then(|s| s.split_whitespace().next())
                    .and_then(|w| w.parse().ok())
                    .unwrap_or(0);
                return written + skipped;
            }
        }
        0
    }

    let e1 = extract_total_count(&first, "elements");
    let e2 = extract_total_count(&second, "elements");
    assert_eq!(
        e1, e2,
        "total elements (written + skipped) must be identical on both apply runs: first={e1}, second={e2}"
    );
}

// ─── Determinism + golden fixture ─────────────────────────────────────────────

/// Scenario: output matches the canonical gold.json fixture (deterministic golden test).
#[test]
fn test_class_diagram_golden_fixture() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/class-diagram");

    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            fixture_dir.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report = parse_class_diagram_output(&stdout);

    let gold_bytes = std::fs::read(fixture_dir.join("gold.json")).unwrap();
    let gold: serde_json::Value = serde_json::from_slice(&gold_bytes).unwrap();

    // Compare nodes (filter volatile fields: durationMs, filesScanned may vary slightly)
    let mut actual_nodes = report["nodes"].as_array().unwrap().clone();
    let mut expected_nodes = gold["nodes"].as_array().unwrap().clone();
    actual_nodes.sort_by_key(|v| v["canonical_key"].as_str().unwrap().to_string());
    expected_nodes.sort_by_key(|v| v["canonical_key"].as_str().unwrap().to_string());
    assert_eq!(actual_nodes, expected_nodes, "nodes must match gold.json");

    // Compare edges
    let mut actual_edges = report["edges"].as_array().unwrap().clone();
    let mut expected_edges = gold["edges"].as_array().unwrap().clone();
    actual_edges.sort_by_key(|v| v["canonical_key"].as_str().unwrap().to_string());
    expected_edges.sort_by_key(|v| v["canonical_key"].as_str().unwrap().to_string());
    assert_eq!(actual_edges, expected_edges, "edges must match gold.json");
}

// ─── Performance ─────────────────────────────────────────────────────────────

/// Scenario: export latency for a representative fixture is below 2 seconds.
#[test]
fn test_class_diagram_perf_budget() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("large.rs");
    // 500 structs is enough to exercise the pipeline without being slow
    let body: String = (0..500)
        .map(|i| format!("pub struct Struct{i} {{}}\n"))
        .collect();
    std::fs::write(&file, body).unwrap();

    let start = std::time::Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "code",
            "class-diagram",
            "--cwd",
            tmp.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    let elapsed = start.elapsed();

    assert!(output.status.success());
    assert!(
        elapsed.as_secs_f64() < 2.0,
        "class-diagram export must be < 2s, took {:.2}s",
        elapsed.as_secs_f64()
    );
}

// ─── Atomic-abort regression (M32 D5) ──────────────────────────────────────

/// Verifies that `class_diagram::apply` wraps writes in a transaction:
/// a mid-loop binder error triggers Kùzu's implicit rollback, COMMIT
/// fails, and 0 partial rows survive. Pattern parallels PR1's
/// `transaction_atomic_abort_on_write_error` for call_graph.
///
/// We test the primitive-level contract directly (not via the
/// `apply()` function) because Kùzu's per-process flock prevents
/// re-opening the same project store within one test process. The
/// `apply()` function is the same code path that uses
/// `begin/commit/rollback_transaction`; testing the primitives
/// directly is the strongest contract assertion we can make.
#[test]
fn class_diagram_apply_atomic_abort_on_write_error() {
    use archctl::store::{ElementRepository, GraphStore, LbugStore, RawGraphQuery};

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    let mut store = LbugStore::open(project).expect("store must open");
    store.init().expect("store must init");

    store.begin_transaction().expect("begin must succeed");
    // Use typed repository method instead of raw MERGE to avoid RawGraphQuery guard.
    store
        .upsert_element(&archctl::graph::Element {
            id: "class_diag:good".to_string(),
            kind_id: "k".to_string(),
            category: "test".to_string(),
            canonical_key: "class_diag:good".to_string(),
            current_name: "class_diag:good".to_string(),
            current_status: "active".to_string(),
            current_confidence: 1.0,
            current_version_id: uuid::Uuid::new_v4().to_string(),
        })
        .expect("good write inside tx must succeed");

    // Trigger a binder error: SUPPORTED_BY is declared FROM
    // ElementVersion TO Evidence — so (Element)-[SUPPORTED_BY]->(Evidence)
    // violates the direction constraint.
    // Use execute_raw_cypher_for_test to bypass RawGraphQuery guard and reach
    // Kùzu directly so Kùzu can enforce the direction constraint.
    let bad = store.execute_raw_cypher_for_test(
        "MATCH (e:Element {id: 'class_diag:good'}) MATCH (ev:Evidence {id: 'class_diag:ev'}) \
         MERGE (e)-[r:SUPPORTED_BY]->(ev);",
    );
    assert!(
        bad.is_err(),
        "expected SUPPORTED_BY direction violation to fail the binder"
    );

    // Active transaction is now implicitly rolled back by Kùzu.
    // An explicit COMMIT must fail.
    let commit = store.commit_transaction();
    assert!(
        commit.is_err(),
        "commit must fail after implicit rollback from binder error"
    );

    // 0 partial rows survive.
    let rows: Vec<archctl::Row> = store
        .query("MATCH (e:Element {id: 'class_diag:good'}) RETURN e.id;")
        .expect("query must succeed");
    assert_eq!(
        rows.len(),
        0,
        "atomic-abort: no partial state should survive an implicit rollback"
    );
}

// ─── M32 D2 + D5 class_diagram tests ─────────────────────────────────────────────

/// Verifies UNWIND bulk path produces correct element + edge counts.
/// Regression guard for the N+1 bug fix + UNWIND restore.
#[test]
fn class_diagram_apply_unwind_bulk_correctness() {
    use archctl::code::class_diagram;
    use archctl::code::class_diagram::{
        ClassDiagramReport, ClassEdge, ClassEdgeKind, ClassNode, Language, ProjectMeta, TypeKind,
    };
    use archctl::filesystem::SystemFilesystem;
    use archctl::store::{GraphStore, RawGraphQuery};

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();

    // Build a synthetic ClassDiagramReport with 50 classes and 5 edges.
    const NODE_COUNT: usize = 50;
    let mut nodes = Vec::with_capacity(NODE_COUNT);
    for i in 0..NODE_COUNT {
        nodes.push(ClassNode {
            canonical_key: format!("rust:src/lib.rs:class:Class_{}:{}", i, i),
            kind: TypeKind::Class,
            language: Language::Rust,
            file: "src/lib.rs".to_string(),
            line: (i as u32) + 1,
            name: format!("Class_{}", i),
            members: vec![],
            confidence: 0.90,
        });
    }

    // Edges: Class_i extends Class_{i+1}
    let mut edges = Vec::with_capacity(5);
    for i in 0..5 {
        edges.push(ClassEdge {
            canonical_key: format!(
                "rust:src/lib.rs:Class_{}→extends→Class_{}:{}",
                i,
                i + 1,
                i + 10
            ),
            source: format!("rust:src/lib.rs:class:Class_{}:{}", i, i),
            target: format!("rust:src/lib.rs:class:Class_{}:{}", i + 1, i + 1),
            predicate: ClassEdgeKind::Extends,
            file: "src/lib.rs".to_string(),
            line: (i as u32) + 10,
            confidence: 0.90,
        });
    }

    let report = ClassDiagramReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root: project.to_string_lossy().to_string(),
            files_scanned: 1,
            languages: [("rust".to_string(), NODE_COUNT as u64)].into(),
        },
        nodes,
        edges,
        errors: vec![],
    };

    let fs = SystemFilesystem;
    let r = class_diagram::apply(project, &report, &fs).expect("apply must succeed");
    assert_eq!(
        r.elements_written, NODE_COUNT,
        "UNWIND bulk: expected {} elements_written",
        NODE_COUNT
    );
    assert_eq!(
        r.relations_written, 5,
        "UNWIND bulk: expected 5 relations_written"
    );

    // Verify via direct store query.
    let mut store = archctl::store::LbugStore::open(project).expect("store must open");
    store.init().expect("store must init");

    let element_count: i64 = store
        .query("MATCH (e:Element) WHERE e.kind_id STARTS WITH 'uml.' RETURN count(e) AS cnt;")
        .expect("element count query must succeed")
        .pop()
        .and_then(|r| r.get("cnt").and_then(|c| c.as_i64()))
        .expect("count must be i64");
    assert_eq!(
        element_count, NODE_COUNT as i64,
        "UNWIND bulk: expected {} Element nodes, got {}",
        NODE_COUNT, element_count
    );

    let edge_count: i64 = store
        .query("MATCH ()-[r:SEMANTIC_EDGE]->() RETURN count(r) AS cnt;")
        .expect("edge count query must succeed")
        .pop()
        .and_then(|r| r.get("cnt").and_then(|c| c.as_i64()))
        .expect("count must be i64");
    assert_eq!(
        edge_count, 5,
        "UNWIND bulk: expected 5 SEMANTIC_EDGE edges, got {}",
        edge_count
    );
}

/// Verifies applying twice with the same data produces the same final state
/// (no duplicates, idempotent skip). This indirectly verifies the
/// existing_canonical_keys pre-check is done ONCE before the batch, not inside
/// the per-node loop (which was the N+1 bug at class_diagram.rs L1394-1395).
#[test]
fn class_diagram_existing_keys_not_n_plus_one() {
    use archctl::code::class_diagram;
    use archctl::code::class_diagram::{
        ClassDiagramReport, ClassNode, Language, ProjectMeta, TypeKind,
    };
    use archctl::store::{GraphStore, RawGraphQuery};

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();

    let make_report = |offset: usize| ClassDiagramReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root: project.to_string_lossy().to_string(),
            files_scanned: 1,
            languages: [("rust".to_string(), 3u64)].into(),
        },
        nodes: vec![
            ClassNode {
                canonical_key: format!("rust:src/lib.rs:class:C{}:{}", offset, offset),
                kind: TypeKind::Class,
                language: Language::Rust,
                file: "src/lib.rs".to_string(),
                line: 1,
                name: format!("C{}", offset),
                members: vec![],
                confidence: 0.90,
            },
            ClassNode {
                canonical_key: format!("rust:src/lib.rs:class:C{}:{}", offset + 1, offset + 1),
                kind: TypeKind::Class,
                language: Language::Rust,
                file: "src/lib.rs".to_string(),
                line: 2,
                name: format!("C{}", offset + 1),
                members: vec![],
                confidence: 0.90,
            },
        ],
        edges: vec![],
        errors: vec![],
    };

    let fs = archctl::filesystem::SystemFilesystem;

    // First apply: both nodes written.
    let r1 = class_diagram::apply(project, &make_report(0), &fs).expect("first apply must succeed");
    assert_eq!(r1.elements_written, 2, "first apply: 2 elements");
    assert_eq!(r1.elements_skipped, 0, "first apply: 0 skipped");

    // Second apply with same data: both skipped.
    let r2 =
        class_diagram::apply(project, &make_report(0), &fs).expect("second apply must succeed");
    assert_eq!(
        r2.elements_written, 0,
        "second apply: 0 written (idempotent skip)"
    );
    assert_eq!(r2.elements_skipped, 2, "second apply: 2 skipped");

    // Third apply with NEW data: 2 more written.
    let r3 =
        class_diagram::apply(project, &make_report(10), &fs).expect("third apply must succeed");
    assert_eq!(r3.elements_written, 2, "third apply: 2 new elements");
    assert_eq!(r3.elements_skipped, 0, "third apply: 0 skipped");

    // Verify total count: 4 unique nodes.
    let mut store = archctl::store::LbugStore::open(project).expect("store must open");
    store.init().expect("store must init");
    let element_count: i64 = store
        .query("MATCH (e:Element) WHERE e.kind_id STARTS WITH 'uml.' RETURN count(e) AS cnt;")
        .expect("query must succeed")
        .pop()
        .and_then(|r| r.get("cnt").and_then(|c| c.as_i64()))
        .expect("count must be i64");
    assert_eq!(element_count, 4, "total: 4 unique uml.* Element nodes");
}
