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

#[derive(RustEmbed)]
#[folder = "assets-view/"]
struct ViewAssets;

#[derive(Debug, Clone, Default)]
pub struct ViewOptions {
    /// Port to bind. `0` = ephemeral (default).
    pub port: u16,
    /// Project directory to export bundles from (for `/api/export`).
    /// If `None`, the endpoint returns a clear error.
    pub project_dir: Option<String>,
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

fn handle_api_export(project_dir: Option<&str>) -> Result<(String, Vec<u8>)> {
    let dir = project_dir.ok_or_else(|| {
        anyhow::anyhow!("no project_dir configured — run `archctl view --cwd <dir>`")
    })?;
    let fs = crate::filesystem::system_filesystem();
    let info = crate::project::resolve_project(dir);
    let store = crate::store::open_and_init(&info.project_dir)?;

    // Export to a temp dir, then inline the bundle files as a JSON envelope.
    let bundle_dir =
        std::env::temp_dir().join(format!("archctl-view-export-{}", uuid::Uuid::new_v4()));
    let report = crate::diagram::run_export(
        &*store,
        "container:*",
        &bundle_dir,
        &crate::clock::SystemClock,
        &*fs,
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
    });
    let body = serde_json::to_vec_pretty(&payload)?;
    Ok(("application/json".to_string(), body))
}

/// Pure request handler — testable without a socket.
///
/// Returns `(status, content_type, body)`.
pub fn handle_request(
    method: &str,
    url: &str,
    project_dir: Option<&str>,
) -> (tiny_http::StatusCode, String, Vec<u8>) {
    match (method, url) {
        ("GET", "/api/health") => {
            let payload = serde_json::json!({
                "status": "ok",
                "version": env!("CARGO_PKG_VERSION"),
            });
            (
                tiny_http::StatusCode(200),
                "application/json".to_string(),
                serde_json::to_vec(&payload).unwrap_or_default(),
            )
        }
        ("GET", "/api/export") => match handle_api_export(project_dir) {
            Ok((mime, body)) => (tiny_http::StatusCode(200), mime, body),
            Err(e) => (
                tiny_http::StatusCode(500),
                "application/json".to_string(),
                serde_json::json!({ "error": e.to_string() })
                    .to_string()
                    .into_bytes(),
            ),
        },
        ("GET", path) => match serve_static(path) {
            Some((mime, body)) => (tiny_http::StatusCode(200), mime, body),
            None => (
                tiny_http::StatusCode(404),
                "text/plain; charset=utf-8".to_string(),
                b"not found".to_vec(),
            ),
        },
        _ => (
            tiny_http::StatusCode(405),
            "text/plain; charset=utf-8".to_string(),
            b"method not allowed".to_vec(),
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

    println!("archctl view — http://{addr}");
    println!("workbench: archview (embedded, ADR-033)");
    println!("project:   {}", project_dir.as_deref().unwrap_or("<none>"));
    println!("press Ctrl+C to stop");

    let server = tiny_http::Server::from_listener(listener, None)
        .map_err(|e| anyhow::anyhow!("tiny_http init failed: {e}"))?;

    for request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().clone();
        let project = project_dir.clone();

        let (status, mime, body) =
            handle_request(method.as_str(), url.as_str(), project.as_deref());

        let _ = request.respond(
            tiny_http::Response::from_data(body)
                .with_status_code(status)
                .with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], mime.as_bytes())
                        .expect("valid header"),
                )
                // ADR-020: SharedArrayBuffer requires COOP/COEP.
                .with_header(
                    tiny_http::Header::from_bytes(
                        &b"Cross-Origin-Opener-Policy"[..],
                        b"same-origin",
                    )
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
                    tiny_http::Header::from_bytes(
                        &b"Cross-Origin-Resource-Policy"[..],
                        b"same-origin",
                    )
                    .expect("valid header"),
                ),
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn status_of(method: &str, url: &str) -> u16 {
        let (status, _, _) = handle_request(method, url, None);
        status.0
    }

    #[test]
    fn health_returns_ok_and_version() {
        let (status, mime, body) = handle_request("GET", "/api/health", None);
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
        let (status, mime, body) = handle_request("GET", "/", None);
        assert_eq!(status.0, 200);
        assert!(mime.starts_with("text/html"));
        assert!(!body.is_empty());
    }

    #[test]
    fn missing_asset_is_404() {
        let (status, _, body) = handle_request("GET", "/nope.js", None);
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
    fn export_without_project_is_500_json() {
        let (status, mime, body) = handle_request("GET", "/api/export", None);
        assert_eq!(status.0, 500);
        assert_eq!(mime, "application/json");
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["error"].as_str().unwrap().contains("project_dir"));
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
}
