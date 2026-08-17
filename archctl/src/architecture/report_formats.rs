//! Presentation module — projects `PolicyReport` into SARIF 2.1.0 and JUnit XML.
//!
//! These projectors are pure: no I/O, no store access, no side effects.

use crate::architecture::policy::{PolicyReport, Severity};
use serde::Serialize;

// ─────────────────────────────────────────────────────────────────────────────
// SARIF 2.1.0 minimal types
// ─────────────────────────────────────────────────────────────────────────────

/// SARIF 2.1.0 log root.
#[derive(Serialize)]
pub struct SarifLog {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub runs: Vec<SarifRun>,
}

/// One run — we emit one run per policy evaluation.
#[derive(Serialize)]
pub struct SarifRun {
    pub tool: SarifTool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub results: Vec<SarifResult>,
}

/// Tool driver descriptor.
#[derive(Serialize)]
pub struct SarifTool {
    pub driver: SarifDriver,
}

/// Driver identity (archctl itself).
#[derive(Serialize)]
pub struct SarifDriver {
    pub name: String,
    pub version: String,
    #[serde(rename = "informationUri")]
    pub information_uri: String,
}

/// A single SARIF result (one policy violation).
#[derive(Serialize)]
pub struct SarifResult {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub level: String,
    pub message: SarifMessage,
    pub locations: Vec<SarifLocation>,
}

/// Human-readable message text.
#[derive(Serialize)]
pub struct SarifMessage {
    pub text: String,
}

/// Location pointing at the violating subject.
#[derive(Serialize)]
pub struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    pub physical_location: SarifPhysicalLocation,
}

/// Physical location (file/artifact).
#[derive(Serialize)]
pub struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    pub artifact_location: SarifArtifactLocation,
}

/// Artifact URI — we use `archctl://graph/<subject.id>` as a virtual URI.
#[derive(Serialize)]
pub struct SarifArtifactLocation {
    pub uri: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Project a `PolicyReport` into a SARIF 2.1.0 `SarifLog`.
///
/// Severity mapping:
/// - `Error` → `"error"`
/// - `Warning` → `"warning"`
/// - `Info` → `"note"`
///
/// Each violation becomes one `SarifResult` with:
/// - `ruleId` = violation rule name
/// - `level` = mapped severity
/// - `message.text` = violation message
/// - `locations[0].physicalLocation.artifactLocation.uri` = `archctl://graph/<subject.id>`
pub fn to_sarif(report: &PolicyReport) -> SarifLog {
    let results: Vec<SarifResult> = report
        .violations
        .iter()
        .map(|v| {
            let level = match v.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "note",
            };
            SarifResult {
                rule_id: v.rule.clone(),
                level: level.to_string(),
                message: SarifMessage {
                    text: v.message.clone(),
                },
                locations: vec![SarifLocation {
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation {
                            uri: format!("archctl://graph/{}", v.subject.id),
                        },
                    },
                }],
            }
        })
        .collect();

    SarifLog {
        schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json".to_string(),
        version: "2.1.0".to_string(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: "archctl".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    information_uri: "https://github.com/Rubentxu/arch-stack".to_string(),
                },
            },
            results,
        }],
    }
}

/// Project a `PolicyReport` into a JUnit XML string.
///
/// Root `<testsuites tests=N failures=F skipped=S name="archctl-policy">`.
///
/// One `<testsuite name="archctl-policy">` containing per-violation `<testcase>`:
/// - Error / Warning → `<failure type="error|warning" message="...">`
/// - Info → `<skipped/>`
///
/// `tests` = total violations, `failures` = error+warning count,
/// `skipped` = info count.
pub fn to_junit_xml(report: &PolicyReport) -> String {
    let errors = report
        .violations
        .iter()
        .filter(|v| v.severity == Severity::Error || v.severity == Severity::Warning)
        .count();
    let skipped = report
        .violations
        .iter()
        .filter(|v| v.severity == Severity::Info)
        .count();
    let tests = report.violations.len();

    let mut s = String::new();
    s.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    s.push('\n');
    s.push_str(&format!(
        r#"<testsuites tests="{}" failures="{}" skipped="{}" name="archctl-policy">"#,
        tests, errors, skipped
    ));
    s.push('\n');

    // One testsuite wrapper (matches the typical JUnit consumer expectation);
    // inner counts mirror the root attributes (single-suite report).
    s.push_str(&format!(
        r#"  <testsuite name="archctl-policy" tests="{tests}" failures="{errors}" skipped="{skipped}">"#
    ));
    s.push('\n');

    for v in &report.violations {
        let (_level_tag, level_attr) = match v.severity {
            Severity::Error => ("failure", "error"),
            Severity::Warning => ("failure", "warning"),
            Severity::Info => ("skipped", "info"),
        };

        let escaped_msg = xml_escape(&v.message);
        let escaped_rule = xml_escape(&v.rule);
        let escaped_id = xml_escape(&v.subject.id);

        match v.severity {
            Severity::Error | Severity::Warning => {
                s.push_str(&format!(
                    r#"    <testcase name="{}" classname="{}"><failure type="{}" message="{}"/></testcase>"#,
                    escaped_id, escaped_rule, level_attr, escaped_msg
                ));
            }
            Severity::Info => {
                s.push_str(&format!(
                    r#"    <testcase name="{}" classname="{}"><skipped/></testcase>"#,
                    escaped_id, escaped_rule
                ));
            }
        }
        s.push('\n');
    }

    s.push_str("  </testsuite>\n");
    s.push_str("</testsuites>\n");

    s
}

/// Escape characters that are illegal inside an XML attribute or text node.
///
/// Handles: `& < > " '`
fn xml_escape(s: &str) -> String {
    let mut r = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => r.push_str("&amp;"),
            '<' => r.push_str("&lt;"),
            '>' => r.push_str("&gt;"),
            '"' => r.push_str("&quot;"),
            '\'' => r.push_str("&apos;"),
            c => r.push(c),
        }
    }
    r
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::architecture::policy::{
        PolicyParams, PolicyReport, PolicySubject, PolicySummary, Severity,
    };
    use chrono::Utc;

    fn make_report(violations: Vec<(Severity, &str, &str)>) -> PolicyReport {
        let violations = violations
            .into_iter()
            .map(
                |(severity, rule, subject_id)| crate::architecture::policy::Violation {
                    rule: rule.to_string(),
                    severity,
                    subject: PolicySubject {
                        id: subject_id.to_string(),
                        kind: "element".to_string(),
                    },
                    params: PolicyParams::Dependency {
                        target: "c4:container:b".to_string(),
                    },
                    message: format!("{} violation", rule),
                },
            )
            .collect();

        PolicyReport {
            schema_version: "1.0".to_string(),
            capability: "architecture-policy-mvp".to_string(),
            policy_id: "policy.json".to_string(),
            evaluated_at: Utc::now(),
            violations,
            waivers: vec![],
            summary: PolicySummary {
                total: 3,
                passed: 0,
                failed: 3,
                waived: 0,
                fail_on: "error".to_string(),
            },
            warnings: vec![],
        }
    }

    #[test]
    fn sarif_severity_mapping() {
        let report = make_report(vec![
            (Severity::Error, "forbid_dependency", "c4:container:a"),
            (Severity::Warning, "max_fanout", "c4:container:b"),
            (Severity::Info, "evidence_required", "c4:container:c"),
        ]);
        let log = to_sarif(&report);
        let levels: Vec<&str> = log.runs[0]
            .results
            .iter()
            .map(|r| r.level.as_str())
            .collect();
        assert_eq!(levels, vec!["error", "warning", "note"]);
    }

    #[test]
    fn sarif_empty_report() {
        let report = make_report(vec![]);
        let log = to_sarif(&report);
        assert!(log.runs[0].results.is_empty());
        assert_eq!(log.version, "2.1.0");
        assert_eq!(
            log.schema,
            "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json"
        );
    }

    #[test]
    fn sarif_uri_contains_subject_id() {
        let report = make_report(vec![(
            Severity::Error,
            "forbid_dependency",
            "c4:container:x",
        )]);
        let log = to_sarif(&report);
        let uri = &log.runs[0].results[0].locations[0]
            .physical_location
            .artifact_location
            .uri;
        assert!(uri.contains("c4:container:x"), "uri = {uri}");
    }

    #[test]
    fn junit_escapes_special_chars() {
        let report = make_report(vec![(
            Severity::Error,
            "forbid_dependency",
            "c4:container:a",
        )]);
        let report = PolicyReport {
            violations: vec![crate::architecture::policy::Violation {
                rule: "forbid_dependency".to_string(),
                severity: Severity::Error,
                subject: PolicySubject {
                    id: "c4:container:a".to_string(),
                    kind: "element".to_string(),
                },
                params: PolicyParams::Dependency {
                    target: "c4:container:b".to_string(),
                },
                message: r#"a<b>&c"d'e"#.to_string(),
            }],
            ..report
        };
        let xml = to_junit_xml(&report);
        assert!(xml.contains("&lt;"), "should escape <");
        assert!(xml.contains("&gt;"), "should escape >");
        assert!(xml.contains("&amp;"), "should escape &");
        assert!(xml.contains("&quot;"), "should escape double quote");
        assert!(xml.contains("&apos;"), "should escape single quote");
        assert!(!xml.contains("<b>"), "raw <b> must not appear");
        assert!(
            !xml.contains("&c\"d"),
            "raw unescaped chars must not appear"
        );
    }

    #[test]
    fn junit_counts() {
        // 2 errors + 1 warning + 1 info = 4 tests, 3 failures, 1 skipped
        let report = make_report(vec![
            (Severity::Error, "forbid_dependency", "a"),
            (Severity::Error, "forbid_cycle", "b"),
            (Severity::Warning, "max_fanout", "c"),
            (Severity::Info, "evidence_required", "d"),
        ]);
        let xml = to_junit_xml(&report);
        assert!(xml.contains(r#"tests="4""#), "xml = {xml}");
        assert!(xml.contains(r#"failures="3""#), "xml = {xml}");
        assert!(xml.contains(r#"skipped="1""#), "xml = {xml}");
    }

    /// Regression: the inner <testsuite> must carry the same counts as the
    /// root <testsuites> — some CI consumers (GitLab, Azure Pipelines) read
    /// only the inner element and saw empty counters.
    #[test]
    fn junit_inner_testsuite_carries_counts() {
        let report = make_report(vec![
            (Severity::Error, "forbid_dependency", "a"),
            (Severity::Info, "evidence_required", "d"),
        ]);
        let xml = to_junit_xml(&report);
        assert!(
            xml.contains(r#"<testsuite name="archctl-policy" tests="2" failures="1" skipped="1">"#),
            "inner testsuite must carry counts; xml = {xml}"
        );
    }

    #[test]
    fn junit_valid_xml_header() {
        let report = make_report(vec![]);
        let xml = to_junit_xml(&report);
        assert!(
            xml.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#),
            "xml = {xml}"
        );
    }

    #[test]
    fn junit_info_violation_becomes_skipped() {
        let report = make_report(vec![(
            Severity::Info,
            "evidence_required",
            "c4:container:x",
        )]);
        let xml = to_junit_xml(&report);
        assert!(
            xml.contains("<skipped/>"),
            "info should be <skipped/>: {xml}"
        );
        assert!(
            !xml.contains("<failure"),
            "info must not be a <failure>: {xml}"
        );
    }

    #[test]
    fn escape_xml_attr_empty() {
        assert_eq!(xml_escape(""), "");
        assert_eq!(xml_escape("hello"), "hello");
        assert_eq!(
            xml_escape("a & b < c > d \"e\" 'f'"),
            "a &amp; b &lt; c &gt; d &quot;e&quot; &apos;f&apos;"
        );
    }
}
