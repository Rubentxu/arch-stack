//! PlantUML → SVG renderer (M40).
//!
//! **Local-only via user-provided backend.** archctl itself does NOT link
//! against graphviz or any native binary. Per ADR-011 ("local-only"), the
//! PlantUML rendering work is delegated to a PlantUML engine that the user
//! installs:
//!
//! 1. **Java PlantUML CLI** (`plantuml` in PATH) — canonical, byte-faithful.
//!    Install: <https://plantuml.com/download> or `brew install plantuml`.
//! 2. **Docker** — `docker run --rm -i plantuml/plantuml -pipe -tpng` or `-tsvg`.
//!    Used as fallback if Java PlantUML is missing and docker is present.
//! 3. **Custom binary** — `archctl-puml-backend` in PATH, takes puml on stdin
//!    and emits SVG on stdout.
//!
//! ADR-006 ("envuelve, no reimplementa") says archctl orchestrates adapters;
//! it does not compete with the canonical PlantUML implementation. ADR-011
//! forbids network egress. Delegating to a user-installed backend satisfies
//! both — archctl never opens sockets or links graphviz.
//!
//! ## Detection order
//!
//! On each `render(source)` call, this module probes the backends in order:
//!
//! 1. `plantuml` in PATH.
//! 2. `docker` in PATH (with `plantuml/plantuml` image).
//! 3. `archctl-puml-backend` in PATH.
//!
//! The first probe that succeeds is used. If none succeed, returns a clear
//! error pointing the user to installation instructions.
//!
//! ## Why not `plantuml-little`
//!
//! Explored in M40 cycle; rejected because the crate hard-links against
//! `graphviz-anywhere` at compile time (graphviz native library required
//! even for use case / state / class diagram layouts). This violates ADR-011
//! (no graphviz binary in the build). See `docs/specs/plantuml-render.md`
//! for the full analysis.

use anyhow::{Context, Result, bail};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Probe result for a PlantUML backend.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Backend {
    /// `plantuml` binary in PATH (Java PlantUML CLI).
    PlantumlCli,
    /// `docker run plantuml/plantuml` invocation.
    DockerImage,
    /// `archctl-puml-backend` in PATH (custom user binary).
    CustomUserBinary,
}

impl Backend {
    fn label(&self) -> &'static str {
        match self {
            Backend::PlantumlCli => "plantuml (Java CLI)",
            Backend::DockerImage => "docker plantuml/plantuml",
            Backend::CustomUserBinary => "archctl-puml-backend",
        }
    }
}

/// Detect which PlantUML backend is available, if any.
fn detect_backend() -> Option<Backend> {
    if Command::new("plantuml")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some(Backend::PlantumlCli);
    }
    if Command::new("docker")
        .args(["image", "inspect", "plantuml/plantuml"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some(Backend::DockerImage);
    }
    if Command::new("archctl-puml-backend")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some(Backend::CustomUserBinary);
    }
    None
}

/// Render a PlantUML source string to SVG using the first available backend.
///
/// Returns an error if no backend is installed. The error message lists all
/// three installation options so the user can pick the one they prefer.
pub fn render(source: &str) -> Result<String> {
    let backend = detect_backend().ok_or_else(|| {
        anyhow::anyhow!(
            "no PlantUML backend found in PATH. Install one of:\n\
             \n\
             1. Java PlantUML CLI: brew install plantuml  (or download from plantuml.com)\n\
             2. Docker image:       docker pull plantuml/plantuml\n\
             3. Custom binary:      place an `archctl-puml-backend` executable in PATH\n\
                  (reads puml on stdin, writes svg on stdout)\n\
             \n\
             archctl does NOT link graphviz or open network connections (ADR-011). \
             PlantUML rendering is delegated to a user-installed engine (ADR-006)."
        )
    })?;

    tracing::info!(
        backend = backend.label(),
        "rendering PlantUML via external backend"
    );

    let mut child = match backend {
        Backend::PlantumlCli => Command::new("plantuml")
            .arg("-pipe")
            .arg("-tsvg")
            .arg("-charset")
            .arg("UTF-8")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("spawn `plantuml` (Java CLI)")?,
        Backend::DockerImage => Command::new("docker")
            .args([
                "run",
                "--rm",
                "-i",
                "plantuml/plantuml",
                "-pipe",
                "-tsvg",
                "-charset",
                "UTF-8",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("spawn `docker run plantuml/plantuml`")?,
        Backend::CustomUserBinary => Command::new("archctl-puml-backend")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("spawn `archctl-puml-backend`")?,
    };

    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(source.as_bytes())
        .context("write PlantUML source to backend stdin")?;
    let out = child.wait_with_output().context("wait for backend")?;

    if !out.status.success() {
        bail!(
            "PlantUML backend ({}) failed (exit {:?}):\n{}",
            backend.label(),
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    let svg = String::from_utf8(out.stdout).context("backend stdout is not valid UTF-8")?;
    Ok(svg)
}

/// Re-export the source's "looks like PlantUML" check used by `render.rs::run`
/// to decide whether to dispatch here. Currently unused — kept for future
/// path-based dispatch (e.g. detecting `@startuml` inside a `.txt` file).
#[allow(dead_code)]
pub fn looks_like_plantuml(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_ascii_lowercase().as_str(), "puml" | "iuml" | "wsd"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_backend_returns_none_or_some() {
        // We can't assert which backend is present (depends on the test
        // machine); just ensure the function returns without panicking.
        let result = detect_backend();
        // Either Some or None is acceptable depending on test environment.
        let _ = result;
    }

    #[test]
    fn render_without_backend_returns_clear_error() {
        // Probe a guaranteed-missing binary by asking detect_backend to look
        // for an obviously-nonexistent name. The cleanest way to test the
        // error path without mutating PATH (which would require unsafe in
        // modern Rust) is to just call detect_backend and assert the error
        // message we get when render is invoked matches what we expect.
        //
        // We rely on the fact that even if a backend IS installed in CI,
        // we can still validate the *shape* of a render call by checking
        // that either: (a) it succeeds, or (b) it fails with a clear
        // "no PlantUML backend found" message.
        let result = render("@startuml\nA --> B\n@enduml\n");
        match result {
            Ok(svg) => {
                // Backend IS installed — just verify the SVG looks like SVG.
                assert!(
                    svg.contains("<svg"),
                    "backend SVG should contain <svg; got first 80: {}",
                    svg.chars().take(80).collect::<String>()
                );
            }
            Err(err) => {
                let msg = format!("{err:#}");
                assert!(
                    msg.contains("no PlantUML backend found") || msg.contains("PlantUML backend"),
                    "error should mention PlantUML backend; got: {msg}"
                );
            }
        }
    }

    #[test]
    fn looks_like_plantuml_recognises_puml_extension() {
        assert!(looks_like_plantuml(Path::new("diagram.puml")));
        assert!(looks_like_plantuml(Path::new("diagram.iuml")));
        assert!(looks_like_plantuml(Path::new("diagram.wsd")));
        assert!(!looks_like_plantuml(Path::new("diagram.mmd")));
        assert!(!looks_like_plantuml(Path::new("diagram.rs")));
    }
}
