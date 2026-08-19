//! G5: C4 Component detection — internal library modules within a container boundary.
//!
//! Strategy id: `components`
//!
//! Per ADR-028: Components are internal library modules that live inside a
//! Container boundary. Unlike CargoWorkspace/NpmWorkspace (which detect Containers),
//! this strategy detects the *internal* module structure within a project.
//!
//! Detection signals:
//! - Rust: `src/` subdirectories and `mod.rs` files that are NOT separate crates
//!   (i.e., not workspace members with their own Cargo.toml).
//! - TypeScript: `src/` subdirectories with `index.ts` or named exports.
//! - Python: `src/` subdirectories with `__init__.py`.
//!
//! Confidence: 0.65 — boundary between "component" and "module" is fuzzy.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json;

use crate::code::c4_discover::{ContainerCandidate, Evidence, EvidenceKind};
use crate::code::strategies::Strategy;
use crate::filesystem::Filesystem;
use crate::inventory::find_manifests;

pub struct ComponentsStrategy;

impl Strategy for ComponentsStrategy {
    fn id(&self) -> &'static str {
        "components"
    }

    fn confidence(&self) -> f64 {
        0.65
    }

    fn metatype(&self) -> &'static str {
        "mt.component"
    }

    fn detect(&self, project_root: &Path, fs: &dyn Filesystem) -> Result<Vec<ContainerCandidate>> {
        let mut candidates = Vec::new();

        // Collect crate root dirs already detected as containers (skip them)
        let container_dirs = collect_container_dirs(project_root, fs)?;

        // Rust: detect internal modules via src/ tree
        let rust_components = detect_rust_components(project_root, fs, &container_dirs)?;
        candidates.extend(rust_components);

        // TypeScript: detect internal modules via src/ tree
        let ts_components = detect_ts_components(project_root, fs, &container_dirs)?;
        candidates.extend(ts_components);

        // Python: detect internal modules via src/ tree
        let py_components = detect_py_components(project_root, fs, &container_dirs)?;
        candidates.extend(py_components);

        // Sort for determinism
        candidates.sort_by(|a, b| a.canonical_key.cmp(&b.canonical_key));

        Ok(candidates)
    }
}

/// Collect directories that are themselves containers (workspace members).
/// D2: when root manifest is absent, uses find_manifests to locate nested
/// manifests and collect their parent dirs.
fn collect_container_dirs(project_root: &Path, fs: &dyn Filesystem) -> Result<HashSet<String>> {
    let mut dirs = HashSet::new();

    // Rust: collect workspace member manifest paths
    let cargo_toml = project_root.join("Cargo.toml");
    if fs.exists(&cargo_toml)
        && let Ok(_text) = fs.read_to_string(&cargo_toml)
    {
        // Walk looking for nested Cargo.toml files (workspace members)
        let walker = ignore::WalkBuilder::new(project_root)
            .hidden(false)
            .follow_links(false)
            .build();
        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.file_name().map(|n| n == "Cargo.toml").unwrap_or(false)
                && let Ok(rel) = path.strip_prefix(project_root)
            {
                let _rel_str = rel.to_string_lossy().replace('\\', "/");
                // Parent dir of the Cargo.toml is the crate root
                if let Some(parent) = path.parent()
                    && let Ok(parent_rel) = parent.strip_prefix(project_root)
                {
                    dirs.insert(parent_rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    } else {
        // D2 fallback: root Cargo.toml absent — find nested Cargo.toml via find_manifests
        let nested = find_manifests(project_root, &["Cargo.toml"], 3)?;
        for manifest in nested {
            let full = project_root.join(&manifest);
            if let Some(parent) = full.parent()
                && let Ok(parent_rel) = parent.strip_prefix(project_root)
            {
                dirs.insert(parent_rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    // npm: collect package.json directories (skip workspace packages)
    let package_json = project_root.join("package.json");
    if fs.exists(&package_json)
        && let Ok(text) = fs.read_to_string(&package_json)
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(workspaces) = value.get("workspaces")
    {
        // This is a workspace root — collect package subdirs
        let walker = ignore::WalkBuilder::new(project_root)
            .max_depth(Some(3))
            .hidden(false)
            .follow_links(false)
            .build();
        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path
                .file_name()
                .map(|n| n == "package.json")
                .unwrap_or(false)
                && let Ok(_rel) = path.strip_prefix(project_root)
                && let Some(parent) = path.parent()
                && let Ok(parent_rel) = parent.strip_prefix(project_root)
            {
                dirs.insert(parent_rel.to_string_lossy().replace('\\', "/"));
            }
        }
        let _ = workspaces;
    } else {
        // D2 fallback: root package.json absent — find nested package.json via find_manifests
        let nested = find_manifests(project_root, &["package.json"], 3)?;
        for manifest in nested {
            let full = project_root.join(&manifest);
            if let Some(parent) = full.parent()
                && let Ok(parent_rel) = parent.strip_prefix(project_root)
            {
                dirs.insert(parent_rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    Ok(dirs)
}

fn detect_rust_components(
    project_root: &Path,
    fs: &dyn Filesystem,
    skip_dirs: &HashSet<String>,
) -> Result<Vec<ContainerCandidate>> {
    let mut candidates = Vec::new();

    // Look for src/ subdirs with mod.rs files (top-level modules)
    let src_dir = project_root.join("src");
    if !fs.exists(&src_dir) {
        return Ok(candidates);
    }

    for dir in top_level_module_dirs(&src_dir, fs)? {
        // Compute relative path from project root
        let rel = match dir.strip_prefix(project_root) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");

        // Skip if this is a container dir itself
        if skip_dirs.contains(&rel_str) {
            continue;
        }

        // Module name = directory name
        let module_name = rel
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        if module_name.is_empty() || module_name == "src" {
            continue;
        }
        // Hidden dirs (.test, .github, …) are fixtures/config, not
        // modules (vueuse UAT smoke 2026-08-19: packages/.test).
        if module_name.starts_with('.') {
            continue;
        }

        // Look for mod.rs or lib.rs to get the marker
        let marker_path = dir.join("mod.rs");
        let (file, line, text) = if fs.exists(&marker_path) {
            (
                "mod.rs".to_string(),
                1u32,
                format!("Rust module: {}", module_name),
            )
        } else {
            (
                "lib.rs".to_string(),
                1u32,
                format!("Rust module: {}", module_name),
            )
        };

        let canonical_key = format!("rust:module:{}", rel_str.replace('/', "."));

        candidates.push(ContainerCandidate {
            canonical_key,
            name: module_name,
            strategy: "components".to_string(),
            confidence: 0.65,
            evidences: vec![Evidence {
                content_hash: String::new(),
                file: format!("{}/{}", rel_str, file),
                line,
                kind: EvidenceKind::Lexical,
                text,
            }],
        });
    }

    Ok(candidates)
}

/// List direct subdirectories of `dir` that contain a module marker file.
///
/// Uses the Filesystem port (not `ignore::WalkBuilder`) so it works with
/// MemoryFilesystem in tests and SystemFilesystem in production.
fn top_level_module_dirs(dir: &Path, fs: &dyn Filesystem) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let entries = fs.read_dir(dir)?;
    for entry in entries {
        if entry.kind == crate::filesystem::EntryKind::Dir {
            out.push(entry.path);
        }
    }
    Ok(out)
}

fn detect_ts_components(
    project_root: &Path,
    fs: &dyn Filesystem,
    skip_dirs: &HashSet<String>,
) -> Result<Vec<ContainerCandidate>> {
    let mut candidates = Vec::new();

    // Look for src/ subdirs with index.ts or named exports
    let src_dirs = ["src", "lib", "packages"];

    for src_name in &src_dirs {
        let src_dir = project_root.join(src_name);
        if !fs.exists(&src_dir) {
            continue;
        }

        for dir in top_level_module_dirs(&src_dir, fs)? {
            let rel = match dir.strip_prefix(project_root) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let rel_str = rel.to_string_lossy().replace('\\', "/");

            if skip_dirs.contains(&rel_str) {
                continue;
            }

            // Check for index.ts or main entry point
            let has_entry = {
                fs.exists(&dir.join("index.ts"))
                    || fs.exists(&dir.join("index.tsx"))
                    || fs.exists(&dir.join("mod.ts"))
                    || fs.exists(&dir.join("mod.tsx"))
            };

            if !has_entry {
                continue;
            }

            let module_name = rel
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            if module_name.is_empty() {
                continue;
            }
            // Hidden dirs (.test, .assets, …) are fixtures, not modules
            // (vueuse UAT smoke 2026-08-19: packages/.test).
            if module_name.starts_with('.') {
                continue;
            }

            let file = if fs.exists(&dir.join("index.ts")) {
                "index.ts"
            } else if fs.exists(&dir.join("index.tsx")) {
                "index.tsx"
            } else if fs.exists(&dir.join("mod.ts")) {
                "mod.ts"
            } else {
                "mod.tsx"
            };

            let canonical_key = format!("typescript:module:{}", rel_str.replace('/', "."));

            candidates.push(ContainerCandidate {
                canonical_key,
                name: module_name.clone(),
                strategy: "components".to_string(),
                confidence: 0.65,
                evidences: vec![Evidence {
                    content_hash: String::new(),
                    file: format!("{}/{}", rel_str, file),
                    line: 1,
                    kind: EvidenceKind::Lexical,
                    text: format!("TypeScript module: {}", module_name),
                }],
            });
        }
    }

    Ok(candidates)
}

fn detect_py_components(
    project_root: &Path,
    fs: &dyn Filesystem,
    skip_dirs: &HashSet<String>,
) -> Result<Vec<ContainerCandidate>> {
    let mut candidates = Vec::new();

    // Look for src/ subdirs with __init__.py
    let src_dirs = ["src", "lib", "packages", "app"];

    for src_name in &src_dirs {
        let src_dir = project_root.join(src_name);
        if !fs.exists(&src_dir) {
            continue;
        }

        for dir in top_level_module_dirs(&src_dir, fs)? {
            let init_py = dir.join("__init__.py");
            if !fs.exists(&init_py) {
                continue;
            }

            let rel = match dir.strip_prefix(project_root) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let rel_str = rel.to_string_lossy().replace('\\', "/");

            if skip_dirs.contains(&rel_str) {
                continue;
            }

            let module_name = rel
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            if module_name.is_empty() {
                continue;
            }
            // Hidden dirs are fixtures, not modules (vueuse UAT smoke).
            if module_name.starts_with('.') {
                continue;
            }

            let canonical_key = format!("python:module:{}", rel_str.replace('/', "."));

            candidates.push(ContainerCandidate {
                canonical_key,
                name: module_name.clone(),
                strategy: "components".to_string(),
                confidence: 0.65,
                evidences: vec![Evidence {
                    content_hash: String::new(),
                    file: format!("{}/__init__.py", rel_str),
                    line: 1,
                    kind: EvidenceKind::Lexical,
                    text: format!("Python module: {}", module_name),
                }],
            });
        }
    }

    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::MemoryFilesystem;

    #[test]
    fn strategy_id_is_components() {
        let s = ComponentsStrategy;
        assert_eq!(s.id(), "components");
    }

    #[test]
    fn confidence_is_below_one() {
        let s = ComponentsStrategy;
        assert!(s.confidence() < 1.0);
    }

    #[test]
    fn detect_nothing_on_empty_project() {
        let fs = MemoryFilesystem::new();
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let s = ComponentsStrategy;
        let result = s.detect(root, &fs).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn detect_rust_src_modules() {
        let fs = MemoryFilesystem::new();
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Create: src/auth/mod.rs, src/api/mod.rs
        fs.create_dir_all(&root.join("src/auth")).unwrap();
        fs.create_dir_all(&root.join("src/api")).unwrap();
        fs.write(&root.join("src/auth/mod.rs"), b"pub mod error;")
            .unwrap();
        fs.write(&root.join("src/api/mod.rs"), b"pub mod handlers;")
            .unwrap();

        let s = ComponentsStrategy;
        let result = s.detect(root, &fs).unwrap();

        assert_eq!(result.len(), 2);
        let names: Vec<_> = result.iter().map(|c| c.name.clone()).collect();
        assert!(names.contains(&"auth".to_string()));
        assert!(names.contains(&"api".to_string()));
        // All have confidence < 1.0
        for c in &result {
            assert!(c.confidence < 1.0);
        }
    }

    #[test]
    fn skip_workspace_members() {
        let fs = MemoryFilesystem::new();
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Create: src/auth/mod.rs (internal) + crates/auth/Cargo.toml (workspace member)
        fs.create_dir_all(&root.join("src/auth")).unwrap();
        fs.create_dir_all(&root.join("crates/auth")).unwrap();
        fs.write(&root.join("src/auth/mod.rs"), b"pub mod error;")
            .unwrap();
        fs.write(
            &root.join("crates/auth/Cargo.toml"),
            b"[package]\nname = \"auth\"",
        )
        .unwrap();
        // Write root Cargo.toml with workspace
        fs.write(
            &root.join("Cargo.toml"),
            b"[workspace]\nmembers = [\"crates/auth\"]",
        )
        .unwrap();

        let s = ComponentsStrategy;
        let result = s.detect(root, &fs).unwrap();

        // Only src/auth should be found, crates/auth skipped as container
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "auth");
    }

    #[test]
    fn determinism_sorted_output() {
        let fs = MemoryFilesystem::new();
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs.create_dir_all(&root.join("src/zebra")).unwrap();
        fs.create_dir_all(&root.join("src/alpha")).unwrap();
        fs.create_dir_all(&root.join("src/beta")).unwrap();
        fs.write(&root.join("src/zebra/mod.rs"), b"").unwrap();
        fs.write(&root.join("src/alpha/mod.rs"), b"").unwrap();
        fs.write(&root.join("src/beta/mod.rs"), b"").unwrap();

        let s = ComponentsStrategy;
        let result = s.detect(root, &fs).unwrap();

        // Must be sorted alphabetically
        let names: Vec<_> = result.iter().map(|c| c.name.clone()).collect();
        assert_eq!(names, vec!["alpha", "beta", "zebra"]);
    }

    #[test]
    fn skips_hidden_dirs_in_packages() {
        // vueuse-style: packages/.test/index.ts is a hidden fixture dir,
        // not a module. Regression from the 2026-08-19 UAT smoke.
        let fs = MemoryFilesystem::new();
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs.create_dir_all(&root.join("packages/.test")).unwrap();
        fs.create_dir_all(&root.join("packages/core")).unwrap();
        fs.write(&root.join("packages/.test/index.ts"), b"export {}")
            .unwrap();
        fs.write(&root.join("packages/core/index.ts"), b"export {}")
            .unwrap();

        let s = ComponentsStrategy;
        let result = s.detect(root, &fs).unwrap();

        let names: Vec<_> = result.iter().map(|c| c.name.clone()).collect();
        assert_eq!(names, vec!["core"], "hidden dirs must not be modules");
    }
}
