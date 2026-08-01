//! Sequence projection: BFS over code.calls edges into an ordered Interaction list.
//! READ-ONLY by invariant (SCN-217). No graph writes under any flag.

use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::code::call_graph::MessageKind;
use crate::store::{GraphStore, LbugStore};

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

/// Escape a string for use inside a Cypher single-quoted string.
/// All single quotes are doubled (Cypher escaping convention).
/// Private copy matching call_graph.rs:escape_cypher_string.
fn escape_cypher_string(s: &str) -> String {
    s.replace('\'', "\\'")
}

/// Project a sequence diagram from `from` by BFS over code.calls edges.
/// Default depth = 5 (SCN-213), default max_interactions = 500.
pub fn project_sequence(
    project_dir: &std::path::Path,
    from: FromSelector,
    depth_limit: u32,
    max_interactions: Option<u32>,
) -> Result<SequenceReport, SequenceError> {
    let start = Instant::now();

    // Open the store
    let store = LbugStore::open(project_dir)
        .map_err(|e| SequenceError::GraphReadFailed(anyhow::anyhow!("store open failed: {e}")))?;

    // 1. Resolve `from` to canonical_key
    let start_key = resolve_selector(&store as &dyn GraphStore, &from)?;

    // 2. Iterative BFS (NOT recursive — prevents stack overflow)
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, u32)> = VecDeque::new();
    let mut interactions: Vec<Interaction> = Vec::new();
    let mut cyclic = false;
    let mut truncated = false;
    let cap = max_interactions.unwrap_or(500) as usize;
    let mut order_key = 0u32;

    queue.push_back((start_key.clone(), 0));
    visited.insert(start_key.clone());

    while let Some((node_key, depth)) = queue.pop_front() {
        if depth >= depth_limit {
            continue;
        }

        // Find outgoing REL_SOURCE edges from this node
        let cypher = format!(
            "MATCH (src:Element {{canonical_key: '{}'}})-[r:REL_SOURCE]->(tgt:Element) \
             WHERE r.predicate = 'code.calls' \
             RETURN tgt.canonical_key AS receiver, tgt.current_name AS receiver_name, \
                    r.props AS rel_props, r.kind AS rel_kind \
             ORDER BY tgt.canonical_key",
            escape_cypher_string(&node_key)
        );

        let rows = (&store as &dyn GraphStore)
            .query(&cypher)
            .map_err(SequenceError::GraphReadFailed)?;

        for row in rows {
            if interactions.len() >= cap {
                truncated = true;
                break;
            }

            let receiver = row
                .get("receiver")
                .and_then(|c| c.as_str())
                .unwrap_or_default()
                .to_string();
            let receiver_name = row
                .get("receiver_name")
                .and_then(|c| c.as_str())
                .unwrap_or_default()
                .to_string();
            // Use receiver canonical_key if available, otherwise fall back to name
            let receiver_key = if receiver.is_empty() {
                receiver_name.clone()
            } else {
                receiver.clone()
            };

            // Parse message_kind from rel_props JSON
            let message_kind = parse_message_kind_from_row(&row);

            // Extract file/line from rel_props if available
            let (file, line) = extract_location_from_row(&row);

            order_key += 1;
            interactions.push(Interaction {
                order_key,
                sender: node_key.clone(),
                receiver: receiver_key,
                message_kind,
                depth: depth + 1,
                file,
                line,
            });

            if !visited.contains(&receiver) {
                visited.insert(receiver.clone());
                queue.push_back((receiver, depth + 1));
            } else {
                cyclic = true;
            }
        }

        if truncated {
            break;
        }
    }

    let total_reachable = interactions.len();

    Ok(SequenceReport {
        schema_version: "1.0".to_string(),
        from,
        interactions,
        cyclic,
        truncated,
        total_reachable,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Resolve a FromSelector to a canonical_key.
fn resolve_selector(
    store: &dyn GraphStore,
    from: &FromSelector,
) -> Result<String, SequenceError> {
    let cypher = match from {
        FromSelector::ByName { name } => format!(
            "MATCH (e:Element) WHERE e.current_name = '{}' AND e.kind_id IN ['code.function', 'code.method', 'code.closure'] RETURN e.canonical_key AS ck ORDER BY e.canonical_key LIMIT 2",
            escape_cypher_string(name)
        ),
        FromSelector::ByFileLine { file, line } => format!(
            "MATCH (e:Element) WHERE e.file = '{}' AND e.line = {} AND e.kind_id IN ['code.function', 'code.method', 'code.closure'] RETURN e.canonical_key AS ck ORDER BY e.canonical_key LIMIT 2",
            escape_cypher_string(&file.to_string_lossy()),
            line
        ),
        FromSelector::ByCanonicalKey { canonical_key } => format!(
            "MATCH (e:Element {{canonical_key: '{}'}}) RETURN e.canonical_key AS ck LIMIT 1",
            escape_cypher_string(canonical_key)
        ),
    };

    let rows = store
        .query(&cypher)
        .map_err(SequenceError::GraphReadFailed)?;

    match rows.len() {
        0 => Err(SequenceError::SymbolNotFound(format!("{:?}", from))),
        1 => Ok(rows[0]
            .get("ck")
            .and_then(|c| c.as_str())
            .unwrap_or_default()
            .to_string()),
        n => {
            let candidates: Vec<String> = rows
                .iter()
                .filter_map(|r| r.get("ck").and_then(|c| c.as_str()).map(String::from))
                .collect();
            Err(SequenceError::AmbiguousSymbol {
                name: format!("{:?}", from),
                n,
                candidates,
            })
        }
    }
}

/// Parse MessageKind from a row's rel_props JSON.
fn parse_message_kind_from_row(row: &crate::row::Row) -> MessageKind {
    // Try to extract from rel_props JSON: { "message_kind": "sync_call" | "async_call" | "return" }
    if let Some(props_cell) = row.get("rel_props") {
        if let Some(props_str) = props_cell.as_str() {
            // Props are stored as escaped JSON string
            let unescaped = props_str.replace("\\'", "'");
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&unescaped) {
                if let Some(msg_kind) = parsed.get("message_kind").and_then(|v| v.as_str()) {
                    return match msg_kind {
                        "async_call" => MessageKind::AsyncCall,
                        "return" => MessageKind::Return,
                        _ => MessageKind::SyncCall,
                    };
                }
            }
        }
    }
    MessageKind::SyncCall
}

/// Extract file and line from a row's rel_props JSON.
fn extract_location_from_row(row: &crate::row::Row) -> (Option<PathBuf>, Option<u32>) {
    if let Some(props_cell) = row.get("rel_props") {
        if let Some(props_str) = props_cell.as_str() {
            let unescaped = props_str.replace("\\'", "'");
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&unescaped) {
                let file = parsed
                    .get("file")
                    .and_then(|v| v.as_str())
                    .map(PathBuf::from);
                let line = parsed
                    .get("line")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32);
                return (file, line);
            }
        }
    }
    (None, None)
}

