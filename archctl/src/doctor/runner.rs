//! Smoke test runner for doctor scopes.
//!
//! The smoke runner executes a set of deterministic checks for each
//! scope. These are fast, in-process checks suitable for the pre-merge
//! CI gate (P0-12 fast lane).

use super::DoctorScope;
use crate::filesystem::Filesystem;
use std::path::Path;

/// Result of a smoke gate run.
#[derive(Debug, Clone)]
pub struct SmokeResult {
    /// Whether the smoke gate passed.
    pub passed: bool,
    /// Human-readable description of what was checked.
    pub description: String,
    /// Error message if the gate failed.
    pub error: Option<String>,
}

impl SmokeResult {
    /// Create a passed smoke result.
    fn pass(description: impl Into<String>) -> Self {
        Self {
            passed: true,
            description: description.into(),
            error: None,
        }
    }

    /// Create a failed smoke result.
    fn fail(description: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            passed: false,
            description: description.into(),
            error: Some(error.into()),
        }
    }
}

/// Run the smoke gate for a specific doctor scope.
///
/// A smoke gate is a fast, deterministic check that verifies the basic
/// functionality of a scope. It is designed to run in the pre-merge CI
/// gate (P0-12 fast lane).
pub fn run_smoke_gate(
    scope: DoctorScope,
    project_dir: &Path,
    _fs: &dyn Filesystem,
) -> Result<SmokeResult, anyhow::Error> {
    match scope {
        DoctorScope::Storage => run_storage_smoke(project_dir),
    }
}

/// Run the storage scope smoke gate.
///
/// Checks:
/// 1. The lbug store can be opened
/// 2. The schema is initialized
/// 3. Basic read/write operations work
fn run_storage_smoke(project_dir: &Path) -> Result<SmokeResult, anyhow::Error> {
    use super::storage::{LbugStorageProbe, run_storage_probe};

    let probe = LbugStorageProbe::new();
    let report = run_storage_probe(&probe, project_dir)?;

    // Check the fresh_crud finding specifically
    let fresh_finding = report
        .findings
        .iter()
        .find(|f| f.id == "storage.fresh_crud");
    if let Some(finding) = fresh_finding {
        match finding.severity {
            super::storage::Severity::Ok => Ok(SmokeResult::pass(finding.detail.clone())),
            _ => Ok(SmokeResult::fail("storage.fresh_crud", &finding.detail)),
        }
    } else {
        Ok(SmokeResult::fail(
            "storage.fresh_crud",
            "finding not present",
        ))
    }
}

/// Run all smoke gates for the doctor module.
///
/// Returns the exit code: 0 if all gates pass, 1 if any fail.
pub fn run_all_smoke_gates(project_dir: &Path, fs: &dyn Filesystem) -> Result<i32, anyhow::Error> {
    let scopes = [DoctorScope::Storage];
    let mut failed = 0;

    for scope in &scopes {
        let result = run_smoke_gate(*scope, project_dir, fs)?;
        if result.passed {
            println!("SMOKE: OK — {}", result.description);
        } else {
            println!(
                "SMOKE: FAIL — {}: {}",
                result.description,
                result.error.unwrap_or_default()
            );
            failed += 1;
        }
    }

    if failed > 0 { Ok(1) } else { Ok(0) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::MemoryFilesystem;
    use tempfile::TempDir;

    #[test]
    fn smoke_gate_storage_with_temp_dir() {
        let tmp = TempDir::new().unwrap();
        let fs = MemoryFilesystem::new();
        let result = run_smoke_gate(DoctorScope::Storage, tmp.path(), &fs)
            .expect("smoke gate must not error");
        assert!(
            result.passed,
            "fresh temp dir should pass smoke: {:?}",
            result.error
        );
    }
}
