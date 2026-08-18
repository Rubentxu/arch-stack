//! `archctl view` — serve the embedded archview workbench (ADR-033).
//!
//! One-shot HTTP server on 127.0.0.1 serving the archview static bundle
//! embedded via `rust-embed`, plus a minimal read-only API that connects
//! the workbench to the LadybugDB graph (export pipeline).
//!
//! Invariants (ADR-011 / ADR-019 / ADR-020):
//! - Bind ONLY to 127.0.0.1 (never public).
//! - COOP/COEP headers for SharedArrayBuffer (WASM multi-thread).
//! - `Cross-Origin-Resource-Policy: same-origin` (blocked network by default).
//! - No daemon: process exits with Ctrl+C (ADR-010).

use anyhow::{Context, Result};
use rust_embed::RustEmbed;
use std::net::TcpListener;
use std::path::Path;

pub mod editor;
pub mod source;
pub mod workspace;

#[derive(RustEmbed)]
#[folder = "assets-view/"]
struct ViewAssets;

use std::sync::Arc;

/// View options for the archview server.
#[derive(Clone)]
pub struct ViewOptions {
    /// Port to bind. `0` = ephemeral (default).
    pub port: u16,
    /// Project directory to export bundles from (for `/api/export`).
    /// If `None`, the endpoint returns a clear error.
    pub project_dir: Option<String>,
    /// Environment port for editor resolution.
    /// `Debug` is intentionally hand-derived because `dyn Environment`
    /// is not `Debug`; we just print the type id.
    pub env: Arc<dyn crate::environment::Environment>,
}

impl std::fmt::Debug for ViewOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewOptions")
            .field("port", &self.port)
            .field("project_dir", &self.project_dir)
            .field(
                "env",
                &std::any::type_name::<dyn crate::environment::Environment>(),
            )
            .finish()
    }
}

impl Default for ViewOptions {
    fn default() -> Self {
        Self {
            port: 0,
            project_dir: None,
            env: crate::environment::system_environment(),
        }
    }
}

/// Server result: the bound address, for tests and user output.
#[derive(Debug)]
pub struct ServerInfo {
    pub addr: std::net::SocketAddr,
}

fn content_type_for(path: &str) -> &'static str {
    if path.ends_with(".js") {
        "text/javascript"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".html") || path.is_empty() || path == "/" {
        "text/html; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}

fn resolve_asset_path(path: &str) -> String {
    // Normalize: strip leading '/', reject traversal, default to index.
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() || trimmed == "index.html" {
        return "index.html".to_string();
    }
    // Basic traversal guard (defense in depth; tiny_http already splits path).
    if trimmed.contains("..") {
        return String::new();
    }
    trimmed.to_string()
}

fn serve_static(request_path: &str) -> Option<(String, Vec<u8>)> {
    let asset_path = resolve_asset_path(request_path);
    if asset_path.is_empty() {
        return None;
    }
    let file = ViewAssets::get(&asset_path)?;
    let mime = content_type_for(&asset_path);
    Some((mime.to_string(), file.data.into_owned()))
}

fn handle_api_export(
    project_dir: Option<&str>,
    selector: Option<&str>,
) -> Result<(String, Vec<u8>)> {
    // Documented default (D3): no selector → container:* (backward compatible)
    let selector = selector.unwrap_or("container:*");
    crate::diagram::selector::parse(selector).context("invalid view selector")?;

    if project_dir.is_none() {
        let payload = serde_json::json!({
            "empty": true,
            "warning": "no project_dir configured — run `archctl view --cwd <dir>`",
        });
        return Ok((
            "application/json".to_string(),
            serde_json::to_vec_pretty(&payload)?,
        ));
    }
    let dir = project_dir.unwrap();
    let fs = crate::filesystem::system_filesystem();
    let info = crate::project::resolve_project(dir);
    let store = crate::store::open_and_init(&info.project_dir)?;

    // Export to a temp dir, then inline the bundle files as a JSON envelope.
    let bundle_dir =
        std::env::temp_dir().join(format!("archctl-view-export-{}", uuid::Uuid::new_v4()));
    let report = crate::diagram::run_export(
        &*store,
        selector,
        &bundle_dir,
        &crate::clock::SystemClock,
        &*fs,
        crate::diagram::export_types::ExportProfile::Default,
        &info.project_dir,
    )
    .context("export failed")?;

    let read = |name: &str| -> Result<serde_json::Value> {
        let raw = std::fs::read_to_string(bundle_dir.join(name))
            .with_context(|| format!("read {name}"))?;
        serde_json::from_str(&raw).with_context(|| format!("parse {name}"))
    };
    let payload = serde_json::json!({
        "manifest": read("manifest.json")?,
        "projection": read("projection.json")?,
        "evidence": read("evidence.json")?,
        "styles": read("styles.json")?,
        "report": {
            "elementCount": report.element_count,
            "edgeCount": report.edge_count,
            "evidenceCount": report.evidence_count,
        },
        "empty": report.empty,
        "warning": report.warning,
    });
    let body = serde_json::to_vec_pretty(&payload)?;
    Ok(("application/json".to_string(), body))
}

/// Pure request handler — testable without a socket.
///
/// Returns `(status, content_type, body, extra_headers)`. The 4th tuple
/// element carries extra HTTP headers (e.g. `X-Truncated` for source
/// preview per ADR-041 §4); empty for handlers that don't need them.
pub fn handle_request(
    method: &str,
    url: &str,
    project_dir: Option<&str>,
    env: &dyn crate::environment::Environment,
) -> (
    tiny_http::StatusCode,
    String,
    Vec<u8>,
    Vec<(String, String)>,
) {
    handle_request_with_body(method, url, project_dir, env, &[])
}

/// Same as [`handle_request`] but with a request body for PUT/POST.
///
/// GETs ignore the body; PUT/POST handlers consume it.
pub fn handle_request_with_body(
    method: &str,
    url: &str,
    project_dir: Option<&str>,
    env: &dyn crate::environment::Environment,
    body: &[u8],
) -> (
    tiny_http::StatusCode,
    String,
    Vec<u8>,
    Vec<(String, String)>,
) {
    match (method, url) {
        ("GET", path) if path == "/api/health" || path.starts_with("/api/health?") => {
            let payload = serde_json::json!({
                "status": "ok",
                "version": env!("CARGO_PKG_VERSION"),
            });
            (
                tiny_http::StatusCode(200),
                "application/json".to_string(),
                serde_json::to_vec(&payload).unwrap_or_default(),
                vec![],
            )
        }
        ("GET", path) if path == "/api/export" || path.starts_with("/api/export?") => {
            let selector = path
                .strip_prefix("/api/export?")
                .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("selector=")));
            match handle_api_export(project_dir, selector) {
                Ok((mime, body)) => (tiny_http::StatusCode(200), mime, body, vec![]),
                Err(e) => (
                    tiny_http::StatusCode(400),
                    "application/json".to_string(),
                    serde_json::json!({ "error": e.to_string() })
                        .to_string()
                        .into_bytes(),
                    vec![],
                ),
            }
        }
        // ---- Workspace state API (ADR-041) ----
        ("GET", path) if path == "/api/workspace" || path.starts_with("/api/workspace?") => {
            handle_api_workspace_get(project_dir)
        }
        ("PUT", path) if path == "/api/workspace" || path.starts_with("/api/workspace?") => {
            handle_api_workspace_put(body, project_dir)
        }
        ("GET", path) if path.starts_with("/api/source?") => {
            handle_api_source_get(path, project_dir)
        }
        ("POST", path) if path == "/api/open-editor" || path.starts_with("/api/open-editor?") => {
            handle_api_open_editor_post(body, project_dir, env)
        }
        ("GET", path) => match serve_static(path) {
            Some((mime, body)) => (tiny_http::StatusCode(200), mime, body, vec![]),
            None => (
                tiny_http::StatusCode(404),
                "text/plain; charset=utf-8".to_string(),
                b"not found".to_vec(),
                vec![],
            ),
        },
        _ => (
            tiny_http::StatusCode(405),
            "text/plain; charset=utf-8".to_string(),
            b"method not allowed".to_vec(),
            vec![],
        ),
    }
}

/// Run the one-shot server until the process is interrupted.
pub fn run(options: ViewOptions) -> Result<ServerInfo> {
    // Fast-fail: if no assets were embedded, surface the actionable error.
    if ViewAssets::get("index.html").is_none() {
        anyhow::bail!(
            "view assets not embedded — run: pnpm build (archview) && scripts/embed-view.sh"
        );
    }

    let listener =
        TcpListener::bind(("127.0.0.1", options.port)).context("bind 127.0.0.1 failed")?;
    let addr = listener.local_addr().context("local_addr failed")?;
    let project_dir = options.project_dir.clone();
    let env = options.env.clone();

    println!("archctl view — http://{addr}");
    println!("workbench: archview (embedded, ADR-033)");
    println!("project:   {}", project_dir.as_deref().unwrap_or("<none>"));
    println!("press Ctrl+C to stop");

    let server = tiny_http::Server::from_listener(listener, None)
        .map_err(|e| anyhow::anyhow!("tiny_http init failed: {e}"))?;

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().clone();
        let project = project_dir.clone();
        let env_ref: &dyn crate::environment::Environment = &*env;

        // Read the request body for PUT/POST before dispatching; GET/HEAD
        // ignore the body. ADR-041: workspace state endpoints accept JSON
        // bodies up to 16 KiB (workspace.json is ~200 bytes; 16 KiB leaves
        // headroom for future filters/selection growth).
        let needs_body = matches!(method.as_str(), "PUT" | "POST" | "PATCH");
        let mut body_buf: Vec<u8> = Vec::new();
        if needs_body {
            use std::io::Read;
            let _ = request
                .as_reader()
                .take(16 * 1024)
                .read_to_end(&mut body_buf);
        }

        let (status, mime, body, extra_headers) = handle_request_with_body(
            method.as_str(),
            url.as_str(),
            project.as_deref(),
            env_ref,
            &body_buf,
        );

        let mut response = tiny_http::Response::from_data(body)
            .with_status_code(status)
            .with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], mime.as_bytes())
                    .expect("valid header"),
            )
            // ADR-020: SharedArrayBuffer requires COOP/COEP.
            .with_header(
                tiny_http::Header::from_bytes(&b"Cross-Origin-Opener-Policy"[..], b"same-origin")
                    .expect("valid header"),
            )
            .with_header(
                tiny_http::Header::from_bytes(
                    &b"Cross-Origin-Embedder-Policy"[..],
                    b"require-corp",
                )
                .expect("valid header"),
            )
            // ADR-011: blocked network by default.
            .with_header(
                tiny_http::Header::from_bytes(&b"Cross-Origin-Resource-Policy"[..], b"same-origin")
                    .expect("valid header"),
            );

        // Apply extra response headers (e.g. `X-Truncated` per ADR-041 §4).
        for (name, value) in extra_headers {
            if let Ok(h) = tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes()) {
                response = response.with_header(h);
            }
        }

        let _ = request.respond(response);
    }

    Ok(ServerInfo { addr })
}

/// Parse a `--port` arg: 0..=65535, or error.
pub fn parse_port(raw: &str) -> Result<u16> {
    let port: u32 = raw
        .parse()
        .with_context(|| format!("invalid port: {raw}"))?;
    if port > 65535 {
        anyhow::bail!("port out of range: {raw}");
    }
    Ok(port as u16)
}

/// True when the given path looks like a real project dir (has Cargo.toml,
/// package.json, go.mod, pyproject.toml, etc.).
pub fn looks_like_project_dir(path: &Path) -> bool {
    for marker in [
        "Cargo.toml",
        "package.json",
        "go.mod",
        "pyproject.toml",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
    ] {
        if path.join(marker).is_file() {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Workspace state API helpers (ADR-041)
// ---------------------------------------------------------------------------

/// GET /api/workspace — load workspace state from XDG project dir.
fn handle_api_workspace_get(
    project_dir: Option<&str>,
) -> (
    tiny_http::StatusCode,
    String,
    Vec<u8>,
    Vec<(String, String)>,
) {
    let project_dir = match project_dir {
        Some(d) => d,
        None => return json_error(500, "xdg_inaccessible"),
    };
    let cwd = Path::new(project_dir);
    match workspace::WorkspaceStore::load(cwd) {
        Ok(Some(state)) => {
            let body = serde_json::to_vec(&serde_json::json!({
                "workspace": state,
                "version": "1.0"
            }))
            .unwrap_or_default();
            (
                tiny_http::StatusCode(200),
                "application/json".to_string(),
                body,
                vec![],
            )
        }
        Ok(None) => {
            // No workspace.json yet → return null.
            let body = serde_json::to_vec(&serde_json::json!({
                "workspace": serde_json::Value::Null,
                "version": "1.0"
            }))
            .unwrap_or_default();
            (
                tiny_http::StatusCode(200),
                "application/json".to_string(),
                body,
                vec![],
            )
        }
        Err(e) => json_error(500, &format!("xdg_inaccessible: {}", e)),
    }
}

/// GET /api/source?file=<path>&line=<n> — read source file with path validation.
fn handle_api_source_get(
    url_with_query: &str,
    project_dir: Option<&str>,
) -> (
    tiny_http::StatusCode,
    String,
    Vec<u8>,
    Vec<(String, String)>,
) {
    let project_dir = match project_dir {
        Some(d) => d,
        None => return json_error(500, "project_dir required"),
    };
    // Parse query params.
    let file = url_with_query.strip_prefix("/api/source?").and_then(|q| {
        q.split('&')
            .find_map(|kv| kv.strip_prefix("file="))
            .map(|s| percent_decode(s).unwrap_or_else(|| s.to_string()))
    });
    let file = match file {
        Some(f) => f,
        None => return json_error(400, "missing required param: file"),
    };
    let line = url_with_query.strip_prefix("/api/source?").and_then(|q| {
        q.split('&')
            .find_map(|kv| kv.strip_prefix("line="))
            .and_then(|s| s.parse::<u32>().ok())
    });

    let cwd = Path::new(project_dir);
    let file_path = Path::new(&file);
    match source::source_preview(file_path, line, cwd) {
        Ok(preview) => {
            let body = serde_json::to_vec(&preview).unwrap_or_default();
            // ADR-041 §4: emit X-Truncated header when source preview was capped
            // at MAX_LINES so clients can detect truncation without parsing body.
            let extra = if preview.truncated {
                vec![("X-Truncated".to_string(), "true".to_string())]
            } else {
                vec![]
            };
            (
                tiny_http::StatusCode(200),
                "application/json".to_string(),
                body,
                extra,
            )
        }
        Err(workspace::WorkspaceError::PathOutsideScope { .. }) => {
            json_error(403, "path_outside_scope")
        }
        Err(workspace::WorkspaceError::NotFound) => json_error(404, "file_not_found"),
        Err(workspace::WorkspaceError::IsDirectory(_)) => json_error(400, "is_directory"),
        Err(
            workspace::WorkspaceError::PathInvalid(_) | workspace::WorkspaceError::CwdInvalid(_),
        ) => json_error(400, "invalid_path"),
        Err(workspace::WorkspaceError::Io(_)) => json_error(500, "io_error"),
        Err(
            workspace::WorkspaceError::Json(_) | workspace::WorkspaceError::SchemaValidation(_),
        ) => json_error(500, "internal_validation_error"),
    }
}

/// POST /api/open-editor — spawn the user's editor. Body parsed and
/// validated against cwd by the router before reaching this handler.
fn handle_api_open_editor_post(
    body: &[u8],
    project_dir: Option<&str>,
    env: &dyn crate::environment::Environment,
) -> (
    tiny_http::StatusCode,
    String,
    Vec<u8>,
    Vec<(String, String)>,
) {
    let project_dir = match project_dir {
        Some(d) => d,
        None => return json_error(500, "project_dir required"),
    };
    #[derive(serde::Deserialize)]
    struct OpenEditorBody {
        file: String,
        line: u32,
    }
    let parsed: OpenEditorBody = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(_) => return json_error(400, "invalid JSON body"),
    };
    let cwd = Path::new(project_dir);
    let file_path = Path::new(&parsed.file);
    // Validate path containment — exhaustive match across all WorkspaceError
    // variants (debt-verify H5: brittle `if let PathOutsideScope` was a contract
    // break — NotFound/PathInvalid/CwdInvalid fell through to 500).
    if let Err(e) = workspace::validate_path_under_cwd(file_path, cwd) {
        match e {
            workspace::WorkspaceError::PathOutsideScope { .. } => {
                return json_error(403, "path_outside_scope");
            }
            workspace::WorkspaceError::NotFound => {
                return json_error(404, "file_not_found");
            }
            workspace::WorkspaceError::PathInvalid(_)
            | workspace::WorkspaceError::CwdInvalid(_) => {
                return json_error(400, "invalid_path");
            }
            workspace::WorkspaceError::Io(_) => {
                return json_error(500, "io_error");
            }
            workspace::WorkspaceError::IsDirectory(_) => {
                return json_error(400, "is_directory");
            }
            // Schema/Json variants cannot originate here — keep the catch-all
            // explicit so future variants surface as 500 rather than silently
            // bypassing the guard.
            workspace::WorkspaceError::SchemaValidation(_) | workspace::WorkspaceError::Json(_) => {
                return json_error(500, "internal_validation_error");
            }
        }
    }
    // Resolve editor using injected environment.
    let editor = match editor::resolve_editor(env) {
        Some(e) => e,
        None => return json_error(503, "no_editor_configured: set $EDITOR or $VISUAL"),
    };
    // Spawn (don't wait).
    match editor::spawn_editor(file_path, parsed.line, &editor) {
        Ok(_) => (
            tiny_http::StatusCode(204),
            "application/json".to_string(),
            vec![],
            vec![],
        ),
        Err(e) => json_error(500, &format!("editor_spawn_failed: {}", e)),
    }
}

/// PUT /api/workspace — save workspace state atomically. Body parsed and
/// schema-validated by the router before reaching this handler.
fn handle_api_workspace_put(
    body: &[u8],
    project_dir: Option<&str>,
) -> (
    tiny_http::StatusCode,
    String,
    Vec<u8>,
    Vec<(String, String)>,
) {
    let project_dir = match project_dir {
        Some(d) => d,
        None => return json_error(500, "project_dir required"),
    };
    let cwd = Path::new(project_dir);
    // Deserialize and validate.
    let state: workspace::WorkspaceState = match serde_json::from_slice(body) {
        Ok(s) => s,
        Err(e) => {
            // ADR-041 §2.2: structured error response {error, details}.
            return json_error_structured(400, "invalid_schema", &e.to_string());
        }
    };
    // Validate JSON Schema constraints that serde can't enforce
    // (const, enum, pattern, range) per ADR-041 §3.
    if let Err(e) = state.validate() {
        return json_error_structured(400, "invalid_schema", &e.to_string());
    }
    // Atomic save.
    match workspace::WorkspaceStore::save(&state, cwd) {
        Ok(()) => (
            tiny_http::StatusCode(204),
            "application/json".to_string(),
            vec![],
            vec![],
        ),
        Err(e) => json_error(500, &format!("save_failed: {}", e)),
    }
}

/// Helper: build a JSON error response.
fn json_error(
    status: u16,
    message: &str,
) -> (
    tiny_http::StatusCode,
    String,
    Vec<u8>,
    Vec<(String, String)>,
) {
    let body = serde_json::to_vec(&serde_json::json!({ "error": message })).unwrap_or_default();
    (
        tiny_http::StatusCode(status),
        "application/json".to_string(),
        body,
        vec![],
    )
}

/// Helper: build a structured JSON error response with `error` + `details`
/// fields (ADR-041 §2.2 contract for validation errors).
fn json_error_structured(
    status: u16,
    error: &str,
    details: &str,
) -> (
    tiny_http::StatusCode,
    String,
    Vec<u8>,
    Vec<(String, String)>,
) {
    let body = serde_json::to_vec(&serde_json::json!({
        "error": error,
        "details": details,
    }))
    .unwrap_or_default();
    (
        tiny_http::StatusCode(status),
        "application/json".to_string(),
        body,
        vec![],
    )
}

/// Percent-decode a query parameter using the `percent-encoding` crate
/// (UTF-8 aware). The previous hand-rolled implementation mapped each
/// decoded byte to `char`, silently corrupting multi-byte UTF-8 sequences
/// (e.g. `%C3%A9` for `é`); see debt-verify dup-002 / SMELL-15.
fn percent_decode(s: &str) -> Option<String> {
    percent_encoding::percent_decode_str(s)
        .decode_utf8()
        .ok()
        .map(|c| c.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn status_of(method: &str, url: &str) -> u16 {
        let env = crate::environment::fixed_environment();
        let (status, _, _, _) = handle_request(method, url, None, &*env);
        status.0
    }

    #[test]
    fn health_returns_ok_and_version() {
        let env = crate::environment::fixed_environment();
        let (status, mime, body, _) = handle_request("GET", "/api/health", None, &*env);
        assert_eq!(status.0, 200);
        assert_eq!(mime, "application/json");
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn index_served_when_embedded() {
        // Only meaningful when assets were embedded (embed-view.sh).
        if ViewAssets::get("index.html").is_none() {
            return;
        }
        let env = crate::environment::fixed_environment();
        let (status, mime, body, _) = handle_request("GET", "/", None, &*env);
        assert_eq!(status.0, 200);
        assert!(mime.starts_with("text/html"));
        assert!(!body.is_empty());
    }

    #[test]
    fn missing_asset_is_404() {
        let env = crate::environment::fixed_environment();
        let (status, _, body, _) = handle_request("GET", "/nope.js", None, &*env);
        assert_eq!(status.0, 404);
        assert_eq!(body, b"not found");
    }

    #[test]
    fn traversal_is_rejected() {
        // ".." paths must not resolve to embedded files (defense in depth).
        assert_eq!(status_of("GET", "/../Cargo.toml"), 404);
    }

    #[test]
    fn non_get_is_405() {
        assert_eq!(status_of("POST", "/"), 405);
    }

    #[test]
    fn export_without_project_is_200_empty_json() {
        let env = crate::environment::fixed_environment();
        let (status, mime, body, _) = handle_request("GET", "/api/export", None, &*env);
        assert_eq!(status.0, 200);
        assert_eq!(mime, "application/json");
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["empty"], true);
        assert!(
            json["warning"].as_str().unwrap().contains("no project_dir"),
            "expected warning to mention 'no project_dir', got: {:?}",
            json["warning"]
        );
    }

    #[test]
    fn content_types_are_guessed() {
        assert!(content_type_for("x.js").contains("javascript"));
        assert!(content_type_for("x.css").contains("css"));
        assert!(content_type_for("x.json").contains("json"));
        assert!(content_type_for("").contains("html"));
        assert_eq!(content_type_for("x.bin"), "application/octet-stream");
    }

    #[test]
    fn parse_port_accepts_range_and_rejects_bad() {
        assert_eq!(parse_port("0").unwrap(), 0);
        assert_eq!(parse_port("65535").unwrap(), 65535);
        assert!(parse_port("70000").is_err());
        assert!(parse_port("abc").is_err());
    }

    #[test]
    fn project_dir_detection_uses_markers() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!looks_like_project_dir(tmp.path()));
        std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
        assert!(looks_like_project_dir(tmp.path()));
        std::fs::remove_file(tmp.path().join("Cargo.toml")).unwrap();
        std::fs::write(tmp.path().join("package.json"), "").unwrap();
        assert!(looks_like_project_dir(tmp.path()));
    }

    #[test]
    fn full_request_round_trip_over_socket() {
        // Spin the real server on an ephemeral port in a thread, then talk
        // to it with a raw TcpStream. Guards the loop wiring (headers).
        if ViewAssets::get("index.html").is_none() {
            return;
        }
        let opts = ViewOptions {
            port: 0,
            project_dir: None,
            env: crate::environment::fixed_environment(),
        };
        let handle = std::thread::spawn(move || {
            let _ = run(opts);
        });
        std::thread::sleep(std::time::Duration::from_millis(300));

        // Can't know the ephemeral port from outside; bind one ourselves.
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        // Spawn on a fixed port instead (simpler for the round trip).
        let opts = ViewOptions {
            port,
            project_dir: None,
            env: crate::environment::fixed_environment(),
        };
        let handle2 = std::thread::spawn(move || {
            let _ = run(opts);
        });
        std::thread::sleep(std::time::Duration::from_millis(500));

        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(
            stream,
            "GET /api/health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut buf = String::new();
        use std::io::Read;
        stream.read_to_string(&mut buf).unwrap();

        assert!(buf.starts_with("HTTP/1.1 200"));
        assert!(buf.contains("application/json"));
        assert!(buf.contains("\"status\":\"ok\""));
        assert!(buf.contains("Cross-Origin-Opener-Policy: same-origin"));
        assert!(buf.contains("Cross-Origin-Embedder-Policy: require-corp"));
        assert!(buf.contains("Cross-Origin-Resource-Policy: same-origin"));
        drop(handle);
        drop(handle2);
    }

    #[test]
    fn export_with_selector_splits_path() {
        // Query string ?selector=... is stripped before routing; when no project_dir
        // is set, the early-return fires with 200 + "no project_dir" warning.
        let env = crate::environment::fixed_environment();
        let (status, mime, body, _) =
            handle_request("GET", "/api/export?selector=context:myapp", None, &*env);
        assert_eq!(status.0, 200);
        assert_eq!(mime, "application/json");
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["empty"], true);
        assert!(
            json["warning"].as_str().unwrap().contains("no project_dir"),
            "expected 'no project_dir' warning, got: {:?}",
            json["warning"]
        );
    }

    #[test]
    fn export_without_selector_uses_default() {
        // Without ?selector=, handle_api_export uses "container:*" as default.
        // Same behavior as export_without_project_is_200_empty_json.
        let env = crate::environment::fixed_environment();
        let (status, mime, body, _) = handle_request("GET", "/api/export", None, &*env);
        assert_eq!(status.0, 200);
        assert_eq!(mime, "application/json");
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["empty"], true);
        assert!(
            json["warning"].as_str().unwrap().contains("no project_dir"),
            "expected 'no project_dir' warning, got: {:?}",
            json["warning"]
        );
    }

    #[test]
    fn export_invalid_selector_returns_400() {
        // Invalid selector "bogus" → selector::parse fails → HTTP 400 + JSON error.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
        let env = crate::environment::fixed_environment();
        let (status, mime, body, _) = handle_request(
            "GET",
            "/api/export?selector=bogus",
            Some(tmp.path().to_str().unwrap()),
            &*env,
        );
        assert_eq!(
            status.0, 400,
            "expected 400 for invalid selector, got: {}",
            status.0
        );
        assert_eq!(mime, "application/json");
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            json["error"]
                .as_str()
                .unwrap()
                .contains("invalid view selector"),
            "expected 'invalid view selector' error, got: {:?}",
            json["error"]
        );
    }

    #[test]
    fn health_unaffected_by_query_string() {
        // /api/health with query string should still return 200 + status ok.
        let env = crate::environment::fixed_environment();
        let (status, mime, body, _) = handle_request("GET", "/api/health?x=1", None, &*env);
        assert_eq!(status.0, 200);
        assert_eq!(mime, "application/json");
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[test]
    fn export_like_paths_do_not_match_export_guard() {
        // /api/exportx (no query) must NOT match the export guard — it should
        // fall through to the static branch → 404, not export a bundle.
        let env = crate::environment::fixed_environment();
        let (status, _, _, _) = handle_request("GET", "/api/exportx", None, &*env);
        assert_eq!(
            status.0, 404,
            "expected 404 for /api/exportx, got: {}",
            status.0
        );
        // /api/export-extra similarly must not match.
        let (status, _, _, _) = handle_request("GET", "/api/export-extra", None, &*env);
        assert_eq!(
            status.0, 404,
            "expected 404 for /api/export-extra, got: {}",
            status.0
        );
    }

    // ---- Workspace API tests (ADR-041) ----

    #[test]
    fn get_workspace_no_project_returns_500() {
        let env = crate::environment::fixed_environment();
        let (status, _, body, _) = handle_request("GET", "/api/workspace", None, &*env);
        assert_eq!(status.0, 500);
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["error"].as_str().unwrap().contains("xdg_inaccessible"));
    }

    #[test]
    fn get_workspace_no_file_returns_200_null() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
        let env = crate::environment::fixed_environment();
        let (status, _, body, _) = handle_request(
            "GET",
            "/api/workspace",
            Some(tmp.path().to_str().unwrap()),
            &*env,
        );
        assert_eq!(status.0, 200);
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["workspace"].is_null());
        assert_eq!(json["version"], "1.0");
    }

    #[test]
    fn get_source_valid_file_returns_200() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        std::fs::write(src_dir.join("main.rs"), "fn main() {}\n").unwrap();
        let env = crate::environment::fixed_environment();
        let (status, _, body, _) = handle_request(
            "GET",
            "/api/source?file=src/main.rs&line=1",
            Some(tmp.path().to_str().unwrap()),
            &*env,
        );
        assert_eq!(status.0, 200);
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["file"], "src/main.rs");
        assert!(json["content"].is_array());
    }

    #[test]
    fn get_source_path_traversal_returns_403() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
        let env = crate::environment::fixed_environment();
        let (status, _, body, _) = handle_request(
            "GET",
            "/api/source?file=../../../etc/passwd&line=1",
            Some(tmp.path().to_str().unwrap()),
            &*env,
        );
        assert_eq!(status.0, 403);
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            json["error"]
                .as_str()
                .unwrap()
                .contains("path_outside_scope")
        );
    }

    #[test]
    fn get_source_missing_file_returns_404() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
        let env = crate::environment::fixed_environment();
        let (status, _, body, _) = handle_request(
            "GET",
            "/api/source?file=nonexistent.rs&line=1",
            Some(tmp.path().to_str().unwrap()),
            &*env,
        );
        assert_eq!(status.0, 404);
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["error"].as_str().unwrap().contains("not_found"));
    }

    #[test]
    fn get_source_directory_returns_400() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        let env = crate::environment::fixed_environment();
        let (status, _, _, _) = handle_request(
            "GET",
            "/api/source?file=src&line=1",
            Some(tmp.path().to_str().unwrap()),
            &*env,
        );
        assert_eq!(status.0, 400);
    }

    #[test]
    fn percent_decode_handles_simple_strings() {
        // Test basic percent decoding.
        let result = percent_decode("hello").unwrap();
        assert_eq!(result, "hello");
        let result = percent_decode("src/main.rs").unwrap();
        assert_eq!(result, "src/main.rs");
    }

    #[test]
    fn percent_decode_handles_encoded_chars() {
        let result = percent_decode("file%20with%20spaces").unwrap();
        assert_eq!(result, "file with spaces");
    }
}
