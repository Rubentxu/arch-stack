//! Integration tests for the call-graph extraction engine.
//!
//! These tests use real filesystem + direct tree-sitter extraction against synthetic TempDir fixtures.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use tempfile::TempDir;

use archctl::Row;
use archctl::code::call_graph::{self, Language};
use archctl::filesystem::SystemFilesystem;
use archctl::store::{GraphStore, LbugStore};

// ─── Integration tests ─────────────────────────────────────────────────────────

#[test]
fn test_call_graph_report_schema_version() {
    // Minimal report to verify schemaVersion field
    let report = archctl::code::call_graph::CallGraphReport {
        schema_version: "1.0".to_string(),
        project: archctl::code::call_graph::ProjectMeta {
            root: "/tmp".to_string(),
            files_scanned: 0,
            languages: BTreeMap::new(),
            duration_ms: 0,
        },
        nodes: vec![],
        edges: vec![],
        errors: vec![],
    };

    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains("\"schemaVersion\":\"1.0\""));
}

#[test]
fn test_language_enum_value_variants() {
    use clap::ValueEnum;
    let variants = archctl::code::call_graph::Language::value_variants();
    assert!(variants.contains(&archctl::code::call_graph::Language::Rust));
    assert!(variants.contains(&archctl::code::call_graph::Language::TypeScript));
    assert!(variants.contains(&archctl::code::call_graph::Language::Python));
}

#[test]
fn test_apply_report_fields() {
    let report = archctl::code::call_graph::ApplyReport {
        elements_written: 5,
        elements_skipped: 2,
        relations_written: 3,
        relations_skipped: 1,
        evidences_written: 3,
        source_artifacts_written: 2,
        seed_writes: 1,
        duration_ms: 42,
    };

    assert_eq!(report.elements_written, 5);
    assert_eq!(report.elements_skipped, 2);
    assert_eq!(report.seed_writes, 1);
}

#[test]
fn test_call_graph_error_serialize() {
    let err = archctl::code::call_graph::ExtractError {
        strategy: "rust".to_string(),
        path: "src/lib.rs".to_string(),
        message: "TSG parse error".to_string(),
    };
    let json = serde_json::to_string(&err).unwrap();
    assert!(json.contains("TSG parse error"));
    assert!(json.contains("rust"));
}

// ─── Regression: apply round-trips to graph store ─────────────────────────────────

/// Write a file to a temp project directory, creating parent dirs as needed.
fn write(project: &Path, rel: &str, content: &str) {
    let path = project.join(rel);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, content).expect("write temp file");
}

#[test]
fn test_call_graph_apply_persists_elements_and_evidences() {
    // Smoke fixture: caller() calls helper() and other_helper()
    let tmp = TempDir::new().unwrap();
    let project = tmp.path();

    write(
        project,
        "Cargo.toml",
        r#"[package]
name = "smoke"
version = "0.1.0"
edition = "2021"
"#,
    );
    write(
        project,
        "src/lib.rs",
        "pub fn caller() { helper(); other_helper(); }\n\
         pub fn helper() {}\n\
         pub fn other_helper() {}\n",
    );

    // Extract call graph
    let fs = SystemFilesystem;
    let report =
        call_graph::extract(project, &[Language::Rust], None, &fs).expect("extract must succeed");
    assert_eq!(report.nodes.len(), 3, "expected 3 function nodes");
    assert_eq!(report.edges.len(), 2, "expected 2 call edges");

    // Apply to graph store
    let r = call_graph::apply(project, &report, &SystemFilesystem).expect("apply must succeed");
    assert_eq!(r.elements_written, 3, "should write 3 elements");
    assert_eq!(r.relations_written, 2, "should write 2 relations");
    assert_eq!(r.evidences_written, 2, "should write 2 evidences");

    // Verify Element rows persisted via LbugStore query
    let mut store = LbugStore::open(project).expect("store must open");
    store.init().expect("store must init");

    let elements: Vec<Row> = store
        .query("MATCH (e:Element) WHERE e.kind_id = 'code.function' RETURN count(e) AS cnt;")
        .expect("element count query must succeed");
    let cnt = elements[0]
        .get("cnt")
        .and_then(|c| c.as_i64())
        .expect("count must be i64");
    assert_eq!(cnt, 3, "expected 3 code.function Element rows");

    // Verify Evidence rows with derived classification persisted
    let evidences: Vec<Row> = store
        .query("MATCH (ev:Evidence {classification: 'derived'}) RETURN count(ev) AS cnt;")
        .expect("evidence count query must succeed");
    let ev_cnt = evidences[0]
        .get("cnt")
        .and_then(|c| c.as_i64())
        .expect("evidence count must be i64");
    assert!(ev_cnt >= 2, "expected at least 2 derived Evidence rows");
}
