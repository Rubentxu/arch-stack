//! Integration tests for the CURRENT_VERSION relationship integrity contract.
//!
//! CRITICAL-1 regression test: for every writer (call_graph, state_machine,
//! class_diagram, c4_discover), after apply, every Element with a CURRENT_VERSION
//! edge must satisfy `element.current_version_id == version.id`.
//!
//! This is the graph integrity contract that was broken when uuid::Uuid::new_v4()
//! was used independently in two places, generating mismatched version IDs.

use tempfile::TempDir;

use archctl::store::{GraphStore, LbugStore, RawGraphQuery};

/// Verify the CURRENT_VERSION integrity contract: no Element with a CURRENT_VERSION
/// edge has mismatched element.current_version_id vs version.id.
fn assert_current_version_integrity(store: &LbugStore, writer_name: &str) {
    // Find any Element with CURRENT_VERSION where the IDs don't match
    let mismatches = store
        .query(
            "MATCH (e:Element)-[:CURRENT_VERSION]->(v:ElementVersion) \
             WHERE e.current_version_id <> v.id \
             RETURN e.id AS element_id, e.current_version_id AS elem_ver, v.id AS version_id;",
        )
        .expect("current_version integrity query must succeed");

    if !mismatches.is_empty() {
        for row in &mismatches {
            let elem_id = row.get("element_id").and_then(|c| c.as_str()).unwrap_or("?");
            let elem_ver = row.get("elem_ver").and_then(|c| c.as_str()).unwrap_or("?");
            let ver_id = row.get("version_id").and_then(|c| c.as_str()).unwrap_or("?");
            eprintln!(
                "CURRENT_VERSION mismatch in {}: element_id={}, element.current_version_id={}, version.id={}",
                writer_name, elem_id, elem_ver, ver_id
            );
        }
    }

    assert!(
        mismatches.is_empty(),
        "CURRENT_VERSION integrity violated for {}: {} elements have mismatched current_version_id/id",
        writer_name,
        mismatches.len()
    );
}

// ─── call_graph ────────────────────────────────────────────────────────────────

/// Regression test: call_graph::apply must produce consistent CURRENT_VERSION edges.
#[test]
fn call_graph_current_version_integrity() {
    use archctl::code::call_graph::{CallGraphReport, FunctionKind, Language, ProjectMeta};
    use archctl::filesystem::SystemFilesystem;

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();

    // Minimal call_graph report with one function node
    let report = CallGraphReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root: project.to_string_lossy().to_string(),
            files_scanned: 1,
            languages: [("rust".to_string(), 1)].into(),
            duration_ms: 0,
        },
        nodes: vec![archctl::code::call_graph::FunctionNode {
            canonical_key: "rust:src/lib.rs:function:main:10".to_string(),
            kind: FunctionKind::Function,
            language: Language::Rust,
            file: "src/lib.rs".to_string(),
            content_hash: "sha256:abc".to_string(),
            line: 10,
            name: "main".to_string(),
            fq_name: "crate::main".to_string(),
            confidence: 0.95,
            parent: None,
        }],
        edges: vec![],
        errors: vec![],
    };

    archctl::code::call_graph::apply(project, &report, &SystemFilesystem)
        .expect("call_graph apply must succeed");

    let mut store = LbugStore::open(project).expect("store must open");
    store.init().expect("store must init");

    assert_current_version_integrity(&store, "call_graph");
}

// ─── state_machine ───────────────────────────────────────────────────────────

/// Regression test: state_machine::apply must produce consistent CURRENT_VERSION edges.
#[test]
fn state_machine_current_version_integrity() {
    use archctl::code::state_machine::{
        State, StateKind, StateMachine, StateMachineReport, Transition,
    };
    use archctl::filesystem::SystemFilesystem;

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();

    let report = StateMachineReport {
        schema_version: "1.0".to_string(),
        project: archctl::code::state_machine::ProjectMeta {
            root: project.to_string_lossy().to_string(),
            files_scanned: 1,
            languages: [("rust".to_string(), 1)].into(),
        },
        machines: vec![StateMachine {
            canonical_key: "rust:src/lib.rs:state_machine:Test:3".to_string(),
            name: "TestSM".to_string(),
            file: "src/lib.rs".to_string(),
            content_hash: "sha256:abc".to_string(),
            states: vec![
                State { name: "S1".to_string(), kind: StateKind::Initial, line: 4 },
                State { name: "S2".to_string(), kind: StateKind::Regular, line: 5 },
            ],
            transitions: vec![Transition {
                from: "S1".to_string(),
                to: "S2".to_string(),
                trigger: None,
                guard: None,
                line: 7,
            }],
            confidence: 0.90,
        }],
    };

    archctl::code::state_machine::apply(project, &report, &SystemFilesystem)
        .expect("state_machine apply must succeed");

    let mut store = LbugStore::open(project).expect("store must open");
    store.init().expect("store must init");

    assert_current_version_integrity(&store, "state_machine");
}

// ─── class_diagram ───────────────────────────────────────────────────────────

/// Regression test: class_diagram::apply must produce consistent CURRENT_VERSION edges.
#[test]
fn class_diagram_current_version_integrity() {
    use archctl::code::class_diagram::{ClassDiagramReport, ClassMember, ClassNode};
    use archctl::filesystem::SystemFilesystem;

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();

    let report = ClassDiagramReport {
        schema_version: "1.0".to_string(),
        project: archctl::code::class_diagram::ProjectMeta {
            root: project.to_string_lossy().to_string(),
            files_scanned: 1,
            languages: [("rust".to_string(), 1)].into(),
        },
        nodes: vec![ClassNode {
            canonical_key: "rust:src/lib.rs:class:Foo:5".to_string(),
            kind: archctl::code::class_diagram::TypeKind::Class,
            language: archctl::code::class_diagram::Language::Rust,
            file: "src/lib.rs".to_string(),
            line: 5,
            name: "Foo".to_string(),
            members: vec![ClassMember {
                name: "field1".to_string(),
                member_kind: "field".to_string(),
                signature: "i32".to_string(),
                line: 6,
            }],
            confidence: 0.95,
        }],
        edges: vec![],
        errors: vec![],
    };

    archctl::code::class_diagram::apply(project, &report, &SystemFilesystem)
        .expect("class_diagram apply must succeed");

    let mut store = LbugStore::open(project).expect("store must open");
    store.init().expect("store must init");

    assert_current_version_integrity(&store, "class_diagram");
}

// ─── c4_discover ─────────────────────────────────────────────────────────────

/// Regression test: c4_discover::apply must produce consistent CURRENT_VERSION edges.
#[test]
fn c4_discover_current_version_integrity() {
    use archctl::code::c4_discover::{
        Container, DiscoverReport, Evidence, EvidenceKind, ProjectMeta,
    };
    use archctl::filesystem::SystemFilesystem;

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();

    let report = DiscoverReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root: project.to_string_lossy().to_string(),
            files_scanned: 1,
            languages: [("rust".to_string(), 1)].into(),
            duration_ms: 0,
        },
        discovered: vec![Container {
            canonical_key: "rust:src/lib.rs:container:MyContainer:1".to_string(),
            name: "MyContainer".to_string(),
            strategy: "single".to_string(),
            confidence: 0.90,
            merged_from: vec![],
            evidences: vec![Evidence {
                file: "src/lib.rs".to_string(),
                content_hash: "sha256:abc".to_string(),
                line: 1,
                kind: EvidenceKind::Structural,
                text: "container declaration".to_string(),
            }],
        }],
        errors: vec![],
    };

    archctl::code::c4_discover::apply(project, &report, &SystemFilesystem)
        .expect("c4_discover apply must succeed");

    let mut store = LbugStore::open(project).expect("store must open");
    store.init().expect("store must init");

    assert_current_version_integrity(&store, "c4_discover");
}
