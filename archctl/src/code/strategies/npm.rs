//! S2: npm/yarn/pnpm workspace detection.
//!
//! Detects workspace members (one Container per package) for:
//! - npm/yarn workspaces declared in `package.json` (`workspaces` array
//!   or `{packages: [...]}` object), and
//! - pnpm monorepos declared in `pnpm-workspace.yaml` (`packages:` list).
//!
//! Found by the 2026-08-19 UAT smoke on `vueuse/vueuse`: the previous
//! implementation (a) ignored `pnpm-workspace.yaml` entirely and (b) passed
//! glob patterns like `packages/*` to the directory walker as literal
//! paths, so no workspace member was ever detected.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::code::c4_discover::{ContainerCandidate, Evidence, EvidenceKind};
use crate::code::strategies::Strategy;
use crate::filesystem::{EntryKind, Filesystem};
use crate::inventory::find_manifests;

pub struct NpmWorkspace;

impl Strategy for NpmWorkspace {
    fn id(&self) -> &'static str {
        "npm-workspace"
    }
    fn confidence(&self) -> f64 {
        0.80
    }
    fn metatype(&self) -> &'static str {
        "mt.container"
    }

    fn detect(&self, project_root: &Path, fs: &dyn Filesystem) -> Result<Vec<ContainerCandidate>> {
        // D2: root-first, then fallback to nearest nested package.json with workspaces
        let pkg_json = project_root.join("package.json");
        let (manifest_path, glob_base): (PathBuf, PathBuf) = if fs.exists(&pkg_json) {
            (pkg_json.clone(), project_root.to_path_buf())
        } else {
            // Fallback: find first nested package.json that has a workspaces field
            let nested = find_manifests(project_root, &["package.json"], 3)?;
            let mut candidate: Option<PathBuf> = None;
            for p in nested {
                let full = project_root.join(&p);
                if !fs.exists(&full) {
                    continue;
                }
                let Ok(text) = fs.read_to_string(&full) else {
                    continue;
                };
                let Ok(value) = serde_json::from_str::<Value>(&text) else {
                    continue;
                };
                if value.get("workspaces").is_some() {
                    candidate = Some(full);
                    break;
                }
            }
            match candidate {
                Some(p) => (p.clone(), p.parent().unwrap_or(project_root).to_path_buf()),
                None => return Ok(Vec::new()),
            }
        };

        let pkg_json_text = fs
            .read_to_string(&manifest_path)
            .with_context(|| format!("read {}", manifest_path.display()))?;
        let pkg: Value = serde_json::from_str(&pkg_json_text).context("parse package.json")?;

        // workspaces can be ["packages/*"] or { "packages": ["packages/*"] }
        let npm_patterns: Vec<String> = match &pkg.get("workspaces") {
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
        // pnpm: pnpm-workspace.yaml `packages:` list (vueuse-style monorepos).
        // Only consulted when package.json declares no `workspaces` field.
        let patterns: Vec<String> = if npm_patterns.is_empty() {
            pnpm_workspace_patterns(&glob_base, fs)?
        } else {
            npm_patterns
        };
        if patterns.is_empty() {
            return Ok(Vec::new());
        }

        let mut candidates = Vec::new();
        let member_dirs = expand_workspace_members(&glob_base, &patterns, fs)?;
        for member_dir in member_dirs {
            for pkg_path in member_package_manifests(&member_dir, fs)? {
                let rel_path = pkg_path
                    .strip_prefix(project_root)
                    .unwrap_or(&pkg_path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let package_text = fs
                    .read_to_string(&pkg_path)
                    .with_context(|| format!("read {}", pkg_path.display()))?;
                let package: Value = serde_json::from_str(&package_text)
                    .with_context(|| format!("parse {}", pkg_path.display()))?;
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

/// Parse the `packages:` list of a pnpm-workspace.yaml (line-based; the
/// project avoids adding a YAML dependency — see ADR-006). Only the list
/// items of the top-level `packages:` key are collected; the section ends
/// at the next top-level key.
fn pnpm_workspace_patterns(manifest_dir: &Path, fs: &dyn Filesystem) -> Result<Vec<String>> {
    let yaml_path = manifest_dir.join("pnpm-workspace.yaml");
    if !fs.exists(&yaml_path) {
        return Ok(Vec::new());
    }
    let text = fs
        .read_to_string(&yaml_path)
        .with_context(|| format!("read {}", yaml_path.display()))?;

    let mut out = Vec::new();
    let mut in_packages = false;
    for line in text.lines() {
        if !in_packages {
            if line.trim_start() == "packages:" {
                in_packages = true;
            }
            continue;
        }
        // A non-indented line starts a new top-level key — end of section.
        if !line.starts_with([' ', '\t']) {
            break;
        }
        let Some(item) = line.trim_start().strip_prefix("- ") else {
            continue;
        };
        let item = item.trim();
        if item.is_empty() || item.starts_with('#') {
            continue;
        }
        // Strip surrounding quotes ('packages/*' or "packages/*").
        let item = item
            .strip_prefix('\'')
            .or_else(|| item.strip_prefix('"'))
            .map(|s| {
                s.strip_suffix('\'')
                    .or_else(|| s.strip_suffix('"'))
                    .unwrap_or(s)
            })
            .unwrap_or(item);
        out.push(item.to_string());
    }
    Ok(out)
}

/// Expand workspace glob patterns into concrete member directories.
///
/// Supported patterns (enough for npm/yarn/pnpm conventions):
/// - exact dirs (`playgrounds`, `apps/web`)
/// - single-level globs with a trailing `/*` (`packages/*`)
/// - exclusions prefixed with `!` (`!packages/.test`, `!playgrounds`)
///
/// Hidden directories (name starting with `.`) are never workspace members.
fn expand_workspace_members(
    base: &Path,
    patterns: &[String],
    fs: &dyn Filesystem,
) -> Result<Vec<PathBuf>> {
    let (includes, excludes): (Vec<String>, Vec<String>) =
        patterns.iter().cloned().partition(|p| !p.starts_with('!'));
    let excludes: Vec<String> = excludes
        .into_iter()
        .map(|p| {
            p.trim_start_matches('!')
                .trim_start_matches('/')
                .to_string()
        })
        .collect();

    let mut dirs: Vec<PathBuf> = Vec::new();
    for pat in &includes {
        let pat = pat.trim_start_matches("./");
        if pat.is_empty() {
            continue;
        }
        match pat.rfind('*') {
            Some(star) => {
                // Only trailing "/*" (or "/*/**" — treated as one level) is
                // expanded. Mid-path globs are out of scope: skip the pattern
                // rather than guess.
                let rest = &pat[star + 1..];
                if !rest.is_empty() && rest != "/" && rest != "/**" {
                    continue;
                }
                let prefix = pat[..star].trim_end_matches('/');
                let prefix_dir = base.join(prefix);
                if !fs.exists(&prefix_dir) {
                    continue;
                }
                for entry in fs.read_dir(&prefix_dir)? {
                    if entry.kind != EntryKind::Dir {
                        continue;
                    }
                    if let Some(name) = entry.path.file_name()
                        && name.to_string_lossy().starts_with('.')
                    {
                        continue;
                    }
                    dirs.push(entry.path);
                }
            }
            None => {
                let dir = base.join(pat);
                if fs.exists(&dir) {
                    dirs.push(dir);
                }
            }
        }
    }

    dirs.sort();
    dirs.dedup();
    dirs.retain(|d| {
        let rel = d
            .strip_prefix(base)
            .unwrap_or(d)
            .to_string_lossy()
            .replace('\\', "/");
        !excludes
            .iter()
            .any(|e| rel == *e || rel.starts_with(&format!("{}/", e.trim_end_matches('/'))))
    });

    Ok(dirs)
}

/// Package manifests of a member dir: `dir/package.json`, or one nested
/// level below (scoped packages like `packages/@scope/pkg/package.json`).
fn member_package_manifests(dir: &Path, fs: &dyn Filesystem) -> Result<Vec<PathBuf>> {
    let direct = dir.join("package.json");
    if fs.exists(&direct) {
        return Ok(vec![direct]);
    }
    let mut out = Vec::new();
    let children = fs.read_dir(dir).unwrap_or_default();
    for entry in children {
        if entry.kind != EntryKind::Dir {
            continue;
        }
        let pkg = entry.path.join("package.json");
        if fs.exists(&pkg) {
            out.push(pkg);
        }
    }
    out.sort();
    Ok(out)
}

fn find_name_field_line(text: &str) -> Option<u32> {
    for (i, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("\"name\"") {
            return Some((i + 1) as u32);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::MemoryFilesystem;
    use std::collections::HashSet;

    fn fs_with(files: &[(&str, &str)]) -> (tempfile::TempDir, MemoryFilesystem) {
        let tmp = tempfile::tempdir().unwrap();
        let fs = MemoryFilesystem::new();
        let root = tmp.path();
        for (rel, content) in files {
            let full = root.join(rel);
            if let Some(parent) = full.parent() {
                fs.create_dir_all(parent).unwrap();
            }
            fs.write(&full, content.as_bytes()).unwrap();
        }
        (tmp, fs)
    }

    #[test]
    fn detects_npm_workspaces_array() {
        let (tmp, fs) = fs_with(&[
            (
                "package.json",
                r#"{"name":"mono","workspaces":["packages/*"]}"#,
            ),
            ("packages/a/package.json", r#"{"name":"a"}"#),
            ("packages/b/package.json", r#"{"name":"b"}"#),
        ]);
        let cands = NpmWorkspace.detect(tmp.path(), &fs).unwrap();
        let names: HashSet<String> = cands.into_iter().map(|c| c.name).collect();
        assert_eq!(names, HashSet::from(["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn detects_pnpm_workspace_yaml() {
        // vueuse-style: no `workspaces` key, monorepo declared in
        // pnpm-workspace.yaml with an exclusion.
        let (tmp, fs) = fs_with(&[
            (
                "package.json",
                r#"{"name":"@vueuse/monorepo","private":true}"#,
            ),
            (
                "pnpm-workspace.yaml",
                "shamefullyHoist: true\n\npackages:\n  - packages/*\n  - '!playgrounds'\n",
            ),
            ("packages/core/package.json", r#"{"name":"@vueuse/core"}"#),
            (
                "packages/shared/package.json",
                r#"{"name":"@vueuse/shared"}"#,
            ),
            ("packages/.test/index.ts", "export {}"),
            ("playgrounds/app/package.json", r#"{"name":"playground"}"#),
        ]);
        let cands = NpmWorkspace.detect(tmp.path(), &fs).unwrap();
        let names: HashSet<String> = cands.into_iter().map(|c| c.name).collect();
        // Hidden dirs are never members; excluded patterns are skipped.
        assert_eq!(
            names,
            HashSet::from(["@vueuse/core".to_string(), "@vueuse/shared".to_string()])
        );
    }

    #[test]
    fn detects_scoped_workspace_members() {
        let (tmp, fs) = fs_with(&[
            (
                "package.json",
                r#"{"name":"mono","workspaces":["packages/*"]}"#,
            ),
            (
                "packages/@scope/pkg/package.json",
                r#"{"name":"@scope/pkg"}"#,
            ),
        ]);
        let cands = NpmWorkspace.detect(tmp.path(), &fs).unwrap();
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].name, "@scope/pkg");
    }

    #[test]
    fn empty_without_workspace_signals() {
        let (tmp, fs) = fs_with(&[("package.json", r#"{"name":"single"}"#)]);
        let cands = NpmWorkspace.detect(tmp.path(), &fs).unwrap();
        assert!(cands.is_empty(), "single packages belong to npm-single");
    }

    #[test]
    fn pnpm_workspace_without_packages_key_yields_no_patterns() {
        // zustand-style pnpm-workspace.yaml: no `packages:` section → no
        // workspace members.
        let (tmp, fs) = fs_with(&[
            ("package.json", r#"{"name":"zustand"}"#),
            ("pnpm-workspace.yaml", "allowBuilds:\n  esbuild: false\n"),
        ]);
        let patterns = pnpm_workspace_patterns(tmp.path(), &fs).unwrap();
        assert!(patterns.is_empty());
    }
}
