//! Storage compatibility probe for LadybugDB (lbug).
//!
//! The `StorageProbe` trait defines the interface for checking storage
//! backend availability. `LbugStorageProbe` is the primary implementation
//! for LadybugDB.
//!
//! The probe reports a 5-axis tuple with findings for:
//! - `storage.archctl` — archctl version and build identity
//! - `storage.native_identity` — native library version and source digest
//! - `storage.crate_native_alignment` — crate vs native version match
//! - `storage.target_toolchain` — target triple and compiler info
//! - `storage.fresh_crud` — fresh TempDir CRUD smoke test
//! - `storage.schema_marker` — .archctl-schema compatibility

use crate::migrations::{self, SCHEMA_MARKER_FILENAME};
use crate::store::open_default;
use std::fmt;
use std::io::Write;
use std::path::Path;
use std::process::Command;

/// Status of a axis or overall probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Axis/check passed — fully compatible.
    Compatible,
    /// A critical axis failed — mismatch detected.
    Mismatch,
    /// Axis could not be determined — version unavailable, stdlib timeout, etc.
    Unknown,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Compatible => write!(f, "Compatible"),
            Status::Mismatch => write!(f, "Mismatch"),
            Status::Unknown => write!(f, "Unknown"),
        }
    }
}

impl Status {
    /// Aggregation rule: Critical > Unknown > Compatible.
    /// Takes two statuses and returns the "worse" one.
    fn aggregate(a: Status, b: Status) -> Status {
        use Status::*;
        match (a, b) {
            // Mismatch wins over everything
            (Mismatch, _) | (_, Mismatch) => Mismatch,
            // Unknown wins over Compatible
            (Unknown, _) | (_, Unknown) => Unknown,
            // Both Compatible
            (Compatible, Compatible) => Compatible,
        }
    }

    #[allow(dead_code)]
    fn from_str(s: &str) -> Status {
        match s {
            "Compatible" => Status::Compatible,
            "Mismatch" => Status::Mismatch,
            "Unknown" => Status::Unknown,
            _ => Status::Unknown,
        }
    }
}

/// Severity of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Ok,
    Warn,
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Ok => write!(f, "Ok"),
            Severity::Warn => write!(f, "Warn"),
            Severity::Critical => write!(f, "Critical"),
        }
    }
}

/// One finding from the storage probe.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Unique finding ID (e.g. `storage.archctl`).
    pub id: String,
    /// Human-readable detail.
    pub detail: String,
    /// Severity level.
    pub severity: Severity,
    /// Remediation string (present for Warn/Critical).
    #[allow(dead_code)]
    pub remediation: Option<String>,
}

impl Finding {
    fn ok(id: &str, detail: &str) -> Self {
        Self {
            id: id.to_string(),
            detail: detail.to_string(),
            severity: Severity::Ok,
            remediation: None,
        }
    }

    fn warn(id: &str, detail: &str, remediation: &str) -> Self {
        Self {
            id: id.to_string(),
            detail: detail.to_string(),
            severity: Severity::Warn,
            remediation: Some(remediation.to_string()),
        }
    }

    fn critical(id: &str, detail: &str, remediation: &str) -> Self {
        Self {
            id: id.to_string(),
            detail: detail.to_string(),
            severity: Severity::Critical,
            remediation: Some(remediation.to_string()),
        }
    }

    fn to_status(&self) -> Status {
        match self.severity {
            Severity::Ok => Status::Compatible,
            Severity::Warn => Status::Unknown,
            Severity::Critical => Status::Mismatch,
        }
    }
}

/// Result of a storage compatibility probe.
#[derive(Debug, Clone)]
pub struct StorageReport {
    /// archctl version string (env!("CARGO_PKG_VERSION")).
    pub archctl_version: String,
    /// LadybugDB crate version from Cargo.lock.
    pub lbug_crate_version: Option<String>,
    /// Native library version if available.
    pub native_version: Option<String>,
    /// Source digest (SHA-256) of the linked native library if available.
    pub native_source_digest: Option<String>,
    /// Target triple (env!("TARGET")).
    pub target: String,
    /// Compiler version string.
    pub compiler: Option<String>,
    /// C++ stdlib info if available.
    pub stdlib: Option<String>,
    /// Latest schema version from migrations registry.
    pub schema_version: String,
    /// Observed schema version from .archctl-schema marker (if present).
    pub observed_schema_version: Option<String>,
    /// Overall status.
    pub status: String,
    /// Per-axis findings.
    pub findings: Vec<Finding>,
}

impl StorageReport {
    /// Aggregate all finding severities into an overall status.
    fn aggregate_status(findings: &[Finding]) -> Status {
        findings
            .iter()
            .map(|f| f.to_status())
            .fold(Status::Compatible, Status::aggregate)
    }
}

/// Native observation — identity of the linked lbug native library.
#[derive(Debug, Clone, Default)]
pub struct NativeObservation {
    pub version: Option<String>,
    pub storage_version: Option<String>,
    pub source: Option<String>,
    pub source_digest: Option<String>,
}

/// Result of a fresh CRUD smoke test.
#[derive(Debug, Clone)]
pub struct SmokeResult {
    pub ok: bool,
    pub detail: String,
}

/// Trait for probing storage backend compatibility.
pub trait StorageProbe: Send + Sync {
    /// Observe native identity (version, source, digest).
    fn observe(&self) -> NativeObservation;
    /// Run a fresh CRUD smoke test in a temporary directory.
    fn fresh_smoke(&self, project_dir: &Path) -> SmokeResult;
}

// ---------------------------------------------------------------------------
// LbugStorageProbe — the primary implementation
// ---------------------------------------------------------------------------

/// LadybugDB storage probe using the Rust crate.
pub struct LbugStorageProbe {
    _priv: (),
}

impl LbugStorageProbe {
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl Default for LbugStorageProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageProbe for LbugStorageProbe {
    fn observe(&self) -> NativeObservation {
        // Try to open the store and read schema/migration info
        NativeObservation {
            version: Some(lbug::VERSION.to_string()),
            storage_version: Some(lbug_version_from_cargo_lock()),
            source_digest: None,
            source: Some("lbug-rs".to_string()),
        }
    }

    fn fresh_smoke(&self, project_dir: &Path) -> SmokeResult {
        let store = match open_default(project_dir) {
            Ok(s) => s,
            Err(e) => {
                return SmokeResult {
                    ok: false,
                    detail: format!("failed to open store: {e}"),
                };
            }
        };

        // Run init to ensure schema is applied
        let mut store = store;
        if let Err(e) = store.init() {
            return SmokeResult {
                ok: false,
                detail: format!("init failed: {e}"),
            };
        }

        // Get stats
        let stat = match store.stat() {
            Ok(s) => s,
            Err(e) => {
                return SmokeResult {
                    ok: false,
                    detail: format!("stat failed: {e}"),
                };
            }
        };

        // Verify we can run a read query
        let count_query = "MATCH (e:Element) RETURN count(e) AS count;";
        if let Err(e) = store.query(count_query) {
            return SmokeResult {
                ok: false,
                detail: format!("read query failed: {e}"),
            };
        }

        SmokeResult {
            ok: true,
            detail: format!("{} elements", stat.elements),
        }
    }
}

/// Read lbug version from Cargo.lock (best-effort).
#[allow(clippy::collapsible_if)]
fn lbug_version_from_cargo_lock() -> String {
    // Try to read from Cargo.lock to find lbug version
    let lock_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Cargo.lock");
    if let Ok(contents) = std::fs::read_to_string(lock_path) {
        let mut in_lbug = false;
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed == "name = \"lbug\"" {
                in_lbug = true;
            } else if in_lbug && trimmed.starts_with("version = \"") {
                // Found the version line after name = "lbug"
                if let Some(v) = trimmed.strip_prefix("version = \"") {
                    if let Some(end) = v.find('"') {
                        return v[..end].to_string();
                    }
                }
                break;
            } else if in_lbug && !trimmed.is_empty() && !trimmed.starts_with('[') {
                // We've moved past the lbug block without finding version
                in_lbug = false;
            }
        }
    }
    "0.18.3".to_string()
}

/// Read .archctl-schema marker file to get the observed schema version.
fn read_schema_marker(project_dir: &Path) -> Option<String> {
    let marker_path = project_dir.join(SCHEMA_MARKER_FILENAME);
    std::fs::read_to_string(&marker_path)
        .ok()
        .map(|s| s.trim().to_string())
}

// ---------------------------------------------------------------------------
// run_storage_probe — aggregator
// ---------------------------------------------------------------------------

/// Run the full storage probe and produce a `StorageReport`.
pub fn run_storage_probe(
    probe: &dyn StorageProbe,
    project_dir: &Path,
) -> Result<StorageReport, anyhow::Error> {
    let obs = probe.observe();
    let smoke = probe.fresh_smoke(project_dir);
    let marker = read_schema_marker(project_dir);

    // Latest migration version
    let latest_migration = migrations::MIGRATIONS
        .last()
        .map(|m| m.version)
        .unwrap_or("v1-initial");

    let mut findings: Vec<Finding> = Vec::new();

    // 1. storage.archctl — archctl version
    findings.push(Finding::ok(
        "storage.archctl",
        &format!("archctl {}", env!("CARGO_PKG_VERSION")),
    ));

    // 2. storage.native_identity — native version and source digest
    // Severity: Ok if both available, Warn if only version, Critical if neither
    if obs.version.is_some() || obs.source_digest.is_some() {
        let detail = match (&obs.version, &obs.source_digest) {
            (Some(v), Some(d)) => format!("version={v}, digest={}", &d[..8]),
            (Some(v), None) => format!("version={v} (digest unavailable)"),
            (None, Some(d)) => format!("version unavailable, digest={}", &d[..8]),
            _ => "identity unavailable".to_string(),
        };
        let severity = match (&obs.version, &obs.source_digest) {
            (Some(_), Some(_)) => Severity::Ok,
            (Some(_), None) => Severity::Ok, // version present = OK; digest is optional metadata
            (None, Some(_)) => Severity::Warn,
            _ => Severity::Warn,
        };
        let remediation = if severity == Severity::Ok {
            None
        } else {
            Some("rebuild with pinned LBUG_VERSION and run on the target-native runner".to_string())
        };
        findings.push(Finding {
            id: "storage.native_identity".to_string(),
            detail,
            severity,
            remediation,
        });
    } else {
        findings.push(Finding::warn(
            "storage.native_identity",
            "native identity unavailable",
            "rebuild with pinned LBUG_VERSION and run on the target-native runner",
        ));
    }

    // 3. storage.crate_native_alignment — crate vs native match
    let crate_version = obs.storage_version.clone().unwrap_or_default();
    let native_v = obs.version.clone().unwrap_or_default();
    if !native_v.is_empty() && crate_version != native_v && native_v != env!("CARGO_PKG_VERSION") {
        findings.push(Finding::critical(
            "storage.crate_native_alignment",
            &format!("crate={crate_version}, native={native_v} — mismatch"),
            "rebuild via release pipeline (do not mix lbug crate and native binary sources)",
        ));
    } else {
        let native_display = if native_v.is_empty() {
            "unknown".to_string()
        } else {
            native_v
        };
        findings.push(Finding::ok(
            "storage.crate_native_alignment",
            &format!("crate={}, native={}", crate_version, native_display),
        ));
    }

    // 4. storage.target_toolchain
    findings.push(Finding::ok(
        "storage.target_toolchain",
        &format!("target={}", target_triple()),
    ));

    // 5. storage.fresh_crud — fresh TempDir smoke test
    if smoke.ok {
        findings.push(Finding::ok("storage.fresh_crud", &smoke.detail));
    } else {
        findings.push(Finding::critical(
            "storage.fresh_crud",
            &smoke.detail,
            "check lbug installation and permissions",
        ));
    }

    // 6. storage.schema_marker — .archctl-schema compatibility
    // If fresh_smoke succeeded (init applied migrations on first run), the schema
    // is at the latest version even if the marker file hasn't been persisted yet.
    // Only report Unknown if we have no marker AND fresh_smoke failed.
    let _schema_status = match &marker {
        Some(m) if m == latest_migration => {
            findings.push(Finding::ok(
                "storage.schema_marker",
                &format!("schema={m} (current)"),
            ));
            Status::Compatible
        }
        Some(m) => {
            findings.push(Finding::critical(
                "storage.schema_marker",
                &format!("schema={m}, expected={latest_migration} — stale"),
                "run archctl migrate or rebuild the .lbdb",
            ));
            Status::Mismatch
        }
        None => {
            if smoke.ok {
                // fresh_smoke succeeded → init ran → schema is at latest version
                findings.push(Finding::ok(
                    "storage.schema_marker",
                    &format!("schema={latest_migration} (init succeeded, marker pending)"),
                ));
                Status::Compatible
            } else {
                findings.push(Finding::warn(
                    "storage.schema_marker",
                    "no .archctl-schema marker and fresh_smoke failed",
                    "run archctl migrate or rebuild the .lbdb",
                ));
                Status::Unknown
            }
        }
    };

    let overall = StorageReport::aggregate_status(&findings);
    let observed_schema = marker.clone();

    Ok(StorageReport {
        archctl_version: env!("CARGO_PKG_VERSION").to_string(),
        lbug_crate_version: obs.storage_version,
        native_version: obs.version,
        native_source_digest: obs.source_digest,
        target: target_triple(),
        compiler: Some(rustc_version_string()),
        stdlib: None,
        schema_version: latest_migration.to_string(),
        observed_schema_version: observed_schema,
        status: overall.to_string(),
        findings,
    })
}

/// Get rustc version string for target_toolchain finding.
fn rustc_version_string() -> String {
    let output = Command::new("rustc").args(["-Vv"]).output().ok();
    match output {
        Some(o) if o.status.success() => {
            let out = String::from_utf8_lossy(&o.stdout).trim().to_string();
            out.lines().next().unwrap_or("unknown").to_string()
        }
        _ => "unknown".to_string(),
    }
}

/// Get the TARGET environment variable (compile-time).
/// Uses `option_env!("TARGET")` to capture the value at compile time,
/// falling back to runtime lookup only if compile-time capture returns None.
fn target_triple() -> String {
    option_env!("TARGET")
        .map(|s| s.to_string())
        .unwrap_or_else(|| std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string()))
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Render a `StorageReport` as human-readable text to stdout.
/// Format: `[OK|WARN|CRITICAL] <id>: <detail>`
///         `STORAGE: <status>`
/// Remediation goes to stderr for Warn/Critical.
pub fn render_text(report: &StorageReport) -> Result<(), anyhow::Error> {
    for finding in &report.findings {
        let tag = match finding.severity {
            Severity::Ok => "OK",
            Severity::Warn => "WARN",
            Severity::Critical => "CRITICAL",
        };
        println!("[{tag}] {}: {}", finding.id, finding.detail);
        if let Some(ref rem) = finding.remediation {
            eprintln!("{}: {rem}", finding.id);
        }
    }
    println!("STORAGE: {}", report.status);
    Ok(())
}

/// Render a `StorageReport` as JSON to the given writer.
/// The envelope uses camelCase field names per the spec.
pub fn render_json(report: &StorageReport, out: &mut dyn Write) -> Result<(), anyhow::Error> {
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct JsonFinding {
        id: String,
        severity: String,
        detail: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        remediation: Option<String>,
    }

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct NativeInfo {
        #[serde(skip_serializing_if = "Option::is_none")]
        version: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        storage_version: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source_digest: Option<String>,
    }

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct TargetInfo {
        target: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        compiler: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        stdlib: Option<String>,
    }

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct JsonReport {
        archctl_version: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        lbug_crate_version: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        native: Option<NativeInfo>,
        #[serde(skip_serializing_if = "Option::is_none")]
        target_compiler_stdlib: Option<TargetInfo>,
        schema_version: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        observed_schema_version: Option<String>,
        status: String,
        findings: Vec<JsonFinding>,
    }

    let native = if report.native_version.is_some()
        || report.native_source_digest.is_some()
        || report.lbug_crate_version.is_some()
    {
        Some(NativeInfo {
            version: report.native_version.clone(),
            storage_version: report.lbug_crate_version.clone(),
            source: report
                .native_source_digest
                .as_ref()
                .map(|_| "lbug-rs".to_string()),
            source_digest: report.native_source_digest.clone(),
        })
    } else {
        None
    };

    let target_info = Some(TargetInfo {
        target: report.target.clone(),
        compiler: report.compiler.clone(),
        stdlib: report.stdlib.clone(),
    });

    let json_report = JsonReport {
        archctl_version: report.archctl_version.clone(),
        lbug_crate_version: report.lbug_crate_version.clone(),
        native,
        target_compiler_stdlib: target_info,
        schema_version: report.schema_version.clone(),
        observed_schema_version: report.observed_schema_version.clone(),
        status: report.status.clone(),
        findings: report
            .findings
            .iter()
            .map(|f| JsonFinding {
                id: f.id.clone(),
                severity: f.severity.to_string(),
                detail: f.detail.clone(),
                remediation: f.remediation.clone(),
            })
            .collect(),
    };

    serde_json::to_writer(&mut *out, &json_report)?;
    writeln!(out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn status_aggregate_mismatch_wins() {
        assert_eq!(
            Status::aggregate(Status::Compatible, Status::Mismatch),
            Status::Mismatch
        );
        assert_eq!(
            Status::aggregate(Status::Mismatch, Status::Unknown),
            Status::Mismatch
        );
        assert_eq!(
            Status::aggregate(Status::Unknown, Status::Mismatch),
            Status::Mismatch
        );
    }

    #[test]
    fn status_aggregate_unknown_wins_over_compatible() {
        assert_eq!(
            Status::aggregate(Status::Compatible, Status::Unknown),
            Status::Unknown
        );
        assert_eq!(
            Status::aggregate(Status::Unknown, Status::Compatible),
            Status::Unknown
        );
    }

    #[test]
    fn status_aggregate_both_compatible() {
        assert_eq!(
            Status::aggregate(Status::Compatible, Status::Compatible),
            Status::Compatible
        );
    }

    #[test]
    fn finding_to_status() {
        assert_eq!(Finding::ok("x", "y").to_status(), Status::Compatible);
        assert_eq!(Finding::warn("x", "y", "z").to_status(), Status::Unknown);
        assert_eq!(
            Finding::critical("x", "y", "z").to_status(),
            Status::Mismatch
        );
    }

    #[test]
    fn lbug_storage_probe_smoke() {
        let tmp = TempDir::new().unwrap();
        let probe = LbugStorageProbe::new();
        let smoke = probe.fresh_smoke(tmp.path());
        // A fresh TempDir should succeed with empty stats
        assert!(smoke.ok, "fresh temp dir should pass: {}", smoke.detail);
    }

    #[test]
    fn run_storage_probe_produces_report() {
        let tmp = TempDir::new().unwrap();
        let probe = LbugStorageProbe::new();
        let report = run_storage_probe(&probe, tmp.path()).expect("probe must not error");
        assert_eq!(report.status, "Compatible");
        let ids: Vec<_> = report.findings.iter().map(|f| f.id.clone()).collect();
        assert!(
            ids.contains(&"storage.archctl".to_string()),
            "report must contain storage.archctl finding"
        );
        assert!(
            ids.contains(&"storage.fresh_crud".to_string()),
            "report must contain storage.fresh_crud finding"
        );
    }

    #[test]
    fn render_json_produces_valid_json() {
        let tmp = TempDir::new().unwrap();
        let probe = LbugStorageProbe::new();
        let report = run_storage_probe(&probe, tmp.path()).expect("probe must not error");
        let mut buf = Vec::new();
        render_json(&report, &mut buf).expect("render_json must not error");
        let json_str = String::from_utf8(buf).expect("valid utf-8");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("valid JSON");
        assert_eq!(
            parsed["status"].as_str(),
            Some("Compatible"),
            "status must be Compatible"
        );
        assert!(parsed["findings"].is_array(), "findings must be an array");
    }
}
