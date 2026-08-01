//! Sequence projection: BFS over code.calls edges into an ordered Interaction list.
//! READ-ONLY by invariant (SCN-217). No graph writes under any flag.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::code::call_graph::MessageKind;

/// JSON Schema for SequenceReport (JSON Schema 2020-12).
/// NOTE: 3 levels up (archctl/src/code/ → repo root), matching c4_discover.rs:16.
pub const SEQUENCE_REPORT_SCHEMA: &str =
    include_str!("../../../schemas/sequence-report.schema.json");

/// One tuple of a sequence diagram. Matches behavior.interaction
/// (DATA-MODEL-LADYBUGDB.md §9): sender → receiver with a message_kind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Interaction {
    /// 1-based, BFS visit order (SCN-215)
    pub order_key: u32,
    /// FunctionNode.canonical_key (caller)
    pub sender: String,
    /// callee canonical_key or unresolved name
    pub receiver: String,
    /// SyncCall | AsyncCall | Return
    pub message_kind: MessageKind,
    /// 0..=depth_limit
    pub depth: u32,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
}

/// Top-level report emitted by `archctl code sequence --json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceReport {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    /// The selector used to start the projection
    pub from: FromSelector,
    /// Interactions in BFS visit order
    pub interactions: Vec<Interaction>,
    /// True if a cycle was detected (SCN-212)
    pub cyclic: bool,
    /// True if hit depth or max_interactions limit
    pub truncated: bool,
    /// Count of all reachable interactions (before truncation)
    pub total_reachable: usize,
    /// Time to compute in milliseconds
    pub duration_ms: u64,
}

/// --from selector. 3 forms (SCN-216): name | file:line | canonical_key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FromSelector {
    ByName { name: String },
    ByFileLine { file: PathBuf, line: u32 },
    ByCanonicalKey { canonical_key: String },
}

/// Errors during sequence projection.
#[derive(Debug, thiserror::Error)]
pub enum SequenceError {
    #[error("ambiguous symbol '{name}' — {n} candidates: {candidates:?}")]
    AmbiguousSymbol {
        name: String,
        n: usize,
        candidates: Vec<String>,
    },
    #[error("symbol not found: {0}")]
    SymbolNotFound(String),
    #[error("depth limit exceeded: {actual} > {max}")]
    DepthLimitExceeded { actual: u32, max: u32 },
    #[error("graph read failed: {0}")]
    GraphReadFailed(#[from] anyhow::Error),
}
