//! Doctor module — architecture compatibility and storage compatibility checks.
//!
//! This module provides the `doctor` subcommand. It checks XDG layout,
//! source identity, renderer availability, and scope manifests.
//!
//! ## Scope architecture
//!
//! The `DoctorScope` enum enumerates available diagnostic scopes.
//! The `runner` submodule orchestrates smoke gates for each scope.

pub mod manifest;
pub mod runner;
pub mod storage;

use crate::cli::CliContext;
use crate::environment::Environment;
use crate::filesystem::Filesystem;
use crate::identity::{identity_summary, resolve_source_identity};
use crate::scope::{ScopeCheckReport, check_all_scopes, render_report_line};
use crate::xdg::{resolve_xdg, user_home};

// Re-export storage types for use in CLI and tests.
pub use storage::{
    Finding as StorageFinding, Severity as StorageSeverity, Status as StorageStatus, StorageProbe,
    StorageReport, render_json, render_text, run_storage_probe,
};

use std::path::Path;
use std::process::Command;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Original doctor types (XDG, renderers, binaries)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Finding {
    pub id: String,
    pub detail: String,
    pub severity: Severity,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Severity {
    Ok,
    Warn,
    Fail,
}

/// Run the main doctor diagnostics (XDG, renderers, binaries).
pub fn run(ctx: &CliContext) -> Result<i32, anyhow::Error> {
    let layout = resolve_xdg();
    let mut findings: Vec<Finding> = Vec::new();
    // The Environment port is the boundary. doctor.rs is an internal
    // entry point — it uses SystemEnvironment directly. Tests can
    // inject a context via a future `run_with_env` if needed.
    let cwd = crate::environment::SystemEnvironment.current_dir().ok();
    let cwd_str = cwd
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    for (id, path) in [
        ("xdg.data", &layout.data),
        ("xdg.config", &layout.config),
        ("xdg.state", &layout.state),
        ("xdg.cache", &layout.cache),
    ] {
        let ok = ctx.fs.create_dir_all(path).is_ok();
        findings.push(Finding {
            id: id.to_string(),
            detail: path.display().to_string(),
            severity: if ok { Severity::Ok } else { Severity::Fail },
        });
    }

    findings.push(http_finding(
        "renderer.structurizr",
        "http://localhost:18080/",
    ));
    findings.push(http_finding("renderer.plantuml", "http://localhost:18000/"));
    findings.push(binary_finding("opencode.cli", "opencode"));
    findings.push(binary_finding("archctl.cli", "archctl"));

    println!("archctl doctor");
    let identity = resolve_source_identity(&cwd_str, &*ctx.fs)?;
    info!(home = %user_home().display(), "doctor starting");
    for f in &findings {
        let tag = match f.severity {
            Severity::Ok => "OK  ",
            Severity::Warn => "WARN",
            Severity::Fail => "FAIL",
        };
        println!("  [{tag}] {}: {}", f.id, f.detail);
    }
    println!("  sourceIdentity: {}", identity_summary(&identity));
    let failed = findings
        .iter()
        .filter(|f| f.severity == Severity::Fail)
        .count();
    if failed > 0 {
        warn!(failures = failed, "doctor detected failures");
        println!("DOCTOR: FAIL");
        Ok(1)
    } else {
        info!("doctor: all OK");
        println!("DOCTOR: OK");
        Ok(0)
    }
}

fn http_finding(id: &str, url: &str) -> Finding {
    let probe = Command::new("curl")
        .args([
            "-sS",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "--max-time",
            "2",
            url,
        ])
        .output();
    let ok = match probe {
        Ok(o) => o.status.success() && String::from_utf8_lossy(&o.stdout).trim().starts_with('2'),
        Err(_) => false,
    };
    Finding {
        id: id.to_string(),
        detail: if ok {
            format!("reachable ({url})")
        } else {
            format!("not reachable ({url})")
        },
        severity: if ok { Severity::Ok } else { Severity::Warn },
    }
}

fn binary_finding(id: &str, name: &str) -> Finding {
    let probe = Command::new(name).arg("--version").output();
    let ok = matches!(&probe, Ok(o) if o.status.success());
    let detail = match probe {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .to_string(),
        _ => "not on PATH".to_string(),
    };
    Finding {
        id: id.to_string(),
        detail,
        severity: if ok { Severity::Ok } else { Severity::Warn },
    }
}

/// Run the scope gates against every manifest under `manifests/`.
/// Returns the exit code (0 if all gates pass or no manifests
/// exist; 1 if any gate fails).
/// Designed to be called from `archctl doctor --check-scope` but is
/// Run scope gates for specific scope IDs, or all scopes if `scope_ids`
/// is empty.  If a scope ID is not found, it is silently skipped.
pub fn check_scope(
    cwd: &std::path::Path,
    scope_ids: Vec<String>,
    fs: &dyn Filesystem,
) -> Result<i32, anyhow::Error> {
    let manifests_dir = cwd.join("manifests");
    if !manifests_dir.exists() {
        println!("(no manifests/ directory at {})", cwd.display());
        println!("SCOPE: OK (no scopes declared)");
        return Ok(0);
    }
    let all_reports = check_all_scopes(cwd, fs)?;
    // If specific IDs given, filter; otherwise check all.
    let reports: Vec<_> = if scope_ids.is_empty() {
        all_reports
    } else {
        all_reports
            .into_iter()
            .filter(|r| scope_ids.contains(&r.scope_id))
            .collect()
    };
    print_scope_reports(&reports);
    let failed = reports.iter().filter(|r| !r.passed()).count();
    if failed > 0 {
        println!("SCOPE: FAIL");
        Ok(1)
    } else if reports.is_empty() {
        println!("(no matching scopes)");
        println!("SCOPE: OK");
        Ok(0)
    } else {
        println!("SCOPE: OK");
        Ok(0)
    }
}

/// Render a list of scope check reports in a way that is human-
/// readable. The summary line is per-scope; full findings go to
/// stderr for tools that want detail.
fn print_scope_reports(reports: &[ScopeCheckReport]) {
    for report in reports {
        println!("{}", render_report_line(report));
        for f in &report.findings {
            eprintln!(
                "    [{}] {}: {}",
                match f.severity {
                    crate::scope::ScopeSeverity::Fail => "FAIL",
                    crate::scope::ScopeSeverity::Warn => "WARN",
                },
                f.gate.name(),
                f.message
            );
        }
    }
}

// ---------------------------------------------------------------------------
// New scope-based doctor system
// ---------------------------------------------------------------------------

/// Available doctor diagnostic scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorScope {
    /// Storage compatibility: LadybugDB (lbug) availability and basic operations.
    Storage,
}

impl std::fmt::Display for DoctorScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DoctorScope::Storage => write!(f, "storage"),
        }
    }
}

impl std::str::FromStr for DoctorScope {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "storage" => Ok(DoctorScope::Storage),
            _ => Err(format!("unknown doctor scope: {s}")),
        }
    }
}

/// Run the doctor diagnostics for a specific scope.
///
/// Returns the exit code: 0 if all checks pass, 1 if any fail.
/// If `json` is true, emits a JSON envelope to stdout.
pub fn run_scope(scope: DoctorScope, project_dir: &Path, json: bool) -> Result<i32, anyhow::Error> {
    match scope {
        DoctorScope::Storage => {
            let probe = storage::LbugStorageProbe::new();
            let report = storage::run_storage_probe(&probe, project_dir)?;
            if json {
                storage::render_json(&report, &mut std::io::stdout())?;
            } else {
                storage::render_text(&report)?;
            }
            // Exit code: 0 for Compatible, 1 for Mismatch/Unknown
            let status = &report.status;
            match status.as_str() {
                "Compatible" => Ok(0),
                _ => Ok(1),
            }
        }
    }
}

/// Validate manifest contracts for all scopes.
///
/// Checks that every scope's manifest exists and that the declared
/// symbols and invariants match the actual code.
pub fn validate_manifests(project_dir: &Path, fs: &dyn Filesystem) -> Result<i32, anyhow::Error> {
    manifest::validate_manifests(project_dir, fs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_scope_parse_storage() {
        let scope: DoctorScope = "storage".parse().unwrap();
        assert_eq!(scope, DoctorScope::Storage);
    }

    #[test]
    fn doctor_scope_parse_unknown() {
        let result: Result<DoctorScope, _> = "unknown".parse();
        assert!(result.is_err());
    }

    #[test]
    fn doctor_scope_display() {
        assert_eq!(DoctorScope::Storage.to_string(), "storage");
    }
}
