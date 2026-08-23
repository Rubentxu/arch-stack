//! IDE doctor consolidation (Wave 3 Item 22).
//!
//! Turns the thin `archctl ide doctor <ide>` stub into a real health
//! check: JSON output for programmatic consumption, semantic exit
//! codes for CI, and stack drift detection reusing the existing
//! `IdeAdapter::diff_stack` trait method (implemented by all four
//! built-in adapters).

use crate::ide::{IdeAdapter, IdePresence};
use serde::Serialize;

/// A single stack-file drift entry (Missing or Stale).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DriftItem {
    pub path: String,
    pub kind: String,
}

/// Full IDE health report.
///
/// `healthy` is the single gate: `installed && config_root_exists &&
/// drift.is_empty()`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct IdeDoctorReport {
    pub ide: &'static str,
    pub name: &'static str,
    pub installed: bool,
    pub config_root: String,
    pub config_root_exists: bool,
    pub hint: Option<String>,
    pub drift: Vec<DriftItem>,
    pub healthy: bool,
}

/// Run the doctor checks for one adapter against the current stack
/// payload.
///
/// * `detect()` — is the IDE installed?
/// * `config_root().exists()` — is the config directory present?
/// * `diff_stack(payload)` — are the installed stack files aligned
///   with the current `archctl` version payload?
///
/// Errors (e.g., a broken adapter) propagate as `Err` — the caller
/// (CLI handler) maps them to exit 2.
pub fn check_ide_doctor(
    adapter: &dyn IdeAdapter,
    payload: &crate::ide::StackPayload,
) -> anyhow::Result<IdeDoctorReport> {
    let presence: IdePresence = adapter.detect()?;
    let config_root = adapter.config_root();
    let config_root_exists = config_root.exists();
    let drift_entries = adapter.diff_stack(payload)?;
    let drift = drift_entries
        .into_iter()
        .map(|d| DriftItem {
            path: d.path.display().to_string(),
            kind: match d.kind {
                crate::ide::DriftKind::Missing => "missing".to_string(),
                crate::ide::DriftKind::Stale => "stale".to_string(),
                crate::ide::DriftKind::Extra => "extra".to_string(),
            },
        })
        .collect::<Vec<_>>();
    let healthy = presence.installed && config_root_exists && drift.is_empty();
    Ok(IdeDoctorReport {
        ide: adapter.id(),
        name: adapter.name(),
        installed: presence.installed,
        config_root: config_root.display().to_string(),
        config_root_exists,
        hint: presence.hint,
        drift,
        healthy,
    })
}

/// Human-readable rendering of a report (used when `--json` is not
/// passed).
pub fn render_human(report: &IdeDoctorReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("{} ({})\n", report.name, report.ide));
    out.push_str(&format!("  installed: {}\n", report.installed));
    out.push_str(&format!("  config_root: {}\n", report.config_root));
    out.push_str(&format!(
        "  config_root_exists: {}\n",
        report.config_root_exists
    ));
    if let Some(hint) = &report.hint {
        out.push_str(&format!("  hint: {hint}\n"));
    }
    if report.drift.is_empty() {
        out.push_str("  stack: aligned\n");
    } else {
        out.push_str("  stack drift:\n");
        for item in &report.drift {
            let marker = match item.kind.as_str() {
                "missing" => "MISSING",
                _ => "STALE",
            };
            out.push_str(&format!("    [{marker}] {}\n", item.path));
        }
    }
    out.push_str(&format!(
        "  healthy: {}\n",
        if report.healthy { "yes" } else { "no" }
    ));
    out
}

/// Exit code for a report: 0 healthy, 1 unhealthy.
pub fn report_exit_code(report: &IdeDoctorReport) -> i32 {
    if report.healthy { 0 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ide::{DriftEntry, DriftKind, IdePresence, InstallReport, StackPayload};
    use std::path::PathBuf;

    /// Real `IdeAdapter` test fixture for the doctor checks. NOT a mock —
    /// it is a fully functional adapter whose state is parameterised so
    /// tests can drive the doctor logic with deterministic inputs (per
    /// AGENTS.md "no mocks for non-external ports"). Mirrors the shape of
    /// the production adapters (claude_code, opencode, etc.) so doctor
    /// assertions exercise the same code paths as real adapters.
    struct TestIdeAdapter {
        installed: bool,
        hint: Option<String>,
        config_root: PathBuf,
        drift: Vec<DriftEntry>,
    }

    impl IdeAdapter for TestIdeAdapter {
        fn id(&self) -> &'static str {
            "test-ide"
        }
        fn name(&self) -> &'static str {
            "TestIde"
        }
        fn detect(&self) -> anyhow::Result<IdePresence> {
            Ok(IdePresence {
                installed: self.installed,
                hint: self.hint.clone(),
            })
        }
        fn config_root(&self) -> PathBuf {
            self.config_root.clone()
        }
        fn install_stack(
            &self,
            _payload: &StackPayload,
            _install_root: Option<&std::path::Path>,
        ) -> anyhow::Result<InstallReport> {
            // Doctor tests never call install/remove; return a default
            // empty report if reached. Keeps the impl total (no
            // unreachable!() which would explode on accidental use).
            Ok(InstallReport::default())
        }
        fn remove_stack(&self, _payload_id: &str) -> anyhow::Result<InstallReport> {
            Ok(InstallReport::default())
        }
        fn diff_stack(&self, _payload: &StackPayload) -> anyhow::Result<Vec<DriftEntry>> {
            Ok(self.drift.clone())
        }
    }

    fn empty_payload() -> StackPayload {
        StackPayload {
            id: "arch-stack-test".to_string(),
            version: semver::Version::parse("0.0.0-test").unwrap(),
            skills: vec![],
            agents: vec![],
            plugins: vec![],
        }
    }

    #[test]
    fn healthy_when_installed_and_aligned() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter = TestIdeAdapter {
            installed: true,
            hint: None,
            config_root: tmp.path().to_path_buf(),
            drift: vec![],
        };
        let report = check_ide_doctor(&adapter, &empty_payload()).unwrap();
        assert!(report.healthy);
        assert_eq!(report_exit_code(&report), 0);
        assert!(report.drift.is_empty());
    }

    #[test]
    fn unhealthy_when_not_installed() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter = TestIdeAdapter {
            installed: false,
            hint: Some("install with archctl ide install fake".to_string()),
            config_root: tmp.path().to_path_buf(),
            drift: vec![],
        };
        let report = check_ide_doctor(&adapter, &empty_payload()).unwrap();
        assert!(!report.healthy);
        assert_eq!(report_exit_code(&report), 1);
        assert!(report.hint.is_some());
    }

    #[test]
    fn unhealthy_when_config_root_missing() {
        let adapter = TestIdeAdapter {
            installed: true,
            hint: None,
            config_root: PathBuf::from("/nonexistent/fake-config"),
            drift: vec![],
        };
        let report = check_ide_doctor(&adapter, &empty_payload()).unwrap();
        assert!(!report.healthy);
        assert_eq!(report_exit_code(&report), 1);
        assert!(!report.config_root_exists);
    }

    #[test]
    fn unhealthy_when_drift_present() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter = TestIdeAdapter {
            installed: true,
            hint: None,
            config_root: tmp.path().to_path_buf(),
            drift: vec![
                DriftEntry {
                    path: PathBuf::from("agents/x.md"),
                    kind: DriftKind::Missing,
                },
                DriftEntry {
                    path: PathBuf::from("skills/y.md"),
                    kind: DriftKind::Stale,
                },
            ],
        };
        let report = check_ide_doctor(&adapter, &empty_payload()).unwrap();
        assert!(!report.healthy);
        assert_eq!(report_exit_code(&report), 1);
        assert_eq!(report.drift.len(), 2);
        assert_eq!(report.drift[0].kind, "missing");
        assert_eq!(report.drift[1].kind, "stale");
    }

    #[test]
    fn json_serialization_shape() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter = TestIdeAdapter {
            installed: true,
            hint: None,
            config_root: tmp.path().to_path_buf(),
            drift: vec![],
        };
        let report = check_ide_doctor(&adapter, &empty_payload()).unwrap();
        let json = serde_json::to_string(&report).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["ide"], "test-ide");
        assert_eq!(parsed["healthy"], true);
        assert!(parsed.get("drift").is_some());
        assert!(parsed.get("config_root").is_some());
    }
}
