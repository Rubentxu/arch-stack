//! Integration tests for `GET /api/explain` (ADR-062, workbench action
//! palette). Exercises `handle_request` directly — no socket, no HTTP
//! framing — following the `view_workspace.rs` pattern.
//!
//! Store seeding happens at the project's XDG-resolved path (the same
//! path `handle_api_explain` opens), derived deterministically from the
//! per-test TempDir, so runs never collide and no project data is touched.

use archctl::store::{GraphStore, LbugStore};
use tempfile::TempDir;

fn call(
    method: &str,
    url: &str,
    project_dir: Option<&str>,
) -> (u16, String, Vec<u8>, Vec<(String, String)>) {
    let (status, mime, body, extras) = archctl::view::handle_request(
        method,
        url,
        project_dir,
        &archctl::environment::SystemEnvironment,
    );
    (status.0, mime, body, extras)
}

fn parse_json(body: &[u8]) -> serde_json::Value {
    serde_json::from_slice(body).expect("valid JSON")
}

/// Create a project-looking tempdir and seed a minimal element + version
/// graph at the XDG-resolved store path (the same path the handler opens).
/// Returns the tempdir cwd string; the seeded store is dropped so the
/// single-writer flock (ADR-010) is free for the handler.
fn seed_project() -> (TempDir, String) {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    let cwd = tmp.path().to_string_lossy().into_owned();
    let info = archctl::project::resolve_project(&cwd);
    let mut store = LbugStore::open(&info.project_dir).unwrap();
    store.init().unwrap();
    store
        .execute_raw_cypher_for_test(
            "CREATE (:Element {id: 'elm:svc', kind_id: 'container', category: 'c4', \
             canonical_key: 'elm:svc', current_name: 'TestService', current_status: 'active', \
             current_confidence: 0.9, current_version_id: 'v1'})",
        )
        .expect("seed element");
    store
        .execute_raw_cypher_for_test(
            "CREATE (:ElementVersion {id: 'v1', element_id: 'elm:svc', name: 'TestService', \
             status: 'active', origin: 'ast-grep', confidence: 0.9})",
        )
        .expect("seed element version");
    drop(store);
    (tmp, cwd)
}

// ---------------------------------------------------------------------------
// S8 — invalid identifier → 400
// ---------------------------------------------------------------------------

#[test]
fn explain_invalid_identifier_returns_400() {
    // "bad id!" carries a space and `!` — rejected by validate_identifier.
    let (status, mime, body, _) = call("GET", "/api/explain?id=bad%20id!", None);
    assert_eq!(status, 400, "expected 400, got {status}");
    assert!(mime.contains("json"));
    let json = parse_json(&body);
    assert!(
        json["error"]
            .as_str()
            .unwrap()
            .contains("invalid identifier")
    );
}

// ---------------------------------------------------------------------------
// S9 — no project_dir → 409
// ---------------------------------------------------------------------------

#[test]
fn explain_without_project_dir_returns_409() {
    let (status, mime, body, _) = call("GET", "/api/explain?id=elm:svc", None);
    assert_eq!(status, 409, "expected 409, got {status}");
    assert!(mime.contains("json"));
    let json = parse_json(&body);
    assert!(
        json["error"]
            .as_str()
            .unwrap()
            .contains("no project_dir configured")
    );
}

// ---------------------------------------------------------------------------
// S7 — seeded element → 200 with explain report
// ---------------------------------------------------------------------------

#[test]
fn explain_seeded_element_returns_200_report() {
    let (_tmp, cwd) = seed_project();
    let (status, mime, body, _) = call("GET", "/api/explain?id=elm:svc", Some(&cwd));
    assert_eq!(
        status,
        200,
        "expected 200, got {status}: {}",
        String::from_utf8_lossy(&body)
    );
    assert!(mime.contains("json"));
    let json = parse_json(&body);
    assert_eq!(json["subject"]["id"], "elm:svc");
    assert_eq!(json["subject"]["kind"], "element");
    // No evidence seeded → honesty principle: unsubstantiated.
    assert_eq!(json["provenance"]["unsubstantiated"], true);
}

// ---------------------------------------------------------------------------
// S10 — unknown (valid) id → 404
// ---------------------------------------------------------------------------

#[test]
fn explain_unknown_id_returns_404() {
    let (_tmp, cwd) = seed_project();
    let (status, _mime, body, _) = call("GET", "/api/explain?id=elm:nope", Some(&cwd));
    assert_eq!(status, 404, "expected 404, got {status}");
    let json = parse_json(&body);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

// ---------------------------------------------------------------------------
// Relation ids route to the relation path (regression for the id router)
// ---------------------------------------------------------------------------

#[test]
fn explain_relation_id_without_relation_returns_404_not_400() {
    let (_tmp, cwd) = seed_project();
    // `rel:missing` is a valid identifier shape; the relation path reports
    // RelationNotFound → 404 (never 400).
    let (status, _mime, _body, _) = call("GET", "/api/explain?id=rel:missing", Some(&cwd));
    assert_eq!(status, 404, "expected 404, got {status}");
}
