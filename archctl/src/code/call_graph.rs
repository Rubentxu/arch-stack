//! Call-graph extraction engine + report types.
//!
//! The `c4_discover.rs` analogue for caller→callee edges. Pure extraction
//! (no LLM, no clock) via tree-sitter-graph; deterministic; idempotent apply.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, clap::ValueEnum)]
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


