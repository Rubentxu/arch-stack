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
            let (display_name, line) = find_label_or_default(file_name);
            let display_name = if let Some(override_name) = read_label_title(path) {
                override_name
            } else {
                display_name
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

fn read_label_title(_path: &Path) -> Option<String> {
    // TODO: read the Dockerfile and parse LABEL org.opencontainers.image.title
    None
}
