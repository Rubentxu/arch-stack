//! Test helpers shared across PlantUML e2e tests.
//!
//! M56: extracted the `backend_available()` helper from 5 e2e test files
//! (`plantuml_render_e2e`, `usecase_view_plantuml_e2e`,
//! `sequence_view_plantuml_e2e`, `state_view_plantuml_e2e`,
//! `c4_view_plantuml_e2e`). All tests use the same skip-on-missing-backend
//! pattern per M40 + M43 + M48 + M49 + M50 conventions.
//!
//! To use, add to the integration test file:
//!
//! ```ignore
//! use archctl::test_helpers::plantuml::backend_available;
//! ```

/// Probe the system for a usable PlantUML backend.
///
/// Returns `true` if either:
/// - `plantuml` (Java PlantUML CLI) is on PATH and responds to `-version`.
/// - `docker` is on PATH AND the `plantuml/plantuml` image is pulled.
///
/// On `false`, e2e tests should SKIP (early return) instead of failing
/// — this keeps the suite usable on machines without PlantUML installed
/// (CI typical) while giving machines WITH PlantUML a real end-to-end
/// verification.
///
/// Per ADR-011 (local-only renderers) and M40 (delegation strategy).
pub fn backend_available() -> bool {
    java_plantuml_installed() || docker_plantuml_image_pulled()
}

/// Probe for the Java PlantUML CLI.
fn java_plantuml_installed() -> bool {
    std::process::Command::new("plantuml")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Probe for the docker image `plantuml/plantuml`.
fn docker_plantuml_image_pulled() -> bool {
    std::process::Command::new("docker")
        .args(["image", "inspect", "plantuml/plantuml"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
