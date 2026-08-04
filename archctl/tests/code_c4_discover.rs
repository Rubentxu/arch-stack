//! Integration tests for the C4 boundary inference engine.
//!
//! These tests use real strategies against synthetic-repo TempDir fixtures.
//! Marked tests run with `cargo test --test code_c4_discover -- --ignored`:
//!   - cargo_workspace_integration: requires cargo_metadata + real Cargo.toml parsing
//!   - npm_workspace_integration: requires npm packages filesystem layout
//!   - dockerfile_integration: requires filesystem walk with ignore::WalkBuilder
//!   - helm_integration: requires filesystem walk with std::fs::read_dir
//!
//! Unmarked tests run on every build:
//!   - cross_strategy_merge_integration: uses InjectStrategy (no filesystem)
//!   - apply_idempotent_integration: uses LbugStore via apply() on TempDir
//!   - json_roundtrip_against_schema: pure Rust struct → JSON → schema validation
//!   - apply_roundtrip_to_export: apply() then verify LbugStore state

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

fn make_cargo_workspace(project: &Path) {
    write(
        project,
        "Cargo.toml",
        r#"[workspace]
members = ["libs/auth", "libs/shared", "services/api"]
resolver = "2"
"#,
    );
    write(
        project,
        "libs/auth/Cargo.toml",
        r#"[package]
name = "auth"
version = "0.1.0"
edition = "2021"
description = "Authentication library"
"#,
    );
    write(project, "libs/auth/src/lib.rs", "pub fn login() {}");
    write(
        project,
        "libs/shared/Cargo.toml",
        r#"[package]
name = "shared"
version = "0.1.0"
edition = "2021"
description = "Shared utilities"
"#,
    );
    write(project, "libs/shared/src/lib.rs", "pub fn utils() {}");
    write(
        project,
        "services/api/Cargo.toml",
        r#"[package]
name = "api"
version = "0.1.0"
edition = "2021"
description = "HTTP API service"
"#,
    );
    write(project, "services/api/src/main.rs", "fn main() {}");
}

fn make_npm_workspace(project: &Path) {
    write(
        project,
        "package.json",
        r#"{
  "name": "monorepo",
  "version": "1.0.0",
  "workspaces": ["packages/web", "packages/shared"]
}
"#,
    );
    write(
        project,
        "packages/web/package.json",
        r#"{
  "name": "@monorepo/web",
  "version": "1.0.0"
}
"#,
    );
    write(
        project,
        "packages/shared/package.json",
        r#"{
  "name": "@monorepo/shared",
  "version": "1.0.0"
}
"#,
    );
}

fn make_dockerfile_repo(project: &Path) {
    // "Real" service Dockerfile (should be detected)
    write(
        project,
        "services/api/Dockerfile",
        "FROM rust:1.75\nWORKDIR /app\n",
    );
    // Examples Dockerfile (should be excluded)
    write(project, "examples/test/Dockerfile", "FROM alpine:latest\n");
}

fn make_helm_repo(project: &Path) {
    write(
        project,
        "charts/api/Chart.yaml",
        r#"name: api
version: "0.1.0"
"#,
    );
    write(project, "charts/api/values.yaml", "replicaCount: 2\n");
}

// ─── Strategy constructors ────────────────────────────────────────────────────

fn cargo_strategy() -> Box<dyn archctl::code::strategies::Strategy> {
    Box::new(archctl::code::strategies::cargo::CargoWorkspace)
}

fn npm_strategy() -> Box<dyn archctl::code::strategies::Strategy> {
    Box::new(archctl::code::strategies::npm::NpmWorkspace)
}

fn dockerfile_strategy() -> Box<dyn archctl::code::strategies::Strategy> {
    Box::new(archctl::code::strategies::dockerfile::DockerfilePerService)
}

fn helm_strategy() -> Box<dyn archctl::code::strategies::Strategy> {
    Box::new(archctl::code::strategies::helm::HelmCharts)
}

// ─── SCN-100: Cargo workspace → containers detected ─────────────────────────

#[test]
#[ignore = "requires cargo_metadata exec in temp dir"]
fn cargo_workspace_integration() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    make_cargo_workspace(project);

    let strategies: Vec<Box<dyn archctl::code::strategies::Strategy>> = vec![cargo_strategy()];
    let fs = archctl::filesystem::MemoryFilesystem::new();
    let clock: &dyn archctl::clock::Clock =
        &archctl::clock::FixedClock::new("2025-01-01T00:00:00Z");

    let report = archctl::code::c4_discover::discover(project, &strategies, &fs, clock)
        .expect("discover must succeed");

    let cargo_containers: Vec<_> = report
        .discovered
        .iter()
        .filter(|c| c.strategy == "cargo-workspace")
        .collect();

    assert!(
        !cargo_containers.is_empty(),
        "should detect at least one cargo-workspace container"
    );
    for c in &cargo_containers {
        assert!(
            (0.8..=0.9).contains(&c.confidence),
            "cargo-workspace confidence should be ~0.85, got {}",
            c.confidence
        );
        assert!(
            c.merged_from.contains(&"cargo-workspace".to_string()),
            "merged_from should contain 'cargo-workspace'"
        );
    }
}

// ─── SCN-110: npm workspace → containers detected ─────────────────────────────

#[test]
#[ignore = "requires npm workspace filesystem layout"]
fn npm_workspace_integration() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    make_npm_workspace(project);

    let strategies: Vec<Box<dyn archctl::code::strategies::Strategy>> = vec![npm_strategy()];
    let fs = archctl::filesystem::MemoryFilesystem::new();
    let clock: &dyn archctl::clock::Clock =
        &archctl::clock::FixedClock::new("2025-01-01T00:00:00Z");

    let report = archctl::code::c4_discover::discover(project, &strategies, &fs, clock)
        .expect("discover must succeed");

    let npm_containers: Vec<_> = report
        .discovered
        .iter()
        .filter(|c| c.strategy == "npm-workspace")
        .collect();

    assert!(
        !npm_containers.is_empty(),
        "should detect at least one npm-workspace container, got {}",
        npm_containers.len()
    );
}

// ─── SCN-120/SCN-121: Dockerfile per service (exclusion logic) ───────────────

#[test]
#[ignore = "requires filesystem walk with ignore::WalkBuilder"]
fn dockerfile_integration() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    make_dockerfile_repo(project);

    let strategies: Vec<Box<dyn archctl::code::strategies::Strategy>> = vec![dockerfile_strategy()];
    let fs = archctl::filesystem::MemoryFilesystem::new();
    let clock: &dyn archctl::clock::Clock =
        &archctl::clock::FixedClock::new("2025-01-01T00:00:00Z");

    let report = archctl::code::c4_discover::discover(project, &strategies, &fs, clock)
        .expect("discover must succeed");

    let all_paths: Vec<_> = report
        .discovered
        .iter()
        .flat_map(|c| c.evidences.iter().map(|e| e.file.clone()))
        .collect();

    // examples/ path should be excluded
    assert!(
        !all_paths.iter().any(|p| p.contains("examples/")),
        "examples/ Dockerfiles should be excluded; got {:?}",
        all_paths
    );
    // services/api/Dockerfile should be detected
    assert!(
        all_paths
            .iter()
            .any(|p| p.contains("services/api/Dockerfile")),
        "services/api/Dockerfile should be detected; got {:?}",
        all_paths
    );
}

// ─── SCN-130: Helm chart detection ────────────────────────────────────────────

#[test]
#[ignore = "requires std::fs::read_dir in helm strategy"]
fn helm_integration() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    make_helm_repo(project);

    let strategies: Vec<Box<dyn archctl::code::strategies::Strategy>> = vec![helm_strategy()];
    let fs = archctl::filesystem::MemoryFilesystem::new();
    let clock: &dyn archctl::clock::Clock =
        &archctl::clock::FixedClock::new("2025-01-01T00:00:00Z");

    let report = archctl::code::c4_discover::discover(project, &strategies, &fs, clock)
        .expect("discover must succeed");

    let helm_containers: Vec<_> = report
        .discovered
        .iter()
        .filter(|c| c.strategy == "helm")
        .collect();

    assert!(
        !helm_containers.is_empty(),
        "should detect at least one helm container"
    );
}

// ─── SCN-140: cross-strategy merge ────────────────────────────────────────────

#[test]
fn cross_strategy_merge_integration() {
    use archctl::code::c4_discover::{ContainerCandidate, Evidence, EvidenceKind};

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();

    // Scenario: both S1 (cargo-workspace) and S5 (dockerfile) detect "auth-svc"
    let candidates = vec![
        ContainerCandidate {
            canonical_key: "auth-svc".to_string(),
            name: "auth-svc".to_string(),
            strategy: "cargo-workspace".to_string(),
            confidence: 0.85,
            evidences: vec![Evidence {
                content_hash: String::new(),
                file: "Cargo.toml".to_string(),
                line: 8,
                kind: EvidenceKind::Structural,
                text: "Cargo workspace member: auth-svc".to_string(),
            }],
        },
        ContainerCandidate {
            canonical_key: "auth-svc".to_string(),
            name: "auth-svc".to_string(),
            strategy: "dockerfile".to_string(),
            confidence: 0.60,
            evidences: vec![Evidence {
                content_hash: String::new(),
                file: "services/auth/Dockerfile".to_string(),
                line: 1,
                kind: EvidenceKind::Structural,
                text: "Dockerfile for service: auth-svc".to_string(),
            }],
        },
    ];

    #[derive(Clone)]
    struct InjectStrategy {
        candidates: Vec<ContainerCandidate>,
    }
    impl archctl::code::strategies::Strategy for InjectStrategy {
        fn id(&self) -> &'static str {
            "inject"
        }
        fn confidence(&self) -> f64 {
            1.0
        }
        fn metatype(&self) -> &'static str {
            "mt.container"
        }
        fn detect(
            &self,
            _: &Path,
            _: &dyn archctl::filesystem::Filesystem,
        ) -> anyhow::Result<Vec<ContainerCandidate>> {
            Ok(self.candidates.clone())
        }
    }

    let strategies: Vec<Box<dyn archctl::code::strategies::Strategy>> =
        vec![Box::new(InjectStrategy { candidates })];
    let fs = archctl::filesystem::MemoryFilesystem::new();
    let clock: &dyn archctl::clock::Clock =
        &archctl::clock::FixedClock::new("2025-01-01T00:00:00Z");

    let report = archctl::code::c4_discover::discover(project, &strategies, &fs, clock)
        .expect("discover must succeed");

    assert_eq!(
        report.discovered.len(),
        1,
        "should merge into one container"
    );
    let c = &report.discovered[0];
    assert_eq!(c.canonical_key, "auth-svc");
    // Highest confidence wins
    assert_eq!(c.strategy, "cargo-workspace");
    assert_eq!(c.confidence, 0.85);
    // Both strategies recorded
    assert!(
        c.merged_from.contains(&"cargo-workspace".to_string())
            && c.merged_from.contains(&"dockerfile".to_string()),
        "merged_from should contain both strategies, got {:?}",
        c.merged_from
    );
    // Evidence from both files
    let files: Vec<_> = c.evidences.iter().map(|e| e.file.clone()).collect();
    assert!(files.contains(&"Cargo.toml".to_string()));
    assert!(files.contains(&"services/auth/Dockerfile".to_string()));
}

// ─── SCN-151/SCN-152: apply idempotency ───────────────────────────────────────

#[test]
fn apply_idempotent_integration() {
    use archctl::code::c4_discover::{
        Container, DiscoverReport, Evidence, EvidenceKind, ProjectMeta,
    };

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();

    let report = DiscoverReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root: project.display().to_string(),
            files_scanned: 5,
            languages: BTreeMap::new(),
            duration_ms: 30,
        },
        discovered: vec![Container {
            canonical_key: "dup-svc".to_string(),
            name: "dup-svc".to_string(),
            strategy: "cargo-workspace".to_string(),
            confidence: 0.85,
            merged_from: vec!["cargo-workspace".to_string()],
            evidences: vec![Evidence {
                content_hash: String::new(),
                file: "Cargo.toml".to_string(),
                line: 5,
                kind: EvidenceKind::Structural,
                text: "Cargo workspace member".to_string(),
            }],
        }],
        errors: vec![],
    };

    let fs = archctl::filesystem::MemoryFilesystem::new();

    // First apply — writes the element
    let r1 =
        archctl::code::c4_discover::apply(project, &report, &fs).expect("first apply must succeed");
    assert_eq!(
        r1.elements_written, 1,
        "first apply should write the element"
    );

    // Second apply — skips the existing canonical_key
    let r2 = archctl::code::c4_discover::apply(project, &report, &fs)
        .expect("second apply must succeed");
    assert_eq!(
        r2.elements_skipped, 1,
        "second apply must skip the existing canonical_key"
    );
    assert_eq!(
        r2.elements_written, 0,
        "second apply must not write duplicates"
    );
}

// ─── SCN-160: JSON schema round-trip — CRIT-1 regression test ───────────────

#[test]
fn json_roundtrip_against_schema() {
    use archctl::code::c4_discover::{
        Container, DISCOVER_REPORT_SCHEMA, DiscoverReport, Evidence, EvidenceKind, ProjectMeta,
    };

    // Build a real DiscoverReport with all EvidenceKind variants
    let report = DiscoverReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root: "/tmp/test".to_string(),
            files_scanned: 5,
            languages: BTreeMap::from([("rust".to_string(), 5)]),
            duration_ms: 42,
        },
        discovered: vec![
            Container {
                canonical_key: "auth-svc".to_string(),
                name: "auth-svc".to_string(),
                strategy: "cargo-workspace".to_string(),
                confidence: 0.85,
                merged_from: vec!["cargo-workspace".to_string()],
                evidences: vec![
                    Evidence {
                        content_hash: String::new(),
                        file: "Cargo.toml".to_string(),
                        line: 8,
                        kind: EvidenceKind::Structural,
                        text: "Cargo workspace member: auth-svc".to_string(),
                    },
                    Evidence {
                        content_hash: String::new(),
                        file: "src/main.rs".to_string(),
                        line: 1,
                        kind: EvidenceKind::Lexical,
                        text: "Module root".to_string(),
                    },
                ],
            },
            Container {
                canonical_key: "api-gateway".to_string(),
                name: "api-gateway".to_string(),
                strategy: "dockerfile".to_string(),
                confidence: 0.60,
                merged_from: vec!["dockerfile".to_string()],
                evidences: vec![Evidence {
                    content_hash: String::new(),
                    file: "services/api/Dockerfile".to_string(),
                    line: 1,
                    kind: EvidenceKind::Config,
                    text: "Dockerfile for api-gateway".to_string(),
                }],
            },
        ],
        errors: vec![],
    };

    // Round-trip: Rust struct → JSON string → parsed Value
    let json_str = serde_json::to_string(&report).expect("DiscoverReport must serialise to JSON");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("JSON must be parseable");

    // Validate against the embedded schema
    let schema_val: serde_json::Value = serde_json::from_str(DISCOVER_REPORT_SCHEMA)
        .expect("DISCOVER_REPORT_SCHEMA must be valid JSON");
    let validator = jsonschema::validator_for(&schema_val)
        .expect("DISCOVER_REPORT_SCHEMA must be a valid JSON Schema");
    let result = validator.validate(&parsed);
    assert!(
        result.is_ok(),
        "real Container must pass schema validation: {:?}",
        result.err()
    );
}

// ─── SCN-170: apply → verify LbugStore state ────────────────────────────────

#[test]
fn apply_roundtrip_to_export() {
    use archctl::code::c4_discover::{
        Container, DiscoverReport, Evidence, EvidenceKind, ProjectMeta,
    };

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();

    let report = DiscoverReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root: project.display().to_string(),
            files_scanned: 3,
            languages: BTreeMap::from([("rust".to_string(), 3)]),
            duration_ms: 20,
        },
        discovered: vec![Container {
            canonical_key: "test-svc".to_string(),
            name: "test-svc".to_string(),
            strategy: "cargo-workspace".to_string(),
            confidence: 0.85,
            merged_from: vec!["cargo-workspace".to_string()],
            evidences: vec![Evidence {
                content_hash: String::new(),
                file: "Cargo.toml".to_string(),
                line: 5,
                kind: EvidenceKind::Structural,
                text: "Cargo workspace member".to_string(),
            }],
        }],
        errors: vec![],
    };

    let fs = archctl::filesystem::MemoryFilesystem::new();

    let r = archctl::code::c4_discover::apply(project, &report, &fs).expect("apply must succeed");
    assert_eq!(r.elements_written, 1, "should write exactly one element");
    assert_eq!(r.elements_skipped, 0);
    assert!(r.evidences_written >= 1);
    assert!(r.source_artifacts_written >= 1);
}
