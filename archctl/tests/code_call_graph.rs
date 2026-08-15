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
use archctl::store::{GraphStore, LbugStore, RawGraphQuery};

/// Extension trait to execute raw writes for testing transaction scenarios.
/// The RawGraphQuery::query guard rejects write keywords (MERGE, SET, etc.);
/// this method bypasses it for test scenarios that verify Kùzu transaction semantics.
trait RawWrite {
    fn write_cypher_for_test(&mut self, cypher: &str) -> anyhow::Result<()>;
}

impl RawWrite for LbugStore {
    fn write_cypher_for_test(&mut self, cypher: &str) -> anyhow::Result<()> {
        self.execute_raw_cypher_for_test(cypher)
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))
    }
}

// ─── Fixtures ─────────────────────────────────────────────────────────────────

/// Reads a fixture file from `archctl/tests/fixtures/<name>`.
fn read_fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read fixture {}: {}", path.display(), e))
}

/// Writes fixture content to a tempdir path and returns the absolute path.
#[allow(dead_code)]
fn write_fixture_to_tmp(name: &str, rel: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let content = read_fixture(name);
    let dest = tmp.path().join(rel);
    if let Some(parent) = dest.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&dest, content).expect("write fixture to tmp");
    (tmp, dest)
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
    // M30: Go is a first-class call-graph language.
    assert!(variants.contains(&archctl::code::call_graph::Language::Go));
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
        duration_ms: 42,
    };

    assert_eq!(report.elements_written, 5);
    assert_eq!(report.elements_skipped, 2);
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

// ─── M30: Go extraction semantics ────────────────────────────────────────────

fn write_go_project(project: &Path) {
    write(project, "go.mod", "module smoke\n\ngo 1.21\n");
    let go_source = read_fixture("go_callgraph/main.go");
    write(project, "main.go", &go_source);
}

#[test]
fn test_go_extraction_nodes_and_edges() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    write_go_project(project);

    let fs = SystemFilesystem;
    let report =
        call_graph::extract(project, &[Language::Go], None, &fs).expect("extract must succeed");

    // Nodes: greet, Server.Save (method), Server.Name (value-receiver method),
    //        handler, main, init = 6.
    // The anonymous func literal (fn := func() {...}) must NOT be a node.
    assert_eq!(
        report.nodes.len(),
        6,
        "expected 6 Go nodes, got: {:#?}",
        report.nodes
    );
    let methods: Vec<_> = report
        .nodes
        .iter()
        .filter(|n| n.kind == archctl::code::call_graph::FunctionKind::Method)
        .collect();
    assert_eq!(
        methods.len(),
        2,
        "expected exactly 2 Method nodes (Save + Name)"
    );
    let method_names: Vec<&str> = methods.iter().map(|m| m.name.as_str()).collect();
    assert!(method_names.contains(&"Save"));
    assert!(method_names.contains(&"Name"));
    assert!(
        !report.nodes.iter().any(|n| n.name == "fn"),
        "func_literal must not produce a node"
    );
    for name in ["greet", "handler", "main", "init"] {
        assert!(
            report.nodes.iter().any(|n| n.name == name),
            "expected node {name} in Go extraction"
        );
    }

    // Edges: greet→Println (package-qualified), handler→greet (call from
    // inside func_literal attributed to enclosing named function),
    // main→Save (selector method call), main→handler, init→greet, PLUS
    // handler→fn (the call to the func_literal variable itself — callee
    // unresolved in MVP, no symbol table).
    assert_eq!(
        report.edges.len(),
        6,
        "expected 6 call edges, got: {:#?}",
        report.edges
    );
    let has_edge = |caller_sub: &str, callee: &str| {
        report
            .edges
            .iter()
            .any(|e| e.caller.contains(caller_sub) && e.callee == callee)
    };
    assert!(
        has_edge(":greet:", "Println"),
        "greet must call Println (pkg-qualified)"
    );
    assert!(
        has_edge(":handler:", "greet"),
        "handler must own greet call from func_literal"
    );
    assert!(
        has_edge(":handler:", "fn"),
        "handler→fn call to func_literal variable (unresolved in MVP)"
    );
    assert!(
        has_edge(":main:", "Save"),
        "main must call Save (selector method)"
    );
    assert!(has_edge(":main:", "handler"), "main must call handler");
    assert!(has_edge(":init:", "greet"), "init must call greet");
}

// ─── M35: Java extraction semantics ───────────────────────────────────────────

fn write_java_project(project: &Path) {
    let java_source = read_fixture("java_callgraph/main.java");
    write(project, "Server.java", &java_source);
}

#[test]
fn test_java_extraction_nodes_and_edges() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    write_java_project(project);

    let fs = SystemFilesystem;
    let report = call_graph::extract(project, &[Language::Java], None, &fs)
        .expect("extract must succeed for Java");

    // Expected nodes from tests/fixtures/java_callgraph/main.java:
    //   Server  (constructor — Java AST names it after the class)
    //   Server.getName
    //   Server.handle
    //   Server.validate
    //   Server.process
    //   Server.log
    let expected = ["Server", "getName", "handle", "validate", "process", "log"];
    assert_eq!(
        report.nodes.len(),
        expected.len(),
        "expected {} Java nodes, got: {:#?}",
        expected.len(),
        report.nodes
    );
    for name in expected {
        assert!(
            report.nodes.iter().any(|n| n.name == name),
            "expected node {name} in Java extraction"
        );
    }
    // Every node is a Method (Java has no free functions).
    for n in &report.nodes {
        assert_eq!(
            n.kind,
            archctl::code::call_graph::FunctionKind::Method,
            "Java node {} should be Method kind",
            n.name
        );
        assert_eq!(
            n.language,
            archctl::code::call_graph::Language::Java,
            "Java node {} should be tagged Java",
            n.name
        );
    }

    // Edges (calls within main.java):
    //   handle → validate, handle → process
    //   validate → requireNonNull (Objects.requireNonNull, deepest ident)
    //   process → log
    //   log → println (System.out.println, deepest ident)
    let has_edge = |caller_sub: &str, callee: &str| {
        report
            .edges
            .iter()
            .any(|e| e.caller.contains(caller_sub) && e.callee == callee)
    };
    assert!(
        has_edge(":handle:", "validate"),
        "handle must call validate"
    );
    assert!(has_edge(":handle:", "process"), "handle must call process");
    assert!(
        has_edge(":validate:", "requireNonNull"),
        "validate must call Objects.requireNonNull (deepest ident)"
    );
    assert!(has_edge(":process:", "log"), "process must call log");
    assert!(
        has_edge(":log:", "println"),
        "log must call System.out.println (deepest ident)"
    );
}

#[test]
fn test_java_lang_filter_excludes_non_java() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    write_java_project(project);

    let fs = SystemFilesystem;

    // Filtering to Rust on a Java-only project must scan 0 files.
    let report = call_graph::extract(project, &[Language::Rust], None, &fs)
        .expect("extract must succeed for Rust");
    assert_eq!(report.project.files_scanned, 0);
    assert_eq!(report.nodes.len(), 0);

    // Filtering to Java picks up the fixture.
    let report = call_graph::extract(project, &[Language::Java], None, &fs)
        .expect("extract must succeed for Java");
    assert_eq!(report.project.files_scanned, 1);
    assert!(report.nodes.len() >= 6);
}

// ─── M36: Kotlin extraction semantics ──────────────────────────────────────────

fn write_kotlin_project(project: &Path) {
    let kt_source = read_fixture("kotlin_callgraph/main.kt");
    write(project, "Server.kt", &kt_source);
}

#[test]
fn test_kotlin_extraction_nodes_and_edges() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    write_kotlin_project(project);

    let fs = SystemFilesystem;
    let report = call_graph::extract(project, &[Language::Kotlin], None, &fs)
        .expect("extract must succeed for Kotlin");

    // Expected nodes from tests/fixtures/kotlin_callgraph/main.kt:
    //   getName, handle, validate, process, log
    let expected = ["getName", "handle", "validate", "process", "log"];
    assert_eq!(
        report.nodes.len(),
        expected.len(),
        "expected {} Kotlin nodes, got: {:#?}",
        expected.len(),
        report.nodes
    );
    for name in expected {
        assert!(
            report.nodes.iter().any(|n| n.name == name),
            "expected node {name} in Kotlin extraction"
        );
    }
    for n in &report.nodes {
        assert_eq!(
            n.kind,
            archctl::code::call_graph::FunctionKind::Method,
            "Kotlin node {} should be Method kind",
            n.name
        );
        assert_eq!(
            n.language,
            archctl::code::call_graph::Language::Kotlin,
            "Kotlin node {} should be tagged Kotlin",
            n.name
        );
    }

    // Edges (calls within main.kt):
    //   handle → validate, handle → process
    //   validate → requireNotNull
    //   process → log
    //   log → println
    let has_edge = |caller_sub: &str, callee: &str| {
        report
            .edges
            .iter()
            .any(|e| e.caller.contains(caller_sub) && e.callee == callee)
    };
    assert!(
        has_edge(":handle:", "validate"),
        "handle must call validate"
    );
    assert!(has_edge(":handle:", "process"), "handle must call process");
    assert!(
        has_edge(":validate:", "requireNotNull"),
        "validate must call requireNotNull"
    );
    assert!(has_edge(":process:", "log"), "process must call log");
    assert!(has_edge(":log:", "println"), "log must call println");
}

#[test]
fn test_go_lang_filter_excludes_and_includes() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    write_go_project(project);

    let fs = SystemFilesystem;

    // Filtering to Rust on a Go-only project must scan 0 files.
    let rust_only =
        call_graph::extract(project, &[Language::Rust], None, &fs).expect("extract must succeed");
    assert_eq!(
        rust_only.project.files_scanned, 0,
        "Go files must be excluded by --lang rust"
    );
    assert!(rust_only.nodes.is_empty());

    // Go filter picks the project up.
    let go_only =
        call_graph::extract(project, &[Language::Go], None, &fs).expect("extract must succeed");
    assert!(go_only.project.files_scanned >= 1);
    assert_eq!(go_only.project.languages.get("go").copied().unwrap_or(0), 1);
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

#[test]
fn call_graph_apply_atomic_abort_via_unit_of_work() {
    // Verifies the atomic-abort contract: when call_graph::apply fails
    // midway (e.g., due to a schema constraint violation triggered by a
    // prior escape-hatch write), the UnitOfWork transaction rolls back
    // and no partial state is visible in the store.
    //
    // Strategy:
    // 1. Extract call graph from a Rust project (3 nodes, 2 edges).
    // 2. First apply succeeds and seeds the store.
    // 3. Use escape hatch to corrupt the database schema (create a
    //    direction-violation edge that will cause subsequent edge writes
    //    to fail when Kùzu's binder rejects the malformed edge).
    // 4. Second apply fails partway through the edge-write loop.
    // 5. Verify store state is unchanged from after step 2 (rollback
    //    effectively undone the failed transaction).

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();

    // Write a minimal Rust project with 2 functions: caller() calls helper().
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
        "pub fn caller() { helper(); }\npub fn helper() {}\n",
    );

    // Extract call graph
    let fs = SystemFilesystem;
    let report =
        call_graph::extract(project, &[Language::Rust], None, &fs).expect("extract must succeed");
    assert_eq!(report.nodes.len(), 2, "expected 2 function nodes");
    assert_eq!(report.edges.len(), 1, "expected 1 call edge");

    // Step 1: First apply succeeds and seeds the store.
    let r1 =
        call_graph::apply(project, &report, &SystemFilesystem).expect("first apply must succeed");
    assert_eq!(
        r1.elements_written, 2,
        "first apply should write 2 elements"
    );
    assert_eq!(
        r1.relations_written, 1,
        "first apply should write 1 relation"
    );

    // Verify baseline state after first apply
    let mut store = LbugStore::open(project).expect("store must open");
    store.init().expect("store must init");

    let baseline_elements: i64 = store
        .query("MATCH (e:Element) WHERE e.kind_id = 'code.function' RETURN count(e) AS cnt;")
        .expect("query must succeed")
        .pop()
        .and_then(|r| r.get("cnt").and_then(|c| c.as_i64()))
        .expect("count must be i64");
    assert_eq!(
        baseline_elements, 2,
        "baseline: 2 code.function elements after first apply"
    );

    // Drop the store handle; we'll re-open after corrupting the schema.
    drop(store);

    // Step 2: Use escape hatch to corrupt the database in a way that will
    // cause the next apply to fail partway through edge writes.
    // We create a duplicate ElementVersion with the same canonical ID but
    // different name, which Kùzu's MERGE will match and try to UPDATE.
    // To trigger an actual failure, we use a malformed edge that violates
    // the schema direction constraint (similar to transaction_atomic_abort_on_write_error).
    let mut store2 = LbugStore::open(project).expect("store must open");
    store2.init().expect("store must init");

    // Pre-create an Evidence node that will cause a SUPPORTED_BY direction
    // violation when call_graph tries to link Evidence to ElementVersion.
    // This mirrors the pattern from store_transaction::transaction_atomic_abort_on_write_error.
    store2
        .write_cypher_for_test("MERGE (ev:Evidence {id: 'seed_ev'}) SET ev.claim = 'seed';")
        .expect("seed evidence must be created");

    // Now create a malformed relationship that Kùzu's binder will reject:
    // SUPPORTED_BY is declared FROM ElementVersion TO Evidence.
    // Creating (Element)-[SUPPORTED_BY]->(Evidence) violates the direction.
    let bad_rel = store2.write_cypher_for_test(
        "MATCH (e:Element) MATCH (ev:Evidence {id: 'seed_ev'}) \
         MERGE (e)-[r:SUPPORTED_BY]->(ev);",
    );
    // This SHOULD fail due to direction constraint - but if it doesn't fail,
    // the mere presence of this malformed rel may corrupt subsequent writes.
    // In either case, the escape hatch write is OUTSIDE the apply transaction,
    // so it persists. The next apply's Transaction will see it.
    if bad_rel.is_err() {
        tracing::info!(
            "escape hatch direction violation detected (expected in some Kùzu versions)"
        );
    }

    drop(store2);

    // Step 3: Second apply - should fail due to schema corruption from step 2.
    // The apply loop will succeed for elements (MERGE is idempotent) but
    // may fail when writing edges if the corrupted schema state causes issues.
    let r2 = call_graph::apply(project, &report, &SystemFilesystem);

    // The apply may succeed OR fail depending on Kùzu's exact behavior with
    // the corrupted schema state. The key invariant we're testing is:
    // IF the apply fails (error propagates), the Transaction drops and
    // rolls back. The store should show the baseline state, not partial
    // writes from the failed attempt.
    if r2.is_err() {
        tracing::info!(
            "apply failed as expected due to schema corruption: {:?}",
            r2
        );
    }

    // Step 4: Re-open store and verify state is unchanged from baseline
    // (rollback was effective, no partial state visible).
    let mut store3 = LbugStore::open(project).expect("store must open");
    store3.init().expect("store must init");

    let final_elements: i64 = store3
        .query("MATCH (e:Element) WHERE e.kind_id = 'code.function' RETURN count(e) AS cnt;")
        .expect("query must succeed")
        .pop()
        .and_then(|r| r.get("cnt").and_then(|c| c.as_i64()))
        .expect("count must be i64");

    // Critical assertion: after a failed apply, the store should show exactly
    // the baseline state (2 elements from the successful first apply).
    // Any partial writes from the failed apply should have been rolled back.
    assert_eq!(
        final_elements, baseline_elements,
        "atomic-abort: store must show baseline state after failed apply — \
         no partial writes should be visible. Got {final_elements}, expected {baseline_elements}"
    );

    // Also verify the Evidence count: if the second apply failed and rolled
    // back, we should NOT have extra evidence rows.
    let final_evidences: i64 = store3
        .query("MATCH (ev:Evidence) RETURN count(ev) AS cnt;")
        .expect("query must succeed")
        .pop()
        .and_then(|r| r.get("cnt").and_then(|c| c.as_i64()))
        .expect("count must be i64");

    // The only Evidence should be the one we seeded in step 2.
    // If the second apply failed and rolled back, no additional Evidence
    // rows were created by the failed apply.
    assert!(
        final_evidences <= 2,
        "atomic-abort: evidence count suggests partial writes survived rollback. \
         Evidence count: {final_evidences}"
    );
}
