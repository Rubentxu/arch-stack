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

    // Idempotent: the key counts (elements_written, relations_written) must be identical.
    // First run: migrations applied + 1 element written. Second run: migrations skipped,
    // MERGE is idempotent so element count stays the same.
    fn extract_count(stdout: &str, field: &str) -> usize {
        for line in stdout.lines() {
            if line.contains(field) {
                // e.g. "Applied 1 elements (0 skipped), 0 relations ..."
                if let Some(n) = line.split_whitespace().find(|w| w.parse::<usize>().is_ok()) {
                    return n.parse().unwrap();
                }
            }
        }
        0
    }

    let e1 = extract_count(&first, "element");
    let e2 = extract_count(&second, "element");
    assert_eq!(
        e1, e2,
        "elements_written must be identical on both apply runs: first={e1}, second={e2}"
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
    use archctl::store::{GraphStore, LbugStore, RawGraphQuery};

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    let mut store = LbugStore::open(project).expect("store must open");
    store.init().expect("store must init");

    store.begin_transaction().expect("begin must succeed");
    store
        .query("MERGE (e:Element {id: 'class_diag:good'}) SET e.kind_id = 'k';")
        .expect("good write inside tx must succeed");

    // Trigger a binder error: SUPPORTED_BY is declared FROM
    // ElementVersion TO Evidence — so (Element)-[SUPPORTED_BY]->(Evidence)
    // violates the direction constraint.
    let bad = store.query(
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
