use crate::environment::Environment;
use crate::identity::{identity_summary, resolve_source_identity};
use crate::scope::{check_all_scopes, render_report_line, ScopeCheckReport};
use crate::xdg::{resolve_xdg, user_home};
use std::process::Command;
use tracing::{info, warn};

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

pub fn run() -> Result<i32, anyhow::Error> {
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
        let ok = path.exists() || std::fs::create_dir_all(path).is_ok();
        findings.push(Finding {
            id: id.to_string(),
            detail: path.display().to_string(),
            severity: if ok { Severity::Ok } else { Severity::Fail },
        });
    }

    findings.push(http_finding("renderer.structurizr", "http://localhost:18080/"));
    findings.push(http_finding("renderer.plantuml", "http://localhost:18000/"));
    findings.push(binary_finding("opencode.cli", "opencode"));
    findings.push(binary_finding("archctl.cli", "archctl"));

    println!("archctl doctor");
    let identity = resolve_source_identity(&cwd_str);
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
    let failed = findings.iter().filter(|f| f.severity == Severity::Fail).count();
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
        .args(["-sS", "-o", "/dev/null", "-w", "%{http_code}", "--max-time", "2", url])
        .output();
    let ok = match probe {
        Ok(o) => o.status.success() && String::from_utf8_lossy(&o.stdout).trim().starts_with('2'),
        Err(_) => false,
    };
    Finding {
        id: id.to_string(),
        detail: if ok { format!("reachable ({url})") } else { format!("not reachable ({url})") },
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
/// also useful from tests.
pub fn check_scope(cwd: &std::path::Path) -> Result<i32, anyhow::Error> {
    let manifests_dir = cwd.join("manifests");
    if !manifests_dir.exists() {
        println!("(no manifests/ directory at {})", cwd.display());
        println!("SCOPE: OK (no scopes declared)");
        return Ok(0);
    }
    let reports = check_all_scopes(cwd)?;
    print_scope_reports(&reports);
    let failed = reports.iter().filter(|r| !r.passed()).count();
    if failed > 0 {
        println!("SCOPE: FAIL");
        Ok(1)
    } else if reports.is_empty() {
        println!("(no manifests under manifests/)");
        println!("SCOPE: OK (no scopes declared)");
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
