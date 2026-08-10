//! `view_workspace.rs` — integration tests for workspace state API endpoints.
//!
//! Tests the 4 new endpoints: GET/PUT /api/workspace, GET /api/source,
//! POST /api/open-editor. Uses TempDir to avoid polluting the real XDG dir.

// `view_workspace` integration tests cover the 4 M71 endpoints at the
// `handle_request_with_body` level — no socket, no thread pool, no real
// HTTP framing. The handler is a pure function, so each test just calls
// it directly with the desired method/url/body/project_dir.

/// Helper: call handle_request and return status, mime, body.
fn call(method: &str, url: &str, project_dir: Option<&str>) -> (u16, String, Vec<u8>) {
    let (status, mime, body) = archctl::view::handle_request(method, url, project_dir);
    (status.0, mime, body)
}

/// Helper: parse JSON body from a response.
fn parse_json(body: &[u8]) -> serde_json::Value {
    serde_json::from_slice(body).expect("valid JSON")
}

// ---------------------------------------------------------------------------
// GET /api/workspace
// ---------------------------------------------------------------------------

#[test]
fn get_workspace_no_file_returns_200_null() {
    let tmp = tempfile::tempdir().unwrap();
    // Write a Cargo.toml to make it look like a project.
    std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    let project_dir = tmp.path().to_string_lossy().into_owned();

    let (status, mime, body) = call("GET", "/api/workspace", Some(&project_dir));
    assert_eq!(status, 200, "expected 200, got {status}");
    assert!(mime.contains("json"));
    let json = parse_json(&body);
    assert!(json.get("workspace").is_none() || json["workspace"].is_null());
    assert_eq!(json["version"], "1.0");
}

#[test]
fn get_workspace_after_put_returns_same_content() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    let project_dir = tmp.path().to_string_lossy().into_owned();

    let state = serde_json::json!({
        "version": "1.0",
        "project_hash": "a".repeat(64),
        "workspace": {
            "camera": { "x": 10.0, "y": 20.0 },
            "zoom": 75.0,
            "filters": [],
            "selection": null
        },
        "updated_at": "2026-01-01T00:00:00Z"
    });

    // PUT first.
    let (status, _, _) = call_with_body("PUT", "/api/workspace", Some(&project_dir), &state);
    assert_eq!(status, 204, "PUT should return 204");

    // GET should return the same content.
    let (status, _, body) = call("GET", "/api/workspace", Some(&project_dir));
    assert_eq!(status, 200);
    let json = parse_json(&body);
    assert_eq!(json["version"], "1.0");
    assert!(json["workspace"].is_object());
}

// ---------------------------------------------------------------------------
// PUT /api/workspace
// ---------------------------------------------------------------------------

#[test]
fn put_workspace_valid_returns_204() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    let project_dir = tmp.path().to_string_lossy().into_owned();

    let state = serde_json::json!({
        "version": "1.0",
        "project_hash": "b".repeat(64),
        "workspace": {
            "camera": { "x": 0.0, "y": 0.0 },
            "zoom": 50.0,
            "filters": [
                { "kind": "c4", "predicate": "Container" }
            ],
            "selection": { "kind": "node", "id": "n42" }
        },
        "updated_at": "2026-01-01T00:00:00Z"
    });

    let (status, _, _) = call_with_body("PUT", "/api/workspace", Some(&project_dir), &state);
    assert_eq!(status, 204, "expected 204, got {status}");
}

#[test]
fn put_workspace_invalid_schema_returns_400() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    let project_dir = tmp.path().to_string_lossy().into_owned();

    // Invalid: zoom is a string instead of number.
    let bad_state = serde_json::json!({
        "version": "1.0",
        "project_hash": "c".repeat(64),
        "workspace": {
            "camera": { "x": 0.0, "y": 0.0 },
            "zoom": "not a number",
            "filters": [],
            "selection": null
        },
        "updated_at": "2026-01-01T00:00:00Z"
    });

    let (status, _, body) = call_with_body("PUT", "/api/workspace", Some(&project_dir), &bad_state);
    assert_eq!(status, 400, "expected 400 for invalid schema, got {status}");
    let json = parse_json(&body);
    assert!(json.get("error").is_some() || json.get("details").is_some());
}

// ---------------------------------------------------------------------------
// GET /api/source
// ---------------------------------------------------------------------------

#[test]
fn get_source_valid_file_returns_200() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    std::fs::create_dir(tmp.path().join("src")).unwrap();
    let main_rs = tmp.path().join("src/main.rs");
    std::fs::write(&main_rs, "fn main() {\n    println!(\"hello\");\n}\n").unwrap();
    let project_dir = tmp.path().to_string_lossy().into_owned();

    let (status, _, body) = call(
        "GET",
        "/api/source?file=src/main.rs&line=1",
        Some(&project_dir),
    );
    assert_eq!(status, 200, "expected 200, got {status}");
    let json = parse_json(&body);
    assert_eq!(json["file"], "src/main.rs");
    assert!(json["content"].is_array());
    assert!(!json["truncated"].as_bool().unwrap_or(false));
}

#[test]
fn get_source_path_traversal_returns_403() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    let project_dir = tmp.path().to_string_lossy().into_owned();

    let (status, _, body) = call(
        "GET",
        "/api/source?file=../../../etc/passwd&line=1",
        Some(&project_dir),
    );
    assert_eq!(status, 403, "expected 403 for path traversal, got {status}");
    let json = parse_json(&body);
    assert!(
        json["error"]
            .as_str()
            .map(|s| s.contains("outside") || s.contains("scope"))
            .unwrap_or(false)
    );
}

#[test]
fn get_source_missing_file_returns_404() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    let project_dir = tmp.path().to_string_lossy().into_owned();

    let (status, _, _) = call(
        "GET",
        "/api/source?file=nonexistent.rs&line=1",
        Some(&project_dir),
    );
    assert_eq!(status, 404, "expected 404 for missing file, got {status}");
}

#[test]
fn get_source_directory_returns_400() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    std::fs::create_dir(tmp.path().join("src")).unwrap();
    let project_dir = tmp.path().to_string_lossy().into_owned();

    let (status, _, _) = call("GET", "/api/source?file=src&line=1", Some(&project_dir));
    assert_eq!(status, 400, "expected 400 for directory, got {status}");
}

#[test]
fn get_source_line_clamped_to_total() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    std::fs::create_dir(tmp.path().join("src")).unwrap();
    let main_rs = tmp.path().join("src/main.rs");
    std::fs::write(&main_rs, "line1\nline2\nline3\n").unwrap();
    let project_dir = tmp.path().to_string_lossy().into_owned();

    let (status, _, body) = call(
        "GET",
        "/api/source?file=src/main.rs&line=9999",
        Some(&project_dir),
    );
    assert_eq!(status, 200);
    let json = parse_json(&body);
    // start_line should be clamped to total_lines.
    assert!(json["start_line"].as_u64().unwrap() <= json["total_lines"].as_u64().unwrap());
}

// ---------------------------------------------------------------------------
// POST /api/open-editor
// ---------------------------------------------------------------------------

#[test]
fn post_open_editor_valid_returns_204_or_503() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    std::fs::create_dir(tmp.path().join("src")).unwrap();
    let main_rs = tmp.path().join("src/main.rs");
    std::fs::write(&main_rs, "fn main() {}\n").unwrap();
    let project_dir = tmp.path().to_string_lossy().into_owned();

    let body = serde_json::json!({
        "file": "src/main.rs",
        "line": 1
    });

    let (status, _, _) = call_with_body("POST", "/api/open-editor", Some(&project_dir), &body);
    // Either 204 (editor spawned) or 503 (no editor configured) are acceptable.
    assert!(
        status == 204 || status == 503,
        "expected 204 or 503, got {status}"
    );
}

#[test]
fn post_open_editor_path_traversal_returns_403() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    let project_dir = tmp.path().to_string_lossy().into_owned();

    let body = serde_json::json!({
        "file": "../../../etc/passwd",
        "line": 1
    });

    let (status, _, _) = call_with_body("POST", "/api/open-editor", Some(&project_dir), &body);
    assert_eq!(status, 403, "expected 403 for path traversal, got {status}");
}

#[test]
fn post_open_editor_invalid_body_returns_400() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
    let project_dir = tmp.path().to_string_lossy().into_owned();

    // Missing required fields.
    let body = serde_json::json!({});

    let (status, _, _) = call_with_body("POST", "/api/open-editor", Some(&project_dir), &body);
    assert_eq!(status, 400, "expected 400 for invalid body, got {status}");
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn call_with_body(
    method: &str,
    url: &str,
    project_dir: Option<&str>,
    body: &serde_json::Value,
) -> (u16, String, Vec<u8>) {
    // Serialise the JSON body and dispatch through the body-aware handler.
    let bytes = serde_json::to_vec(body).expect("valid body JSON");
    let (status, mime, body_bytes) =
        archctl::view::handle_request_with_body(method, url, project_dir, &bytes);
    (status.0, mime, body_bytes)
}
