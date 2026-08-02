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
