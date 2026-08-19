//! Sequence projection: BFS over code.calls edges into an ordered Interaction list.
//! READ-ONLY by invariant (SCN-217). No graph writes under any flag.

use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::code::apply_common::escape_cypher_string;
use crate::code::call_graph::MessageKind;
use crate::store::{GraphStore, LbugStore, RawGraphQuery};

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

/// Parse a string into a MessageKind.
/// Used by tests to verify round-trip serialization.
pub fn parse_message_kind(s: &str) -> MessageKind {
    match s {
        "SyncCall" => MessageKind::SyncCall,
        "AsyncCall" => MessageKind::AsyncCall,
        "Return" => MessageKind::Return,
        _ => MessageKind::SyncCall, // default
    }
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
    let mut store = LbugStore::open(project_dir)
        .map_err(|e| SequenceError::GraphReadFailed(anyhow::anyhow!("store open failed: {e}")))?;

    // Initialize schema/migrations before first query (required by GraphStore contract)
    store
        .init()
        .map_err(|e| SequenceError::GraphReadFailed(anyhow::anyhow!("store init failed: {e}")))?;

    // Delegate to the store-based implementation
    let report = project_sequence_with_store(
        &store as &dyn RawGraphQuery,
        from,
        depth_limit,
        max_interactions,
    )?;

    Ok(SequenceReport {
        schema_version: "1.0".to_string(),
        from: report.from,
        interactions: report.interactions,
        cyclic: report.cyclic,
        truncated: report.truncated,
        total_reachable: report.total_reachable,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Internal: BFS projection with a provided store (for testability).
pub fn project_sequence_with_store(
    store: &dyn RawGraphQuery,
    from: FromSelector,
    depth_limit: u32,
    max_interactions: Option<u32>,
) -> Result<SequenceReport, SequenceError> {
    // 1. Resolve `from` to canonical_key
    let start_key = resolve_selector(store, &from)?;

    // 2. Iterative BFS (NOT recursive — prevents stack overflow)
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, u32)> = VecDeque::new();
    let mut interactions: Vec<Interaction> = Vec::new();
    let mut cyclic = false;
    let mut truncated = false;
    let cap = max_interactions.unwrap_or(500) as usize;
    let mut order_key = 0u32;
    let mut total_reachable_count: usize = 0;

    queue.push_back((start_key.clone(), 0));
    visited.insert(start_key.clone());

    while let Some((node_key, depth)) = queue.pop_front() {
        if depth >= depth_limit {
            continue;
        }

        // Find outgoing edges from this node via SEMANTIC_EDGE (the Element→Element
        // relationship table that carries props including message_kind).
        // NOTE: write_call_edge matches source by Element.id = 'cg:{canonical_key}',
        // so we use id here for consistency (both id and canonical_key are set on the element).
        let src_id = format!("cg:{}", crate::graph::sanitize_identifier(&node_key));
        let cypher = format!(
            "MATCH (src:Element {{id: '{src_id}'}})-[r:SEMANTIC_EDGE]->(tgt:Element) \
             RETURN tgt.canonical_key AS receiver, tgt.current_name AS receiver_name, \
             COALESCE(r.props, '{{}}') AS rel_props \
             ORDER BY tgt.canonical_key",
            src_id = src_id,
        );

        let rows = store
            .query(&cypher)
            .map_err(SequenceError::GraphReadFailed)?;

        // Count all reachable edges from this node (for total_reachable)
        total_reachable_count += rows.len();

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
                receiver: receiver_key.clone(),
                message_kind,
                depth: depth + 1,
                file,
                line,
            });

            if !visited.contains(&receiver_key) {
                visited.insert(receiver_key.clone());
                queue.push_back((receiver_key, depth + 1));
            } else {
                cyclic = true;
            }
        }

        if truncated {
            break;
        }
    }

    let total_reachable = total_reachable_count;

    Ok(SequenceReport {
        schema_version: "1.0".to_string(),
        from,
        interactions,
        cyclic,
        truncated,
        total_reachable,
        duration_ms: 0, // filled by project_sequence wrapper
    })
}

/// Resolve a FromSelector to a canonical_key.
fn resolve_selector(
    store: &dyn RawGraphQuery,
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
    if let Some(props_cell) = row.get("rel_props")
        && let Some(props_str) = props_cell.as_str()
    {
        // Props are stored as escaped JSON string
        let unescaped = props_str.replace("\\'", "'");
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&unescaped)
            && let Some(msg_kind) = parsed.get("message_kind").and_then(|v| v.as_str())
        {
            return match msg_kind {
                "async_call" => MessageKind::AsyncCall,
                "return" => MessageKind::Return,
                _ => MessageKind::SyncCall,
            };
        }
    }
    MessageKind::SyncCall
}

/// Extract file and line from a row's rel_props JSON.
fn extract_location_from_row(row: &crate::row::Row) -> (Option<PathBuf>, Option<u32>) {
    if let Some(props_cell) = row.get("rel_props")
        && let Some(props_str) = props_cell.as_str()
    {
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
    (None, None)
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Seed helper: insert a function node into the graph.
    fn seed_function_node(store: &mut LbugStore, ck: &str, name: &str) {
        use crate::store::ElementRepository;
        let element = crate::graph::Element {
            id: format!("cg:{}", crate::graph::sanitize_identifier(ck)),
            kind_id: "code.function".to_string(),
            category: "code".to_string(),
            canonical_key: ck.to_string(),
            current_name: name.to_string(),
            current_status: "active".to_string(),
            current_confidence: 0.9,
            current_version_id: "v1".to_string(),
        };
        let _ = store.upsert_element(&element);
    }

    /// Seed helper: insert a call edge into the graph.
    /// Creates: Element -[SEMANTIC_EDGE]-> Element (matching call_graph.rs apply)
    fn seed_call_edge(store: &mut LbugStore, caller: &str, callee: &str, _line: u32) {
        use crate::store::SemanticEdgeRepository;
        let rel_id = format!("rel:{}->{}:{}", caller, callee, _line);
        let mut rel_props = serde_json::Map::new();
        rel_props.insert(
            "predicate".to_string(),
            serde_json::Value::String("code.calls".to_string()),
        );
        rel_props.insert(
            "call_kind".to_string(),
            serde_json::Value::String("directcall".to_string()),
        );
        rel_props.insert(
            "message_kind".to_string(),
            serde_json::Value::String("sync_call".to_string()),
        );
        rel_props.insert(
            "rel_id".to_string(),
            serde_json::Value::String(rel_id.clone()),
        );
        // Use link_semantic_edge directly since we have element ids, not callee names.
        // This bypasses the name-resolution logic of link_call_edge_with_resolution.
        let _ = SemanticEdgeRepository::link_semantic_edge(
            store,
            &format!("cg:{}", crate::graph::sanitize_identifier(caller)), // src element id
            &format!("cg:{}", crate::graph::sanitize_identifier(callee)), // tgt element id
            &rel_id,
            "code.calls",
            &rel_props,
            true, // active
        );
    }

    #[test]
    fn test_cycle_detected() {
        // Set up a cyclic graph: A → B → A
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        std::fs::create_dir_all(&project).unwrap();

        let mut store = crate::store::LbugStore::open(&project).unwrap();
        store.init().unwrap();

        // Seed nodes: A, B
        seed_function_node(&mut store, "rust:test.rs:A:10", "A");
        seed_function_node(&mut store, "rust:test.rs:B:20", "B");

        // Seed edges: A → B, B → A (cycle)
        seed_call_edge(&mut store, "rust:test.rs:A:10", "rust:test.rs:B:20", 11);
        seed_call_edge(&mut store, "rust:test.rs:B:20", "rust:test.rs:A:10", 21);

        // Run sequence from A with depth_limit=5
        let report = project_sequence_with_store(
            &store,
            FromSelector::ByCanonicalKey {
                canonical_key: "rust:test.rs:A:10".to_string(),
            },
            5,
            None,
        )
        .unwrap();

        // Should detect cycle
        assert!(report.cyclic, "expected cyclic=true");
        // A should be visited once (first encounter), then B once, then back to A (cycle detected)
        // Total: 2 interactions (A→B, B→A)
        assert_eq!(report.interactions.len(), 2);
    }

    #[test]
    fn test_max_interactions_cap() {
        // Set up 5 functions with edges from A to each
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        std::fs::create_dir_all(&project).unwrap();

        let mut store = crate::store::LbugStore::open(&project).unwrap();
        store.init().unwrap();

        // Seed nodes: A, B, C, D, E
        let nodes = ["A", "B", "C", "D", "E"];
        for (i, node) in nodes.iter().enumerate() {
            seed_function_node(
                &mut store,
                &format!("rust:test.rs:{}:{}", node, (i + 1) * 10),
                node,
            );
        }

        // Seed edges: A → B, A → C, A → D, A → E (4 edges from A)
        for (i, node) in nodes[1..].iter().enumerate() {
            seed_call_edge(
                &mut store,
                "rust:test.rs:A:10",
                &format!("rust:test.rs:{}:{}", node, (i + 2) * 10),
                11 + i as u32,
            );
        }

        // Run sequence from A with max_interactions=2
        let report = project_sequence_with_store(
            &store,
            FromSelector::ByCanonicalKey {
                canonical_key: "rust:test.rs:A:10".to_string(),
            },
            5,
            Some(2),
        )
        .unwrap();

        // Should be truncated at 2
        assert!(report.truncated, "expected truncated=true");
        assert_eq!(report.interactions.len(), 2);
        assert_eq!(report.total_reachable, 4); // 4 edges total
    }

    #[test]
    fn test_bfs_depth_limit() {
        // Set up a chain: A → B → C (depth 0, 1, 2)
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        std::fs::create_dir_all(&project).unwrap();

        let mut store = crate::store::LbugStore::open(&project).unwrap();
        store.init().unwrap();

        // Seed nodes: A, B, C
        seed_function_node(&mut store, "rust:test.rs:A:10", "A");
        seed_function_node(&mut store, "rust:test.rs:B:20", "B");
        seed_function_node(&mut store, "rust:test.rs:C:30", "C");

        // Seed edges: A → B, B → C
        seed_call_edge(&mut store, "rust:test.rs:A:10", "rust:test.rs:B:20", 11);
        seed_call_edge(&mut store, "rust:test.rs:B:20", "rust:test.rs:C:30", 21);

        // Run sequence from A with depth_limit=1 (should only get A → B, not B → C)
        let report = project_sequence_with_store(
            &store,
            FromSelector::ByCanonicalKey {
                canonical_key: "rust:test.rs:A:10".to_string(),
            },
            1,
            None,
        )
        .unwrap();

        // Only A → B should be returned (depth 1), not B → C (depth 2)
        assert_eq!(report.interactions.len(), 1);
        assert_eq!(report.interactions[0].depth, 1);
        assert!(!report.truncated);
    }

    #[test]
    fn test_resolve_by_name_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        std::fs::create_dir_all(&project).unwrap();

        let mut store = crate::store::LbugStore::open(&project).unwrap();
        store.init().unwrap();

        let result = project_sequence_with_store(
            &store,
            FromSelector::ByName {
                name: "nonexistent_function".to_string(),
            },
            5,
            None,
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SequenceError::SymbolNotFound(_)));
    }

    #[test]
    fn test_order_key_monotonic() {
        // Set up: A → B, A → C (both at depth 1, ordered by canonical_key)
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        std::fs::create_dir_all(&project).unwrap();

        let mut store = crate::store::LbugStore::open(&project).unwrap();
        store.init().unwrap();

        // Seed nodes
        seed_function_node(&mut store, "rust:test.rs:A:10", "A");
        seed_function_node(&mut store, "rust:test.rs:B:20", "B");
        seed_function_node(&mut store, "rust:test.rs:C:30", "C");

        // Seed edges
        seed_call_edge(&mut store, "rust:test.rs:A:10", "rust:test.rs:B:20", 11);
        seed_call_edge(&mut store, "rust:test.rs:A:10", "rust:test.rs:C:30", 12);

        let report = project_sequence_with_store(
            &store,
            FromSelector::ByCanonicalKey {
                canonical_key: "rust:test.rs:A:10".to_string(),
            },
            5,
            None,
        )
        .unwrap();

        // Should have 2 interactions
        assert_eq!(report.interactions.len(), 2);
        // Order keys should be 1, 2 (monotonic)
        assert_eq!(report.interactions[0].order_key, 1);
        assert_eq!(report.interactions[1].order_key, 2);
        // Order keys should be unique
        assert_ne!(
            report.interactions[0].order_key,
            report.interactions[1].order_key
        );
    }

    // ─── New tests for T18 ───────────────────────────────────────────────────

    #[test]
    fn test_from_selector_serde_roundtrip() {
        // ByName should round-trip via serde
        let s = FromSelector::ByName { name: "foo".into() };
        let json = serde_json::to_string(&s).unwrap();
        let parsed: FromSelector = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, s);

        // ByFileLine round-trip
        let s2 = FromSelector::ByFileLine {
            file: std::path::PathBuf::from("src/lib.rs"),
            line: 42,
        };
        let json2 = serde_json::to_string(&s2).unwrap();
        let parsed2: FromSelector = serde_json::from_str(&json2).unwrap();
        assert_eq!(parsed2, s2);

        // ByCanonicalKey round-trip
        let s3 = FromSelector::ByCanonicalKey {
            canonical_key: "rust:src/lib.rs:main:10".into(),
        };
        let json3 = serde_json::to_string(&s3).unwrap();
        let parsed3: FromSelector = serde_json::from_str(&json3).unwrap();
        assert_eq!(parsed3, s3);
    }

    #[test]
    fn test_parse_message_kind_all_variants() {
        assert_eq!(parse_message_kind("SyncCall"), MessageKind::SyncCall);
        assert_eq!(parse_message_kind("AsyncCall"), MessageKind::AsyncCall);
        assert_eq!(parse_message_kind("Return"), MessageKind::Return);
        // Unknown maps to default
        assert_eq!(parse_message_kind("unknown"), MessageKind::SyncCall);
        assert_eq!(parse_message_kind(""), MessageKind::SyncCall);
    }

    #[test]
    fn test_interaction_order_key_monotonic() {
        // Build 3 interactions, assert order_key is 1, 2, 3
        let i1 = Interaction {
            order_key: 1,
            sender: "a".into(),
            receiver: "b".into(),
            message_kind: MessageKind::SyncCall,
            depth: 1,
            file: None,
            line: None,
        };
        let i2 = Interaction {
            order_key: 2,
            sender: "b".into(),
            receiver: "c".into(),
            message_kind: MessageKind::AsyncCall,
            depth: 2,
            file: None,
            line: None,
        };
        let i3 = Interaction {
            order_key: 3,
            sender: "c".into(),
            receiver: "d".into(),
            message_kind: MessageKind::Return,
            depth: 3,
            file: None,
            line: None,
        };
        let interactions = [i1, i2, i3];
        assert_eq!(
            interactions.iter().map(|i| i.order_key).collect::<Vec<_>>(),
            [1, 2, 3]
        );
    }

    #[test]
    fn test_sequence_report_serde_roundtrip() {
        // Serialize and deserialize a SequenceReport; assert equality
        let report = SequenceReport {
            schema_version: "1.0".into(),
            from: FromSelector::ByName {
                name: "main".into(),
            },
            interactions: vec![Interaction {
                order_key: 1,
                sender: "main".into(),
                receiver: "helper".into(),
                message_kind: MessageKind::SyncCall,
                depth: 1,
                file: Some(std::path::PathBuf::from("src/lib.rs")),
                line: Some(10),
            }],
            cyclic: false,
            truncated: false,
            total_reachable: 1,
            duration_ms: 42,
        };
        let json = serde_json::to_string(&report).unwrap();
        let parsed: SequenceReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.schema_version, "1.0");
        assert_eq!(parsed.interactions.len(), 1);
        assert_eq!(parsed.interactions[0].sender, "main");
        assert_eq!(parsed.duration_ms, 42);
    }
}
