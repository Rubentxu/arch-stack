//! Integration tests for the call-graph extraction engine.
//!
//! These tests use real filesystem + direct tree-sitter extraction against synthetic TempDir fixtures.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use tempfile::TempDir;

// ─── Fixture helpers ────────────────────────────────────────────────────────────

/// Write a file to a project directory, creating parent dirs as needed.
fn write(project: &Path, rel: &str, content: &str) {
    let path = project.join(rel);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, content).expect("write temp file");
}

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
