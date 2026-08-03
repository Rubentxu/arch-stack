//! S2: npm/yarn/pnpm workspace detection.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::code::c4_discover::{ContainerCandidate, Evidence, EvidenceKind};
use crate::code::strategies::Strategy;
use crate::filesystem::Filesystem;

pub struct NpmWorkspace;

impl Strategy for NpmWorkspace {
    fn id(&self) -> &'static str {
        "npm-workspace"
    }
    fn confidence(&self) -> f64 {
        0.80
    }

    fn detect(&self, project_root: &Path, fs: &dyn Filesystem) -> Result<Vec<ContainerCandidate>> {
        let pkg_json = project_root.join("package.json");
        if !fs.exists(&pkg_json) {
            return Ok(Vec::new());
        }

        let pkg_json_text = fs
            .read_to_string(&pkg_json)
            .with_context(|| format!("read {}", pkg_json.display()))?;
        let pkg: Value = serde_json::from_str(&pkg_json_text).context("parse package.json")?;

        // workspaces can be ["packages/*"] or { "packages": ["packages/*"] }
        let workspaces: Vec<String> = match &pkg.get("workspaces") {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            Some(Value::Object(obj)) => obj
                .get("packages")
                .and_then(|p| p.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };

        let mut candidates = Vec::new();
        for glob_pattern in workspaces {
            // glob_pattern like "packages/*" or "apps/*"
            // Walk the root of the glob pattern and look for subdirs with package.json
            let glob_root = project_root.join(&glob_pattern);
            // Walk one level deep (the glob pattern's root)
            let walker = ignore::WalkBuilder::new(&glob_root)
                .max_depth(Some(2))
                .standard_filters(false)
                .follow_links(false)
                .build();

            for entry in walker {
                let entry = entry.context("walk workspace glob")?;
                let path = entry.path();
                // Only look at directories (not the root itself)
                if path == glob_root {
                    continue;
                }
                let pkg_path = path.join("package.json");
                if !fs.exists(&pkg_path) {
                    continue;
                }
                let rel_path = pkg_path
                    .strip_prefix(project_root)
                    .unwrap_or(&pkg_path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let package_text = fs.read_to_string(&pkg_path)?;
                let package: Value =
                    serde_json::from_str(&package_text).context("parse workspace package.json")?;
                let name = package
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| rel_path.trim_end_matches("/package.json").to_string());

                let line = find_name_field_line(&package_text).unwrap_or(1);
                let text = format!("npm workspace package: {}", name);

                candidates.push(ContainerCandidate {
                    canonical_key: name.clone(),
                    name,
                    strategy: self.id().to_string(),
                    confidence: self.confidence(),
                    evidences: vec![Evidence {
                        content_hash: String::new(),
                        file: rel_path,
                        line,
                        kind: EvidenceKind::Structural,
                        text,
                    }],
                });
            }
        }

        Ok(candidates)
    }
}

fn find_name_field_line(text: &str) -> Option<u32> {
    for (i, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("\"name\"") {
            return Some((i + 1) as u32);
        }
    }
    None
}
