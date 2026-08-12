use anyhow::{Context, Result};
use cargo_metadata::{Metadata, MetadataCommand};
use ignore::{DirEntry, WalkBuilder};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Directories to prune unconditionally during tree walks — independent of
/// `.gitignore`. Exact-name match on the directory component; files with
/// the same name are preserved. `vendor` intentionally excluded (Go source).
pub static BUILD_DIR_BLOCKLIST: &[&str] = &[
    "target",
    "node_modules",
    "dist",
    "build",
    ".git",
    ".venv",
    "__pycache__",
    ".gradle",
];

/// One file or directory entry in the project tree, with the smallest
/// set of fields the agents actually consume (relative path, kind,
/// size, language for files).
#[derive(Debug, Serialize)]
pub struct Entry {
    pub path: String,
    pub kind: EntryKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    File,
    Dir,
}

/// Walk the project tree, respecting `.gitignore` / `.ignore` /
/// global ignores. Hard cap on entries protects against pathologically
/// large repos (node_modules with millions of files, etc.).
pub fn tree(root: &Path, max_depth: Option<usize>, max_entries: usize) -> Result<Vec<Entry>> {
    let mut builder = WalkBuilder::new(root);
    builder.follow_links(false);
    builder.hidden(true);
    builder.standard_filters(true);
    builder.parents(false); // we walk a single project, don't pull .gitignore from parents
    builder.require_git(false); // archctl runs on any project, not just git repos
    if let Some(d) = max_depth {
        builder.max_depth(Some(d));
    }
    // D1: prune build directories by exact name, regardless of .gitignore.
    // Files named the same as a blocklist entry are kept; non-UTF8 names pass through.
    builder.filter_entry(|de| {
        let is_dir = de.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if !is_dir {
            return true;
        }
        let Some(name) = de.file_name().to_str() else {
            return true;
        };
        !BUILD_DIR_BLOCKLIST.contains(&name)
    });
    let walker = builder.build();

    let mut out = Vec::with_capacity(1024);
    let mut count = 0usize;
    for entry in walker {
        let entry = entry.with_context(|| format!("walk {}", root.display()))?;
        count += 1;
        if count > max_entries {
            anyhow::bail!("tree walk exceeded {max_entries} entries; narrow with --max-depth");
        }
        out.push(to_entry(&entry, root)?);
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn to_entry(de: &DirEntry, root: &Path) -> Result<Entry> {
    let path = de.path();
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    let kind = if de.file_type().map(|t| t.is_dir()).unwrap_or(false) {
        EntryKind::Dir
    } else {
        EntryKind::File
    };
    let size_bytes = if matches!(kind, EntryKind::File) {
        de.metadata().ok().map(|m| m.len())
    } else {
        None
    };
    let language = if matches!(kind, EntryKind::File) {
        detect_language(path).map(str::to_string)
    } else {
        None
    };
    Ok(Entry {
        path: rel,
        kind,
        size_bytes,
        language,
    })
}

/// Language histogram: ordered map of `language -> (file_count, byte_count)`.
/// Extensions not recognised are bucketed under `"other"`.
#[derive(Debug, Default, Serialize)]
pub struct LanguageSummary {
    pub total_files: usize,
    pub total_bytes: u64,
    pub languages: BTreeMap<String, LanguageStat>,
}

#[derive(Debug, Default, Serialize)]
pub struct LanguageStat {
    pub files: usize,
    pub bytes: u64,
}

pub fn languages(
    root: &Path,
    max_depth: Option<usize>,
    max_entries: usize,
) -> Result<LanguageSummary> {
    let entries = tree(root, max_depth, max_entries)?;
    let mut summary = LanguageSummary::default();
    for e in entries {
        if !matches!(e.kind, EntryKind::File) {
            continue;
        }
        summary.total_files += 1;
        let bytes = e.size_bytes.unwrap_or(0);
        summary.total_bytes += bytes;
        let bucket = e.language.unwrap_or_else(|| "other".to_string());
        let stat = summary.languages.entry(bucket).or_default();
        stat.files += 1;
        stat.bytes += bytes;
    }
    Ok(summary)
}

/// Walk the project and yield every file with a recognised extension
/// under one of the supported ast-grep languages. Used by evidence
/// extraction to know which files to parse.
pub fn supported_files(root: &Path, max_entries: usize) -> Result<Vec<(PathBuf, &'static str)>> {
    let entries = tree(root, None, max_entries)?;
    let mut out = Vec::new();
    for e in entries {
        if !matches!(e.kind, EntryKind::File) {
            continue;
        }
        if let Some(lang) = detect_language(Path::new(&e.path))
            && crate::astgrep::Lang::from_label(lang).is_some()
        {
            out.push((PathBuf::from(e.path), lang));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// Map file extension -> canonical language label. Returns `None` for
/// extensions we don't recognise. Used by both the language histogram
/// and the evidence extractor (which only walks languages that ast-grep
/// can parse via the Lang enum).
pub fn detect_language(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let lang = match name.as_str() {
        "dockerfile" => "dockerfile",
        "makefile" => "makefile",
        "cmakelists.txt" => "cmake",
        _ => match ext.as_deref()? {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" | "mjs" | "cjs" => "javascript",
            "py" | "pyi" => "python",
            "go" => "go",
            "java" => "java",
            "kt" | "kts" => "kotlin",
            "rb" => "ruby",
            "cs" => "csharp",
            "c" | "h" => "c",
            "cpp" | "cxx" | "cc" | "hpp" | "hxx" | "hh" => "cpp",
            "m" | "mm" => "objective-c",
            "swift" => "swift",
            "scala" | "sc" => "scala",
            "clj" | "cljs" | "cljc" => "clojure",
            "ex" | "exs" => "elixir",
            "erl" | "hrl" => "erlang",
            "hs" => "haskell",
            "ml" | "mli" => "ocaml",
            "fs" | "fsx" => "fsharp",
            "lua" => "lua",
            "pl" | "pm" => "perl",
            "php" => "php",
            "sh" | "bash" => "shell",
            "sql" => "sql",
            "yaml" | "yml" => "yaml",
            "json" => "json",
            "toml" => "toml",
            "xml" => "xml",
            "html" | "htm" => "html",
            "css" => "css",
            "scss" => "scss",
            "sass" => "sass",
            "md" | "markdown" => "markdown",
            "vue" => "vue",
            "svelte" => "svelte",
            "dart" => "dart",
            "zig" => "zig",
            "nim" => "nim",
            "cr" => "crystal",
            "r" => "r",
            "jl" => "julia",
            "proto" => "protobuf",
            "graphql" | "gql" => "graphql",
            _ => return None,
        },
    };
    Some(lang)
}

pub fn walk_to_paths(entries: Vec<Entry>) -> Vec<PathBuf> {
    entries
        .into_iter()
        .filter(|e| matches!(e.kind, EntryKind::File))
        .map(|e| PathBuf::from(e.path))
        .collect()
}

/// Deterministic manifest discovery at depth ≤ max_depth. Returns RELATIVE
/// PathBuf (consistent with supported_files / walk_to_paths), sorted
/// lexicographically, deduped. Reuses tree() so it inherits D1 pruning
/// (blocklist of build directories). Symlinks are not followed (tree sets
/// follow_links(false)).
pub fn find_manifests(root: &Path, names: &[&str], max_depth: usize) -> Result<Vec<PathBuf>> {
    let entries = tree(root, Some(max_depth), 50_000)?;
    let mut results: Vec<PathBuf> = entries
        .into_iter()
        .filter(|e| {
            if !matches!(e.kind, EntryKind::File) {
                return false;
            }
            let file_name = Path::new(&e.path).file_name().and_then(|n| n.to_str());
            names.iter().any(|name| file_name == Some(name))
        })
        .map(|e| PathBuf::from(e.path))
        .collect();
    results.sort();
    results.dedup();
    Ok(results)
}

/// Resolve Cargo dependencies for a workspace member via cargo_metadata.
/// Returns metadata for all packages in the workspace; use `package_filter`
/// to select a specific member.
pub fn depends(manifest_path: Option<&Path>) -> Result<Metadata> {
    let mut cmd = MetadataCommand::new();
    if let Some(p) = manifest_path {
        cmd.manifest_path(p);
    }
    let metadata = cmd
        .exec()
        .context("cargo_metadata exec failed — is this a Cargo project?")?;
    Ok(metadata)
}

/// Dependency summary for a single package: name, version, and whether it is a dev/build dependency.
#[derive(Debug, Serialize)]
pub struct DepInfo {
    pub name: String,
    pub version: String,
    pub kind: DepKind,
}

/// Kind of dependency as declared in Cargo.toml.
#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum DepKind {
    Normal,
    Dev,
    Build,
}

impl From<cargo_metadata::DependencyKind> for DepKind {
    fn from(kind: cargo_metadata::DependencyKind) -> Self {
        match kind {
            cargo_metadata::DependencyKind::Development => DepKind::Dev,
            cargo_metadata::DependencyKind::Build => DepKind::Build,
            _ => DepKind::Normal,
        }
    }
}

/// Collect all dependencies (normal + dev + build) for every package in the workspace.
pub fn depends_summary(manifest_path: Option<&Path>) -> Result<Vec<DepInfo>> {
    let metadata = depends(manifest_path)?;
    let mut out = Vec::new();
    for package in &metadata.packages {
        for dep in &package.dependencies {
            out.push(DepInfo {
                name: dep.name.clone(),
                version: dep.req.to_string(),
                kind: dep.kind.into(),
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out.dedup_by(|a, b| a.name == b.name && a.kind == b.kind);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src/lib.rs"), "pub fn x() {}").unwrap();
        std::fs::write(tmp.path().join("src/app.py"), "def x(): pass").unwrap();
        std::fs::write(tmp.path().join("src/Foo.java"), "class Foo {}").unwrap();
        std::fs::write(tmp.path().join("README.md"), "# proj").unwrap();
        std::fs::create_dir_all(tmp.path().join("node_modules/lib")).unwrap();
        std::fs::write(tmp.path().join("node_modules/lib/index.js"), "// noise").unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "node_modules/\ntarget/\n").unwrap();
        tmp
    }

    #[test]
    fn tree_respects_gitignore() {
        let tmp = fixture();
        let entries = tree(tmp.path(), None, 10_000).unwrap();
        let paths: Vec<_> = entries.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.iter().any(|p| p.contains("src/lib.rs")));
        assert!(paths.iter().any(|p| p.contains("src/app.py")));
        assert!(
            !paths.iter().any(|p| p.contains("node_modules")),
            "node_modules should be ignored: {paths:?}"
        );
    }

    #[test]
    fn tree_max_depth_limits_recursion() {
        let tmp = fixture();
        let entries = tree(tmp.path(), Some(1), 10_000).unwrap();
        assert!(entries.iter().all(|e| !e.path.contains('/')));
    }

    #[test]
    fn languages_counts_by_extension() {
        let tmp = fixture();
        let summary = languages(tmp.path(), None, 10_000).unwrap();
        assert_eq!(summary.languages.get("rust").map(|s| s.files), Some(1));
        assert_eq!(summary.languages.get("python").map(|s| s.files), Some(1));
        assert_eq!(summary.languages.get("java").map(|s| s.files), Some(1));
        assert_eq!(summary.languages.get("markdown").map(|s| s.files), Some(1));
        assert!(
            !summary.languages.contains_key("javascript"),
            "node_modules ignored"
        );
    }

    #[test]
    fn supported_files_filters_to_ast_grep_languages() {
        let tmp = fixture();
        let files = supported_files(tmp.path(), 10_000).unwrap();
        let paths: Vec<_> = files.iter().map(|(p, _)| p.display().to_string()).collect();
        assert!(paths.iter().any(|p| p.ends_with("lib.rs")));
        assert!(paths.iter().any(|p| p.ends_with("app.py")));
        assert!(paths.iter().any(|p| p.ends_with("Foo.java")));
        // README.md is markdown but markdown isn't in Lang registry.
        assert!(!paths.iter().any(|p| p.ends_with("README.md")));
    }

    #[test]
    fn detect_language_known_extensions() {
        assert_eq!(detect_language(Path::new("foo.rs")), Some("rust"));
        assert_eq!(detect_language(Path::new("Foo.TS")), Some("typescript"));
        assert_eq!(detect_language(Path::new("Foo.java")), Some("java"));
        assert_eq!(detect_language(Path::new("Foo.kt")), Some("kotlin"));
        assert_eq!(detect_language(Path::new("Dockerfile")), Some("dockerfile"));
        assert_eq!(detect_language(Path::new("unknown.xyz")), None);
    }

    #[test]
    fn detect_language_path_without_extension() {
        assert_eq!(detect_language(Path::new("README")), None);
        assert_eq!(detect_language(Path::new("/abs/path/no-ext")), None);
    }

    // ─── D1: build-dir pruning ───────────────────────────────────────────────

    #[test]
    fn tree_prunes_build_dirs_without_gitignore() {
        // Scenario: root has a target/ dir with 1000 files but no .gitignore
        // that lists target/. Walker must prune it entirely.
        let tmp = tempfile::tempdir().unwrap();
        // Create src/ first so the source file can be written
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        // Populate target/ with many files (simulate pathological case)
        let target_dir = tmp.path().join("target");
        std::fs::create_dir_all(&target_dir).unwrap();
        for i in 0..100 {
            std::fs::write(target_dir.join(format!("file_{}.rs", i)), "fn x() {}").unwrap();
        }
        // Also add a real source file that must still appear
        std::fs::write(tmp.path().join("src/lib.rs"), "pub fn x() {}").unwrap();

        let entries = tree(tmp.path(), None, 10_000).unwrap();
        let paths: Vec<_> = entries.iter().map(|e| e.path.as_str()).collect();
        assert!(
            !paths.iter().any(|p| p.contains("target/")),
            "target/ should be pruned: {paths:?}"
        );
        assert!(
            paths.iter().any(|p| p.contains("src/lib.rs")),
            "src/lib.rs should still appear"
        );
    }

    #[test]
    fn tree_does_not_prune_vendor() {
        // vendor/ is Go source and must NOT be pruned (explicit design decision).
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("vendor/github.com/user/lib")).unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(
            tmp.path().join("vendor/github.com/user/lib/lib.go"),
            "package lib",
        )
        .unwrap();
        std::fs::write(tmp.path().join("src/main.go"), "package main").unwrap();

        let entries = tree(tmp.path(), None, 10_000).unwrap();
        let paths: Vec<_> = entries.iter().map(|e| e.path.as_str()).collect();
        assert!(
            paths.iter().any(|p| p.contains("vendor/")),
            "vendor/ should NOT be pruned: {paths:?}"
        );
        assert!(
            paths.iter().any(|p| p.contains("src/main.go")),
            "src/main.go should appear"
        );
    }

    #[test]
    fn tree_prunes_build_dir_named_source() {
        // Documented limitation: a directory literally named "build/" that holds
        // source code is indistinguishable from a build output dir and is pruned.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("build/src")).unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("build/src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(tmp.path().join("src/app.py"), "def x(): pass").unwrap();

        let entries = tree(tmp.path(), None, 10_000).unwrap();
        let paths: Vec<_> = entries.iter().map(|e| e.path.as_str()).collect();
        assert!(
            !paths.iter().any(|p| p.contains("build/")),
            "build/ source dir should be pruned (limitation): {paths:?}"
        );
        assert!(
            paths.iter().any(|p| p.contains("src/app.py")),
            "src/app.py should appear"
        );
    }

    // ─── D2a: find_manifests helper ─────────────────────────────────────────

    #[test]
    fn find_manifests_discovers_nested() {
        // Find Cargo.toml nested at depth 2 (archctl/Cargo.toml from repo root).
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("libs/auth/src")).unwrap();
        std::fs::write(
            tmp.path().join("libs/auth/Cargo.toml"),
            "[package]\nname = \"auth\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src/lib.rs"), "pub fn x() {}").unwrap();

        let manifests = find_manifests(tmp.path(), &["Cargo.toml"], 3).unwrap();
        assert!(
            manifests
                .iter()
                .any(|p| p.ends_with("libs/auth/Cargo.toml")),
            "nested Cargo.toml should be found: {manifests:?}"
        );
    }

    #[test]
    fn find_manifests_excludes_deep_manifests() {
        // Manifest at depth 5 must be excluded when max_depth=3.
        let tmp = tempfile::tempdir().unwrap();
        // depth 5: crates/inner/team/work/pkg/Cargo.toml
        std::fs::create_dir_all(tmp.path().join("crates/inner/team/work/pkg")).unwrap();
        std::fs::write(
            tmp.path().join("crates/inner/team/work/pkg/Cargo.toml"),
            "[package]\nname = \"pkg\"\n",
        )
        .unwrap();
        // depth 2: libs/auth/Cargo.toml (should be found)
        std::fs::create_dir_all(tmp.path().join("libs/auth/src")).unwrap();
        std::fs::write(
            tmp.path().join("libs/auth/Cargo.toml"),
            "[package]\nname = \"auth\"\n",
        )
        .unwrap();

        let manifests = find_manifests(tmp.path(), &["Cargo.toml"], 3).unwrap();
        assert!(
            manifests
                .iter()
                .any(|p| p.ends_with("libs/auth/Cargo.toml")),
            "depth-2 manifest should be found"
        );
        assert!(
            !manifests
                .iter()
                .any(|p| p.ends_with("crates/inner/team/work/pkg/Cargo.toml")),
            "depth-5 manifest should be excluded: {manifests:?}"
        );
    }

    #[test]
    fn find_manifests_sorted_and_deduped() {
        // Results must be sorted lexicographically and deduped.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("libs/b")).unwrap();
        std::fs::create_dir_all(tmp.path().join("libs/a")).unwrap();
        std::fs::create_dir_all(tmp.path().join("libs/a")).unwrap(); // duplicate dir
        std::fs::write(
            tmp.path().join("libs/a/Cargo.toml"),
            "[package]\nname = \"a\"\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("libs/b/Cargo.toml"),
            "[package]\nname = \"b\"\n",
        )
        .unwrap();

        let manifests = find_manifests(tmp.path(), &["Cargo.toml"], 5).unwrap();
        let display: Vec<_> = manifests.iter().map(|p| p.display().to_string()).collect();
        assert_eq!(
            display,
            sorted_display(&display),
            "should be sorted: {display:?}"
        );
        // No duplicates
        assert_eq!(manifests.len(), 2, "should be deduped: {manifests:?}");
    }

    #[test]
    fn find_manifests_empty_when_no_manifest() {
        // Returns empty vec, no error, when no manifest is present.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src/lib.rs"), "pub fn x() {}").unwrap();

        let manifests = find_manifests(tmp.path(), &["Cargo.toml"], 3).unwrap();
        assert!(
            manifests.is_empty(),
            "should be empty when no manifest: {manifests:?}"
        );
    }

    // Helper for sorted comparison
    fn sorted_display(paths: &[String]) -> Vec<String> {
        let mut sorted = paths.to_vec();
        sorted.sort();
        sorted
    }
}
