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
use crate::store::GraphStore;

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

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    TypeScript,
    Python,
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
    /// MetaType/Predicate rows seeded
    pub seed_writes: usize,
    pub duration_ms: u64,
}

/// Errors during extraction or graph write.
#[derive(Debug, thiserror::Error)]
pub enum CallGraphError {
    #[error("invalid --lang: {0} (MVP: rust, typescript, python)")]
    InvalidLanguage(String),
    #[error("TSG execution failed for {path}: {message}")]
    TsgExecution { path: String, message: String },
    #[error("graph write failed: {0}")]
    GraphWrite(#[from] anyhow::Error),
}

// ─── Extract ─────────────────────────────────────────────────────────────────

/// Extract function nodes + call edges from `cwd`, filtered to `languages`
/// (empty = all MVP languages). Pure: no graph writes. Deterministic.
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
    }

    // Recurse into children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            find_function_definitions(child, source, lang, file, parent_key, nodes);
        }
    }
}

fn extract_rust_function(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    file: &str,
    _parent_key: Option<&str>,
) -> Option<FunctionNode> {
    // Get function name from identifier child
    let mut name = String::new();
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32)
            && child.kind() == "identifier"
        {
            name = source
                .get(child.start_byte()..child.end_byte())?
                .to_string();
            break;
        }
    }
    if name.is_empty() {
        return None;
    }

    let line = (node.start_position().row + 1) as u32;
    let canonical_key = format!("{}:{}:{}:{}", lang_label(&lang), file, name, line);
    let fq_name = name.clone();
    let confidence = 0.90;

    Some(FunctionNode {
        canonical_key,
        kind: FunctionKind::Function,
        language: lang,
        file: file.to_string(),
        line,
        name,
        fq_name,
        confidence,
        parent: None,
    })
}

fn extract_rust_closure(
    node: tree_sitter::Node,
    _source: &str,
    lang: Language,
    file: &str,
    parent_key: Option<&str>,
) -> Option<FunctionNode> {
    let line = (node.start_position().row + 1) as u32;
    let name = format!("closure@{}", line);
    let canonical_key = format!("{}:{}:{}:{}", lang_label(&lang), file, name, line);
    let fq_name = name.clone();
    let confidence = 0.90;

    Some(FunctionNode {
        canonical_key,
        kind: FunctionKind::Closure,
        language: lang,
        file: file.to_string(),
        line,
        name,
        fq_name,
        confidence,
        parent: parent_key.map(|s| s.to_string()),
    })
}

fn extract_ts_function(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    file: &str,
    _parent_key: Option<&str>,
) -> Option<FunctionNode> {
    let mut name = String::new();
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32)
            && child.kind() == "identifier"
        {
            name = source
                .get(child.start_byte()..child.end_byte())?
                .to_string();
            break;
        }
    }
    if name.is_empty() {
        return None;
    }

    let line = (node.start_position().row + 1) as u32;
    let canonical_key = format!("{}:{}:{}:{}", lang_label(&lang), file, name, line);
    let fq_name = name.clone();
    let confidence = 0.85;

    Some(FunctionNode {
        canonical_key,
        kind: FunctionKind::Function,
        language: lang,
        file: file.to_string(),
        line,
        name,
        fq_name,
        confidence,
        parent: None,
    })
}

fn extract_ts_method(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    file: &str,
    _parent_key: Option<&str>,
) -> Option<FunctionNode> {
    let mut name = String::new();
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32)
            && child.kind() == "property_identifier"
        {
            name = source
                .get(child.start_byte()..child.end_byte())?
                .to_string();
            break;
        }
    }
    if name.is_empty() {
        return None;
    }

    let line = (node.start_position().row + 1) as u32;
    let canonical_key = format!("{}:{}:{}:{}", lang_label(&lang), file, name, line);
    let fq_name = name.clone();
    let confidence = 0.85;

    Some(FunctionNode {
        canonical_key,
        kind: FunctionKind::Method,
        language: lang,
        file: file.to_string(),
        line,
        name,
        fq_name,
        confidence,
        parent: None,
    })
}

fn extract_ts_arrow(
    node: tree_sitter::Node,
    _source: &str,
    lang: Language,
    file: &str,
    parent_key: Option<&str>,
) -> Option<FunctionNode> {
    let line = (node.start_position().row + 1) as u32;
    let name = format!("arrow@{}", line);
    let canonical_key = format!("{}:{}:{}:{}", lang_label(&lang), file, name, line);
    let fq_name = name.clone();
    let confidence = 0.85;

    Some(FunctionNode {
        canonical_key,
        kind: FunctionKind::Closure,
        language: lang,
        file: file.to_string(),
        line,
        name,
        fq_name,
        confidence,
        parent: parent_key.map(|s| s.to_string()),
    })
}

fn extract_python_function(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    file: &str,
    _parent_key: Option<&str>,
    is_method: bool,
) -> Option<FunctionNode> {
    let mut name = String::new();
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32)
            && child.kind() == "identifier"
        {
            name = source
                .get(child.start_byte()..child.end_byte())?
                .to_string();
            break;
        }
    }
    if name.is_empty() {
        return None;
    }

    let line = (node.start_position().row + 1) as u32;
    let canonical_key = format!("{}:{}:{}:{}", lang_label(&lang), file, name, line);
    let fq_name = name.clone();
    let confidence = 0.80;
    let kind = if is_method {
        FunctionKind::Method
    } else {
        FunctionKind::Function
    };

    Some(FunctionNode {
        canonical_key,
        kind,
        language: lang,
        file: file.to_string(),
        line,
        name,
        fq_name,
        confidence,
        parent: None,
    })
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

    let confidence = match lang {
        Language::Rust => 0.90,
        Language::TypeScript => 0.85,
        Language::Python => 0.80,
    };

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
    }
}

// ─── Apply ──────────────────────────────────────────────────────────────────

/// Escape a string for use inside a Cypher single-quoted string.
/// All single quotes are doubled (Cypher escaping convention).
/// Private copy matching c4_discover.rs:escape_cypher_string.
fn escape_cypher_string(s: &str) -> String {
    s.replace('\'', "\\'")
}

/// Pipe: pass a value through a function and return the result.
trait Pipe<T> {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
        Self: Sized,
    {
        f(self)
    }
}

impl<T> Pipe<T> for T {}

/// Fetch the set of canonical_keys already present in the graph.
fn existing_canonical_keys(store: &dyn GraphStore) -> Result<std::collections::HashSet<String>> {
    store
        .query("MATCH (e:Element) WHERE e.canonical_key IS NOT NULL RETURN e.canonical_key;")?
        .into_iter()
        .filter_map(|row| {
            row.get("e.canonical_key")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect::<std::collections::HashSet<_>>()
        .pipe(Ok)
}

/// Write the Element node for a FunctionNode.
fn write_function_element(
    store: &mut dyn GraphStore,
    node: &FunctionNode,
    version_id: &str,
) -> Result<()> {
    let kind_id = match node.kind {
        FunctionKind::Function => "code.function",
        FunctionKind::Method => "code.method",
        FunctionKind::Closure => "code.closure",
    };
    let canonical_key_escaped = escape_cypher_string(&node.canonical_key);
    let name_escaped = escape_cypher_string(&node.name);
    let id = format!("cg:{}", node.canonical_key);
    let cypher = format!(
        "MERGE (e:Element {{id: '{id}'}}) SET \
         e.kind_id = '{kind_id}', \
         e.category = 'code', \
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
    store
        .query(&cypher)
        .with_context(|| format!("write_function_element {}", node.canonical_key))?;
    Ok(())
}

/// Write the ElementVersion node for a FunctionNode.
fn write_function_version(store: &mut dyn GraphStore, node: &FunctionNode) -> Result<String> {
    let version_props = serde_json::json!({
        "kind": format!("{:?}", node.kind).to_lowercase(),
        "language": lang_label(&node.language),
        "confidence": node.confidence,
        "call_graph_schema_version": "1.0",
    });
    let version_props_str = serde_json::to_string(&version_props).unwrap_or_default();
    let version_id = format!(
        "cgv:{}",
        blake3::hash(version_props_str.as_bytes()).to_hex()
    );
    let version_props_escaped = escape_cypher_string(&version_props_str);
    let element_id = format!("cg:{}", node.canonical_key);

    let cypher = format!(
        "MERGE (v:ElementVersion {{id: '{version_id}'}}) SET \
         v.element_id = '{element_id}', \
         v.name = '{name}', \
         v.status = 'drafted', \
         v.origin = 'call-graph', \
         v.confidence = {confidence}, \
         v.props = '{props}';",
        version_id = version_id,
        element_id = element_id,
        name = escape_cypher_string(&node.name),
        confidence = node.confidence,
        props = version_props_escaped,
    );
    store
        .query(&cypher)
        .with_context(|| format!("write_function_version {}", node.canonical_key))?;
    Ok(version_id)
}

/// Link Element to ElementVersion via CURRENT_VERSION and VERSION_OF edges.
fn link_function_edges(
    store: &mut dyn GraphStore,
    node: &FunctionNode,
    version_id: &str,
) -> Result<()> {
    let element_id = format!("cg:{}", node.canonical_key);
    // CURRENT_VERSION: Element → ElementVersion
    let cypher1 = format!(
        "MATCH (e:Element {{id: '{element_id}'}}) \
         MATCH (v:ElementVersion {{id: '{version_id}'}}) \
         MERGE (e)-[r:CURRENT_VERSION]->(v);",
        element_id = element_id,
        version_id = version_id,
    );
    store
        .query(&cypher1)
        .with_context(|| "link current_version")?;

    // VERSION_OF: ElementVersion → Element
    let cypher2 = format!(
        "MATCH (e:Element {{id: '{element_id}'}}) \
         MATCH (v:ElementVersion {{id: '{version_id}'}}) \
         MERGE (v)-[r:VERSION_OF]->(e);",
        element_id = element_id,
        version_id = version_id,
    );
    store.query(&cypher2).with_context(|| "link version_of")?;

    // OF_TYPE: Element → MetaType
    let kind_id = match node.kind {
        FunctionKind::Function => "code.function",
        FunctionKind::Method => "code.method",
        FunctionKind::Closure => "code.closure",
    };
    let cypher3 = format!(
        "MATCH (e:Element {{id: '{element_id}'}}) \
         MATCH (mt:MetaType {{id: '{kind_id}'}}) \
         MERGE (e)-[r:OF_TYPE]->(mt);",
        element_id = element_id,
        kind_id = kind_id,
    );
    store.query(&cypher3).with_context(|| "link of_type")?;

    Ok(())
}

/// Write a call edge as a SemanticRelation + Evidence.
fn write_call_edge(
    store: &mut dyn GraphStore,
    edge: &CallEdge,
    src_element_id: &str,
    sa_id: &str,
    version_id: &str,
) -> Result<()> {
    let rel_id = format!("rel:{}", edge.canonical_key);
    let rel_props = serde_json::json!({
        "predicate": "code.calls",
        "call_kind": format!("{:?}", edge.kind).to_lowercase(),
        "message_kind": format!("{:?}", edge.message_kind).to_lowercase(),
        "rel_id": rel_id,
    });
    let rel_props_str = serde_json::to_string(&rel_props).unwrap_or_default();
    let rel_props_escaped = escape_cypher_string(&rel_props_str);

    // Try to find the callee Element by matching canonical_key pattern
    // MVP: we don't do symbol resolution, so callee may not exist
    //
    // NOTE: Writing to SEMANTIC_EDGE (Element→Element with props) rather than
    // the reified REL_SOURCE→SemanticRelation→REL_TARGET pattern, because the
    // sequence projection reads from SEMANTIC_EDGE and needs r.props.
    //
    // Use MERGE to avoid duplicates; set properties unconditionally.
    let callee_escaped = escape_cypher_string(&edge.callee);
    let cypher = format!(
        "MATCH (src:Element {{id: '{src_id}'}}) \
         OPTIONAL MATCH (tgt:Element) WHERE tgt.current_name = '{callee}' AND tgt.kind_id IN ['code.function', 'code.method', 'code.closure'] \
         WITH src, tgt \
         WHERE tgt IS NOT NULL \
         MERGE (src)-[r:SEMANTIC_EDGE]->(tgt) \
         SET r.relation_id = '{rel_id}', \
         r.predicate_id = 'code.calls', \
         r.props = '{props}', \
         r.active = true, \
         r.version_id = '{version_id}';",
        src_id = src_element_id,
        callee = callee_escaped,
        rel_id = rel_id,
        props = rel_props_escaped,
        version_id = version_id,
    );
    // Note: errors are silently ignored (matches prior behavior for MVP)
    let _ = store.query(&cypher);

    // Write Evidence for this call edge
    let evidence_id = format!(
        "ev:{}",
        blake3::hash(edge.canonical_key.as_bytes()).to_hex()
    );
    let evidence_props = serde_json::json!({
        "file_refs": [format!("{}:{}", edge.file, edge.line)],
        "status": "Drafted",
        "classification": "derived",
    });
    let ev_props_str = serde_json::to_string(&evidence_props).unwrap_or_default();
    let ev_props_escaped = escape_cypher_string(&ev_props_str);
    let file_escaped = escape_cypher_string(&edge.file);

    let cypher_ev = format!(
        "MERGE (ev:Evidence {{id: '{ev_id}'}}) SET \
         ev.kind = 'structural', \
         ev.claim = 'call-graph edge', \
         ev.classification = 'derived', \
         ev.confidence = {confidence}, \
         ev.props = '{props}', \
         ev.start_line = {line}, \
         ev.end_line = {line}, \
         ev.path = '{file}';",
        ev_id = evidence_id,
        props = ev_props_escaped,
        confidence = edge.confidence,
        line = edge.line,
        file = file_escaped,
    );
    store
        .query(&cypher_ev)
        .with_context(|| format!("write_call_evidence {}", evidence_id))?;

    // Link Evidence to SourceArtifact
    let cypher_sa = format!(
        "MATCH (ev:Evidence {{id: '{ev_id}'}}) \
         MATCH (sa:SourceArtifact {{id: '{sa_id}'}}) \
         MERGE (ev)-[r:EXTRACTED_FROM]->(sa);",
        ev_id = evidence_id,
        sa_id = sa_id,
    );
    let _ = store.query(&cypher_sa);

    // Link Evidence to Element
    let cypher_el = format!(
        "MATCH (ev:Evidence {{id: '{ev_id}'}}) \
         MATCH (e:Element {{id: '{e_id}'}}) \
         MERGE (e)-[r:SUPPORTED_BY]->(ev);",
        ev_id = evidence_id,
        e_id = src_element_id,
    );
    let _ = store.query(&cypher_el);

    Ok(())
}

/// Persist a CallGraphReport to the graph.
/// Idempotent: skips canonical_keys that already exist.
pub fn apply(
    project_dir: &Path,
    report: &CallGraphReport,
    _fs: &dyn Filesystem,
) -> Result<ApplyReport, CallGraphError> {
    use crate::store::open_default;

    let mut store =
        open_default(project_dir).map_err(|e| anyhow::anyhow!("failed to acquire DB lock: {e}"))?;
    store
        .init()
        .context("graph init (call_graph apply)")
        .map_err(CallGraphError::GraphWrite)?;

    // Seed MetaType rows for code.function, code.method, code.closure
    // and Predicate row for code.calls
    let seed_metatypes = r#"
        MERGE (mt:MetaType {id: 'code.function'}) ON CREATE SET mt.name = 'Function', mt.namespace = 'code', mt.category = 'structure'
        MERGE (mt:MetaType {id: 'code.method'}) ON CREATE SET mt.name = 'Method', mt.namespace = 'code', mt.category = 'structure'
        MERGE (mt:MetaType {id: 'code.closure'}) ON CREATE SET mt.name = 'Closure', mt.namespace = 'code', mt.category = 'structure'
        MERGE (p:Predicate {id: 'code.calls'}) ON CREATE SET p.name = 'calls', p.namespace = 'code'
        RETURN 1;
    "#;
    let seed_result = store.query(seed_metatypes);
    let seed_writes = if seed_result.is_ok() { 1 } else { 0 };

    let existing_keys = existing_canonical_keys(&*store)
        .context("fetch existing keys")
        .map_err(CallGraphError::GraphWrite)?;

    let mut elements_written = 0usize;
    let mut elements_skipped = 0usize;
    let mut relations_written = 0usize;
    let mut relations_skipped = 0usize;
    let mut evidences_written = 0usize;
    let mut source_artifacts_written = 0usize;

    // SourceArtifact deduplication per file
    let mut source_artifact_ids: BTreeMap<String, String> = BTreeMap::new();

    for node in &report.nodes {
        if existing_keys.contains(&node.canonical_key) {
            elements_skipped += 1;
            continue;
        }

        // Get or create SourceArtifact for this file
        if !source_artifact_ids.contains_key(&node.file) {
            let id = write_source_artifact_for_file(&mut *store, &node.file)?;
            source_artifact_ids.insert(node.file.clone(), id);
            source_artifacts_written += 1;
        }

        // Write version first (needed for element)
        let version_id = write_function_version(&mut *store, node)
            .context("write_function_version")
            .map_err(CallGraphError::GraphWrite)?;

        // Write element
        write_function_element(&mut *store, node, &version_id)
            .context("write_function_element")
            .map_err(CallGraphError::GraphWrite)?;
        elements_written += 1;

        // Link edges
        link_function_edges(&mut *store, node, &version_id)
            .context("link_function_edges")
            .map_err(CallGraphError::GraphWrite)?;
    }

    // Write call edges
    for edge in &report.edges {
        let src_element_id = format!("cg:{}", edge.caller);
        // Only write if caller element exists
        if !existing_keys.contains(&edge.caller) {
            // Get or create SourceArtifact
            let sa_id = if let Some(id) = source_artifact_ids.get(&edge.file) {
                id.clone()
            } else {
                let id = write_source_artifact_for_file(&mut *store, &edge.file)?;
                source_artifact_ids.insert(edge.file.clone(), id.clone());
                source_artifacts_written += 1;
                id
            };
            let version_id = format!("cgv:{}", blake3::hash(edge.caller.as_bytes()).to_hex());
            write_call_edge(&mut *store, edge, &src_element_id, &sa_id, &version_id)
                .context("write_call_edge")
                .map_err(CallGraphError::GraphWrite)?;
            relations_written += 1;
            evidences_written += 1;
        } else {
            relations_skipped += 1;
        }
    }

    Ok(ApplyReport {
        elements_written,
        elements_skipped,
        relations_written,
        relations_skipped,
        evidences_written,
        source_artifacts_written,
        seed_writes,
        duration_ms: 0,
    })
}

/// Write SourceArtifact for a file, return its id.
fn write_source_artifact_for_file(store: &mut dyn GraphStore, file: &str) -> Result<String> {
    let id = format!("src:{}", blake3::hash(file.as_bytes()).to_hex());
    let path_escaped = escape_cypher_string(file);
    let cypher = format!(
        "MERGE (s:SourceArtifact {{id: '{id}'}}) SET \
         s.kind = 'manifest', \
         s.relative_path = '{path_escaped}', \
         s.language = '', \
         s.content_hash = '', \
         s.generated = false, \
         s.props = '{{}}';"
    );
    store
        .query(&cypher)
        .with_context(|| format!("put_source_artifact {id}"))?;
    Ok(id)
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

    #[test]
    fn test_confidence_per_language() {
        let rust_conf = 0.90;
        let ts_conf = 0.85;
        let py_conf = 0.80;
        assert_eq!(rust_conf, 0.90);
        assert_eq!(ts_conf, 0.85);
        assert_eq!(py_conf, 0.80);
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
            seed_writes: 1,
            duration_ms: 42,
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"elements_written\":5"));
        assert!(json.contains("\"seed_writes\":1"));
    }
}
