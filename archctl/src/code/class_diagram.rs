//! UML class-diagram extraction engine + report types.
//!
//! Direct tree-sitter CST walk per language (Rust, TypeScript, Python).
//! Populates `uml.*` MetaTypes already declared in `metamodel-core.json`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use ast_grep_language::SupportLang;
use serde::{Deserialize, Serialize};
use tree_sitter::{Parser, Tree};

use crate::filesystem::Filesystem;

/// JSON Schema for ClassDiagramReport (JSON Schema 2020-12).
pub const CLASS_DIAGRAM_REPORT_SCHEMA: &str =
    include_str!("../../../schemas/class-diagram-report.schema.json");

// ─── Carrier types ────────────────────────────────────────────────────────────

/// Kind of a UML type declaration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum TypeKind {
    Class,
    Interface,
    Trait,
    Enum,
    Record,
}

/// A type declaration (class, interface, trait, enum, record).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassNode {
    /// `<lang>:<file>:<kind>:<name>:<line>`
    pub canonical_key: String,
    pub kind: TypeKind,
    pub language: Language,
    /// Relative to --cwd
    pub file: String,
    /// 1-based declaration line
    pub line: u32,
    pub name: String,
    /// Methods + fields (unsorted)
    pub members: Vec<ClassMember>,
    /// 0.90 | 0.85 | 0.80
    pub confidence: f64,
}

/// A method or field declared inside a type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassMember {
    pub name: String,
    /// e.g. "fn", "field"
    pub member_kind: String,
    /// Signature or type annotation (for display)
    pub signature: String,
    /// 1-based line
    pub line: u32,
}

/// Relationship between two types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassEdge {
    /// `<lang>:<file>:<src>→<pred>→<tgt>:<line>`
    pub canonical_key: String,
    pub source: String,
    pub target: String,
    pub predicate: ClassEdgeKind,
    /// Relative to --cwd
    pub file: String,
    /// 1-based
    pub line: u32,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum ClassEdgeKind {
    Extends,
    Implements,
    Composes,
}

/// Source language.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    TypeScript,
    Python,
}

impl Language {
    fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "rs" => Some(Language::Rust),
            "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => Some(Language::TypeScript),
            "py" => Some(Language::Python),
            _ => None,
        }
    }
}

/// Per-project metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectMeta {
    pub root: String,
    #[serde(rename = "filesScanned")]
    pub files_scanned: u64,
    pub languages: BTreeMap<String, u64>,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
}

/// One per-file failure (graceful degradation). CLI exits 0.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractError {
    pub path: String,
    pub message: String,
}

/// Top-level report emitted by `archctl code class-diagram --json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassDiagramReport {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub project: ProjectMeta,
    pub nodes: Vec<ClassNode>,
    pub edges: Vec<ClassEdge>,
    pub errors: Vec<ExtractError>,
}

/// Report from a successful `--apply` run.
#[derive(Debug, Clone, Serialize)]
pub struct ApplyReport {
    pub elements_written: usize,
    pub elements_skipped: usize,
    pub relations_written: usize,
    pub relations_skipped: usize,
    pub evidences_written: usize,
    pub source_artifacts_written: usize,
    pub seed_writes: usize,
    pub duration_ms: u64,
}

/// Errors during extraction or graph write.
#[derive(Debug, thiserror::Error)]
pub enum ClassDiagramError {
    #[error("graph write failed: {0}")]
    GraphWrite(#[from] anyhow::Error),
}

// ─── Options ─────────────────────────────────────────────────────────────────

/// Options for class-diagram extraction.
#[derive(Debug, Clone)]
pub struct ClassDiagramOptions {
    /// Language filter (empty = all MVP languages).
    pub languages: Vec<Language>,
    /// Optional file-path selector prefix (e.g. `file:src/main.rs`).
    pub selector: Option<String>,
}

// ─── Stub ─────────────────────────────────────────────────────────────────────

/// Extract class-diagram from `cwd`. Pure: no graph writes. Deterministic.
pub fn run_class_diagram(
    cwd: &Path,
    opts: &ClassDiagramOptions,
    fs: &dyn Filesystem,
) -> Result<ClassDiagramReport, ClassDiagramError> {
    let _start = Instant::now();
    let _root = cwd.to_string_lossy().to_string();
    // TODO: implement extraction
    Ok(ClassDiagramReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root: _root,
            files_scanned: 0,
            languages: BTreeMap::new(),
            duration_ms: 0,
        },
        nodes: Vec::new(),
        edges: Vec::new(),
        errors: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_version() {
        assert_eq!(CLASS_DIAGRAM_REPORT_SCHEMA.contains("schemaVersion"), true);
    }

    #[test]
    fn test_type_kind_ordering() {
        use std::cmp::Ordering;
        assert_eq!(TypeKind::Class.cmp(&TypeKind::Interface), Ordering::Less);
        assert_eq!(TypeKind::Enum.cmp(&TypeKind::Trait), Ordering::Greater);
    }

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("go"), None);
    }
}
