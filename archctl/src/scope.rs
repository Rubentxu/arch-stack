//! Per-scope manifest + static gates.
//!
//! ADR-016: archctl is split into "scopes" — groups of files that
//! together implement one architectural concern. Each scope declares
//! its contract in a `manifests/<id>.toml` file. The
//! `archctl doctor --check-scope` subcommand verifies that the
//! contract holds in the code: the declared files exist, the
//! declared public symbols exist, the declared invariants hold
//! (literal-text search), and the test count is met.
//!
//! ## What this module hides
//!
//! - **The TOML parsing details.** Callers see [`ScopeManifest`];
//!   the `toml` crate is an implementation detail.
//! - **File I/O.** [`ScopeManifest::load`] uses the [`Filesystem`]
//!   port (or `SystemFilesystem` in production).
//!
//! ## What this module does NOT hide
//!
//! - **The gate semantics.** Each gate is a separate function so
//!   they can be tested individually. Callers compose them.
//! - **Failure reporting.** Gates return [`ScopeCheckReport`] with
//!   every violation listed — callers decide how to render.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Path of the manifests directory, relative to the project root.
pub const MANIFESTS_DIR: &str = "manifests";

/// One scope's contract.
///
/// TOML layout is deliberately flat: the manifest is a list of
/// top-level keys (`id`, `version`, `description`, `editable`, …)
/// rather than nested under `[scope]`. The flat layout makes
/// individual fields greppable in CI logs and code review tools,
/// and avoids serde's quirks with nested TOML tables under dotted
/// keys.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct ScopeManifest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,

    #[serde(default, alias = "editable")]
    pub editable_files: Vec<String>,
    #[serde(default)]
    pub public_symbols: Vec<String>,
    #[serde(default)]
    pub must_hold: Vec<String>,
    #[serde(default, alias = "minimum_count")]
    pub minimum_tests: u32,
    /// Optional sub-directory containing the cargo crate, relative
    /// to `project_root`. Default `.` (Cargo.toml sits next to
    /// `manifests/`). For monorepos where the manifests live at the
    /// workspace root, set this to the crate path
    /// (e.g. `"archctl"`).
    #[serde(default)]
    pub cargo_dir: Option<String>,
}



impl ScopeManifest {
    /// Load a single manifest by its scope id (without extension).
    /// Looks under `<project_root>/manifests/<id>.toml`.
    pub fn load(project_root: &Path, scope_id: &str) -> Result<Self> {
        let path = project_root.join(MANIFESTS_DIR).join(format!("{scope_id}.toml"));
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("read manifest {}", path.display()))?;
        let manifest: ScopeManifest = toml::from_str(&text)
            .with_context(|| format!("parse manifest {}", path.display()))?;
        Ok(manifest)
    }

    /// Load every `*.toml` file under `manifests/` and return the
    /// parsed manifests. Files that fail to parse are surfaced as
    /// errors (the doctor should report them, not silently skip).
    pub fn load_all(project_root: &Path) -> Result<Vec<(String, Self)>> {
        let dir = project_root.join(MANIFESTS_DIR);
        let entries = std::fs::read_dir(&dir)
            .with_context(|| format!("read directory {}", dir.display()))?;
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| anyhow::anyhow!("manifest without stem: {}", path.display()))?
                .to_string();
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("read manifest {}", path.display()))?;
            let manifest: ScopeManifest = toml::from_str(&text)
                .with_context(|| format!("parse manifest {}", path.display()))?;
            out.push((id, manifest));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }
}

/// Outcome of running the gates against one scope.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScopeCheckReport {
    pub scope_id: String,
    pub findings: Vec<ScopeFinding>,
}

impl ScopeCheckReport {
    fn ok(scope_id: impl Into<String>) -> Self {
        Self {
            scope_id: scope_id.into(),
            findings: Vec::new(),
        }
    }

    fn failing(scope_id: impl Into<String>, findings: Vec<ScopeFinding>) -> Self {
        Self {
            scope_id: scope_id.into(),
            findings,
        }
    }

    /// True iff no finding has `Severity::Fail`.
    pub fn passed(&self) -> bool {
        !self.findings.iter().any(|f| matches!(f.severity, ScopeSeverity::Fail))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeFinding {
    pub gate: ScopeGate,
    pub message: String,
    pub severity: ScopeSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeSeverity {
    /// A hard failure — the contract is violated. The gate must
    /// not pass.
    Fail,
    /// A soft warning — informational, not blocking. Reserved for
    /// future use; today every finding is `Fail`.
    Warn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScopeGate {
    /// `editable_files` from the manifest must exist in the
    /// project tree.
    EditableFilesExist,
    /// `public_symbols` from the manifest must be defined as `pub`
    /// in at least one of the scope's editable files.
    PublicSymbolsExist,
    /// `must_hold` invariants must appear as literal substrings
    /// somewhere in the scope's editable files.
    MustHoldInvariantsHold,
    /// The workspace must report at least `minimum_count` passing
    /// tests when `cargo test` runs.
    TestCountMeetsMinimum,
}

impl ScopeGate {
    pub fn name(self) -> &'static str {
        match self {
            ScopeGate::EditableFilesExist => "editable_files_exist",
            ScopeGate::PublicSymbolsExist => "public_symbols_exist",
            ScopeGate::MustHoldInvariantsHold => "must_hold_invariants",
            ScopeGate::TestCountMeetsMinimum => "test_count",
        }
    }
}

// ---------------------------------------------------------------------------
// Gates — each one is a separate function so tests can target them
// individually. The composition is in `check_scope` below.
// ---------------------------------------------------------------------------

/// Gate 1: every editable file declared in the manifest must exist
/// on disk. Missing files are a hard failure (they cannot implement
/// any contract).
pub fn gate_editable_files_exist(
    project_root: &Path,
    manifest: &ScopeManifest,
) -> Vec<ScopeFinding> {
    let mut findings = Vec::new();
    for path_str in &manifest.editable_files {
        let path = project_root.join(path_str);
        if !path.exists() {
            findings.push(ScopeFinding {
                gate: ScopeGate::EditableFilesExist,
                message: format!(
                    "editable file declared in manifest does not exist: {}",
                    path.display()
                ),
                severity: ScopeSeverity::Fail,
            });
        }
    }
    findings
}

/// Gate 2: every declared public symbol must exist as a `pub` item
/// in at least one of the scope's editable files.
///
/// We accept any of these textual patterns (literal substring, then
/// word-boundary check via the trailing boundary):
///   - `pub struct <Symbol>`
///   - `pub trait <Symbol>`
///   - `pub enum <Symbol>`
///   - `pub fn <Symbol>`
///   - `pub use ... <Symbol>` (re-export)
///   - `<Symbol>` appearing on a line that starts with `pub ` (catches
///     `pub const`, `pub static`, `pub type`)
///
/// This is deliberately a heuristic — exact semantic verification
/// would require `cargo rustdoc -- --output-format json` and the
/// payoff is not worth the build-time cost for a static gate.
pub fn gate_public_symbols_exist(
    project_root: &Path,
    manifest: &ScopeManifest,
) -> Vec<ScopeFinding> {
    let mut findings = Vec::new();
    let mut found_anywhere: std::collections::HashMap<&str, bool> = manifest
        .public_symbols
        .iter()
        .map(|s| (s.as_str(), false))
        .collect();
    for path_str in &manifest.editable_files {
        let Ok(text) = std::fs::read_to_string(project_root.join(path_str)) else {
            continue;
        };
        for symbol in manifest.public_symbols.iter().map(|s| s.as_str()) {
            if symbol_visible(&text, symbol) {
                found_anywhere.insert(symbol, true);
            }
        }
    }
    for (symbol, found) in &found_anywhere {
        if !found {
            findings.push(ScopeFinding {
                gate: ScopeGate::PublicSymbolsExist,
                message: format!(
                    "declared public symbol `{symbol}` not found as `pub` item \
                     in any of the scope's editable files",
                ),
                severity: ScopeSeverity::Fail,
            });
        }
    }
    findings
}

/// Returns true if `symbol` appears in `text` as a public item.
///
/// Public items in Rust look like `pub <keyword> <Symbol>` (where
/// `<keyword>` is `struct`, `trait`, `enum`, `fn`, `const`, `static`,
/// `type`, `use`), or `<Symbol>` after `pub use` on the same line. We
/// check word boundaries so `Evidence` does not match `EvidenceKind`.
fn symbol_visible(text: &str, symbol: &str) -> bool {
    let patterns = [
        format!("pub struct {symbol}"),
        format!("pub trait {symbol}"),
        format!("pub enum {symbol}"),
        format!("pub fn {symbol}"),
        format!("pub const {symbol}"),
        format!("pub static {symbol}"),
        format!("pub type {symbol}"),
    ];
    for p in &patterns {
        if contains_word(text, p) {
            return true;
        }
    }
    // `pub use foo::bar::{Symbol, Other}` or `pub use foo::Symbol;`
    for line in text.lines() {
        let trimmed = line.trim_start();
        let after_pub = trimmed
            .strip_prefix("pub use ")
            .or_else(|| trimmed.strip_prefix("pub const "))
            .or_else(|| trimmed.strip_prefix("pub static "))
            .or_else(|| trimmed.strip_prefix("pub type "));
        if let Some(rest) = after_pub {
            // Inside the line, find Symbol as a whole word.
            if contains_word(rest, symbol) {
                return true;
            }
        }
    }
    false
}

/// Does `text` contain `needle` as a whole word? Whole-word means the
/// character before and after the match is not an identifier
/// continuation (alphanumeric or `_`).
fn contains_word(text: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let mut start = 0;
    while let Some(idx) = text[start..].find(needle) {
        let abs = start + idx;
        let before_ok = abs == 0 || !is_ident_cont(text.as_bytes()[abs - 1]);
        let after = abs + needle.len();
        let after_ok = after >= text.len() || !is_ident_cont(text.as_bytes()[after]);
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

fn is_ident_cont(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Gate 3: each `must_hold` invariant must appear as a literal
/// substring in at least one of the scope's editable files. This
/// catches the common case where the contract is asserted in a
/// comment near the code that maintains it, and the gate verifies
/// the comment matches reality.
pub fn gate_must_hold_invariants(
    project_root: &Path,
    manifest: &ScopeManifest,
) -> Vec<ScopeFinding> {
    let mut findings = Vec::new();
    let all_text: String = manifest
        .editable_files
        .iter()
        .filter_map(|p| std::fs::read_to_string(project_root.join(p)).ok())
        .collect::<Vec<_>>()
        .join("\n");
    for invariant in &manifest.must_hold {
        if !all_text.contains(invariant) {
            findings.push(ScopeFinding {
                gate: ScopeGate::MustHoldInvariantsHold,
                message: format!(
                    "invariant not found in scope source: {invariant:?}"
                ),
                severity: ScopeSeverity::Fail,
            });
        }
    }
    findings
}

/// Gate 4: the workspace's `cargo test` output must report at least
/// `minimum_count` passing tests. The gate shells out to `cargo
/// test` (no in-process test runner — that would re-implement
/// cargo's test selection logic and miss doctests, integration
/// tests, etc.). The implementation is gated on `cargo` being on
/// `$PATH`; if it is missing, the gate returns a hard failure with
/// a message explaining how to install it.
///
/// Output is parsed with a regex over `cargo test`'s summary line.
/// `cargo test` prints lines like:
///   `test result: ok. 69 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`
/// Anything else means the run did not complete cleanly.
pub fn gate_test_count_meets_minimum(
    project_root: &Path,
    manifest: &ScopeManifest,
) -> Vec<ScopeFinding> {
    if manifest.minimum_tests == 0 {
        return Vec::new();
    }
    let cargo_dir = match resolve_cargo_dir(project_root, manifest) {
        Ok(d) => d,
        Err(msg) => {
            return vec![ScopeFinding {
                gate: ScopeGate::TestCountMeetsMinimum,
                message: msg,
                severity: ScopeSeverity::Fail,
            }];
        }
    };
    let output = match std::process::Command::new("cargo")
        .arg("test")
        .arg("--quiet")
        .arg("--no-fail-fast")
        .current_dir(cargo_dir)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            return vec![ScopeFinding {
                gate: ScopeGate::TestCountMeetsMinimum,
                message: format!(
                    "could not run `cargo test`: {e}. Install cargo and ensure it is on $PATH."
                ),
                severity: ScopeSeverity::Fail,
            }];
        }
    };
    if !output.status.success() {
        return vec![ScopeFinding {
            gate: ScopeGate::TestCountMeetsMinimum,
            message: "`cargo test` did not exit cleanly — the test gate cannot read \
                     a pass count from a failing run. Fix the tests first."
                .to_string(),
            severity: ScopeSeverity::Fail,
        }];
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let passed = parse_test_pass_count(&stdout);
    match passed {
        Some(n) if n >= manifest.minimum_tests as u64 => Vec::new(),
        Some(n) => vec![ScopeFinding {
            gate: ScopeGate::TestCountMeetsMinimum,
            message: format!(
                "test count below minimum: got {n}, required {}",
                manifest.minimum_tests
            ),
            severity: ScopeSeverity::Fail,
        }],
        None => vec![ScopeFinding {
            gate: ScopeGate::TestCountMeetsMinimum,
            message: "could not parse test pass count from `cargo test` output".to_string(),
            severity: ScopeSeverity::Fail,
        }],
    }
}

/// Resolve the directory where `cargo test` should run for `manifest`.
/// If the manifest declares `cargo_dir`, that sub-directory of
/// `project_root` is used (must contain a `Cargo.toml`). Otherwise
/// the doctor walks up looking for the closest `Cargo.toml`.
fn resolve_cargo_dir(
    project_root: &Path,
    manifest: &ScopeManifest,
) -> Result<std::path::PathBuf, String> {
    if let Some(rel) = &manifest.cargo_dir {
        let abs = project_root.join(rel);
        if !abs.join("Cargo.toml").is_file() {
            return Err(format!(
                "manifest declares `cargo_dir = {rel:?}` but no Cargo.toml exists at {path}",
                path = abs.display()
            ));
        }
        return Ok(abs);
    }
    match find_cargo_root(project_root) {
        Some(d) => Ok(d),
        None => Err(format!(
            "could not find a Cargo.toml at {path} or any parent directory — \
             the test gate requires a cargo project. Set `cargo_dir` in the manifest \
             if the crate lives in a sub-directory.",
            path = project_root.display()
        )),
    }
}

/// Walk up from `start` looking for a `Cargo.toml`. Returns the
/// directory containing it, or `None` if no Cargo.toml was found
/// before reaching the filesystem root.
fn find_cargo_root(start: &Path) -> Option<std::path::PathBuf> {
    let mut cur: Option<&Path> = Some(start);
    while let Some(dir) = cur {
        if dir.join("Cargo.toml").is_file() {
            return Some(dir.to_path_buf());
        }
        cur = dir.parent();
    }
    None
}

/// Parse the `test result: ok. <N> passed` lines out of
/// `cargo test`'s stdout and return the **sum** of all `passed`
/// counts. `cargo test` emits one summary per test binary plus
/// one for the doctests, so a single-line parser would always
/// read the integration-test or doctest bin last. Summing is the
/// honest signal: a scope's `minimum_tests` is the floor for
/// the total passing tests across the crate.
fn parse_test_pass_count(stdout: &str) -> Option<u64> {
    let mut total: u64 = 0;
    let mut seen_any = false;
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("test result: ok.") {
            // rest looks like " 69 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
            let mut tokens = rest.split_whitespace();
            if let Some(count) = tokens.next() {
                if let Ok(n) = count.parse::<u64>() {
                    total += n;
                    seen_any = true;
                }
            }
        }
    }
    if seen_any {
        Some(total)
    } else {
        None
    }
}

/// Run every gate for a single scope. The test-count gate is
/// skipped if `include_test_count` is false (so unit tests for the
/// gates themselves do not recurse).
pub fn check_scope(
    project_root: &Path,
    manifest: &ScopeManifest,
    include_test_count: bool,
) -> ScopeCheckReport {
    let scope_id = manifest.id.clone();
    let mut findings = Vec::new();
    findings.extend(gate_editable_files_exist(project_root, manifest));
    findings.extend(gate_public_symbols_exist(project_root, manifest));
    findings.extend(gate_must_hold_invariants(project_root, manifest));
    if include_test_count {
        findings.extend(gate_test_count_meets_minimum(project_root, manifest));
    }
    if findings.is_empty() {
        ScopeCheckReport::ok(scope_id)
    } else {
        ScopeCheckReport::failing(scope_id, findings)
    }
}

/// Run the gates against every scope under `manifests/`.
pub fn check_all_scopes(project_root: &Path) -> Result<Vec<ScopeCheckReport>> {
    let manifests = ScopeManifest::load_all(project_root)?;
    let mut reports = Vec::new();
    for (_id, manifest) in &manifests {
        reports.push(check_scope(project_root, manifest, true));
    }
    Ok(reports)
}

// ---------------------------------------------------------------------------
// `doctor --check-scope` integration
// ---------------------------------------------------------------------------

/// Render a single scope report as a single-line summary suitable
/// for `--json` output. The full findings list is in
/// [`ScopeCheckReport::findings`] for callers that want detail.
pub fn render_report_line(report: &ScopeCheckReport) -> String {
    let status = if report.passed() { "OK  " } else { "FAIL" };
    let n_findings = report.findings.len();
    format!(
        "[{status}] scope {} ({} finding{})",
        report.scope_id,
        n_findings,
        if n_findings == 1 { "" } else { "s" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn fixture() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn write_manifest(dir: &Path, name: &str, body: &str) -> PathBuf {
        let manifests = dir.join(MANIFESTS_DIR);
        std::fs::create_dir_all(&manifests).unwrap();
        let path = manifests.join(format!("{name}.toml"));
        std::fs::write(&path, body).unwrap();
        path
    }

    fn make_source_file(dir: &Path, rel_path: &str, body: &str) -> PathBuf {
        let path = dir.join(rel_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn load_parses_minimal_manifest() {
        let tmp = fixture();
        write_manifest(
            tmp.path(),
            "demo",
            r#"
id = "demo"
version = "0.1.0"
description = "for tests"
"#,
        );
        let m = ScopeManifest::load(tmp.path(), "demo").unwrap();
        assert_eq!(m.id, "demo");
        assert_eq!(m.version, "0.1.0");
        assert!(m.editable_files.is_empty());
        assert!(m.public_symbols.is_empty());
        assert!(m.must_hold.is_empty());
        assert_eq!(m.minimum_tests, 0);
    }

    #[test]
    fn load_full_manifest_round_trips() {
        let tmp = fixture();
        let body = r#"
id = "M3"
version = "0.1.0"
description = "evidence pipeline"
editable = ["archctl/src/evidence.rs", "archctl/src/tsg.rs"]
public_symbols = ["Evidence", "GraphStore"]
must_hold = ["does not call std::fs directly"]
minimum_tests = 60
"#;
        write_manifest(tmp.path(), "M3", body);
        let raw = std::fs::read_to_string(tmp.path().join("manifests/M3.toml")).unwrap();
        eprintln!("DEBUG raw=\n{raw}");
        let m = ScopeManifest::load(tmp.path(), "M3").unwrap();
        eprintln!("DEBUG m={:#?}", m);
        assert_eq!(m.editable_files.len(), 2);
        assert_eq!(m.public_symbols, vec!["Evidence", "GraphStore"]);
        assert_eq!(m.must_hold.len(), 1);
        assert_eq!(m.minimum_tests, 60);
        // round-trip via serde_json (not TOML, since toml re-emission
        // may reformat comments). We just check structural equality.
        let json = serde_json::to_string(&m).unwrap();
        let back: ScopeManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn load_all_returns_every_manifest_sorted() {
        let tmp = fixture();
        write_manifest(tmp.path(), "zeta", "id=\"zeta\"\nversion=\"0.1.0\"\ndescription=\"z\"\n");
        write_manifest(tmp.path(), "alpha", "id=\"alpha\"\nversion=\"0.1.0\"\ndescription=\"a\"\n");
        write_manifest(tmp.path(), "mid", "id=\"mid\"\nversion=\"0.1.0\"\ndescription=\"m\"\n");
        let all = ScopeManifest::load_all(tmp.path()).unwrap();
        let ids: Vec<&str> = all.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(ids, vec!["alpha", "mid", "zeta"]);
    }

    #[test]
    fn gate_editable_files_passes_when_all_exist() {
        let tmp = fixture();
        make_source_file(tmp.path(), "archctl/src/a.rs", "");
        make_source_file(tmp.path(), "archctl/src/b.rs", "");
        let m = ScopeManifest {
            id: "demo".into(),
            version: "0.1.0".into(),
            description: String::new(),
            editable_files: vec!["archctl/src/a.rs".into(), "archctl/src/b.rs".into()],
            ..Default::default()
        };
        assert!(gate_editable_files_exist(tmp.path(), &m).is_empty());
    }

    #[test]
    fn gate_editable_files_fails_when_one_missing() {
        let tmp = fixture();
        make_source_file(tmp.path(), "a.rs", "");
        let m = ScopeManifest {
            id: "demo".into(),
            version: "0.1.0".into(),
            description: String::new(),
            editable_files: vec!["a.rs".into(), "missing.rs".into()],
            ..Default::default()
        };
        let f = gate_editable_files_exist(tmp.path(), &m);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].gate, ScopeGate::EditableFilesExist);
        assert!(f[0].message.contains("missing.rs"));
    }

    #[test]
    fn gate_public_symbols_passes_when_all_found() {
        let tmp = fixture();
        make_source_file(
            tmp.path(),
            "lib.rs",
            "pub struct Evidence;\npub trait GraphStore {}\n",
        );
        let m = ScopeManifest {
            id: "demo".into(),
            version: "0.1.0".into(),
            description: String::new(),
            editable_files: vec!["lib.rs".into()],
            public_symbols: vec!["Evidence".into(), "GraphStore".into()],
            ..Default::default()
        };
        assert!(gate_public_symbols_exist(tmp.path(), &m).is_empty());
    }

    #[test]
    fn gate_public_symbols_fails_when_missing() {
        let tmp = fixture();
        make_source_file(tmp.path(), "lib.rs", "pub struct Evidence;\n");
        let m = ScopeManifest {
            id: "demo".into(),
            version: "0.1.0".into(),
            description: String::new(),
            editable_files: vec!["lib.rs".into()],
            public_symbols: vec!["Evidence".into(), "NonExistent".into()],
            ..Default::default()
        };
        let f = gate_public_symbols_exist(tmp.path(), &m);
        assert_eq!(f.len(), 1);
        assert!(f[0].message.contains("NonExistent"));
    }

    #[test]
    fn gate_must_hold_passes_when_text_present() {
        let tmp = fixture();
        make_source_file(
            tmp.path(),
            "lib.rs",
            "// this module does not call std::fs directly",
        );
        let m = ScopeManifest {
            id: "demo".into(),
            version: "0.1.0".into(),
            description: String::new(),
            editable_files: vec!["lib.rs".into()],
            must_hold: vec!["does not call std::fs directly".into()],
            ..Default::default()
        };
        assert!(gate_must_hold_invariants(tmp.path(), &m).is_empty());
    }

    #[test]
    fn gate_must_hold_fails_when_text_missing() {
        let tmp = fixture();
        make_source_file(tmp.path(), "lib.rs", "pub fn x() {}");
        let m = ScopeManifest {
            id: "demo".into(),
            version: "0.1.0".into(),
            description: String::new(),
            editable_files: vec!["lib.rs".into()],
            must_hold: vec!["does not call std::fs directly".into()],
            ..Default::default()
        };
        let f = gate_must_hold_invariants(tmp.path(), &m);
        assert_eq!(f.len(), 1);
        assert!(f[0].message.contains("does not call std::fs directly"));
    }

    #[test]
    fn parse_test_pass_count_sums_all_summaries() {
        // `cargo test` emits one summary per test binary. A scope's
        // `minimum_tests` is a floor on the total passing tests
        // across the crate, so the parser sums all summary lines.
        let stdout = r#"
running 5 tests
test result: ok. 5 passed; 0 failed

running 10 tests
test result: ok. 10 passed; 0 failed
"#;
        assert_eq!(parse_test_pass_count(stdout), Some(15));
    }

    #[test]
    fn parse_test_pass_count_returns_none_when_no_summary() {
        assert_eq!(parse_test_pass_count("compiling foo\n"), None);
    }

    #[test]
    fn check_scope_returns_ok_when_all_gates_pass() {
        let tmp = fixture();
        make_source_file(tmp.path(), "a.rs", "pub fn x() {}\n");
        let body = format!(
            r#"
id = "demo"
version = "0.1.0"
description = "x"
editable = ["a.rs"]
public_symbols = ["x"]
must_hold = ["pub fn x"]
minimum_tests = 0
"#
        );
        write_manifest(tmp.path(), "demo", &body);
        let m = ScopeManifest::load(tmp.path(), "demo").unwrap();
        // skip test-count gate to avoid recursing into `cargo test`
        let r = check_scope(tmp.path(), &m, false);
        assert!(r.passed(), "expected pass, got findings: {:#?}", r.findings);
    }

    #[test]
    fn check_scope_aggregates_findings_across_gates() {
        let tmp = fixture();
        let body = r#"
id = "demo"
version = "0.1.0"
description = "x"
editable = ["nonexistent.rs"]
public_symbols = ["MissingSymbol"]
must_hold = ["nonexistent invariant text"]
minimum_tests = 0
"#;
        write_manifest(tmp.path(), "demo", body);
        let m = ScopeManifest::load(tmp.path(), "demo").unwrap();
        let r = check_scope(tmp.path(), &m, false);
        assert!(!r.passed());
        // 3 gates fire: editable_files, public_symbols, must_hold.
        assert_eq!(r.findings.len(), 3);
        let gates: std::collections::BTreeSet<_> = r
            .findings
            .iter()
            .map(|f| f.gate)
            .collect();
        assert!(gates.contains(&ScopeGate::EditableFilesExist));
        assert!(gates.contains(&ScopeGate::PublicSymbolsExist));
        assert!(gates.contains(&ScopeGate::MustHoldInvariantsHold));
    }

    #[test]
    fn render_report_line_is_stable() {
        let mut report = ScopeCheckReport::ok("demo");
        assert_eq!(render_report_line(&report), "[OK  ] scope demo (0 findings)");
        report.findings.push(ScopeFinding {
            gate: ScopeGate::EditableFilesExist,
            message: "missing".into(),
            severity: ScopeSeverity::Fail,
        });
        assert_eq!(render_report_line(&report), "[FAIL] scope demo (1 finding)");
    }

    #[test]
    fn check_all_scopes_handles_empty_dir() {
        let tmp = fixture();
        std::fs::create_dir_all(tmp.path().join(MANIFESTS_DIR)).unwrap();
        let reports = check_all_scopes(tmp.path()).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn scope_manifest_default_via_derive() {
        // Verify that omitted sections produce sensible defaults
        // via #[serde(default)] at the field level on the parent.
        let m: ScopeManifest = toml::from_str(
            r#"
id = "x"
version = "0.1.0"
description = "y"
"#,
        )
        .unwrap();
        assert_eq!(m.editable_files, Vec::<String>::new());
        assert_eq!(m.public_symbols, Vec::<String>::new());
        assert_eq!(m.must_hold, Vec::<String>::new());
        assert_eq!(m.minimum_tests, 0);
    }

    #[test]
    fn parse_error_surfaces_clearly() {
        let tmp = fixture();
        write_manifest(tmp.path(), "broken", "this is not valid toml\n");
        let err = ScopeManifest::load(tmp.path(), "broken").unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("parse manifest") || msg.contains("broken"),
            "error chain should mention the manifest path: {msg}"
        );
    }

    // smoke: gate name is human-readable
    #[test]
    fn gate_names_are_stable_strings() {
        assert_eq!(ScopeGate::EditableFilesExist.name(), "editable_files_exist");
        assert_eq!(ScopeGate::PublicSymbolsExist.name(), "public_symbols_exist");
        assert_eq!(ScopeGate::MustHoldInvariantsHold.name(), "must_hold_invariants");
        assert_eq!(ScopeGate::TestCountMeetsMinimum.name(), "test_count");
        // also: used as a BTreeMap key somewhere later
        let mut m: BTreeMap<ScopeGate, u32> = BTreeMap::new();
        m.insert(ScopeGate::EditableFilesExist, 1);
        assert_eq!(m[&ScopeGate::EditableFilesExist], 1);
    }
}
