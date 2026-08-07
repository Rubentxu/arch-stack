//! S5: Dockerfile per service detection.

use std::path::{Path, PathBuf};

use anyhow::Result;
use ignore::WalkBuilder;

use crate::code::c4_discover::{ContainerCandidate, Evidence, EvidenceKind};
use crate::code::strategies::Strategy;
use crate::filesystem::Filesystem;

const EXCLUDED_PATH_PREFIXES: &[&str] = &[
    "examples/",
    "docs/",
    "tools/",
    "test/",
    "tests/",
    "scripts/",
    "bin/",
];

pub struct DockerfilePerService;

impl Strategy for DockerfilePerService {
    fn id(&self) -> &'static str {
        "dockerfile"
    }
    fn confidence(&self) -> f64 {
        0.60
    }
    fn metatype(&self) -> &'static str {
        "mt.container"
    }

    fn detect(&self, project_root: &Path, _fs: &dyn Filesystem) -> Result<Vec<ContainerCandidate>> {
        let mut candidates = Vec::new();
        let walker = WalkBuilder::new(project_root)
            .standard_filters(true)
            .max_depth(Some(6))
            .build();

        for entry in walker {
            let entry = entry?;
            let path = entry.path();
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let lower = file_name.to_ascii_lowercase();
            // Match: Dockerfile, Dockerfile.{dev,prod,staging,test,local,...},
            // or *.dockerfile. A bare `starts_with("dockerfile.")` would
            // also match the strategy's own source `dockerfile.rs` (and any
            // `dockerfile.*` doc file) — restrict the dotted variant to a
            // known environment suffix.
            let dotted_env = lower
                .strip_prefix("dockerfile.")
                .filter(|suffix| {
                    !suffix.is_empty()
                        && matches!(
                            *suffix,
                            "dev" | "prod" | "production" | "staging" | "test" | "local" | "debug"
                        )
                })
                .is_some();
            if !(lower == "dockerfile" || dotted_env || lower.ends_with(".dockerfile")) {
                continue;
            }
            let rel_path = path
                .strip_prefix(project_root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");

            // Exclude fixture paths
            if EXCLUDED_PATH_PREFIXES
                .iter()
                .any(|p| rel_path.starts_with(p))
            {
                continue;
            }

            let parent_dir = path
                .parent()
                .and_then(|p| p.strip_prefix(project_root).ok())
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            let parent_name = PathBuf::from(&parent_dir)
                .file_name()
                .and_then(|s| s.to_str())
                .map(String::from)
                .unwrap_or_else(|| "root".to_string());

            let canonical_key = format!(
                "{}-{}",
                parent_name,
                file_name
                    .strip_suffix(".dockerfile")
                    .unwrap_or(file_name)
                    .trim_start_matches("Dockerfile.")
            )
            .trim_end_matches('-')
            .to_string();

            // Try to find LABEL org.opencontainers.image.title="..." in the file
            // (returns the line where the label appears so the evidence points
            // at the real declaration; falls back to line 1 on the file name).
            let (display_name, line) = match read_label_title_with_line(path) {
                Some((n, l)) => (n, l),
                None => find_label_or_default(file_name),
            };

            candidates.push(ContainerCandidate {
                canonical_key,
                name: display_name,
                strategy: self.id().to_string(),
                confidence: self.confidence(),
                evidences: vec![Evidence {
                    content_hash: String::new(),
                    file: rel_path,
                    line,
                    kind: EvidenceKind::Structural,
                    text: format!("Dockerfile for service: {}", parent_name),
                }],
            });
        }

        Ok(candidates)
    }
}

fn find_label_or_default(_file_name: &str) -> (String, u32) {
    ("docker".to_string(), 1)
}

fn read_label_title_with_line(path: &Path) -> Option<(String, u32)> {
    // Returns the OCI image title and the 1-based line where it appears.
    // Returns None when the file cannot be read or the label is absent.
    let raw = std::fs::read_to_string(path).ok()?;
    let (value, line) = parse_opencontainers_title_with_line(&raw)?;
    Some((value, line))
}

/// Parse the OCI image title from a Dockerfile body. Returns `(value, line)`
/// where `line` is 1-based. Exposed (via `pub(crate)`) for unit tests.
pub(crate) fn parse_opencontainers_title_with_line(raw: &str) -> Option<(String, u32)> {
    // We do NOT fold continuations: we track line numbers as we scan, so a
    // multi-line LABEL is anchored to the line where `LABEL` begins.
    let needle = "label org.opencontainers.image.title=";
    let mut line: u32 = 1;

    // Lowercased copy for case-insensitive matching while preserving original
    // byte indices for line counting.
    let lower = raw.to_ascii_lowercase();
    let bytes = raw.as_bytes();

    // Find the first occurrence of LABEL org.opencontainers.image.title=
    let idx = lower.find(needle)?;

    // Anchor the reported line to where `LABEL` begins.
    for ch in raw[..idx].chars() {
        if ch == '\n' {
            line += 1;
        }
    }

    // Walk forward, skipping spaces/tabs after the `=`.
    let mut cursor = idx + needle.len();
    while cursor < bytes.len() && (bytes[cursor] == b' ' || bytes[cursor] == b'\t') {
        cursor += 1;
    }
    let quote = bytes.get(cursor).copied()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    cursor += 1;
    let after_open_start = cursor;
    while cursor < bytes.len() && bytes[cursor] != quote {
        cursor += 1;
    }
    if cursor >= bytes.len() {
        return None;
    }
    let value_raw = &raw[after_open_start..cursor];
    let trimmed = value_raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some((trimmed.to_string(), line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_double_quoted_label() {
        let raw = "FROM alpine:3\nLABEL org.opencontainers.image.title=\"my-svc\"\n";
        assert_eq!(
            parse_opencontainers_title_with_line(raw),
            Some(("my-svc".to_string(), 2))
        );
    }

    #[test]
    fn parses_single_quoted_label() {
        let raw = "FROM alpine\nLABEL org.opencontainers.image.title='hello world'\n";
        assert_eq!(
            parse_opencontainers_title_with_line(raw),
            Some(("hello world".to_string(), 2))
        );
    }

    #[test]
    fn case_insensitive_label_keyword() {
        let raw = "label org.opencontainers.image.title=\"ok\"\n";
        assert_eq!(
            parse_opencontainers_title_with_line(raw),
            Some(("ok".to_string(), 1))
        );
    }

    #[test]
    fn label_on_third_line_anchors_correctly() {
        let raw = "FROM alpine\nRUN echo hi\nLABEL org.opencontainers.image.title=\"third\"\n";
        assert_eq!(
            parse_opencontainers_title_with_line(raw),
            Some(("third".to_string(), 3))
        );
    }

    #[test]
    fn missing_label_returns_none() {
        let raw = "FROM alpine\nRUN echo no label here\n";
        assert_eq!(parse_opencontainers_title_with_line(raw), None);
    }

    #[test]
    fn empty_label_value_returns_none() {
        let raw = "LABEL org.opencontainers.image.title=\"\"\n";
        assert_eq!(parse_opencontainers_title_with_line(raw), None);
    }

    #[test]
    fn unquoted_label_returns_none() {
        let raw = "LABEL org.opencontainers.image.title=plain\n";
        assert_eq!(parse_opencontainers_title_with_line(raw), None);
    }

    #[test]
    fn different_label_returns_none() {
        let raw = "LABEL maintainer=\"someone@example.com\"\n";
        assert_eq!(parse_opencontainers_title_with_line(raw), None);
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let raw = "LABEL org.opencontainers.image.title=\"  spaced  \"\n";
        assert_eq!(
            parse_opencontainers_title_with_line(raw),
            Some(("spaced".to_string(), 1))
        );
    }
}
