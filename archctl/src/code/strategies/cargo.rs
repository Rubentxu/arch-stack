//! S1: Cargo workspace detection.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;

use crate::code::c4_discover::{ContainerCandidate, Evidence, EvidenceKind};
use crate::code::strategies::Strategy;
use crate::filesystem::Filesystem;
use crate::inventory::find_manifests;

pub struct CargoWorkspace;

impl Strategy for CargoWorkspace {
    fn id(&self) -> &'static str {
        "cargo-workspace"
    }
    fn confidence(&self) -> f64 {
        0.85
    }
    fn metatype(&self) -> &'static str {
        "mt.container"
    }

    fn detect(&self, project_root: &Path, _fs: &dyn Filesystem) -> Result<Vec<ContainerCandidate>> {
        let cargo_toml = project_root.join("Cargo.toml");
        let manifest_path: Option<PathBuf> = if cargo_toml.exists() {
            Some(cargo_toml)
        } else {
            // D2: fallback — find nearest nested Cargo.toml within depth 3
            let nested = find_manifests(project_root, &["Cargo.toml"], 3)?;
            nested.into_iter().next().map(|p| project_root.join(p))
        };

        let Some(manifest) = manifest_path else {
            return Ok(Vec::new());
        };

        // Use manifest_path (more precise than current_dir; cargo resolves
        // the full workspace from any member manifest path).
        let metadata = MetadataCommand::new()
            .manifest_path(&manifest)
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
            let pkg_manifest_path = package.manifest_path.as_std_path();
            let rel_manifest = pkg_manifest_path
                .strip_prefix(project_root)
                .unwrap_or(pkg_manifest_path)
                .to_string_lossy()
                .replace('\\', "/");

            candidates.push(ContainerCandidate {
                canonical_key: package.name.to_string(),
                name: package.name.to_string(),
                strategy: self.id().to_string(),
                confidence: self.confidence(),
                evidences: vec![Evidence {
                    content_hash: String::new(),
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
