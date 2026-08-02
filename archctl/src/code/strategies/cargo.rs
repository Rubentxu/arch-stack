//! S1: Cargo workspace detection.

use std::path::Path;

use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;

use crate::code::c4_discover::{ContainerCandidate, Evidence, EvidenceKind};
use crate::code::strategies::Strategy;
use crate::filesystem::Filesystem;

pub struct CargoWorkspace;

impl Strategy for CargoWorkspace {
    fn id(&self) -> &'static str {
        "cargo-workspace"
    }
    fn confidence(&self) -> f64 {
        0.85
    }

    fn detect(&self, project_root: &Path, _fs: &dyn Filesystem) -> Result<Vec<ContainerCandidate>> {
        let cargo_toml = project_root.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Ok(Vec::new());
        }

        // Use cargo_metadata for safe TOML parsing + workspace traversal
        let metadata = MetadataCommand::new()
            .current_dir(project_root)
            .no_deps()
            .exec()
            .context("cargo_metadata::MetadataCommand::exec")?;

        let workspace_members = metadata.workspace_members.clone();
        let mut candidates = Vec::new();

        for pkg_id in workspace_members {
            let package = &metadata[&pkg_id];
            // Skip workspace stub: no description means no [package] section
            if package.description.is_none() {
                continue;
            }
            let manifest_path = package.manifest_path.as_std_path();
            let rel_manifest = manifest_path
                .strip_prefix(project_root)
                .unwrap_or(manifest_path)
                .to_string_lossy()
                .replace('\\', "/");

            candidates.push(ContainerCandidate {
                canonical_key: package.name.to_string(),
                name: package.name.to_string(),
                strategy: self.id().to_string(),
                confidence: self.confidence(),
                evidences: vec![Evidence {
                    file: rel_manifest,
                    line: 1,
                    kind: EvidenceKind::Structural,
                    text: format!("Cargo workspace member: {}", package.name),
                }],
            });
        }

        Ok(candidates)
    }
}
