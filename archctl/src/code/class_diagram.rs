//! UML class-diagram extraction engine + report types.
//!
//! Direct tree-sitter CST walk per language (Rust, TypeScript, Python).
//! Populates `uml.*` MetaTypes already declared in `metamodel-core.json`.
//!
//! class-diagram projection deterministic (golden test)
//! ADR-019 class-diagram p99 < 2s for < 10k nodes (bench)

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use ast_grep_core::tree_sitter::LanguageExt;
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
    fn lang_label(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::TypeScript => "typescript",
            Language::Python => "python",
        }
    }

    #[allow(dead_code)]
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
    #[error("unknown selector: {0} — supported forms: file:<path>")]
    UnknownSelector(String),
    #[error("file not found: {0}")]
    FileNotFound(String),
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

// ─── Extraction ───────────────────────────────────────────────────────────────

/// Extract class-diagram from `cwd`. Pure: no graph writes. Deterministic.
pub fn run_class_diagram(
    cwd: &Path,
    opts: &ClassDiagramOptions,
    fs: &dyn Filesystem,
) -> Result<ClassDiagramReport, ClassDiagramError> {
    let root = cwd.to_string_lossy().to_string();

    // Validate selector before doing any work
    if let Some(ref sel) = opts.selector {
        if let Some(path) = sel.strip_prefix("file:") {
            // Validate the selected file exists
            let abs_path = cwd.join(path);
            if !abs_path.is_file() {
                return Err(ClassDiagramError::FileNotFound(path.to_string()));
            }
        } else {
            // Unknown selector prefix — only `file:` is supported
            return Err(ClassDiagramError::UnknownSelector(sel.clone()));
        }
    }

    // Collect files to process
    let mut files_to_process: Vec<(PathBuf, Language)> = Vec::new();
    let mut lang_counts: BTreeMap<String, u64> = BTreeMap::new();

    let walker = ignore::WalkBuilder::new(cwd)
        .hidden(false)
        .follow_links(false)
        .build();

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension() else {
            continue;
        };
        let ext_str = ext.to_string_lossy().to_lowercase();
        let lang = match ext_str.as_str() {
            "rs" => Language::Rust,
            "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => Language::TypeScript,
            "py" => Language::Python,
            _ => continue,
        };
        if !opts.languages.is_empty() && !opts.languages.contains(&lang) {
            continue;
        }
        let rel = path.strip_prefix(cwd).unwrap_or(path);
        let rel_str = rel.to_string_lossy().to_string();

        // Apply file: selector filter
        if let Some(ref sel) = opts.selector
            && let Some(sel_path) = sel.strip_prefix("file:")
            && rel_str != sel_path
        {
            continue;
        }

        files_to_process.push((rel.to_path_buf(), lang));
        let label = lang.lang_label();
        *lang_counts.entry(label.to_string()).or_insert(0) += 1;
    }

    let mut all_nodes: Vec<ClassNode> = Vec::new();
    let mut all_edges: Vec<ClassEdge> = Vec::new();
    let mut errors: Vec<ExtractError> = Vec::new();
    let files_scanned = files_to_process.len() as u64;

    for (rel_path, lang) in &files_to_process {
        let abs_path = cwd.join(rel_path);
        let source = match fs.read_to_string(&abs_path) {
            Ok(s) => s,
            Err(e) => {
                errors.push(ExtractError {
                    path: rel_path.to_string_lossy().to_string(),
                    message: format!("read error: {e}"),
                });
                continue;
            }
        };

        let support_lang = match lang {
            Language::Rust => SupportLang::Rust,
            Language::TypeScript => SupportLang::TypeScript,
            Language::Python => SupportLang::Python,
        };

        let ts_lang = support_lang.get_ts_language();
        let mut parser = Parser::new();
        if parser.set_language(&ts_lang).is_err() {
            errors.push(ExtractError {
                path: rel_path.to_string_lossy().to_string(),
                message: "failed to set tree-sitter language".to_string(),
            });
            continue;
        }
        let tree = match parser.parse(&source, None) {
            Some(t) => t,
            None => {
                errors.push(ExtractError {
                    path: rel_path.to_string_lossy().to_string(),
                    message: "parse failed".to_string(),
                });
                continue;
            }
        };

        let file_str = rel_path.to_string_lossy().to_string();

        match lang {
            Language::Rust => {
                extract_rust(&tree, &source, &file_str, &mut all_nodes, &mut all_edges)
            }
            Language::TypeScript => {
                extract_typescript(&tree, &source, &file_str, &mut all_nodes, &mut all_edges)
            }
            Language::Python => {
                extract_python(&tree, &source, &file_str, &mut all_nodes, &mut all_edges)
            }
        }
    }

    // Extract edges (same-file resolution)
    extract_edges(&all_nodes, &mut all_edges);

    // Sort for determinism
    all_nodes.sort_by(|a, b| a.canonical_key.cmp(&b.canonical_key));
    all_edges.sort_by(|a, b| a.canonical_key.cmp(&b.canonical_key));

    Ok(ClassDiagramReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root,
            files_scanned,
            languages: lang_counts,
        },
        nodes: all_nodes,
        edges: all_edges,
        errors,
    })
}

// ─── Rust extractor ────────────────────────────────────────────────────────────

fn extract_rust(
    tree: &Tree,
    source: &str,
    file: &str,
    nodes: &mut Vec<ClassNode>,
    edges: &mut Vec<ClassEdge>,
) {
    let root = tree.root_node();
    find_rust_types(root, source, file, nodes, edges);
}

fn find_rust_types<'tree>(
    node: tree_sitter::Node<'tree>,
    source: &str,
    file: &str,
    nodes: &mut Vec<ClassNode>,
    edges: &mut Vec<ClassEdge>,
) {
    let kind = node.kind();

    match kind {
        "struct_item" => {
            if let Some(class_node) = extract_rust_struct(node, source, file) {
                nodes.push(class_node);
            }
            return;
        }
        "enum_item" => {
            if let Some(class_node) = extract_rust_enum(node, source, file) {
                nodes.push(class_node);
            }
            return;
        }
        "trait_item" => {
            if let Some(class_node) = extract_rust_trait(node, source, file) {
                nodes.push(class_node);
            }
            return;
        }
        "impl_item" => {
            // Collect impl methods and check for Trait for Type pattern
            let mut members = Vec::new();
            let mut trait_name: Option<String> = None;
            let mut impl_type_name: Option<String> = None;

            for i in 0..node.child_count() {
                if let Some(child) = node.child(i as u32) {
                    match child.kind() {
                        "type_declaration" => {
                            // `impl Trait for Type`
                            for j in 0..child.child_count() {
                                if let Some(inner) = child.child(j as u32)
                                    && inner.kind() == "type_identifier"
                                {
                                    impl_type_name = Some(
                                        source[inner.start_byte()..inner.end_byte()].to_string(),
                                    );
                                }
                            }
                        }
                        "identifier" => {
                            trait_name =
                                Some(source[child.start_byte()..child.end_byte()].to_string());
                        }
                        "declaration" => {
                            // Skip trait bounds in impl
                        }
                        "function_item" => {
                            if let Some(member) = extract_rust_method(child, source) {
                                members.push(member);
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Extract impl_item body methods
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i as u32)
                    && child.kind() == "declaration"
                {
                    for j in 0..child.child_count() {
                        if let Some(fn_item) = child.child(j as u32)
                            && fn_item.kind() == "function_item"
                            && let Some(member) = extract_rust_method(fn_item, source)
                        {
                            members.push(member);
                        }
                    }
                }
            }

            // Emit implements edge if impl Trait for Type
            let impl_trait_line = (node.start_position().row as u32) + 1;
            if let (Some(trait_n), Some(type_n)) = (trait_name, impl_type_name.clone()) {
                let target_key = format!("rust:{}:trait:{}:0", file, trait_n);
                let source_key = format!("rust:{}:class:{}:0", file, type_n);
                // Only emit if both exist (deferred to edge resolution pass)
                edges.push(ClassEdge {
                    canonical_key: format!(
                        "rust:{}:{}→implements→{}:{}",
                        file, source_key, target_key, impl_trait_line
                    ),
                    source: source_key,
                    target: target_key,
                    predicate: ClassEdgeKind::Implements,
                    file: file.to_string(),
                    line: impl_trait_line,
                    confidence: 0.85,
                });
            }

            // Emit a synthetic impl node if we have members but no struct/enum
            // (standalone impl block — not a class per se, but included for completeness)
            if !members.is_empty() {
                let line = (node.start_position().row + 1) as u32;
                let name = impl_type_name.unwrap_or_else(|| format!("impl@{}", line));
                let canonical_key = format!("rust:{}:impl:{}:{}", file, name, line);
                nodes.push(ClassNode {
                    canonical_key,
                    kind: TypeKind::Class,
                    language: Language::Rust,
                    file: file.to_string(),
                    line,
                    name,
                    members,
                    confidence: 0.80,
                });
            }
            return;
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            find_rust_types(child, source, file, nodes, edges);
        }
    }
}

fn extract_rust_struct(node: tree_sitter::Node, source: &str, file: &str) -> Option<ClassNode> {
    let mut name = String::new();
    let mut members = Vec::new();
    let line = (node.start_position().row + 1) as u32;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            match child.kind() {
                "visibility_modifier" => {}
                "type_identifier" | "identifier" => {
                    if name.is_empty() {
                        name = source[child.start_byte()..child.end_byte()].to_string();
                    }
                }
                "field_declaration_list" => {
                    for j in 0..child.child_count() {
                        if let Some(field) = child.child(j as u32)
                            && field.kind() == "field_declaration"
                        {
                            let mut field_name = String::new();
                            let mut sig = String::new();
                            for k in 0..field.child_count() {
                                if let Some(fc) = field.child(k as u32) {
                                    let txt = source[fc.start_byte()..fc.end_byte()].to_string();
                                    if fc.kind() == "identifier" && field_name.is_empty() {
                                        field_name = txt.clone();
                                    }
                                    sig.push_str(&txt);
                                    sig.push(' ');
                                }
                            }
                            let fline = (field.start_position().row + 1) as u32;
                            members.push(ClassMember {
                                name: field_name,
                                member_kind: "field".to_string(),
                                signature: sig.trim().to_string(),
                                line: fline,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if name.is_empty() {
        return None;
    }

    let canonical_key = format!("rust:{}:class:{}:{}", file, name, line);
    Some(ClassNode {
        canonical_key,
        kind: TypeKind::Class,
        language: Language::Rust,
        file: file.to_string(),
        line,
        name,
        members,
        confidence: 0.90,
    })
}

fn extract_rust_enum(node: tree_sitter::Node, source: &str, file: &str) -> Option<ClassNode> {
    let mut name = String::new();
    let line = (node.start_position().row + 1) as u32;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            match child.kind() {
                "visibility_modifier" => {}
                "identifier" if name.is_empty() => {
                    name = source[child.start_byte()..child.end_byte()].to_string();
                }
                _ => {}
            }
        }
    }

    if name.is_empty() {
        return None;
    }

    let canonical_key = format!("rust:{}:enum:{}:{}", file, name, line);
    Some(ClassNode {
        canonical_key,
        kind: TypeKind::Enum,
        language: Language::Rust,
        file: file.to_string(),
        line,
        name,
        members: Vec::new(),
        confidence: 0.90,
    })
}

fn extract_rust_trait(node: tree_sitter::Node, source: &str, file: &str) -> Option<ClassNode> {
    let mut name = String::new();
    let line = (node.start_position().row + 1) as u32;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            match child.kind() {
                "visibility_modifier" => {}
                "identifier" if name.is_empty() => {
                    name = source[child.start_byte()..child.end_byte()].to_string();
                }
                _ => {}
            }
        }
    }

    if name.is_empty() {
        return None;
    }

    let canonical_key = format!("rust:{}:trait:{}:{}", file, name, line);
    Some(ClassNode {
        canonical_key,
        kind: TypeKind::Trait,
        language: Language::Rust,
        file: file.to_string(),
        line,
        name,
        members: Vec::new(),
        confidence: 0.90,
    })
}

fn extract_rust_method(node: tree_sitter::Node, source: &str) -> Option<ClassMember> {
    let mut name = String::new();
    let mut sig_parts = Vec::new();
    let line = (node.start_position().row + 1) as u32;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            let txt = source[child.start_byte()..child.end_byte()].to_string();
            if child.kind() == "identifier" && name.is_empty() {
                name = txt.clone();
            }
            sig_parts.push(txt);
        }
    }

    if name.is_empty() {
        return None;
    }

    Some(ClassMember {
        name,
        member_kind: "fn".to_string(),
        signature: sig_parts.join(" ").replace("  ", " ").trim().to_string(),
        line,
    })
}

// ─── TypeScript extractor ───────────────────────────────────────────────────────

fn extract_typescript(
    tree: &Tree,
    source: &str,
    file: &str,
    nodes: &mut Vec<ClassNode>,
    edges: &mut Vec<ClassEdge>,
) {
    let root = tree.root_node();
    find_ts_types(root, source, file, nodes, edges);
}

fn find_ts_types<'tree>(
    node: tree_sitter::Node<'tree>,
    source: &str,
    file: &str,
    nodes: &mut Vec<ClassNode>,
    edges: &mut Vec<ClassEdge>,
) {
    let kind = node.kind();

    match kind {
        "class_declaration" => {
            if let Some(class_node) = extract_ts_class(node, source, file, edges) {
                nodes.push(class_node);
            }
            return;
        }
        "interface_declaration" => {
            if let Some(iface_node) = extract_ts_interface(node, source, file) {
                nodes.push(iface_node);
            }
            return;
        }
        "enum_declaration" => {
            if let Some(enum_node) = extract_ts_enum(node, source, file) {
                nodes.push(enum_node);
            }
            return;
        }
        "type_alias_declaration" => {
            if let Some(record_node) = extract_ts_type_alias(node, source, file) {
                nodes.push(record_node);
            }
            return;
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            find_ts_types(child, source, file, nodes, edges);
        }
    }
}

fn extract_ts_class(
    node: tree_sitter::Node,
    source: &str,
    file: &str,
    edges: &mut Vec<ClassEdge>,
) -> Option<ClassNode> {
    let mut name = String::new();
    let mut members = Vec::new();
    let line = (node.start_position().row + 1) as u32;
    let mut extends_name: Option<String> = None;
    let mut implements_names: Vec<String> = Vec::new();

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            match child.kind() {
                "identifier" | "type_identifier" => {
                    if name.is_empty() {
                        name = source[child.start_byte()..child.end_byte()].to_string();
                    }
                }
                "class_heritage" => {
                    // class A extends B implements C, D
                    for j in 0..child.child_count() {
                        if let Some(hc) = child.child(j as u32) {
                            match hc.kind() {
                                "extends_clause" => {
                                    // extends_clause has: "extends" keyword + identifier
                                    for k in 0..hc.child_count() {
                                        if let Some(id_node) = hc.child(k as u32)
                                            && id_node.kind() == "identifier"
                                        {
                                            let txt = source
                                                [id_node.start_byte()..id_node.end_byte()]
                                                .to_string();
                                            if extends_name.is_none() {
                                                extends_name = Some(txt);
                                            } else {
                                                implements_names.push(txt);
                                            }
                                        }
                                    }
                                }
                                "implements_clause" => {
                                    // implements_clause has identifier(s) — TS uses type_identifier
                                    for k in 0..hc.child_count() {
                                        if let Some(id_node) = hc.child(k as u32)
                                            && (id_node.kind() == "identifier"
                                                || id_node.kind() == "type_identifier")
                                        {
                                            implements_names.push(
                                                source[id_node.start_byte()..id_node.end_byte()]
                                                    .to_string(),
                                            );
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                "method_definition" => {
                    if let Some(member) = extract_ts_method(child, source) {
                        members.push(member);
                    }
                }
                "public_field_definition" | "private_field_definition" => {
                    if let Some(field) = extract_ts_field(child, source) {
                        members.push(field);
                    }
                }
                _ => {}
            }
        }
    }

    if name.is_empty() {
        return None;
    }

    let canonical_key = format!("typescript:{}:class:{}:{}", file, name, line);

    // Emit extends edge
    if let Some(parent) = extends_name.take() {
        let parent_key = format!("typescript:{}:class:{}:0", file, parent);
        edges.push(ClassEdge {
            canonical_key: format!(
                "typescript:{}:{}→extends→{}:{}",
                file, canonical_key, parent_key, line
            ),
            source: canonical_key.clone(),
            target: parent_key,
            predicate: ClassEdgeKind::Extends,
            file: file.to_string(),
            line,
            confidence: 0.85,
        });
    }

    // Emit implements edges
    for interface in implements_names {
        let iface_key = format!("typescript:{}:interface:{}:0", file, interface);
        edges.push(ClassEdge {
            canonical_key: format!(
                "typescript:{}:{}→implements→{}:{}",
                file, canonical_key, iface_key, line
            ),
            source: canonical_key.clone(),
            target: iface_key,
            predicate: ClassEdgeKind::Implements,
            file: file.to_string(),
            line,
            confidence: 0.85,
        });
    }

    Some(ClassNode {
        canonical_key,
        kind: TypeKind::Class,
        language: Language::TypeScript,
        file: file.to_string(),
        line,
        name,
        members,
        confidence: 0.90,
    })
}

fn extract_ts_interface(node: tree_sitter::Node, source: &str, file: &str) -> Option<ClassNode> {
    let mut name = String::new();
    let line = (node.start_position().row + 1) as u32;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32)
            && (child.kind() == "identifier" || child.kind() == "type_identifier")
            && name.is_empty()
        {
            name = source[child.start_byte()..child.end_byte()].to_string();
        }
    }

    if name.is_empty() {
        return None;
    }

    let canonical_key = format!("typescript:{}:interface:{}:{}", file, name, line);
    Some(ClassNode {
        canonical_key,
        kind: TypeKind::Interface,
        language: Language::TypeScript,
        file: file.to_string(),
        line,
        name,
        members: Vec::new(),
        confidence: 0.90,
    })
}

fn extract_ts_enum(node: tree_sitter::Node, source: &str, file: &str) -> Option<ClassNode> {
    let mut name = String::new();
    let line = (node.start_position().row + 1) as u32;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32)
            && child.kind() == "identifier"
            && name.is_empty()
        {
            name = source[child.start_byte()..child.end_byte()].to_string();
        }
    }

    if name.is_empty() {
        return None;
    }

    let canonical_key = format!("typescript:{}:enum:{}:{}", file, name, line);
    Some(ClassNode {
        canonical_key,
        kind: TypeKind::Enum,
        language: Language::TypeScript,
        file: file.to_string(),
        line,
        name,
        members: Vec::new(),
        confidence: 0.90,
    })
}

fn extract_ts_type_alias(node: tree_sitter::Node, source: &str, file: &str) -> Option<ClassNode> {
    let mut name = String::new();
    let line = (node.start_position().row + 1) as u32;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32)
            && child.kind() == "type_identifier"
            && name.is_empty()
        {
            name = source[child.start_byte()..child.end_byte()].to_string();
        }
    }

    if name.is_empty() {
        return None;
    }

    let canonical_key = format!("typescript:{}:record:{}:{}", file, name, line);
    Some(ClassNode {
        canonical_key,
        kind: TypeKind::Record,
        language: Language::TypeScript,
        file: file.to_string(),
        line,
        name,
        members: Vec::new(),
        confidence: 0.85,
    })
}

fn extract_ts_method(node: tree_sitter::Node, source: &str) -> Option<ClassMember> {
    let mut name = String::new();
    let mut sig_parts = Vec::new();
    let line = (node.start_position().row + 1) as u32;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            let txt = source[child.start_byte()..child.end_byte()].to_string();
            if child.kind() == "property_identifier" && name.is_empty() {
                name = txt.clone();
            }
            sig_parts.push(txt);
        }
    }

    if name.is_empty() {
        return None;
    }

    Some(ClassMember {
        name,
        member_kind: "fn".to_string(),
        signature: sig_parts.join(" ").replace("  ", " ").trim().to_string(),
        line,
    })
}

fn extract_ts_field(node: tree_sitter::Node, source: &str) -> Option<ClassMember> {
    let mut name = String::new();
    let mut sig = String::new();
    let line = (node.start_position().row + 1) as u32;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            let txt = source[child.start_byte()..child.end_byte()].to_string();
            if child.kind() == "property_identifier" && name.is_empty() {
                name = txt.clone();
            }
            sig.push_str(&txt);
            sig.push(' ');
        }
    }

    if name.is_empty() {
        return None;
    }

    Some(ClassMember {
        name,
        member_kind: "field".to_string(),
        signature: sig.trim().to_string(),
        line,
    })
}

// ─── Python extractor ──────────────────────────────────────────────────────────

fn extract_python(
    tree: &Tree,
    source: &str,
    file: &str,
    nodes: &mut Vec<ClassNode>,
    edges: &mut Vec<ClassEdge>,
) {
    let root = tree.root_node();
    find_python_types(root, source, file, nodes, edges);
}

fn find_python_types<'tree>(
    node: tree_sitter::Node<'tree>,
    source: &str,
    file: &str,
    nodes: &mut Vec<ClassNode>,
    edges: &mut Vec<ClassEdge>,
) {
    let kind = node.kind();

    if kind == "class_definition" {
        if let Some(class_node) = extract_python_class(node, source, file, edges) {
            nodes.push(class_node);
        }
        return;
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            find_python_types(child, source, file, nodes, edges);
        }
    }
}

fn extract_python_class(
    node: tree_sitter::Node,
    source: &str,
    file: &str,
    edges: &mut Vec<ClassEdge>,
) -> Option<ClassNode> {
    let mut name = String::new();
    let mut bases: Vec<String> = Vec::new();
    let line = (node.start_position().row + 1) as u32;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            match child.kind() {
                "identifier" => {
                    if name.is_empty() {
                        name = source[child.start_byte()..child.end_byte()].to_string();
                    }
                }
                "argument_list" => {
                    for j in 0..child.child_count() {
                        if let Some(arg) = child.child(j as u32) {
                            // Only capture identifier nodes, not punctuation
                            if arg.kind() == "identifier" {
                                let base_name = source[arg.start_byte()..arg.end_byte()]
                                    .to_string()
                                    .trim()
                                    .to_string();
                                if !base_name.is_empty() && base_name != "object" {
                                    bases.push(base_name);
                                }
                            }
                        }
                    }
                }
                "block" => {
                    // Extract methods inside the class
                }
                _ => {}
            }
        }
    }

    if name.is_empty() {
        return None;
    }

    let canonical_key = format!("python:{}:class:{}:{}", file, name, line);

    // Emit extends edges for each base class
    for base in &bases {
        let base_key = format!("python:{}:class:{}:0", file, base);
        edges.push(ClassEdge {
            canonical_key: format!(
                "python:{}:{}→extends→{}:{}",
                file, canonical_key, base_key, line
            ),
            source: canonical_key.clone(),
            target: base_key,
            predicate: ClassEdgeKind::Extends,
            file: file.to_string(),
            line,
            confidence: 0.85,
        });
    }

    Some(ClassNode {
        canonical_key,
        kind: TypeKind::Class,
        language: Language::Python,
        file: file.to_string(),
        line,
        name,
        members: Vec::new(), // TODO: Python methods (block/function_definition inside class)
        confidence: 0.90,
    })
}

// ─── Edge resolution ──────────────────────────────────────────────────────────

/// Resolve same-file inheritance edges between nodes.
/// Deduces target canonical_keys using same-file convention: {lang}:{file}:{kind}:{name}:0.
fn extract_edges(nodes: &[ClassNode], edges: &mut Vec<ClassEdge>) {
    // Build lookup: name → canonical_key for same-file nodes
    let mut name_to_key: BTreeMap<(Language, &str, &str), String> = BTreeMap::new();
    for node in nodes {
        name_to_key.insert(
            (node.language, &node.file, &node.name),
            node.canonical_key.clone(),
        );
    }

    // Resolve placeholder edges (line=0) to real keys
    let mut resolved_edges: Vec<ClassEdge> = Vec::new();
    for edge in edges.iter() {
        if edge.target.ends_with(":0") && edge.source.ends_with(":0") {
            continue; // skip fully unresolved
        }

        // Parse source: extract lang, file (kind/name re-extracted from source key below)
        let parts: Vec<&str> = edge.source.split(':').collect();
        if parts.len() >= 4 {
            let lang_str = parts[0];
            let file = parts[1];
            let _kind = parts[2];
            let _name = parts[3];

            let lang = match lang_str {
                "rust" => Language::Rust,
                "typescript" => Language::TypeScript,
                "python" => Language::Python,
                _ => continue,
            };

            // Look up target by name (same file, any matching kind)
            let target_parts: Vec<&str> = edge.target.split(':').collect();
            if target_parts.len() >= 2 {
                let target_name = target_parts[1];
                if let Some(real_target) = name_to_key.get(&(lang, file, target_name)) {
                    resolved_edges.push(ClassEdge {
                        canonical_key: format!(
                            "{}:{}:{}→{}→{}:{}",
                            lang_str,
                            file,
                            edge.source.split(':').nth(3).unwrap_or(""),
                            edge.predicate_tag(),
                            real_target.split(':').nth(3).unwrap_or(""),
                            edge.line
                        ),
                        source: edge.source.clone(),
                        target: real_target.clone(),
                        predicate: edge.predicate,
                        file: file.to_string(),
                        line: edge.line,
                        confidence: edge.confidence,
                    });
                    continue;
                }
            }
        }

        // Keep unresolved edges as-is (cross-file, deferred)
        resolved_edges.push(edge.clone());
    }

    edges.clear();
    edges.extend(resolved_edges);

    // Synthesize `composes` edges from intra-file field types. Field
    // members capture their type in the signature (e.g. `pub value: Foo`);
    // if `Foo` resolves to another class in the same file, emit a
    // composes edge. Per spec scenario "intra-file composes" (M12 spec).
    emit_composes_edges(nodes, edges);
}

/// Extract the type token from a field signature like `pub value: Foo`
/// or `pub count: Option<usize>`. Returns `None` for primitives, slices,
/// arrays, references — only identifier-like types that could resolve
/// to a same-file class.
fn field_type_from_signature(sig: &str) -> Option<String> {
    let s = sig.trim();
    // Strip leading visibility/qualifier tokens: pub, pub(crate), etc.
    let s = s
        .strip_prefix("pub")
        .or_else(|| s.strip_prefix("private"))
        .unwrap_or(s)
        .trim_start();
    // Find the `:` that separates name from type
    let colon_pos = s.rfind(':')?;
    let type_part = s[colon_pos + 1..].trim();
    if type_part.is_empty() {
        return None;
    }
    // Reject primitive types and generic/slice/array/ref types — only
    // plain identifier-like type names can resolve to a class.
    if !type_part
        .chars()
        .next()
        .map(|c| c.is_ascii_alphabetic() || c == '_')
        .unwrap_or(false)
    {
        return None;
    }
    let head = type_part
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()?;
    if head.is_empty() {
        return None;
    }
    Some(head.to_string())
}

/// Scan each ClassNode's field members. If the field's type resolves
/// to another ClassNode in the same file, emit a `composes` edge.
/// Idempotent: skips if an edge already exists between the same pair.
fn emit_composes_edges(nodes: &[ClassNode], edges: &mut Vec<ClassEdge>) {
    let mut name_to_key: BTreeMap<(Language, &str, &str), String> = BTreeMap::new();
    for node in nodes {
        name_to_key.insert(
            (node.language, node.file.as_str(), node.name.as_str()),
            node.canonical_key.clone(),
        );
    }

    let mut seen: BTreeSet<(String, String)> = edges
        .iter()
        .map(|e| (e.source.clone(), e.target.clone()))
        .collect();

    for node in nodes {
        for member in &node.members {
            if member.member_kind != "field" {
                continue;
            }
            let Some(field_type) = field_type_from_signature(&member.signature) else {
                continue;
            };
            // Primitive types don't compose (would be noise)
            if is_primitive_type(&field_type) {
                continue;
            }
            let Some(target_key) =
                name_to_key.get(&(node.language, node.file.as_str(), field_type.as_str()))
            else {
                continue;
            };
            let pair = (node.canonical_key.clone(), target_key.clone());
            if seen.contains(&pair) {
                continue;
            }
            seen.insert(pair);
            edges.push(ClassEdge {
                canonical_key: format!(
                    "{}→composes→{}:{}",
                    node.canonical_key, target_key, member.line
                ),
                source: node.canonical_key.clone(),
                target: target_key.clone(),
                predicate: ClassEdgeKind::Composes,
                file: node.file.clone(),
                line: member.line,
                confidence: 0.85,
            });
        }
    }
}

/// Primitive types we explicitly skip for composes edges. Covers
/// Rust scalars + stdlib containers + common TS/Py primitives.
fn is_primitive_type(t: &str) -> bool {
    matches!(
        t,
        // Rust scalars
        "bool" | "char" | "str" | "String"
        | "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
        | "u8" | "u16" | "u32" | "u64" | "u128" | "usize"
        | "f32" | "f64"
        // Common stdlib types that aren't classes
        | "Vec" | "Option" | "Result" | "Box" | "Rc" | "Arc" | "HashMap" | "HashSet"
        | "BTreeMap" | "BTreeSet" | "Cell" | "RefCell"
        // TS/Py primitives
        | "number" | "string" | "boolean" | "any" | "unknown" | "void" | "null" | "undefined"
        | "int" | "float" | "bytes" | "object"
    )
}

impl ClassEdge {
    fn predicate_tag(&self) -> &'static str {
        match self.predicate {
            ClassEdgeKind::Extends => "extends",
            ClassEdgeKind::Implements => "implements",
            ClassEdgeKind::Composes => "composes",
        }
    }
}

// ─── Apply ─────────────────────────────────────────────────────────────────────

/// Escape a string for use inside a Cypher single-quoted string.
fn escape_cypher_string(s: &str) -> String {
    s.replace('\'', "\\'")
}

/// Apply a class-diagram report to the graph store.
pub fn apply(
    project_dir: &Path,
    report: &ClassDiagramReport,
    _fs: &dyn Filesystem,
) -> Result<ApplyReport, ClassDiagramError> {
    use crate::store::open_default;

    let start = Instant::now();
    let mut store = open_default(project_dir).context("open graph store")?;
    store.init().context("graph init")?;

    let mut elements_written = 0;
    let mut elements_skipped = 0;
    let mut relations_written = 0;
    let mut relations_skipped = 0;
    let evidences_written = 0;
    let mut seed_writes = 0;

    // Seed uml MetaTypes and Predicates
    let meta_types = [
        "uml.class",
        "uml.interface",
        "uml.trait",
        "uml.enum",
        "uml.record",
        "uml.operation",
        "uml.attribute",
        "uml.parameter",
        "uml.type_parameter",
    ];
    let predicates = [
        "uml.extends",
        "uml.implements",
        "uml.association",
        "uml.aggregation",
        "uml.composition",
        "uml.depends_on",
    ];

    for mt in &meta_types {
        let q = format!("MERGE (:MetaType {{id: '{}'}});", mt);
        if store.query(&q).is_ok() {
            seed_writes += 1;
        }
    }
    for pred in &predicates {
        let q = format!("MERGE (:Predicate {{id: '{}'}});", pred);
        if store.query(&q).is_ok() {
            seed_writes += 1;
        }
    }

    let version_id = uuid::Uuid::new_v4().to_string();

    for node in &report.nodes {
        let kind_id = match node.kind {
            TypeKind::Class => "uml.class",
            TypeKind::Interface => "uml.interface",
            TypeKind::Trait => "uml.trait",
            TypeKind::Enum => "uml.enum",
            TypeKind::Record => "uml.record",
        };

        let canonical_key_escaped = escape_cypher_string(&node.canonical_key);
        let name_escaped = escape_cypher_string(&node.name);
        let id = format!("cd:{}", node.canonical_key);

        let version_props = serde_json::json!({
            "kind": format!("{:?}", node.kind).to_lowercase(),
            "language": node.language.lang_label(),
            "confidence": node.confidence,
            "members": node.members.len(),
        });
        let version_props_str = version_props.to_string();
        let version_props_escaped = escape_cypher_string(&version_props_str);

        let cypher = format!(
            "MERGE (e:Element {{id: '{id}'}}) SET \
             e.kind_id = '{kind_id}', \
             e.category = 'uml', \
             e.canonical_key = '{canonical_key_escaped}', \
             e.current_name = '{name_escaped}', \
             e.current_status = 'active', \
             e.current_confidence = {confidence}, \
             e.current_version_id = '{version_id}';",
            id = id,
            kind_id = kind_id,
            canonical_key_escaped = canonical_key_escaped,
            name_escaped = name_escaped,
            confidence = node.confidence,
            version_id = version_id,
        );

        match store.query(&cypher) {
            Ok(_) => elements_written += 1,
            Err(_) => elements_skipped += 1,
        }

        // ElementVersion
        let ev_cypher = format!(
            "MATCH (e:Element {{id: '{id}'}}) \
             MERGE (v:ElementVersion {{id: '{version_id}', element_id: '{id}'}}) \
             SET v.props = '{version_props_escaped}';",
            id = id,
            version_id = version_id,
        );
        let _ = store.query(&ev_cypher).ok();
    }

    // Write edges
    for edge in &report.edges {
        let pred_tag = match edge.predicate {
            ClassEdgeKind::Extends => "uml.extends",
            ClassEdgeKind::Implements => "uml.implements",
            ClassEdgeKind::Composes => "uml.composition",
        };

        let _canonical_key_escaped = escape_cypher_string(&edge.canonical_key);
        let source_id = format!("cd:{}", edge.source);
        let target_id = format!("cd:{}", edge.target);
        let rel_id = format!("cd:{}→{}", edge.source, edge.target);

        let rel_props = serde_json::json!({
            "predicate_id": pred_tag,
            "confidence": edge.confidence,
        });
        let rel_props_str = rel_props.to_string();
        let rel_props_escaped = escape_cypher_string(&rel_props_str);

        let cypher = format!(
            "MATCH (s:Element {{id: '{source_id}'}}), (t:Element {{id: '{target_id}'}}) \
             MERGE (s)-[r:SEMANTIC_EDGE {{relation_id: '{rel_id}'}}]->(t) \
             SET r.predicate_id = '{pred_tag}', \
             r.props = '{rel_props_escaped}', \
             r.active = true;",
            source_id = source_id,
            target_id = target_id,
            rel_id = rel_id,
            pred_tag = pred_tag,
            rel_props_escaped = rel_props_escaped,
        );

        match store.query(&cypher) {
            Ok(_) => relations_written += 1,
            Err(_) => relations_skipped += 1,
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(ApplyReport {
        elements_written,
        elements_skipped,
        relations_written,
        relations_skipped,
        evidences_written,
        source_artifacts_written: 0,
        seed_writes,
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_version() {
        assert!(CLASS_DIAGRAM_REPORT_SCHEMA.contains("schemaVersion"));
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

    #[test]
    fn test_escape_cypher_string() {
        assert_eq!(escape_cypher_string("foo"), "foo");
        assert_eq!(escape_cypher_string("o'reilly"), "o\\'reilly");
        assert_eq!(escape_cypher_string(""), "");
    }

    #[test]
    fn test_ts_class_kind() {
        use ast_grep_language::SupportLang;
        use tree_sitter::Parser;
        let source = "class Animal {}";
        let lang = SupportLang::TypeScript;
        let ts_lang = lang.get_ts_language();
        let mut parser = Parser::new();
        parser.set_language(&ts_lang).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let mut found_kinds = Vec::new();
        fn walk(node: tree_sitter::Node, _source: &str, found: &mut Vec<&str>) {
            found.push(node.kind());
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i as u32) {
                    walk(child, _source, found);
                }
            }
        }
        walk(tree.root_node(), source, &mut found_kinds);
        println!("TS kinds: {:?}", found_kinds);
        assert!(
            found_kinds.contains(&"class_declaration"),
            "expected class_declaration in {:?}",
            found_kinds
        );
    }

    #[test]
    fn test_ts_extraction() {
        let source = "class Animal {}";
        let ts_lang = ast_grep_language::SupportLang::TypeScript.get_ts_language();
        let mut parser = Parser::new();
        parser.set_language(&ts_lang).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        super::extract_typescript(&tree, source, "foo.ts", &mut nodes, &mut edges);
        assert!(!nodes.is_empty(), "expected nodes from TS extraction");
        assert_eq!(nodes[0].name, "Animal");
        assert_eq!(nodes[0].kind, TypeKind::Class);
    }

    #[test]
    fn test_ts_extends_tree() {
        let source = "class Dog extends Animal {}";
        let ts_lang = ast_grep_language::SupportLang::TypeScript.get_ts_language();
        let mut parser = Parser::new();
        parser.set_language(&ts_lang).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        super::extract_typescript(&tree, source, "foo.ts", &mut nodes, &mut edges);
        assert!(!nodes.is_empty());
        assert!(!edges.is_empty(), "expected extends edge");
        assert_eq!(edges[0].predicate, ClassEdgeKind::Extends);
    }

    #[test]
    fn test_ts_implements_tree() {
        let source = "class Bar implements IFoo {}";
        let ts_lang = ast_grep_language::SupportLang::TypeScript.get_ts_language();
        let mut parser = Parser::new();
        parser.set_language(&ts_lang).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        super::extract_typescript(&tree, source, "foo.ts", &mut nodes, &mut edges);
        assert!(!nodes.is_empty());
        assert!(!edges.is_empty(), "expected implements edge");
        assert_eq!(edges[0].predicate, ClassEdgeKind::Implements);
    }

    #[test]
    fn test_class_edge_predicate_tag() {
        let extends_edge = ClassEdge {
            canonical_key: "rust:a.rs:A→extends→B:5".to_string(),
            source: "rust:a.rs:class:A:5".to_string(),
            target: "rust:a.rs:class:B:0".to_string(),
            predicate: ClassEdgeKind::Extends,
            file: "a.rs".to_string(),
            line: 5,
            confidence: 0.85,
        };
        assert_eq!(extends_edge.predicate_tag(), "extends");
    }
}
