//! S6: npm single-package detection (M28).
//!
//! Detects a root `package.json` as ONE Container when the repo is NOT a
//! workspace (no `workspaces` field). Repos like zustand/express are
//! single npm packages — the npm-workspace strategy returns nothing for
//! them, leaving a false-negative gap (M28, FP/FN review 2026-08-06).
//!
//! Confidence < 1.0 (0.70): a root package.json is a strong signal of one
//! deployable/package unit, but the container boundary is softer than a
//! cargo workspace member.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::code::c4_discover::{ContainerCandidate, Evidence, EvidenceKind};
use crate::code::strategies::Strategy;
use crate::filesystem::Filesystem;
use crate::inventory::find_manifests;

pub struct NpmSinglePackage;

impl Strategy for NpmSinglePackage {
    fn id(&self) -> &'static str {
        "npm-single"
    }
    fn confidence(&self) -> f64 {
        0.70
    }
    fn metatype(&self) -> &'static str {
        "mt.container"
    }

    fn detect(&self, project_root: &Path, fs: &dyn Filesystem) -> Result<Vec<ContainerCandidate>> {
        // D2: root-first, then fallback to first nested package.json
        let pkg_json: PathBuf = if fs.exists(&project_root.join("package.json")) {
            project_root.join("package.json")
        } else {
            // Fallback: find first nested package.json within depth 3
            let nested = find_manifests(project_root, &["package.json"], 3)?;
            match nested.into_iter().next() {
                Some(p) => project_root.join(p),
                None => return Ok(Vec::new()),
            }
        };

        let pkg_json_text = fs
            .read_to_string(&pkg_json)
            .with_context(|| format!("read {}", pkg_json.display()))?;
        let pkg: Value = serde_json::from_str(&pkg_json_text).context("parse package.json")?;

        // A workspace is declared by: npm `workspaces` (array or
        // {packages:[]}) in package.json, OR a pnpm-workspace.yaml that
        // actually declares `packages:` (zustand ships a pnpm-workspace.yaml
        // with only `allowBuilds:` — that is NOT a monorepo; it is a single
        // package with build config).
        let npm_workspaces = pkg
            .get("workspaces")
            .and_then(|w| match w {
                Value::Array(a) => Some(a.clone()),
                Value::Object(o) => o.get("packages").and_then(Value::as_array).cloned(),
                _ => None,
            })
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        let pnpm_workspace_yaml =
            pnpm_workspace_declares_packages(pkg_json.parent().unwrap_or(project_root), fs);
        if npm_workspaces || pnpm_workspace_yaml {
            return Ok(Vec::new());
        }

        let name = pkg
            .get("name")
            .and_then(|n| n.as_str())
            .map(String::from)
            .unwrap_or_else(|| "package".to_string());

        let line = find_name_field_line(&pkg_json_text).unwrap_or(1);
        let text = format!("npm single package: {}", name);

        let rel_path = pkg_json
            .strip_prefix(project_root)
            .unwrap_or(&pkg_json)
            .to_string_lossy()
            .replace('\\', "/");

        Ok(vec![ContainerCandidate {
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
        }])
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

/// True only when pnpm-workspace.yaml declares a `packages:` list (a real
/// monorepo). A file with only `allowBuilds:`/other config is a single
/// package using pnpm as package manager.
fn pnpm_workspace_declares_packages(project_root: &Path, fs: &dyn Filesystem) -> bool {
    let yaml_path = project_root.join("pnpm-workspace.yaml");
    if !fs.exists(&yaml_path) {
        return false;
    }
    let Ok(text) = fs.read_to_string(&yaml_path) else {
        return false;
    };
    text.lines()
        .any(|l| l.trim_start().starts_with("packages:"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::MemoryFilesystem;

    fn single_package_fs() -> (tempfile::TempDir, MemoryFilesystem) {
        let tmp = tempfile::tempdir().unwrap();
        let fs = MemoryFilesystem::new();
        let root = tmp.path();
        fs.write(
            &root.join("package.json"),
            b"{\n  \"name\": \"zustand\",\n  \"version\": \"4.5.0\"\n}",
        )
        .unwrap();
        (tmp, fs)
    }

    #[test]
    fn detects_single_package_root() {
        let (tmp, fs) = single_package_fs();
        let cands = NpmSinglePackage.detect(tmp.path(), &fs).unwrap();
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].canonical_key, "zustand");
        assert_eq!(cands[0].strategy, "npm-single");
        assert!(cands[0].confidence < 1.0);
        assert_eq!(cands[0].evidences[0].file, "package.json");
    }

    #[test]
    fn skips_workspace_repos() {
        let tmp = tempfile::tempdir().unwrap();
        let fs = MemoryFilesystem::new();
        fs.write(
            &tmp.path().join("package.json"),
            br#"{"name":"mono","workspaces":["packages/*"]}"#,
        )
        .unwrap();
        let cands = NpmSinglePackage.detect(tmp.path(), &fs).unwrap();
        assert!(cands.is_empty(), "workspace repos belong to NpmWorkspace");
    }

    #[test]
    fn skips_pnpm_monorepos() {
        // vueuse-style: no npm workspaces key but pnpm-workspace.yaml
        // declares the monorepo — must NOT be treated as single package.
        let tmp = tempfile::tempdir().unwrap();
        let fs = MemoryFilesystem::new();
        fs.write(
            &tmp.path().join("package.json"),
            br#"{"name":"@vueuse/monorepo","private":true}"#,
        )
        .unwrap();
        fs.write(
            &tmp.path().join("pnpm-workspace.yaml"),
            b"packages:\n  - packages/*\n",
        )
        .unwrap();
        let cands = NpmSinglePackage.detect(tmp.path(), &fs).unwrap();
        assert!(cands.is_empty(), "pnpm monorepos are not single packages");
    }

    #[test]
    fn pnpm_yaml_without_packages_is_single_package() {
        // zustand-style: pnpm-workspace.yaml with only `allowBuilds:` is
        // build config, NOT a monorepo — the root package IS the container.
        let tmp = tempfile::tempdir().unwrap();
        let fs = MemoryFilesystem::new();
        fs.write(
            &tmp.path().join("package.json"),
            br#"{"name":"zustand","version":"4.5.0"}"#,
        )
        .unwrap();
        fs.write(
            &tmp.path().join("pnpm-workspace.yaml"),
            b"allowBuilds:\n  esbuild: false\n",
        )
        .unwrap();
        let cands = NpmSinglePackage.detect(tmp.path(), &fs).unwrap();
        assert_eq!(
            cands.len(),
            1,
            "allowBuilds-only yaml must not block npm-single"
        );
        assert_eq!(cands[0].canonical_key, "zustand");
    }

    #[test]
    fn null_workspaces_is_single_package() {
        // `"workspaces": null` in the manifest is NOT a workspace.
        let tmp = tempfile::tempdir().unwrap();
        let fs = MemoryFilesystem::new();
        fs.write(
            &tmp.path().join("package.json"),
            br#"{"name":"mini","workspaces":null}"#,
        )
        .unwrap();
        let cands = NpmSinglePackage.detect(tmp.path(), &fs).unwrap();
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].canonical_key, "mini");
    }

    #[test]
    fn empty_without_package_json() {
        let tmp = tempfile::tempdir().unwrap();
        let fs = MemoryFilesystem::new();
        let cands = NpmSinglePackage.detect(tmp.path(), &fs).unwrap();
        assert!(cands.is_empty());
    }

    #[test]
    fn derives_name_from_package_json() {
        let tmp = tempfile::tempdir().unwrap();
        let fs = MemoryFilesystem::new();
        fs.write(&tmp.path().join("package.json"), br#"{"name":"express"}"#)
            .unwrap();
        let cands = NpmSinglePackage.detect(tmp.path(), &fs).unwrap();
        assert_eq!(cands[0].canonical_key, "express");
        assert_eq!(cands[0].name, "express");
    }
}
