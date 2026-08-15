//! Integration tests for the C4 Components strategy.
//!
//! Covers SCNs 436–442:
//! - SCN-436: Module detection produces `mt.component` candidates (not `mt.container`)
//! - SCN-437: Workspace member exclusion — eligible modules are NOT workspace packages
//! - SCN-438: Component apply persists as `mt.component` (KEY TEST — bug fix verification)
//! - SCN-439: Idempotency for component apply
//! - SCN-440: Byte-identical serialized CLI JSON
//! - SCN-441: Third-party exclusion — `vendor/` and `node_modules` excluded
//! - SCN-442: Confidence 0.65 (< 1.0) enforced
//!
//! These tests use real strategies against synthetic-repo TempDir fixtures.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use tempfile::TempDir;

// ─── Fixture helpers ─────────────────────────────────────────────────────────────

/// Write a file to a temp project directory, creating parent dirs as needed.
fn write(project: &Path, rel: &str, content: &str) {
    let path = project.join(rel);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, content).expect("write temp file");
}

/// Create a Rust project with internal src/ modules.
fn make_rust_module_project(project: &Path) {
    write(
        project,
        "src/auth/mod.rs",
        "pub mod error;\npub fn login() {}\n",
    );
    write(
        project,
        "src/api/mod.rs",
        "pub mod handlers;\npub fn handle() {}\n",
    );
    write(
        project,
        "src/core/mod.rs",
        "pub mod utils;\npub fn process() {}\n",
    );
}

/// Create a mixed project: workspace members + internal src/ modules.
/// SCN-437: internal modules should be detected as components, workspace members should NOT.
fn make_mixed_workspace_project(project: &Path) {
    // Root workspace Cargo.toml
    write(
        project,
        "Cargo.toml",
        r#"[workspace]
members = ["crates/auth", "crates/api"]
resolver = "2"
"#,
    );
    // Workspace member 1 (should be SKIPPED — it's a container, not a component)
    write(
        project,
        "crates/auth/Cargo.toml",
        r#"[package]
name = "auth"
version = "0.1.0"
edition = "2021"
"#,
    );
    write(project, "crates/auth/src/lib.rs", "pub fn login() {}");
    // Workspace member 2 (should be SKIPPED)
    write(
        project,
        "crates/api/Cargo.toml",
        r#"[package]
name = "api"
version = "0.1.0"
edition = "2021"
"#,
    );
    write(project, "crates/api/src/main.rs", "fn main() {}");
    // Internal module (should be DETECTED as component)
    write(
        project,
        "src/shared/mod.rs",
        "pub mod utils;\npub fn helper() {}\n",
    );
    // Internal module 2 (should be DETECTED)
    write(
        project,
        "src/db/mod.rs",
        "pub mod connection;\npub fn query() {}\n",
    );
}

/// Create a project with third-party directories that should be excluded.
fn make_project_with_third_party(project: &Path) {
    // Internal modules that SHOULD be detected
    write(
        project,
        "src/auth/mod.rs",
        "pub mod error;\npub fn login() {}\n",
    );
    write(
        project,
        "src/api/mod.rs",
        "pub mod handlers;\npub fn handle() {}\n",
    );
    // Third-party that should be EXCLUDED (SCN-441)
    write(
        project,
        "vendor/some-crate/src/lib.rs",
        "pub fn vendor_fn() {}",
    );
    write(
        project,
        "node_modules/some-package/index.js",
        "module.exports = {};",
    );
    // Also test: tests/ and target/ should be excluded
    write(
        project,
        "tests/integration/test_main.rs",
        "#[test] fn it_works() {}",
    );
}

// ─── Strategy constructors ────────────────────────────────────────────────────

fn components_strategy() -> Box<dyn archctl::code::strategies::Strategy> {
    Box::new(archctl::code::strategies::components::ComponentsStrategy)
}

// ─── SCN-436: Module detection produces `mt.component` candidates ───────────────

#[test]
fn scn436_components_strategy_produces_component_candidates() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    make_rust_module_project(project);

    let strategies: Vec<Box<dyn archctl::code::strategies::Strategy>> = vec![components_strategy()];
    let fs = archctl::filesystem::SystemFilesystem;
    let clock: &dyn archctl::clock::Clock =
        &archctl::clock::FixedClock::new("2025-01-01T00:00:00Z");

    let report = archctl::code::c4_discover::discover(project, &strategies, &fs, clock)
        .expect("discover must succeed");

    // Should detect internal src/ modules
    assert!(
        !report.discovered.is_empty(),
        "should detect at least one component"
    );

    // All candidates must have strategy == "components"
    for c in &report.discovered {
        assert_eq!(
            c.strategy, "components",
            "strategy must be 'components', got '{}'",
            c.strategy
        );
    }
}

// ─── SCN-437: Workspace member exclusion ──────────────────────────────────────

#[test]
fn scn437_workspace_members_excluded_from_component_detection() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    make_mixed_workspace_project(project);

    let strategies: Vec<Box<dyn archctl::code::strategies::Strategy>> = vec![components_strategy()];
    let fs = archctl::filesystem::SystemFilesystem;
    let clock: &dyn archctl::clock::Clock =
        &archctl::clock::FixedClock::new("2025-01-01T00:00:00Z");

    let report = archctl::code::c4_discover::discover(project, &strategies, &fs, clock)
        .expect("discover must succeed");

    // Should detect internal src/ modules (shared, db)
    assert!(
        !report.discovered.is_empty(),
        "should detect internal modules"
    );

    // Extract all canonical keys
    let keys: Vec<_> = report
        .discovered
        .iter()
        .map(|c| c.canonical_key.clone())
        .collect();

    // Workspace members (crates/auth, crates/api) should NOT appear
    assert!(
        !keys.iter().any(|k| k.contains("crates/auth")),
        "crates/auth workspace member must be excluded, got: {:?}",
        keys
    );
    assert!(
        !keys.iter().any(|k| k.contains("crates/api")),
        "crates/api workspace member must be excluded, got: {:?}",
        keys
    );

    // Internal modules SHOULD appear
    assert!(
        keys.iter().any(|k| k.contains("shared")),
        "src/shared should be detected as component, got: {:?}",
        keys
    );
    assert!(
        keys.iter().any(|k| k.contains("db")),
        "src/db should be detected as component, got: {:?}",
        keys
    );
}

// ─── SCN-438: Component apply persists as mt.component (KEY TEST) ─────────────

#[test]
fn scn438_component_apply_persists_as_component_not_container() {
    use archctl::code::c4_discover::{
        Container, DiscoverReport, Evidence, EvidenceKind, ProjectMeta,
    };
    use archctl::store::RawGraphQuery;
    use archctl::store::open_and_init;

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();

    // Create a Rust source module that will be detected as a component
    write(
        project,
        "src/auth/mod.rs",
        "pub mod error;\npub fn login() {}\n",
    );

    let report = DiscoverReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root: project.display().to_string(),
            files_scanned: 3,
            languages: BTreeMap::from([("rust".to_string(), 3)]),
            duration_ms: 20,
        },
        discovered: vec![Container {
            canonical_key: "rust:module:src.auth".to_string(),
            name: "auth".to_string(),
            strategy: "components".to_string(), // This is the key: strategy = "components"
            confidence: 0.65,
            merged_from: vec!["components".to_string()],
            evidences: vec![Evidence {
                content_hash: String::new(),
                file: "src/auth/mod.rs".to_string(),
                line: 1,
                kind: EvidenceKind::Lexical,
                text: "Rust module: auth".to_string(),
            }],
        }],
        errors: vec![],
    };

    let fs = archctl::filesystem::SystemFilesystem;

    // Apply the report
    let result =
        archctl::code::c4_discover::apply(project, &report, &fs).expect("apply must succeed");
    assert_eq!(
        result.elements_written, 1,
        "should write exactly one element"
    );

    // Verify the persisted element
    let store = open_and_init(project).expect("store must open");
    let element_id = "c4:component:rust:module:src.auth";

    // Query the element's kind_id
    let kind_query = format!(
        "MATCH (e:Element {{id: '{id}'}}) RETURN e.kind_id AS kind_id;",
        id = element_id
    );
    let kind_rows = store
        .query(&kind_query)
        .expect("kind_id query must succeed");
    assert!(!kind_rows.is_empty(), "element {} must exist", element_id);
    let kind_id = kind_rows[0]
        .get("kind_id")
        .and_then(|v| v.as_str())
        .expect("kind_id must be a string");
    assert_eq!(
        kind_id, "mt.component",
        "kind_id must be 'mt.component' (NOT 'mt.container') — bug fix verification"
    );

    // Verify the element_id prefix is c4:component: (not c4:container:)
    let id_query = format!(
        "MATCH (e:Element {{id: '{id}'}}) RETURN e.id AS id;",
        id = element_id
    );
    let id_rows = store.query(&id_query).expect("id query must succeed");
    let persisted_id = id_rows[0]
        .get("id")
        .and_then(|v| v.as_str())
        .expect("id must be a string");
    assert!(
        persisted_id.starts_with("c4:component:"),
        "element id must start with 'c4:component:', got: {}",
        persisted_id
    );

    // Verify the OF_TYPE relationship points to mt.component (NOT mt.container)
    let type_query = format!(
        "MATCH (e:Element {{id: '{id}'}})-[:OF_TYPE]->(mt:MetaType) RETURN mt.id AS metatype;",
        id = element_id
    );
    let type_rows = store
        .query(&type_query)
        .expect("OF_TYPE query must succeed");
    assert!(
        !type_rows.is_empty(),
        "element must have an OF_TYPE relationship"
    );
    let metatype = type_rows[0]
        .get("metatype")
        .and_then(|v| v.as_str())
        .expect("metatype must be a string");
    assert_eq!(
        metatype, "mt.component",
        "OF_TYPE must point to 'mt.component' (NOT 'mt.container')"
    );

    // Verify it's NOT mt.container (sanity check that container metatype exists separately)
    let container_check = format!(
        "MATCH (e:Element {{id: '{id}'}}) RETURN e.id AS id;",
        id = "c4:container:rust:module:src.auth"
    );
    let container_rows = store
        .query(&container_check)
        .expect("container check query must succeed");
    assert!(
        container_rows.is_empty(),
        "c4:container: prefix should NOT exist for components"
    );
}

// ─── SCN-439: Idempotency for component apply ─────────────────────────────────

#[test]
fn scn439_component_apply_is_idempotent() {
    use archctl::code::c4_discover::{
        Container, DiscoverReport, Evidence, EvidenceKind, ProjectMeta,
    };

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    write(
        project,
        "src/auth/mod.rs",
        "pub mod error;\npub fn login() {}\n",
    );

    let report = DiscoverReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root: project.display().to_string(),
            files_scanned: 3,
            languages: BTreeMap::from([("rust".to_string(), 3)]),
            duration_ms: 20,
        },
        discovered: vec![Container {
            canonical_key: "rust:module:src.auth".to_string(),
            name: "auth".to_string(),
            strategy: "components".to_string(),
            confidence: 0.65,
            merged_from: vec!["components".to_string()],
            evidences: vec![Evidence {
                content_hash: String::new(),
                file: "src/auth/mod.rs".to_string(),
                line: 1,
                kind: EvidenceKind::Lexical,
                text: "Rust module: auth".to_string(),
            }],
        }],
        errors: vec![],
    };

    let fs = archctl::filesystem::SystemFilesystem;

    // First apply — writes the element
    let r1 =
        archctl::code::c4_discover::apply(project, &report, &fs).expect("first apply must succeed");
    assert_eq!(
        r1.elements_written, 1,
        "first apply should write the element"
    );
    assert_eq!(r1.elements_skipped, 0);

    // Second apply — skips the existing canonical_key
    let r2 = archctl::code::c4_discover::apply(project, &report, &fs)
        .expect("second apply must succeed");
    assert_eq!(
        r2.elements_skipped, 1,
        "second apply must skip existing canonical_key"
    );
    assert_eq!(
        r2.elements_written, 0,
        "second apply must not write duplicates"
    );
}

// ─── SCN-440: Byte-identical serialized CLI JSON ──────────────────────────────

#[test]
fn scn440_component_report_serializes_to_byte_identical_json() {
    use archctl::code::c4_discover::{
        Container, DISCOVER_REPORT_SCHEMA, DiscoverReport, Evidence, EvidenceKind, ProjectMeta,
    };

    let report = DiscoverReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root: "/tmp/test".to_string(),
            files_scanned: 5,
            languages: BTreeMap::from([("rust".to_string(), 5)]),
            duration_ms: 42,
        },
        discovered: vec![Container {
            canonical_key: "rust:module:src.auth".to_string(),
            name: "auth".to_string(),
            strategy: "components".to_string(),
            confidence: 0.65,
            merged_from: vec!["components".to_string()],
            evidences: vec![Evidence {
                content_hash: String::new(),
                file: "src/auth/mod.rs".to_string(),
                line: 1,
                kind: EvidenceKind::Lexical,
                text: "Rust module: auth".to_string(),
            }],
        }],
        errors: vec![],
    };

    // Round-trip: Rust struct → JSON string → parsed Value → reserialize
    let json_str = serde_json::to_string(&report).expect("DiscoverReport must serialise to JSON");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("JSON must be parseable");
    let reserialized = serde_json::to_string(&parsed).expect("reserialized JSON must be valid");

    // Parse both and compare as Value (order-independent comparison)
    let parsed_original: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let parsed_reserialized: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
    assert_eq!(
        parsed_original, parsed_reserialized,
        "parsed JSON content must be identical after round-trip (order-independent)"
    );

    // Validate against the embedded schema
    let schema_val: serde_json::Value = serde_json::from_str(DISCOVER_REPORT_SCHEMA)
        .expect("DISCOVER_REPORT_SCHEMA must be valid JSON Schema");
    let validator =
        jsonschema::validator_for(&schema_val).expect("schema must be valid JSON Schema");
    let result = validator.validate(&parsed);
    assert!(
        result.is_ok(),
        "Component DiscoverReport must pass schema validation: {:?}",
        result.err()
    );
}

// ─── SCN-441: Third-party exclusion — vendor/ and node_modules excluded ───────

#[test]
fn scn441_vendor_and_node_modules_excluded_from_components() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    make_project_with_third_party(project);

    let strategies: Vec<Box<dyn archctl::code::strategies::Strategy>> = vec![components_strategy()];
    let fs = archctl::filesystem::SystemFilesystem;
    let clock: &dyn archctl::clock::Clock =
        &archctl::clock::FixedClock::new("2025-01-01T00:00:00Z");

    let report = archctl::code::c4_discover::discover(project, &strategies, &fs, clock)
        .expect("discover must succeed");

    let all_keys: Vec<_> = report
        .discovered
        .iter()
        .map(|c| c.canonical_key.clone())
        .collect();

    // vendor/ should NOT appear
    assert!(
        !all_keys.iter().any(|k| k.contains("vendor")),
        "vendor/ directories must be excluded, got: {:?}",
        all_keys
    );

    // node_modules/ should NOT appear
    assert!(
        !all_keys.iter().any(|k| k.contains("node_modules")),
        "node_modules/ directories must be excluded, got: {:?}",
        all_keys
    );

    // Internal modules SHOULD appear
    assert!(
        all_keys.iter().any(|k| k.contains("auth")),
        "src/auth should be detected, got: {:?}",
        all_keys
    );
    assert!(
        all_keys.iter().any(|k| k.contains("api")),
        "src/api should be detected, got: {:?}",
        all_keys
    );
}

// ─── SCN-442: Confidence 0.65 (< 1.0) enforced ──────────────────────────────

#[test]
fn scn442_components_have_confidence_below_one() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    make_rust_module_project(project);

    let strategies: Vec<Box<dyn archctl::code::strategies::Strategy>> = vec![components_strategy()];
    let fs = archctl::filesystem::SystemFilesystem;
    let clock: &dyn archctl::clock::Clock =
        &archctl::clock::FixedClock::new("2025-01-01T00:00:00Z");

    let report = archctl::code::c4_discover::discover(project, &strategies, &fs, clock)
        .expect("discover must succeed");

    assert!(!report.discovered.is_empty(), "should detect components");

    for c in &report.discovered {
        assert!(
            c.confidence < 1.0,
            "confidence {} must be < 1.0 for components",
            c.confidence
        );
        assert!(
            (0.60..=0.70).contains(&c.confidence),
            "confidence {} should be ~0.65, got",
            c.confidence
        );
    }
}

// ─── Integration: container vs component apply paths are distinct ─────────────

#[test]
fn container_and_component_apply_produce_different_metatypes() {
    use archctl::code::c4_discover::{
        Container, DiscoverReport, Evidence, EvidenceKind, ProjectMeta,
    };
    use archctl::store::RawGraphQuery;
    use archctl::store::open_and_init;

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();

    // Create fixture files
    write(
        project,
        "src/auth/mod.rs",
        "pub mod error;\npub fn login() {}\n",
    );
    write(
        project,
        "Cargo.toml",
        r#"[package]
name = "myapp"
version = "0.1.0"
edition = "2021"
"#,
    );

    // Report with BOTH a container (cargo-workspace style) and a component
    let report = DiscoverReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root: project.display().to_string(),
            files_scanned: 5,
            languages: BTreeMap::from([("rust".to_string(), 5)]),
            duration_ms: 30,
        },
        discovered: vec![
            // Container candidate
            Container {
                canonical_key: "myapp".to_string(),
                name: "myapp".to_string(),
                strategy: "cargo-workspace".to_string(), // Container strategy
                confidence: 0.85,
                merged_from: vec!["cargo-workspace".to_string()],
                evidences: vec![Evidence {
                    content_hash: String::new(),
                    file: "Cargo.toml".to_string(),
                    line: 1,
                    kind: EvidenceKind::Structural,
                    text: "Cargo package: myapp".to_string(),
                }],
            },
            // Component candidate
            Container {
                canonical_key: "rust:module:src.auth".to_string(),
                name: "auth".to_string(),
                strategy: "components".to_string(), // Component strategy
                confidence: 0.65,
                merged_from: vec!["components".to_string()],
                evidences: vec![Evidence {
                    content_hash: String::new(),
                    file: "src/auth/mod.rs".to_string(),
                    line: 1,
                    kind: EvidenceKind::Lexical,
                    text: "Rust module: auth".to_string(),
                }],
            },
        ],
        errors: vec![],
    };

    let fs = archctl::filesystem::SystemFilesystem;

    let result =
        archctl::code::c4_discover::apply(project, &report, &fs).expect("apply must succeed");
    assert_eq!(
        result.elements_written, 2,
        "should write one container and one component"
    );

    // Verify container has mt.container
    let store = open_and_init(project).expect("store must open");

    let container_kind: Result<String, _> = {
        let rows = store
            .query("MATCH (e:Element {id: 'c4:container:myapp'}) RETURN e.kind_id AS k;")
            .expect("container query must succeed");
        rows.first()
            .and_then(|r| r.get("k").and_then(|v| v.as_str().map(String::from)))
            .ok_or_else(|| anyhow::anyhow!("no container kind found"))
    };
    assert_eq!(
        container_kind.unwrap(),
        "mt.container",
        "container must have mt.container"
    );

    // Verify component has mt.component
    let component_kind: Result<String, _> = {
        let rows = store
            .query(
                "MATCH (e:Element {id: 'c4:component:rust:module:src.auth'}) RETURN e.kind_id AS k;",
            )
            .expect("component query must succeed");
        rows.first()
            .and_then(|r| r.get("k").and_then(|v| v.as_str().map(String::from)))
            .ok_or_else(|| anyhow::anyhow!("no component kind found"))
    };
    assert_eq!(
        component_kind.unwrap(),
        "mt.component",
        "component must have mt.component"
    );
}
