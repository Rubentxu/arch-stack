//! Integration tests for `archctl architecture fuse` CLI persistence
//! defaults + staleness expiry (Item 27 residual).
//!
//! Covers:
//! - F-P1: fuse persists fused claims BY DEFAULT (no --persist flag needed)
//! - F-P2: fuse --no-persist keeps stdout-only behaviour (nothing written)
//! - F-P3: fuse --expire-stale --dry-run reports stale claims without deleting
//! - F-P4: fuse --expire-stale deletes stale claims, keeps fresh ones

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use archctl::store::{DiagramRepository, GraphStore, LbugStore, RawGraphQuery};

/// Parse JSON from CLI stdout, skipping any non-JSON prefix lines.
fn parse_json(output: &str) -> serde_json::Value {
    let json_start = output.find('{').expect("expected JSON output");
    serde_json::from_str(&output[json_start..]).expect("valid JSON")
}

/// Seed one Evidence row linked to an ElementVersion.
fn seed_evidence_for_version(
    store: &mut LbugStore,
    ev_id: &str,
    version_id: &str,
    claim: &str,
    observed_at: &str,
) {
    store
        .execute_raw_cypher_for_test(&format!(
            "MERGE (v:ElementVersion {{id: '{version_id}'}}) ON CREATE SET v.element_id = 'el:{version_id}', v.name = 'TestEl', v.status = 'active', v.origin = 'ast-grep', v.confidence = 0.9"
        ))
        .expect("seed element version");
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:Evidence {{id: '{ev_id}', kind: 'call', claim: '{claim}', path: 'src/lib.rs', start_line: 10, end_line: 20, tool_name: 'ast-grep', tool_version: '0.1', rule_id: 'test:rule', props: '{{\"status\":\"active\"}}', content_hash: 'sha256:{ev_id}', observed_at: '{observed_at}'}})"
        ))
        .expect("seed evidence");
    store
        .execute_raw_cypher_for_test(&format!(
            "MATCH (v:ElementVersion {{id: '{version_id}'}}), (e:Evidence {{id: '{ev_id}'}}) CREATE (v)-[:SUPPORTED_BY]->(e)"
        ))
        .expect("link version to evidence");
}

/// Create a seeded project and return its CWD. The CLI resolves the
/// real project dir via `resolve_project` (XDG hash); we seed the SAME
/// dir so CLI writes land where we read.
fn seed_project(observed_at: &str, version_id: &str) -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("proj");
    // The CLI resolves the project dir through the XDG identity hash —
    // replicate it so seeding and reading hit the same store.
    let info = archctl::project::resolve_project(&project.to_string_lossy());
    let project_dir = info.project_dir.clone();
    let mut store = LbugStore::open(&project_dir).unwrap();
    store.init().unwrap();
    // Two observations of the same statement → they fuse into one claim.
    seed_evidence_for_version(
        &mut store,
        "ev:fuse:p1:1",
        version_id,
        "foo returns int",
        observed_at,
    );
    seed_evidence_for_version(
        &mut store,
        "ev:fuse:p1:2",
        version_id,
        "foo returns int",
        observed_at,
    );
    drop(store); // release flock so the CLI can open the project
    (tmp, project)
}

fn run_fuse(project: &Path, version_id: &str, extra: &[&str]) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args(["architecture", "fuse", "--cwd"])
        .arg(project.to_str().unwrap())
        .args(["--json", "--version-id", version_id])
        .args(extra)
        .output()
        .expect("fuse should run");
    assert!(
        output.status.success(),
        "fuse failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    parse_json(&String::from_utf8_lossy(&output.stdout))
}

/// Count persisted FusedClaim rows for a version (in-process read).
fn persisted_claim_count(project_cwd: &Path, version_id: &str) -> usize {
    let info = archctl::project::resolve_project(&project_cwd.to_string_lossy());
    let mut store = LbugStore::open(&info.project_dir).unwrap();
    store.init().unwrap();
    match store
        .read_fused_claim_rows(&[version_id.to_string()])
        .unwrap()
    {
        Some(rows) => rows.len(),
        None => 0,
    }
}

/// F-P1: fuse persists by default — no --persist flag required.
#[test]
fn fuse_persists_by_default() {
    let (_tmp, project) = seed_project("2026-08-01T00:00:00Z", "vid-p1");
    let report = run_fuse(&project, "vid-p1", &[]);

    assert_eq!(report["persisted"], serde_json::json!(true));
    assert_eq!(report["fused_claims"].as_array().unwrap().len(), 1);
    // The claim actually landed in the store (v6 FusedClaim table).
    assert_eq!(persisted_claim_count(&project, "vid-p1"), 1);
}

/// F-P2: fuse --no-persist keeps stdout-only behaviour.
#[test]
fn fuse_no_persist_does_not_write() {
    let (_tmp, project) = seed_project("2026-08-01T00:00:00Z", "vid-p2");
    let report = run_fuse(&project, "vid-p2", &["--no-persist"]);

    assert_eq!(report["persisted"], serde_json::json!(false));
    assert_eq!(persisted_claim_count(&project, "vid-p2"), 0);
}

/// F-P3: fuse --expire-stale --dry-run reports stale claims, deletes nothing.
#[test]
fn fuse_expire_stale_dry_run_reports_without_deleting() {
    // observed_at over 90 days before now → staleness-weighted evaluator
    // flags the claim stale.
    let (_tmp, project) = seed_project("2025-01-01T00:00:00Z", "vid-p3");
    let report = run_fuse(
        &project,
        "vid-p3",
        &[
            "--evaluator",
            "staleness-weighted",
            "--expire-stale",
            "--dry-run",
        ],
    );

    assert_eq!(report["persisted"], serde_json::json!(true));
    assert_eq!(report["expired_stale"], serde_json::json!(1));
    // Dry-run must NOT delete: claim still present.
    assert_eq!(persisted_claim_count(&project, "vid-p3"), 1);
}

/// F-P4: fuse --expire-stale deletes stale claims.
#[test]
fn fuse_expire_stale_deletes_stale_claims() {
    let (_tmp, project) = seed_project("2025-01-01T00:00:00Z", "vid-p4");
    let report = run_fuse(
        &project,
        "vid-p4",
        &["--evaluator", "staleness-weighted", "--expire-stale"],
    );

    assert_eq!(report["persisted"], serde_json::json!(true));
    assert_eq!(report["expired_stale"], serde_json::json!(1));
    // Deleted: no rows remain.
    assert_eq!(persisted_claim_count(&project, "vid-p4"), 0);
}

/// F-P5: expire-stale leaves fresh claims untouched.
#[test]
fn fuse_expire_stale_keeps_fresh_claims() {
    // Fresh observed_at (this week) → claim is NOT stale → nothing expired.
    let (_tmp, project) = seed_project("2026-08-15T00:00:00Z", "vid-p5");
    let report = run_fuse(
        &project,
        "vid-p5",
        &["--evaluator", "staleness-weighted", "--expire-stale"],
    );

    assert_eq!(report["expired_stale"], serde_json::json!(0));
    assert_eq!(persisted_claim_count(&project, "vid-p5"), 1);
}

/// Regression: lbug returns timestamp() columns in a non-RFC3339
/// readback format ("2026-08-15 0:00:00.0 +00:00:00"). Documenting the
/// exact readback so the staleness parser contract stays pinned.
#[test]
fn lbug_timestamp_readback_format_is_not_rfc3339() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("proj");
    let mut store = LbugStore::open(&project).unwrap();
    store.init().unwrap();
    store
        .execute_raw_cypher_for_test(
            "CREATE (:Evidence {id: 'probe:1', kind: 'call', claim: 'c', path: 'p', start_line: 1, end_line: 2, tool_name: 't', tool_version: '0', rule_id: 'r', props: '{}', content_hash: 'h', observed_at: timestamp('2026-08-15T00:00:00Z')})",
        )
        .unwrap();
    let rows = <LbugStore as RawGraphQuery>::query(
        &store,
        "MATCH (e:Evidence {id: 'probe:1'}) RETURN e.observed_at;",
    )
    .unwrap();
    let cell = rows
        .first()
        .and_then(|r| r.column(0))
        .map(|(_, c)| c)
        .expect("observed_at cell");
    let raw = cell.as_str().expect("observed_at as string");
    assert_eq!(raw, "2026-08-15 0:00:00.0 +00:00:00");
    // The raw readback is NOT RFC 3339 — the fusion parser must
    // normalize it (see fusion.rs parse_observed_at).
    assert!(chrono::DateTime::parse_from_rfc3339(raw).is_err());
}
