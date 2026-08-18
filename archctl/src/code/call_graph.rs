//! Call-graph extraction engine + report types.
//!
//! The `c4_discover.rs` analogue for caller→callee edges. Pure extraction
//! (no LLM, no clock) via tree-sitter-graph; deterministic; idempotent apply.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use ast_grep_core::tree_sitter::LanguageExt;
use ast_grep_language::SupportLang;

use serde::{Deserialize, Serialize};
use tree_sitter::{Parser, Tree};

use crate::filesystem::Filesystem;
use crate::store::{ElementRepository, LbugStore, SemanticEdgeRepository};

/// JSON Schema for CallGraphReport (JSON Schema 2020-12).
/// NOTE: 3 levels up (archctl/src/code/ → repo root), matching c4_discover.rs:16.
pub const CALL_GRAPH_REPORT_SCHEMA: &str =
    include_str!("../../../schemas/call-graph-report.schema.json");

// ─── Carrier types ────────────────────────────────────────────────────────────

/// A function/method/closure extracted from source code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionNode {
    /// `<lang>:<file>:<name>:<line>`
    pub canonical_key: String,
    pub kind: FunctionKind,
    pub language: Language,
    /// Relative to --cwd (string, like c4 Evidence.file)
    pub file: String,
    /// `"sha256:<hex>"` of the file content (D2 identity input).
    #[serde(default, rename = "contentHash")]
    pub content_hash: String,
    /// 1-based
    pub line: u32,
    pub name: String,
    /// Rust: crate::module::fn; TS: file.fn; Py: module.fn
    pub fq_name: String,
    /// 0.90 | 0.85 | 0.80 (per language)
    pub confidence: f64,
    /// canonical_key of enclosing function (nested fns/closures)
    pub parent: Option<String>,
}

/// Caller → callee edge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CallEdge {
    /// `<lang>:<file>:<caller>→<callee>:<line>`
    pub canonical_key: String,
    /// FunctionNode.canonical_key
    pub caller: String,
    /// callee name (unresolved in MVP — Phase 2 symbol table)
    pub callee: String,
    pub file: String,
    /// `"sha256:<hex>"` of the file content (D2 identity input).
    #[serde(default, rename = "contentHash")]
    pub content_hash: String,
    /// 1-based call-site line
    pub line: u32,
    pub kind: CallKind,
    pub message_kind: MessageKind,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FunctionKind {
    Function,
    Method,
    Closure,
}

impl FunctionKind {
    // Synthetic labels for anonymous function-like nodes are now passed
    // as a parameter to `extract_function(...)` rather than derived from
    // `FunctionKind`. The language-specific label (`"closure"` for Rust,
    // `"arrow"` for TypeScript) lives at the wrapper site, keeping the
    // `FunctionKind` enum free of presentation concerns. See M34 cycle.
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    TypeScript,
    Python,
    Go,
    Java,
    Kotlin,
}

impl Language {
    /// Returns the extraction confidence for this language.
    pub fn confidence(&self) -> f64 {
        match self {
            Language::Rust => 0.90,
            Language::TypeScript => 0.85,
            Language::Python => 0.80,
            Language::Go => 0.85,
            Language::Java => 0.85,
            Language::Kotlin => 0.85,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CallKind {
    DirectCall,
    MethodCall,
    ChainedCall,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    SyncCall,
    AsyncCall,
    Return,
}

/// Per-project metadata. Mirrors c4_discover::ProjectMeta shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectMeta {
    pub root: String,
    #[serde(rename = "filesScanned")]
    pub files_scanned: u64,
    pub languages: BTreeMap<String, u64>,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
}

/// One per-file failure (graceful degradation, SCN-122). CLI exits 0.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractError {
    pub strategy: String,
    pub path: String,
    pub message: String,
}

/// Top-level report emitted by `archctl code call-graph --json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CallGraphReport {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub project: ProjectMeta,
    pub nodes: Vec<FunctionNode>,
    pub edges: Vec<CallEdge>,
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
    pub duration_ms: u64,
}

/// Errors during extraction or graph write.
#[derive(Debug, thiserror::Error)]
pub enum CallGraphError {
    #[error("invalid --lang: {0} (MVP: rust, typescript, python, go)")]
    #[allow(dead_code)]
    // Reserved for future: clap value_enum guard is strict in MVP; variant kept for spec SCN-11 and future post-parse validation.
    InvalidLanguage(String),
    #[error("TSG execution failed for {path}: {message}")]
    TsgExecution { path: String, message: String },
    #[error("graph write failed: {0}")]
    GraphWrite(#[from] anyhow::Error),
}

// ─── Extract ─────────────────────────────────────────────────────────────────

/// Extract function nodes + call edges from `cwd`, filtered to `languages`
/// (empty = all MVP languages: rust, typescript, python, go). Pure: no graph writes. Deterministic.
pub fn extract(
    cwd: &Path,
    languages: &[Language],
    depth_limit: Option<u32>,
    fs: &dyn Filesystem,
) -> Result<CallGraphReport, CallGraphError> {
    let start = Instant::now();
    let root = cwd.to_string_lossy().to_string();

    // Collect files to process
    let mut files_to_process: Vec<(PathBuf, Language, String)> = Vec::new();
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
        let (lang, lang_label) = match ext_str.as_str() {
            "rs" => (Some(Language::Rust), "rust"),
            "ts" | "tsx" => (Some(Language::TypeScript), "typescript"),
            "js" | "jsx" | "mjs" | "cjs" => (Some(Language::TypeScript), "typescript"),
            "py" => (Some(Language::Python), "python"),
            "go" => (Some(Language::Go), "go"),
            "java" => (Some(Language::Java), "java"),
            "kt" | "kts" => (Some(Language::Kotlin), "kotlin"),
            _ => (None, ""),
        };
        let Some(lang) = lang else { continue };
        if !languages.is_empty() && !languages.contains(&lang) {
            continue;
        }
        let rel = path.strip_prefix(cwd).unwrap_or(path);
        files_to_process.push((rel.to_path_buf(), lang, lang_label.to_string()));
        *lang_counts.entry(lang_label.to_string()).or_insert(0) += 1;
    }

    let mut all_nodes: Vec<FunctionNode> = Vec::new();
    let mut all_edges: Vec<CallEdge> = Vec::new();
    let mut errors: Vec<ExtractError> = Vec::new();
    let files_scanned = files_to_process.len() as u64;

    for (rel_path, lang, lang_label) in files_to_process {
        let abs_path = cwd.join(&rel_path);
        let source = match fs.read_to_string(&abs_path) {
            Ok(s) => s,
            Err(e) => {
                errors.push(ExtractError {
                    strategy: lang_label.clone(),
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
            Language::Go => SupportLang::Go,
            Language::Java => SupportLang::Java,
            Language::Kotlin => SupportLang::Kotlin,
        };

        // Parse with tree-sitter
        let ts_lang = support_lang.get_ts_language();
        let mut parser = Parser::new();
        if parser.set_language(&ts_lang).is_err() {
            errors.push(ExtractError {
                strategy: lang_label.clone(),
                path: rel_path.to_string_lossy().to_string(),
                message: "failed to set tree-sitter language".to_string(),
            });
            continue;
        }
        let tree = match parser.parse(&source, None) {
            Some(t) => t,
            None => {
                errors.push(ExtractError {
                    strategy: lang_label.clone(),
                    path: rel_path.to_string_lossy().to_string(),
                    message: "parse failed".to_string(),
                });
                continue;
            }
        };

        let file_str = rel_path.to_string_lossy().to_string();

        // Extract function definitions via direct tree-sitter walk
        extract_function_definitions(&tree, &source, lang, &file_str, &mut all_nodes);

        // Extract call edges via direct tree-sitter walk
        extract_call_edges(
            &tree,
            &source,
            lang,
            &file_str,
            &all_nodes,
            &mut all_edges,
            depth_limit,
        );

        // Thread the file content hash into this file's carriers (D2 identity
        // input). Computed once per file — the source is already in memory
        // for the tree-sitter parse; no extra I/O.
        let content_hash = crate::evidence::content_hash_of(&source);
        for n in all_nodes.iter_mut().filter(|n| n.file == file_str) {
            n.content_hash = content_hash.clone();
        }
        for e in all_edges.iter_mut().filter(|e| e.file == file_str) {
            e.content_hash = content_hash.clone();
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(CallGraphReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root,
            files_scanned,
            languages: lang_counts,
            duration_ms,
        },
        nodes: all_nodes,
        edges: all_edges,
        errors,
    })
}

/// Walk tree-sitter tree to extract function/method/closure definitions.
fn extract_function_definitions(
    tree: &Tree,
    source: &str,
    lang: Language,
    file: &str,
    nodes: &mut Vec<FunctionNode>,
) {
    let root = tree.root_node();
    find_function_definitions(root, source, lang, file, None, nodes);
}

/// Recursively walk tree-sitter nodes to find function/method/closure definitions.
fn find_function_definitions<'tree>(
    node: tree_sitter::Node<'tree>,
    source: &str,
    lang: Language,
    file: &str,
    parent_key: Option<&str>,
    nodes: &mut Vec<FunctionNode>,
) {
    let kind = node.kind();

    match lang {
        Language::Rust => {
            if kind == "function_item" {
                if let Some(fn_node) = extract_rust_function(node, source, lang, file, parent_key) {
                    nodes.push(fn_node);
                }
                // Don't recurse into function body to avoid double-counting nested functions
                return;
            } else if kind == "impl_item" {
                // Extract methods from impl block
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i as u32)
                        && child.kind() == "function_item"
                        && let Some(fn_node) =
                            extract_rust_function(child, source, lang, file, parent_key)
                    {
                        nodes.push(fn_node);
                    }
                }
                // Don't recurse into impl body further
                return;
            } else if kind == "closure_expression" {
                if let Some(fn_node) = extract_rust_closure(node, source, lang, file, parent_key) {
                    nodes.push(fn_node);
                }
                // Don't recurse into closure body
                return;
            }
        }
        Language::TypeScript => {
            if kind == "function_declaration" {
                if let Some(fn_node) = extract_ts_function(node, source, lang, file, parent_key) {
                    nodes.push(fn_node);
                }
                return;
            } else if kind == "method_definition" {
                if let Some(fn_node) = extract_ts_method(node, source, lang, file, parent_key) {
                    nodes.push(fn_node);
                }
                return;
            } else if kind == "arrow_function" {
                if let Some(fn_node) = extract_ts_arrow(node, source, lang, file, parent_key) {
                    nodes.push(fn_node);
                }
                return;
            }
        }
        Language::Python => {
            if kind == "function_definition" {
                // Check if this is a method (inside a class) or standalone function
                let is_method = parent_key.is_some();
                if let Some(fn_node) =
                    extract_python_function(node, source, lang, file, parent_key, is_method)
                {
                    nodes.push(fn_node);
                }
                return;
            } else if kind == "class_definition" {
                // Recurse into class body to find methods
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i as u32)
                        && child.kind() == "block"
                    {
                        for j in 0..child.child_count() {
                            if let Some(grandchild) = child.child(j as u32)
                                && grandchild.kind() == "function_definition"
                                && let Some(fn_node) = extract_python_function(
                                    grandchild, source, lang, file, parent_key,
                                    true, // is_method
                                )
                            {
                                nodes.push(fn_node);
                            }
                        }
                    }
                }
                return;
            }
        }
        Language::Go => {
            // function_declaration: regular functions (including main, init)
            if kind == "function_declaration" {
                if let Some(fn_node) = extract_go_function(node, source, lang, file, parent_key) {
                    nodes.push(fn_node);
                }
                return;
            } else if kind == "method_declaration" {
                // Methods (receiver functions)
                if let Some(fn_node) = extract_go_method(node, source, lang, file, parent_key) {
                    nodes.push(fn_node);
                }
                return;
            } else if kind == "func_literal" {
                // func_literal is NOT a FunctionNode — anonymous, calls attributed to enclosing named function
                // Do NOT recurse into func_literal body (same guard as Rust closure_expression)
                return;
            }
        }
        Language::Java => {
            // method_declaration: class methods
            if kind == "method_declaration" {
                if let Some(fn_node) = extract_java_method(node, source, lang, file, parent_key) {
                    nodes.push(fn_node);
                }
                return;
            } else if kind == "class_declaration" {
                // Recurse into class body to find methods (incl. constructors)
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i as u32)
                        && child.kind() == "class_body"
                    {
                        for j in 0..child.child_count() {
                            if let Some(member) = child.child(j as u32)
                                && (member.kind() == "method_declaration"
                                    || member.kind() == "constructor_declaration")
                                && let Some(fn_node) =
                                    extract_java_method(member, source, lang, file, parent_key)
                            {
                                nodes.push(fn_node);
                            }
                        }
                    }
                }
                return;
            }
        }
        Language::Kotlin => {
            // function_declaration: covers `fun foo() {}` (top-level AND in classes)
            if kind == "function_declaration" {
                if let Some(fn_node) = extract_kotlin_function(node, source, lang, file, parent_key)
                {
                    nodes.push(fn_node);
                }
                return;
            } else if kind == "class_body" {
                // Recurse into class body to find member functions.
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i as u32)
                        && child.kind() == "function_declaration"
                        && let Some(fn_node) =
                            extract_kotlin_function(child, source, lang, file, parent_key)
                    {
                        nodes.push(fn_node);
                    }
                }
                return;
            }
        }
    }

    // Recurse into children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            find_function_definitions(child, source, lang, file, parent_key, nodes);
        }
    }
}

// ─── Shared extraction helper (D2) ───────────────────────────────────────────

/// D2: Single helper replacing 8 near-identical extractor bodies.
/// Dispatches on `child_kind` for name extraction; uses `kind` for
/// FunctionKind variant; `confidence` is passed explicitly (via
/// `Language::confidence()`) to keep the helper language-agnostic.
///
/// `child_kind`: tree-sitter node kind to look up for the name
///   (e.g. `"identifier"`, `"property_identifier"`, `"field_identifier"`).
///   `None` means generate a synthetic name from line number (closures/arrows).
/// `kind`: the `FunctionKind` variant for the produced node.
/// `confidence`: extraction confidence (typically `lang.confidence()`).
/// `parent_key`: canonical_key of the enclosing function (for nested fns/closures).
/// `synthetic_label`: when `child_kind` is None, the prefix used to build the
///   anonymous name (e.g. `"closure"` for Rust closures, `"arrow"` for TS
///   arrow functions). Caller supplies the language-specific label.
#[allow(clippy::too_many_arguments)]
fn extract_function(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    file: &str,
    child_kind: Option<&str>,
    kind: FunctionKind,
    confidence: f64,
    parent_key: Option<&str>,
    synthetic_label: &str,
) -> Option<FunctionNode> {
    let name = if let Some(ck) = child_kind {
        let mut n = String::new();
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i as u32)
                && child.kind() == ck
            {
                n = source
                    .get(child.start_byte()..child.end_byte())?
                    .to_string();
                break;
            }
        }
        if n.is_empty() {
            return None;
        }
        n
    } else {
        // Synthetic name for closures / arrow functions.
        let line = (node.start_position().row + 1) as u32;
        format!("{}@{}", synthetic_label, line)
    };

    let line = (node.start_position().row + 1) as u32;
    let canonical_key = format!("{}:{}:{}:{}", lang_label(&lang), file, name, line);
    let fq_name = name.clone();

    Some(FunctionNode {
        canonical_key,
        kind,
        language: lang,
        file: file.to_string(),
        content_hash: String::new(),
        line,
        name,
        fq_name,
        confidence,
        parent: parent_key.map(|s| s.to_string()),
    })
}

fn extract_rust_function(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    file: &str,
    parent_key: Option<&str>,
) -> Option<FunctionNode> {
    extract_function(
        node,
        source,
        lang,
        file,
        Some("identifier"),
        FunctionKind::Function,
        lang.confidence(),
        parent_key,
        "fn",
    )
}

fn extract_rust_closure(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    file: &str,
    parent_key: Option<&str>,
) -> Option<FunctionNode> {
    extract_function(
        node,
        source,
        lang,
        file,
        None,
        FunctionKind::Closure,
        lang.confidence(),
        parent_key,
        "closure",
    )
}

fn extract_ts_function(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    file: &str,
    parent_key: Option<&str>,
) -> Option<FunctionNode> {
    extract_function(
        node,
        source,
        lang,
        file,
        Some("identifier"),
        FunctionKind::Function,
        lang.confidence(),
        parent_key,
        "fn",
    )
}

fn extract_ts_method(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    file: &str,
    parent_key: Option<&str>,
) -> Option<FunctionNode> {
    extract_function(
        node,
        source,
        lang,
        file,
        Some("property_identifier"),
        FunctionKind::Method,
        lang.confidence(),
        parent_key,
        "fn",
    )
}

fn extract_ts_arrow(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    file: &str,
    parent_key: Option<&str>,
) -> Option<FunctionNode> {
    extract_function(
        node,
        source,
        lang,
        file,
        None,
        FunctionKind::Closure,
        lang.confidence(),
        parent_key,
        "arrow",
    )
}

fn extract_python_function(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    file: &str,
    parent_key: Option<&str>,
    is_method: bool,
) -> Option<FunctionNode> {
    let kind = if is_method {
        FunctionKind::Method
    } else {
        FunctionKind::Function
    };
    extract_function(
        node,
        source,
        lang,
        file,
        Some("identifier"),
        kind,
        lang.confidence(),
        parent_key,
        "fn",
    )
}

fn extract_go_function(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    file: &str,
    parent_key: Option<&str>,
) -> Option<FunctionNode> {
    extract_function(
        node,
        source,
        lang,
        file,
        Some("identifier"),
        FunctionKind::Function,
        lang.confidence(),
        parent_key,
        "fn",
    )
}

fn extract_go_method(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    file: &str,
    parent_key: Option<&str>,
) -> Option<FunctionNode> {
    extract_function(
        node,
        source,
        lang,
        file,
        Some("field_identifier"),
        FunctionKind::Method,
        lang.confidence(),
        parent_key,
        "fn",
    )
}

// ─── Java extractors (M35) ───────────────────────────────────────────────────

fn extract_java_method(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    file: &str,
    parent_key: Option<&str>,
) -> Option<FunctionNode> {
    // tree-sitter Java: both `method_declaration` and `constructor_declaration`
    // expose the function name as an `identifier` child.
    extract_function(
        node,
        source,
        lang,
        file,
        Some("identifier"),
        FunctionKind::Method,
        lang.confidence(),
        parent_key,
        "fn",
    )
}

fn extract_java_callee(node: tree_sitter::Node, source: &str) -> Option<String> {
    // tree-sitter Java method_invocation shape:
    //   method(args)              → identifier(method) + argument_list(args)
    //   obj.method(args)          → field_access(obj, method) + argument_list
    //   Class.method(args)        → scoped_identifier(Class, method) + argument_list
    //   pkg.Class.method(args)    → chained
    // Strategy: skip `argument_list` (those are the args, not the callee),
    // then within the remaining subtree pick the deepest rightmost identifier.
    fn callee_text<'a>(n: tree_sitter::Node<'a>, source: &str) -> Option<String> {
        let mut found: Option<String> = None;
        walk_callee(n, source, &mut found);
        found
    }

    fn walk_callee<'a>(node: tree_sitter::Node<'a>, source: &str, out: &mut Option<String>) {
        if node.kind() == "argument_list" {
            // Skip the arguments subtree entirely.
            return;
        }
        // Recurse first (pre-order on the *remaining* subtree).
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            walk_callee(child, source, out);
        }
        if node.kind() == "identifier"
            && let Some(text) = source.get(node.start_byte()..node.end_byte())
        {
            // Last-wins: `obj.method` → `method`.
            *out = Some(text.to_string());
        }
    }
    callee_text(node, source)
}

// ─── Kotlin extractors (M36) ───────────────────────────────────────────────────

fn extract_kotlin_function(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    file: &str,
    parent_key: Option<&str>,
) -> Option<FunctionNode> {
    // Kotlin `function_declaration` exposes the name as a `simple_identifier`
    // child. (Equivalent of Java's `identifier`.)
    extract_function(
        node,
        source,
        lang,
        file,
        Some("simple_identifier"),
        FunctionKind::Method,
        lang.confidence(),
        parent_key,
        "fn",
    )
}

fn extract_kotlin_callee(node: tree_sitter::Node, source: &str) -> Option<String> {
    // tree-sitter Kotlin shapes:
    //   navigation_expression:
    //     obj.method(args)   → simple_identifier "obj", navigation_suffix (simple_identifier "method") + value_arguments
    //     method(args)       → simple_identifier "method" + value_arguments
    //   call_expression: (top-level function call)
    //     foo(args)          → simple_identifier "foo" + value_arguments
    //
    // Strategy: skip `value_arguments` (those are the args), then within
    // the remaining subtree pick the deepest rightmost `simple_identifier`.
    fn callee_text<'a>(n: tree_sitter::Node<'a>, source: &str) -> Option<String> {
        let mut found: Option<String> = None;
        walk_callee(n, source, &mut found);
        found
    }

    fn walk_callee<'a>(node: tree_sitter::Node<'a>, source: &str, out: &mut Option<String>) {
        if node.kind() == "value_arguments" {
            return;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            walk_callee(child, source, out);
        }
        if node.kind() == "simple_identifier"
            && let Some(text) = source.get(node.start_byte()..node.end_byte())
        {
            // Last-wins: `obj.method` → `method`.
            *out = Some(text.to_string());
        }
    }
    callee_text(node, source)
}

fn extract_go_callee(node: tree_sitter::Node, source: &str) -> Option<String> {
    // Go call_expression can be:
    // - identifier: helper() -> extract identifier
    // - selector_expression: pkg.Func() or s.Save() -> extract field_identifier
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            let child_kind = child.kind();
            if child_kind == "identifier" {
                return Some(
                    source
                        .get(child.start_byte()..child.end_byte())?
                        .to_string(),
                );
            } else if child_kind == "selector_expression" {
                // For s.Save() or fmt.Println(), extract the field_identifier
                for j in 0..child.child_count() {
                    if let Some(field_child) = child.child(j as u32)
                        && field_child.kind() == "field_identifier"
                    {
                        return Some(
                            source
                                .get(field_child.start_byte()..field_child.end_byte())?
                                .to_string(),
                        );
                    }
                }
            }
        }
    }
    None
}

/// Walk tree-sitter tree to find call expressions and resolve enclosing function.
fn extract_call_edges(
    tree: &Tree,
    source: &str,
    lang: Language,
    file: &str,
    nodes: &[FunctionNode],
    edges: &mut Vec<CallEdge>,
    _depth_limit: Option<u32>,
) {
    let root = tree.root_node();
    find_call_expressions(root, source, lang, file, nodes, edges);
}

/// Recursively walk tree-sitter nodes to find call expressions.
fn find_call_expressions<'tree>(
    node: tree_sitter::Node<'tree>,
    source: &str,
    lang: Language,
    file: &str,
    nodes: &[FunctionNode],
    edges: &mut Vec<CallEdge>,
) {
    // Check if this node is a call expression
    let kind = node.kind();

    match lang {
        Language::Rust => {
            if kind == "call_expression" {
                if let Some(callee) = extract_callee_from_call(node, source) {
                    let line = (node.start_position().row + 1) as u32;
                    edges.push(make_call_edge(
                        nodes,
                        lang,
                        file,
                        &callee,
                        line,
                        CallKind::DirectCall,
                        MessageKind::SyncCall,
                    ));
                }
            } else if kind == "method_call_expression" {
                if let Some(callee) = extract_method_callee(node, source) {
                    let line = (node.start_position().row + 1) as u32;
                    edges.push(make_call_edge(
                        nodes,
                        lang,
                        file,
                        &callee,
                        line,
                        CallKind::MethodCall,
                        MessageKind::SyncCall,
                    ));
                }
            } else if kind == "await_expression" {
                // Look for call_expression inside await
                for i in 0..node.child_count() {
                    let child = node.child(i as u32).unwrap();
                    if child.kind() == "call_expression"
                        && let Some(callee) = extract_callee_from_call(child, source)
                    {
                        let line = (node.start_position().row + 1) as u32;
                        edges.push(make_call_edge(
                            nodes,
                            lang,
                            file,
                            &callee,
                            line,
                            CallKind::DirectCall,
                            MessageKind::AsyncCall,
                        ));
                    }
                }
            }
        }
        Language::TypeScript => {
            if kind == "call_expression"
                && let Some(callee) = extract_ts_callee(node, source)
            {
                let line = (node.start_position().row + 1) as u32;
                edges.push(make_call_edge(
                    nodes,
                    lang,
                    file,
                    &callee,
                    line,
                    CallKind::DirectCall,
                    MessageKind::SyncCall,
                ));
            }
        }
        Language::Python => {
            if kind == "call" {
                let (callee, is_method) = extract_python_callee(node, source);
                if !callee.is_empty() {
                    let line = (node.start_position().row + 1) as u32;
                    let call_kind = if is_method {
                        CallKind::MethodCall
                    } else {
                        CallKind::DirectCall
                    };
                    edges.push(make_call_edge(
                        nodes,
                        lang,
                        file,
                        &callee,
                        line,
                        call_kind,
                        MessageKind::SyncCall,
                    ));
                }
            }
        }
        Language::Go => {
            if kind == "call_expression" {
                // Go call_expression: identifier calls like helper() or pkg.Func()
                // For method calls (s.Save()) and package-qualified (fmt.Println):
                // selector_expression -> field_identifier child
                if let Some(callee) = extract_go_callee(node, source) {
                    let line = (node.start_position().row + 1) as u32;
                    let call_kind = if node
                        .children(&mut node.walk())
                        .any(|c| c.kind() == "selector_expression")
                    {
                        CallKind::MethodCall
                    } else {
                        CallKind::DirectCall
                    };
                    edges.push(make_call_edge(
                        nodes,
                        lang,
                        file,
                        &callee,
                        line,
                        call_kind,
                        MessageKind::SyncCall,
                    ));
                }
            }
        }
        Language::Java => {
            // method_invocation: obj.method(...) or method(...)
            if kind == "method_invocation"
                && let Some(callee) = extract_java_callee(node, source)
            {
                let line = (node.start_position().row + 1) as u32;
                let call_kind = if callee.contains('.') {
                    CallKind::MethodCall
                } else {
                    CallKind::DirectCall
                };
                edges.push(make_call_edge(
                    nodes,
                    lang,
                    file,
                    &callee,
                    line,
                    call_kind,
                    MessageKind::SyncCall,
                ));
            }
        }
        Language::Kotlin => {
            // navigation_expression: obj.method(...) or method(...)
            if kind == "navigation_expression"
                && let Some(callee) = extract_kotlin_callee(node, source)
            {
                let line = (node.start_position().row + 1) as u32;
                let call_kind = if callee.contains('.') {
                    CallKind::MethodCall
                } else {
                    CallKind::DirectCall
                };
                edges.push(make_call_edge(
                    nodes,
                    lang,
                    file,
                    &callee,
                    line,
                    call_kind,
                    MessageKind::SyncCall,
                ));
            } else if kind == "call_expression"
                && let Some(callee) = extract_kotlin_callee(node, source)
            {
                let line = (node.start_position().row + 1) as u32;
                edges.push(make_call_edge(
                    nodes,
                    lang,
                    file,
                    &callee,
                    line,
                    CallKind::DirectCall,
                    MessageKind::SyncCall,
                ));
            }
        }
    }

    // Recurse into children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            find_call_expressions(child, source, lang, file, nodes, edges);
        }
    }
}

fn extract_callee_from_call(node: tree_sitter::Node, source: &str) -> Option<String> {
    for i in 0..node.child_count() {
        let child = node.child(i as u32).unwrap();
        if child.kind() == "identifier" {
            return Some(
                source
                    .get(child.start_byte()..child.end_byte())?
                    .to_string(),
            );
        }
    }
    None
}

fn extract_method_callee(node: tree_sitter::Node, source: &str) -> Option<String> {
    for i in 0..node.child_count() {
        let child = node.child(i as u32).unwrap();
        if child.kind() == "field_identifier" {
            return Some(
                source
                    .get(child.start_byte()..child.end_byte())?
                    .to_string(),
            );
        }
    }
    None
}

fn extract_ts_callee(node: tree_sitter::Node, source: &str) -> Option<String> {
    for i in 0..node.child_count() {
        let child = node.child(i as u32).unwrap();
        if child.kind() == "identifier" {
            return Some(
                source
                    .get(child.start_byte()..child.end_byte())?
                    .to_string(),
            );
        }
    }
    None
}

fn extract_python_callee(node: tree_sitter::Node, source: &str) -> (String, bool) {
    let first_child = node.child(0u32);
    if let Some(child) = first_child {
        if child.kind() == "attribute" {
            // obj.method -> method
            for i in 0..child.child_count() {
                let c2 = child.child(i as u32).unwrap();
                if c2.kind() == "identifier" {
                    let name = source
                        .get(c2.start_byte()..c2.end_byte())
                        .unwrap_or("")
                        .to_string();
                    return (name, true);
                }
            }
        } else if child.kind() == "identifier" {
            return (
                source
                    .get(child.start_byte()..child.end_byte())
                    .unwrap_or("")
                    .to_string(),
                false,
            );
        }
    }
    (String::new(), false)
}

fn make_call_edge(
    nodes: &[FunctionNode],
    lang: Language,
    file: &str,
    callee: &str,
    line: u32,
    kind: CallKind,
    message_kind: MessageKind,
) -> CallEdge {
    // Find the enclosing function for this call site
    // Simple heuristic: find the nearest function node whose line <= call_line
    let caller = nodes
        .iter()
        .filter(|n| n.language == lang && n.file == file && n.line <= line)
        .max_by_key(|n| n.line)
        .map(|n| n.canonical_key.clone())
        .unwrap_or_else(|| format!("{}:{}:<unknown>:0", lang_label(&lang), file));

    let confidence = lang.confidence();

    let canonical_key = format!(
        "{}:{}:{}→{}:{}",
        lang_label(&lang),
        file,
        caller,
        callee,
        line
    );

    CallEdge {
        canonical_key,
        caller,
        callee: callee.to_string(),
        file: file.to_string(),
        content_hash: String::new(),
        line,
        kind,
        message_kind,
        confidence,
    }
}

fn lang_label(lang: &Language) -> &'static str {
    match lang {
        Language::Rust => "rust",
        Language::TypeScript => "typescript",
        Language::Python => "python",
        Language::Go => "go",
        Language::Java => "java",
        Language::Kotlin => "kotlin",
    }
}

// ─── Apply ──────────────────────────────────────────────────────────────────

/// Write a call edge as a `SEMANTIC_EDGE` (Element→Element) row.
///
/// Uses SemanticEdgeRepository::link_call_edge_with_resolution which handles
/// the OPTIONAL MATCH + WITH/WHERE + MERGE pattern internally.
fn write_call_edge(
    store: &mut LbugStore,
    edge: &CallEdge,
    src_element_id: &str,
    sa_id: &str,
    _version_id: &str,
) -> Result<()> {
    let rel_id = format!("rel:{}", edge.canonical_key);
    let mut rel_props = serde_json::Map::new();
    rel_props.insert(
        "predicate".to_string(),
        serde_json::Value::String("code.calls".to_string()),
    );
    rel_props.insert(
        "call_kind".to_string(),
        serde_json::Value::String(format!("{:?}", edge.kind).to_lowercase()),
    );
    rel_props.insert(
        "message_kind".to_string(),
        serde_json::Value::String(format!("{:?}", edge.message_kind).to_lowercase()),
    );
    rel_props.insert(
        "rel_id".to_string(),
        serde_json::Value::String(rel_id.clone()),
    );

    // Note: errors are silently ignored (matches prior behavior for MVP).
    let _ = SemanticEdgeRepository::link_call_edge_with_resolution(
        store,
        src_element_id,
        &edge.callee,
        &rel_id,
        &rel_props,
    )
    .ok();

    // Evidence node via EvidenceRepository::put_structural_evidence
    let evidence_id = format!(
        "ev:{}",
        blake3::hash(edge.canonical_key.as_bytes()).to_hex()
    );
    let mut ev_props = serde_json::Map::new();
    ev_props.insert(
        "file_refs".to_string(),
        serde_json::Value::Array(vec![serde_json::Value::String(format!(
            "{}:{}",
            edge.file, edge.line
        ))]),
    );
    ev_props.insert(
        "status".to_string(),
        serde_json::Value::String("drafted".to_string()),
    );
    ev_props.insert(
        "classification".to_string(),
        serde_json::Value::String("derived".to_string()),
    );
    crate::store::EvidenceRepository::put_structural_evidence(
        store,
        &crate::graph::StructuralEvidence {
            id: evidence_id.clone(),
            kind: "structural".to_string(),
            claim: "call-graph edge".to_string(),
            file: edge.file.clone(),
            line: u64::from(edge.line),
            confidence: edge.confidence,
            rule_id: "call_graph_edge".to_string(),
            props: ev_props,
        },
    )
    .with_context(|| format!("write_call_evidence {}", evidence_id))?;

    // Link Evidence to SourceArtifact
    let _ = crate::store::EvidenceRepository::link_extracted_from(store, &evidence_id, sa_id).ok();

    // Link Evidence to ElementVersion via SUPPORTED_BY
    let _ =
        crate::store::EvidenceRepository::link_supported_by(store, _version_id, &evidence_id).ok();

    // Fuse-on-write (Item 27 residual): recompute fused claims for the
    // affected version. Best-effort — never breaks the extraction.
    if !_version_id.is_empty() {
        let _ = crate::architecture::fusion::recompute_fused_for_versions(
            store,
            &[_version_id.to_string()],
        );
    }

    Ok(())
}

/// Persist a CallGraphReport to the graph.
/// Idempotent: skips canonical_keys that already exist.
pub fn apply(
    project_dir: &Path,
    report: &CallGraphReport,
    _fs: &dyn Filesystem,
) -> Result<ApplyReport, CallGraphError> {
    use crate::code::apply_common::write_source_artifact;
    use crate::store::{GraphStore, LbugStore, UnitOfWork};

    let mut store = LbugStore::open(project_dir)
        .map_err(|e| CallGraphError::GraphWrite(anyhow::anyhow!("failed to open store: {e}")))?;
    store.init().map_err(CallGraphError::GraphWrite)?;

    // M32 BREAK-1: removed the inline seed MERGEs. MetaType/Predicate
    // rows now come from the migration runner (`docs/schema/`). The
    // prior `let seed_writes = 1` was a lie — no seeding actually
    // happened inside the apply. The ApplyReport no longer carries
    // the field; CLI `--json` output drops it.

    let existing_keys = ElementRepository::existing_canonical_keys(&store)
        .context("fetch existing keys")
        .map_err(CallGraphError::GraphWrite)?;

    // D1: wrap all writes in a single Kùzu transaction via UnitOfWork.
    // Rationale and contract: see ADR-036 §D1 and `Transaction`.
    let mut tx = UnitOfWork::begin_transaction(&mut store).map_err(|se| {
        CallGraphError::GraphWrite(anyhow::anyhow!("begin_transaction failed: {se}"))
    })?;

    // No seeding inside the transaction: Kùzu's `runFuncInTransaction`
    // auto-rollbacks on any std::exception (lbug client_context.cpp L658),
    // and seeding via individual MERGEs was found to trigger an implicit
    // COMMIT that cleared the active transaction before our writes
    // completed. We rely on the schema migrations to have already created
    // MetaType/Predicate rows — see `migrations/` runner. If a project
    // is opened without migrations applied, the OF_TYPE / SEMANTIC_EDGE
    // writes below will fail with a binder exception; that surfaces as
    // a normal error and the apply rolls back.

    let mut elements_written = 0usize;
    let elements_skipped = 0usize; // currently unused (UNWIND skips via existing_keys pre-check); kept for ApplyReport contract.
    let mut relations_written = 0usize;
    let mut relations_skipped = 0usize;
    let mut evidences_written = 0usize;
    let mut source_artifacts_written = 0usize;
    let mut source_artifact_ids: BTreeMap<String, String> = BTreeMap::new();

    // D2: UNWIND bulk import. Replaces per-element MERGE loops with
    // batched UNWIND queries (BATCH_SIZE rows per query). The batch is
    // inlined as a Cypher literal list of maps — Kùzu accepts this
    // without prepared statements. Idempotency skip (existing_keys)
    // happens in-memory BEFORE the batch, so the batch is always
    // "new rows only".
    //
    // D2: UNWIND re-shipped 2026-08-16 (was regressed by P1-04 T3 commit 599c863).
    //
    // Trade-off: bigger Cypher strings (BATCH_SIZE × ~200 chars per row
    // ≈ 100KB per query) vs N/BATCH_SIZE query roundtrips. For echo
    // 1307 elements: ~3 queries instead of ~6535. Expected additional
    // 2-10× speedup over PR1's transaction wrap.

    // Reborrow through Transaction to call LbugStore repository methods.
    let s: &mut LbugStore = tx.as_mut();

    // Pre-compute SourceArtifact IDs for all unique files (in memory).
    // Same per-file dedup as PR1; just hoisted out of the per-node loop
    // because we now build batches and can't call write_source_artifact
    // (which itself does a query) inside the batch UNWIND.
    for node in &report.nodes {
        if existing_keys.contains(&node.canonical_key) {
            continue;
        }
        if !source_artifact_ids.contains_key(&node.file) {
            let id = write_source_artifact(
                s,
                &node.file,
                &node.content_hash,
                lang_label(&node.language),
            )
            .context("write_source_artifact")
            .map_err(CallGraphError::GraphWrite)?;
            source_artifact_ids.insert(node.file.clone(), id);
            source_artifacts_written += 1;
        }
    }

    // Build the candidate node batch (skipping existing canonical_keys).
    let candidate_nodes: Vec<&FunctionNode> = report
        .nodes
        .iter()
        .filter(|n| !existing_keys.contains(&n.canonical_key))
        .collect();

    // Per-node repository writes (P1-03: no inline Cypher in apply paths).
    // M32 D2: batch UNWIND via apply_common helpers.
    // Build Element + ElementVersion batches, then call batch helpers once.
    let elements: Vec<crate::graph::Element> = candidate_nodes
        .iter()
        .map(|n| {
            let kind_id = match n.kind {
                FunctionKind::Function => "code.function",
                FunctionKind::Method => "code.method",
                FunctionKind::Closure => "code.closure",
            };
            let version_props = serde_json::json!({
                "kind": format!("{:?}", n.kind).to_lowercase(),
                "language": lang_label(&n.language),
                "confidence": n.confidence,
                "call_graph_schema_version": "1.0",
            });
            let version_props_str = serde_json::to_string(&version_props).unwrap_or_default();
            let version_id = format!(
                "cgv:{}",
                blake3::hash(version_props_str.as_bytes()).to_hex()
            );
            crate::graph::Element {
                id: format!("cg:{}", n.canonical_key),
                kind_id: kind_id.to_string(),
                category: "code".to_string(),
                canonical_key: n.canonical_key.clone(),
                current_name: n.name.clone(),
                current_status: "active".to_string(),
                current_confidence: n.confidence,
                current_version_id: version_id.clone(),
            }
        })
        .collect();

    let mut element_versions: Vec<crate::graph::ElementVersion> =
        Vec::with_capacity(candidate_nodes.len());
    for n in &candidate_nodes {
        let version_props = serde_json::json!({
            "kind": format!("{:?}", n.kind).to_lowercase(),
            "language": lang_label(&n.language),
            "confidence": n.confidence,
            "call_graph_schema_version": "1.0",
        });
        let version_props_str = serde_json::to_string(&version_props).unwrap_or_default();
        let version_id = format!(
            "cgv:{}",
            blake3::hash(version_props_str.as_bytes()).to_hex()
        );
        let mut props_map = serde_json::Map::new();
        for (k, v) in version_props.as_object().cloned().unwrap_or_default() {
            props_map.insert(k, v);
        }
        element_versions.push(crate::graph::ElementVersion {
            id: version_id,
            element_id: format!("cg:{}", n.canonical_key),
            name: n.name.clone(),
            status: "drafted".to_string(),
            origin: "call-graph".to_string(),
            confidence: n.confidence,
            props: props_map,
        });
    }

    elements_written +=
        ElementRepository::batch_upsert_elements(s, &elements).context("batch_upsert_elements")?;
    ElementRepository::batch_upsert_element_versions(s, &element_versions)
        .context("batch_upsert_element_versions")
        .map_err(CallGraphError::GraphWrite)?;

    // HIGH-5: batch OF_TYPE edge links via UNWIND (was per-element loop).
    let of_type_pairs: Vec<(String, String)> = candidate_nodes
        .iter()
        .map(|n| {
            let kind_id = match n.kind {
                FunctionKind::Function => "code.function",
                FunctionKind::Method => "code.method",
                FunctionKind::Closure => "code.closure",
            };
            (format!("cg:{}", n.canonical_key), kind_id.to_string())
        })
        .collect();
    if !of_type_pairs.is_empty() {
        ElementRepository::batch_link_of_type(s, &of_type_pairs)
            .context("batch_link_of_type")
            .map_err(CallGraphError::GraphWrite)?;
    }

    // Write call edges (per-edge, since the OPTIONAL MATCH semantics
    // don't batch cleanly with UNWIND — callee resolution is per-row).
    for edge in &report.edges {
        let src_element_id = format!("cg:{}", edge.caller);
        if !existing_keys.contains(&edge.caller) {
            let sa_id = if let Some(id) = source_artifact_ids.get(&edge.file) {
                id.clone()
            } else {
                let lang_label_edge = edge
                    .canonical_key
                    .split(':')
                    .next()
                    .unwrap_or("")
                    .to_string();
                let id = write_source_artifact(s, &edge.file, &edge.content_hash, &lang_label_edge)
                    .context("write_source_artifact")
                    .map_err(CallGraphError::GraphWrite)?;
                source_artifact_ids.insert(edge.file.clone(), id.clone());
                source_artifacts_written += 1;
                id
            };
            let version_id = format!("cgv:{}", blake3::hash(edge.caller.as_bytes()).to_hex());
            write_call_edge(s, edge, &src_element_id, &sa_id, &version_id)
                .context("write_call_edge")
                .map_err(CallGraphError::GraphWrite)?;
            relations_written += 1;
            evidences_written += 1;
        } else {
            relations_skipped += 1;
        }
    }

    // On error: return propagates, Transaction drops → implicit rollback (tracing::warn! on failure).
    // On success: explicit commit.
    tx.commit()
        .map_err(|se| CallGraphError::GraphWrite(anyhow::anyhow!("commit failed: {se}")))?;

    Ok(ApplyReport {
        elements_written,
        elements_skipped,
        relations_written,
        relations_skipped,
        evidences_written,
        source_artifacts_written,
        duration_ms: 0,
    })
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lang_label_returns_correct_strings() {
        assert_eq!(lang_label(&Language::Rust), "rust");
        assert_eq!(lang_label(&Language::TypeScript), "typescript");
        assert_eq!(lang_label(&Language::Python), "python");
        assert_eq!(lang_label(&Language::Go), "go");
    }

    #[test]
    fn test_canonical_key_determinism() {
        // Canonical key format: lang:file:name:line
        let key1 = format!(
            "{}:{}:{}:{}",
            lang_label(&Language::Rust),
            "src/lib.rs",
            "helper",
            5u32
        );
        let key2 = format!(
            "{}:{}:{}:{}",
            lang_label(&Language::Rust),
            "src/lib.rs",
            "helper",
            5u32
        );
        assert_eq!(key1, key2);
        assert_eq!(key1, "rust:src/lib.rs:helper:5");
    }

    // ─── Characterization tests — D2 safety net ─────────────────────────────────

    /// Recursively find the first node of `kind` in `node`'s subtree.
    /// Used to locate the target function/method node without depending on
    /// the exact tree-sitter child indices (which vary between grammars).
    fn find_first_of_kind<'a>(
        node: tree_sitter::Node<'a>,
        kind: &str,
    ) -> Option<tree_sitter::Node<'a>> {
        if node.kind() == kind {
            return Some(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = find_first_of_kind(child, kind) {
                return Some(found);
            }
        }
        None
    }

    /// Collect every descendant node of `kind` (depth-first, pre-order).
    /// Used to locate all method_invocations in a tree.
    fn collect_by_kind<'a>(node: tree_sitter::Node<'a>, kind: &str) -> Vec<tree_sitter::Node<'a>> {
        let mut out = Vec::new();
        collect_by_kind_into(node, kind, &mut out);
        out
    }

    fn collect_by_kind_into<'a>(
        node: tree_sitter::Node<'a>,
        kind: &str,
        out: &mut Vec<tree_sitter::Node<'a>>,
    ) {
        if node.kind() == kind {
            out.push(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            collect_by_kind_into(child, kind, out);
        }
    }

    /// Rust: function_item → identifier child → FunctionNode.
    #[test]
    fn charac_rust_function() {
        let source = "pub fn helper() {}";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        // tree-sitter Rust grammar names it `function_item`, not `function_declaration`.
        let fn_node = find_first_of_kind(root, "function_item")
            .expect("function_item must exist in parsed source");
        let result = super::extract_rust_function(
            fn_node,
            source,
            super::Language::Rust,
            "src/lib.rs",
            None,
        );
        let node = result.expect("extract_rust_function must return Some");
        assert_eq!(node.name, "helper");
        assert_eq!(node.kind, super::FunctionKind::Function);
        assert_eq!(node.language, super::Language::Rust);
        assert!(node.parent.is_none());
    }

    /// Rust: closure_expression → synthetic name.
    #[test]
    fn charac_rust_closure() {
        let source = "fn outer() { let x = || {}; }";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        // tree-sitter Rust grammar names it `closure_expression`.
        let closure_node = find_first_of_kind(root, "closure_expression")
            .expect("closure_expression must exist in parsed source");
        let result = super::extract_rust_closure(
            closure_node,
            source,
            super::Language::Rust,
            "src/lib.rs",
            Some("rust:src/lib.rs:outer:1"),
        );
        let node = result.expect("extract_rust_closure must return Some");
        assert!(node.name.starts_with("closure@"));
        assert_eq!(node.kind, super::FunctionKind::Closure);
        assert!(node.parent.is_some());
    }

    /// TypeScript: function_declaration → identifier child.
    #[test]
    fn charac_ts_function() {
        let source = "function greet(name: string) { return name; }";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let fn_node = root.child(0).unwrap();
        assert_eq!(fn_node.kind(), "function_declaration");
        let result = super::extract_ts_function(
            fn_node,
            source,
            super::Language::TypeScript,
            "main.ts",
            None,
        );
        let node = result.expect("extract_ts_function must return Some");
        assert_eq!(node.name, "greet");
        assert_eq!(node.kind, super::FunctionKind::Function);
        assert_eq!(node.language, super::Language::TypeScript);
    }

    /// TypeScript: method_definition → property_identifier child.
    #[test]
    fn charac_ts_method() {
        let source = "class Server { Save() { return null; } }";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let method_node = find_first_of_kind(root, "method_definition")
            .expect("method_definition must exist in parsed source");
        let result = super::extract_ts_method(
            method_node,
            source,
            super::Language::TypeScript,
            "main.ts",
            None,
        );
        let node = result.expect("extract_ts_method must return Some");
        assert_eq!(node.name, "Save");
        assert_eq!(node.kind, super::FunctionKind::Method);
    }

    /// TypeScript: arrow_function → synthetic name.
    #[test]
    fn charac_ts_arrow() {
        let source = "const f = (x: number) => x * 2;";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let arrow_node = find_first_of_kind(root, "arrow_function")
            .expect("arrow_function must exist in parsed source");
        let result = super::extract_ts_arrow(
            arrow_node,
            source,
            super::Language::TypeScript,
            "main.ts",
            None,
        );
        let node = result.expect("extract_ts_arrow must return Some");
        assert!(node.name.starts_with("arrow@"));
        assert_eq!(node.kind, super::FunctionKind::Closure);
    }

    /// Python: function_definition → identifier child.
    #[test]
    fn charac_python_function() {
        let source = "def greet(name):\n    return name\n";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let fn_node = root.child(0).unwrap();
        assert_eq!(fn_node.kind(), "function_definition");
        let result = super::extract_python_function(
            fn_node,
            source,
            super::Language::Python,
            "main.py",
            None,
            false,
        );
        let node = result.expect("extract_python_function must return Some");
        assert_eq!(node.name, "greet");
        assert_eq!(node.kind, super::FunctionKind::Function);
        assert_eq!(node.language, super::Language::Python);
    }

    /// Go: function_declaration → identifier child.
    #[test]
    fn charac_go_function() {
        let source = "package main\nfunc greet(name string) string { return name }";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        // package_clause child 0, then function_declaration
        let fn_node = root.child(1).unwrap();
        assert_eq!(fn_node.kind(), "function_declaration");
        let result =
            super::extract_go_function(fn_node, source, super::Language::Go, "main.go", None);
        let node = result.expect("extract_go_function must return Some");
        assert_eq!(node.name, "greet");
        assert_eq!(node.kind, super::FunctionKind::Function);
        assert_eq!(node.language, super::Language::Go);
    }

    /// Go: method_declaration → field_identifier child.
    #[test]
    fn charac_go_method() {
        let source =
            "package main\ntype Server struct{}\nfunc (s *Server) Save() error { return nil }";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let method_node = find_first_of_kind(root, "method_declaration")
            .expect("method_declaration must exist in parsed source");
        let result =
            super::extract_go_method(method_node, source, super::Language::Go, "main.go", None);
        let node = result.expect("extract_go_method must return Some");
        assert_eq!(node.name, "Save");
        assert_eq!(node.kind, super::FunctionKind::Method);
        assert_eq!(node.language, super::Language::Go);
    }

    /// Java: method_declaration → identifier child → FunctionNode.
    #[test]
    fn charac_java_method() {
        let source = "public class C { public void hello() {} }";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let method_node = find_first_of_kind(root, "method_declaration")
            .expect("method_declaration must exist in parsed source");
        let result =
            super::extract_java_method(method_node, source, super::Language::Java, "C.java", None);
        let node = result.expect("extract_java_method must return Some");
        assert_eq!(node.name, "hello");
        assert_eq!(node.kind, super::FunctionKind::Method);
        assert_eq!(node.language, super::Language::Java);
        assert!((node.confidence - 0.85).abs() < f64::EPSILON);
    }

    /// Java: constructor_declaration → identifier child → FunctionNode.
    /// tree-sitter Java names constructors as `constructor_declaration`
    /// (not `method_declaration`), but both share the `identifier` child.
    #[test]
    fn charac_java_constructor() {
        let source = "public class C { public C() {} }";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let ctor_node = find_first_of_kind(root, "constructor_declaration")
            .expect("constructor_declaration must exist in parsed source");
        let result =
            super::extract_java_method(ctor_node, source, super::Language::Java, "C.java", None);
        let node = result.expect("extract_java_method must return Some for constructors");
        assert_eq!(node.name, "C");
        assert_eq!(node.kind, super::FunctionKind::Method);
    }
    /// Java: method_invocation callee extraction (covers `obj.method()`).
    #[test]
    fn charac_java_callee_method_invocation() {
        let source = "public class C { void run() { helper.foo(); bar(); } }";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        // Two method_invocations: `helper.foo()` and `bar()`. We want the
        // first one — assert via deep search order (helper.foo() comes
        // first in source).
        let inv_node = find_first_of_kind(root, "method_invocation")
            .expect("method_invocation must exist in parsed source");
        let callee = super::extract_java_callee(inv_node, source).expect("callee must be Some");
        assert_eq!(callee, "foo");

        // Also assert the simple case: standalone `bar()` resolves to "bar".
        // Find the SECOND method_invocation by walking past the first.
        let invocations: Vec<_> = collect_by_kind(root, "method_invocation");
        assert_eq!(invocations.len(), 2);
        let bar_callee =
            super::extract_java_callee(invocations[1], source).expect("callee must be Some");
        assert_eq!(bar_callee, "bar");
    }

    /// Kotlin: function_declaration → simple_identifier child → FunctionNode.
    #[test]
    fn charac_kotlin_function() {
        let source = "fun helper() {}";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_kotlin_sg::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let fn_node = find_first_of_kind(root, "function_declaration")
            .expect("function_declaration must exist in parsed source");
        let result = super::extract_kotlin_function(
            fn_node,
            source,
            super::Language::Kotlin,
            "main.kt",
            None,
        );
        let node = result.expect("extract_kotlin_function must return Some");
        assert_eq!(node.name, "helper");
        assert_eq!(node.kind, super::FunctionKind::Method);
        assert_eq!(node.language, super::Language::Kotlin);
        assert!((node.confidence - 0.85).abs() < f64::EPSILON);
    }

    /// Kotlin: navigation_expression callee extraction (covers `obj.method()`).
    #[test]
    fn charac_kotlin_callee_navigation_expression() {
        let source = "fun run() { helper.foo(); bar() }";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_kotlin_sg::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        // Two call_expressions: `helper.foo()` and `bar()`.
        let calls: Vec<_> = collect_by_kind(root, "call_expression");
        assert_eq!(calls.len(), 2);

        // First call: helper.foo() — resolve to "foo" (via navigation_suffix).
        let callee = super::extract_kotlin_callee(calls[0], source).expect("callee must be Some");
        assert_eq!(callee, "foo");

        // Second call: bar() — resolve to "bar" (direct simple_identifier).
        let bar_callee =
            super::extract_kotlin_callee(calls[1], source).expect("callee must be Some");
        assert_eq!(bar_callee, "bar");
    }

    #[test]
    fn test_confidence_per_language() {
        // W4: Real regression gate — extract on a Go source file and assert confidence.
        let tmp = tempfile::TempDir::new().unwrap();
        let go_file = tmp.path().join("main.go");
        std::fs::write(
            &go_file,
            "package main\nfunc (s Server) Name() string { return \"\" }\n",
        )
        .unwrap();
        let source = std::fs::read_to_string(&go_file).unwrap();
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(&source, None).unwrap();
        let root = tree.root_node();
        // method_declaration is at child index 1 (after package_clause)
        let method_node = root.child(1).unwrap();
        assert_eq!(method_node.kind(), "method_declaration");
        let result = super::extract_go_method(
            method_node,
            &source,
            super::Language::Go,
            go_file.to_str().unwrap(),
            None,
        );
        let node = result.expect("extract_go_method must return Some");
        assert_eq!(node.confidence, 0.85, "Go confidence must be 0.85");
        assert_eq!(
            node.confidence,
            super::Language::Go.confidence(),
            "lang.confidence() must match"
        );

        // Also verify other languages via Language::confidence() method
        assert_eq!(super::Language::Rust.confidence(), 0.90);
        assert_eq!(super::Language::TypeScript.confidence(), 0.85);
        assert_eq!(super::Language::Python.confidence(), 0.80);
        assert_eq!(super::Language::Go.confidence(), 0.85);
    }

    #[test]
    fn test_call_graph_report_serialize() {
        let report = CallGraphReport {
            schema_version: "1.0".to_string(),
            project: ProjectMeta {
                root: "/tmp".to_string(),
                files_scanned: 2,
                languages: BTreeMap::new(),
                duration_ms: 10,
            },
            nodes: vec![],
            edges: vec![],
            errors: vec![],
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"schemaVersion\":\"1.0\""));
    }

    #[test]
    fn test_extract_error_serialize() {
        let err = ExtractError {
            strategy: "rust".to_string(),
            path: "src/lib.rs".to_string(),
            message: "TSG parse error".to_string(),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("TSG parse error"));
    }

    #[test]
    fn test_apply_report_serialize() {
        let report = ApplyReport {
            elements_written: 5,
            elements_skipped: 2,
            relations_written: 3,
            relations_skipped: 1,
            evidences_written: 3,
            source_artifacts_written: 2,
            duration_ms: 42,
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"elements_written\":5"));
        assert!(
            !json.contains("seed_writes"),
            "seed_writes field must be removed (BREAK-1)"
        );
    }
}
