//! Task Context Compiler — deterministic budgeted context bundle for AI agents.
//!
//! Delegates ranking to P2-07 relevance, enriches with evidence, packs under budget.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::architecture::relevance::{self, RelevanceOptions, RelevanceError, SelectionTrace};
use crate::diagram::export_types::EvidenceEntry;
use crate::store::DiagramRepository;

// ─────────────────────────────────────────────────────────────────────────────
// Carriers
// ─────────────────────────────────────────────────────────────────────────────

/// The task-context-report/1 carrier — output of the `compile_task_context` use case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskContextReport {
    /// Schema version of this report format.
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,

    /// Capability that produced this report.
    pub capability: String,

    /// The original task string.
    pub task: String,

    /// Packed elements, sorted by (score DESC, id ASC), filtered by budget.
    pub elements: Vec<ContextElement>,

    /// Relations whose both endpoints are in `elements`, sorted by (sourceId ASC, targetId ASC, predicateId ASC).
    pub relations: Vec<ContextRelation>,

    /// Budget consumption information.
    pub budget: BudgetInfo,

    /// Trace of the selection process (reused from relevance).
    #[serde(rename = "selectionTrace")]
    pub selection_trace: SelectionTrace,
}

/// A context element: scored element with evidence attached.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextElement {
    /// Element id.
    pub id: String,

    /// Element kind id (e.g. "container", "component").
    #[serde(rename = "kindId")]
    pub kind_id: String,

    /// Element category (e.g. "c4", "uml", "behavior").
    pub category: String,

    /// Element name.
    pub name: String,

    /// Relevance score (0.0–1.0).
    pub score: f64,

    /// Evidence entries linked to this element's current version.
    pub evidence: Vec<EvidenceEntry>,
}

/// A context relation: scored relation whose both endpoints are retained.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextRelation {
    /// Relation id.
    #[serde(rename = "relationId")]
    pub relation_id: String,

    /// Predicate id (e.g. "depends_on", "calls").
    #[serde(rename = "predicateId")]
    pub predicate_id: String,

    /// Source element id.
    #[serde(rename = "sourceId")]
    pub source_id: String,

    /// Target element id.
    #[serde(rename = "targetId")]
    pub target_id: String,

    /// Relevance score (0.0–1.0).
    pub score: f64,
}

/// Budget consumption information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetInfo {
    /// The token budget requested by the caller.
    #[serde(rename = "requestedTokens")]
    pub requested_tokens: usize,

    /// Estimated tokens consumed by the packed elements and relations.
    #[serde(rename = "estimatedTokens")]
    pub estimated_tokens: usize,

    /// True if some elements/relations were dropped due to budget.
    pub truncated: bool,
}

/// Errors specific to task context operations.
#[derive(Debug, Clone)]
pub enum ContextError {
    /// Task string was empty or whitespace-only.
    EmptyTask,
    /// Budget tokens was zero.
    InvalidBudget,
    /// The store returned an error.
    Store(String),
}

impl std::fmt::Display for ContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextError::EmptyTask => write!(f, "context error: empty task"),
            ContextError::InvalidBudget => write!(f, "context error: budget must be > 0"),
            ContextError::Store(msg) => write!(f, "context error: {msg}"),
        }
    }
}

impl std::error::Error for ContextError {}

impl From<anyhow::Error> for ContextError {
    fn from(e: anyhow::Error) -> Self {
        ContextError::Store(e.to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Core compile function
// ─────────────────────────────────────────────────────────────────────────────

/// Compile a deterministic budgeted context bundle for a natural-language task.
///
/// Wraps `relevance::relevance` (P2-07), resolves evidence for retained
/// elements, and packs complete subject units under an explicit token ceiling.
///
/// # Errors
///
/// Returns `ContextError::EmptyTask` if `task` is empty or whitespace-only.
/// Returns `ContextError::InvalidBudget` if `budget_tokens` is zero.
/// Returns `ContextError::Store` if the underlying store fails.
pub fn compile_task_context(
    repo: &dyn DiagramRepository,
    task: &str,
    budget_tokens: usize,
    top: usize,
) -> Result<TaskContextReport, ContextError> {
    // S4: empty or whitespace-only task → EmptyTask
    let task = task.trim();
    if task.is_empty() {
        return Err(ContextError::EmptyTask);
    }

    // S5: budget_tokens == 0 → InvalidBudget
    if budget_tokens == 0 {
        return Err(ContextError::InvalidBudget);
    }

    // Call P2-07 relevance
    let relevance_opts = RelevanceOptions { top, max_hops: 1 };
    let relevance_report = match relevance::relevance(repo, task, &relevance_opts) {
        Ok(r) => r,
        Err(RelevanceError::EmptyQuery) => {
            return Err(ContextError::EmptyTask);
        }
        Err(RelevanceError::Store(msg)) => {
            return Err(ContextError::Store(msg));
        }
    };

    // Build a lookup: element_id → current_version_id for evidence resolution
    let mut version_id_map: BTreeMap<String, String> = BTreeMap::new();
    for category in &["c4", "uml", "behavior"] {
        let elements = repo
            .list_elements(category, None, None)
            .map_err(|e: anyhow::Error| ContextError::Store(e.to_string()))?;
        for elem in elements {
            version_id_map.insert(elem.id.clone(), elem.current_version_id.clone());
        }
    }

    // Since EvidenceEntry doesn't carry version_id, we do per-element evidence lookup
    // to ensure correct evidence-to-element mapping.
    let mut context_elements: Vec<ContextElement> = Vec::new();

    for elem in &relevance_report.elements {
        let version_id = version_id_map.get(&elem.id).cloned().unwrap_or_default();
        let evidence: Vec<EvidenceEntry> = if version_id.is_empty() {
            Vec::new()
        } else {
            repo.list_evidence_for_versions(std::slice::from_ref(&version_id))
                .map_err(|e: anyhow::Error| ContextError::Store(e.to_string()))?
        };

        context_elements.push(ContextElement {
            id: elem.id.clone(),
            kind_id: elem.kind_id.clone(),
            category: elem.category.clone(),
            name: elem.name.clone(),
            score: elem.score,
            evidence,
        });
    }

    // Sort elements by (score DESC, id ASC) — already sorted from relevance, but ensure
    context_elements.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });

    // Build element id set for relation closure
    let element_ids: std::collections::HashSet<String> =
        context_elements.iter().map(|e| e.id.clone()).collect();

    // Relation closure: only include relations where BOTH endpoints are in element_ids
    let mut context_relations: Vec<ContextRelation> = relevance_report
        .relations
        .iter()
        .filter(|r| element_ids.contains(&r.source_id) && element_ids.contains(&r.target_id))
        .map(|r| ContextRelation {
            relation_id: r.relation_id.clone(),
            predicate_id: r.predicate_id.clone(),
            source_id: r.source_id.clone(),
            target_id: r.target_id.clone(),
            score: r.score,
        })
        .collect();

    // Sort relations by (sourceId ASC, targetId ASC, predicateId ASC)
    context_relations.sort_by(|a, b| {
        a.source_id
            .cmp(&b.source_id)
            .then_with(|| a.target_id.cmp(&b.target_id))
            .then_with(|| a.predicate_id.cmp(&b.predicate_id))
    });

    // Pack elements under budget: iterate in rank order, serialize, stop when budget exceeded
    // Token estimate: serialized_json_len / 4 (rounded up via ceiling division)
    let mut accumulated_len: usize = 0;
    let mut truncated = false;
    let total_elements_count = context_elements.len();
    let total_relations_count = context_relations.len();

    // Pre-serialize the report skeleton to get consistent token estimates
    // We iteratively add elements and check the serialized size
    let mut working_report = TaskContextReport {
        schema_version: "1.0".to_string(),
        capability: "architecture-task-context-mvp".to_string(),
        task: task.to_string(),
        elements: Vec::new(),
        relations: Vec::new(),
        budget: BudgetInfo {
            requested_tokens: budget_tokens,
            estimated_tokens: 0,
            truncated: false,
        },
        selection_trace: relevance_report.selection_trace.clone(),
    };

    // First pass: pack as many elements as possible
    for elem in &context_elements {
        // Serialize with current element added
        let test_elements = {
            let mut elems = working_report.elements.clone();
            elems.push(elem.clone());
            elems
        };

        let test_report = TaskContextReport {
            elements: test_elements,
            relations: working_report.relations.clone(),
            ..working_report.clone()
        };

        let serialized_len = serde_json::to_string(&test_report)
            .map(|s| s.len())
            .unwrap_or(usize::MAX);
        let estimated_tokens = serialized_len.div_ceil(4);

        if estimated_tokens <= budget_tokens || working_report.elements.is_empty() {
            // Add this element
            working_report.elements.push(elem.clone());
            accumulated_len = serialized_len;
        } else {
            // Budget exceeded, stop packing
            truncated = !working_report.elements.is_empty();
            break;
        }
    }

    // If we processed all elements without exceeding budget
    if !truncated && !working_report.elements.is_empty() {
        let serialized_len = serde_json::to_string(&working_report)
            .map(|s| s.len())
            .unwrap_or(accumulated_len);
        accumulated_len = serialized_len;
    }

    // Second pass: pack relations whose endpoints are in the retained elements
    let packed_element_ids: std::collections::HashSet<String> = working_report
        .elements
        .iter()
        .map(|e| e.id.clone())
        .collect();

    for rel in &context_relations {
        if packed_element_ids.contains(&rel.source_id)
            && packed_element_ids.contains(&rel.target_id)
        {
            let test_relations = {
                let mut rels = working_report.relations.clone();
                rels.push(rel.clone());
                rels
            };

            let test_report = TaskContextReport {
                elements: working_report.elements.clone(),
                relations: test_relations,
                ..working_report.clone()
            };

            let serialized_len = serde_json::to_string(&test_report)
                .map(|s| s.len())
                .unwrap_or(usize::MAX);
            let estimated_tokens = serialized_len.div_ceil(4);

            if estimated_tokens <= budget_tokens {
                working_report.relations.push(rel.clone());
                accumulated_len = serialized_len;
            } else {
                truncated = true;
                break;
            }
        }
    }

    // Calculate final estimated tokens
    let final_serialized_len = serde_json::to_string(&working_report)
        .map(|s| s.len())
        .unwrap_or(accumulated_len);
    let estimated_tokens = final_serialized_len.div_ceil(4);

    // Determine if we actually truncated (had more elements/relations that didn't fit)
    let had_more_elements = total_elements_count > working_report.elements.len();
    let had_more_relations = total_relations_count > working_report.relations.len();
    truncated = truncated || had_more_elements || had_more_relations;

    working_report.budget = BudgetInfo {
        requested_tokens: budget_tokens,
        estimated_tokens,
        truncated,
    };

    Ok(working_report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::export_types::EvidenceEntry;
    use crate::graph::{ElementRow, SemanticEdgeRow};

    /// A minimal DiagramRepository stub for unit tests.
    struct FakeRepo {
        elements: Vec<ElementRow>,
        edges: Vec<SemanticEdgeRow>,
        element_evidence: Vec<(String, Vec<EvidenceEntry>)>,
    }

    impl FakeRepo {
        fn new() -> Self {
            Self {
                elements: vec![],
                edges: vec![],
                element_evidence: vec![],
            }
        }
        fn with_element(mut self, id: &str, name: &str, confidence: f64, category: &str) -> Self {
            self.elements.push(ElementRow {
                id: id.to_string(),
                kind_id: "container".to_string(),
                category: category.to_string(),
                canonical_key: id.to_string(),
                current_name: name.to_string(),
                current_status: "active".to_string(),
                current_confidence: confidence,
                current_version_id: format!("{}-v1", id),
            });
            self
        }
        fn with_edge(
            mut self,
            relation_id: &str,
            predicate_id: &str,
            source_id: &str,
            target_id: &str,
        ) -> Self {
            self.edges.push(SemanticEdgeRow {
                relation_id: relation_id.to_string(),
                predicate_id: predicate_id.to_string(),
                source_id: source_id.to_string(),
                target_id: target_id.to_string(),
                order_key: "0".to_string(),
                props: serde_json::Map::new(),
            });
            self
        }
        fn with_evidence(mut self, version_id: &str, evidence: EvidenceEntry) -> Self {
            self.element_evidence
                .push((version_id.to_string(), vec![evidence]));
            self
        }
    }

    impl DiagramRepository for FakeRepo {
        fn list_elements(
            &self,
            category: &str,
            _scope: Option<&str>,
            _kind: Option<&str>,
        ) -> anyhow::Result<Vec<ElementRow>> {
            Ok(self
                .elements
                .iter()
                .filter(|e| e.category == category)
                .cloned()
                .collect())
        }
        fn list_semantic_edges(&self, _category: &str) -> anyhow::Result<Vec<SemanticEdgeRow>> {
            Ok(self.edges.clone())
        }
        fn list_evidence_for_versions(
            &self,
            version_ids: &[String],
        ) -> anyhow::Result<Vec<EvidenceEntry>> {
            Ok(version_ids
                .iter()
                .flat_map(|vid| {
                    self.element_evidence
                        .iter()
                        .filter(|(id, _)| id == vid)
                        .flat_map(|(_, ev)| ev.clone())
                        .collect::<Vec<_>>()
                })
                .collect())
        }
        fn list_version_props(
            &self,
            _version_ids: &[String],
        ) -> anyhow::Result<Vec<crate::graph::VersionPropsRow>> {
            Ok(vec![])
        }
        fn read_relation_by_id(
            &self,
            _id: &str,
        ) -> anyhow::Result<Option<crate::graph::RelationRow>> {
            Ok(None)
        }
        fn list_evidence_for_relation_versions(
            &self,
            _version_ids: &[String],
        ) -> anyhow::Result<Vec<EvidenceEntry>> {
            Ok(vec![])
        }
    }

    fn make_evidence(id: &str, status: &str) -> EvidenceEntry {
        EvidenceEntry {
            id: id.to_string(),
            kind: "structural".to_string(),
            claim: "test claim".to_string(),
            path: "src/lib.rs".to_string(),
            start_line: 10,
            end_line: 15,
            tool_name: "ast-grep".to_string(),
            tool_version: "0.1".to_string(),
            rule_id: "test:rule".to_string(),
            content_hash: "sha256:abc".to_string(),
            observed_at: "2026-08-01T00:00:00Z".to_string(),
            status: Some(status.to_string()),
        }
    }

    // -------------------------------------------------------------------------
    // S1: Happy path with evidence
    // -------------------------------------------------------------------------

    #[test]
    fn task_context_happy_path_with_evidence() {
        let repo = FakeRepo::new()
            .with_element("c4:container:orders", "OrderService", 0.9, "c4")
            .with_evidence("c4:container:orders-v1", make_evidence("ev:1", "accepted"));

        let result = compile_task_context(&repo, "OrderService", 4000, 10).unwrap();

        assert_eq!(result.schema_version, "1.0");
        assert_eq!(result.capability, "architecture-task-context-mvp");
        assert_eq!(result.task, "OrderService");
        assert!(!result.elements.is_empty());
        assert_eq!(result.elements[0].id, "c4:container:orders");
        assert!(!result.elements[0].evidence.is_empty());
        assert!(!result.budget.truncated);
        assert!(result.budget.estimated_tokens <= result.budget.requested_tokens);
    }

    // -------------------------------------------------------------------------
    // S2: Budget truncation drops lowest-score elements
    // -------------------------------------------------------------------------

    #[test]
    fn task_context_budget_truncation() {
        let repo = FakeRepo::new()
            .with_element("c4:container:a", "A", 0.9, "c4")
            .with_element("c4:container:b", "B", 0.8, "c4")
            .with_element("c4:container:c", "C", 0.7, "c4");

        // Tiny budget that can only fit one element
        let result = compile_task_context(&repo, "c", 50, 10).unwrap();

        // Should have packed at least one element (even if it alone exceeds budget)
        assert!(!result.elements.is_empty());
        // The highest-scored element should be included
        // Query "c" matches "C" via substring, giving score ~0.56
        // It expands to A and B with lower scores via BFS
        // So C should be first
        assert_eq!(result.elements[0].id, "c4:container:c");
        // Budget should be marked as truncated if we couldn't fit all
        assert!(result.budget.truncated || result.elements.len() < 3);
    }

    // -------------------------------------------------------------------------
    // S3: Relation closure invariant — dangling relations are dropped
    // -------------------------------------------------------------------------

    #[test]
    fn task_context_relation_closure_drops_dangling() {
        let repo = FakeRepo::new()
            .with_element("c4:container:a", "A", 0.9, "c4")
            .with_element("c4:container:b", "B", 0.8, "c4")
            .with_element("c4:container:c", "C", 0.7, "c4")
            .with_edge("rel-a-b", "depends_on", "c4:container:a", "c4:container:b")
            .with_edge("rel-b-c", "calls", "c4:container:b", "c4:container:c");

        // Only fit A — B and C are dropped, so rel-a-b and rel-b-c should both be dropped
        let result = compile_task_context(&repo, "a", 100, 10).unwrap();

        // If A is packed but B is not, no relations should reference B
        for rel in &result.relations {
            assert!(
                result.elements.iter().any(|e| e.id == rel.source_id),
                "relation {} has dangling source",
                rel.relation_id
            );
            assert!(
                result.elements.iter().any(|e| e.id == rel.target_id),
                "relation {} has dangling target",
                rel.relation_id
            );
        }
    }

    // -------------------------------------------------------------------------
    // S4: Empty or whitespace-only task → EmptyTask
    // -------------------------------------------------------------------------

    #[test]
    fn task_context_empty_task_error() {
        let repo = FakeRepo::new();

        let result = compile_task_context(&repo, "", 4000, 10);
        assert!(matches!(result, Err(ContextError::EmptyTask)));

        let result = compile_task_context(&repo, "   ", 4000, 10);
        assert!(matches!(result, Err(ContextError::EmptyTask)));
    }

    // -------------------------------------------------------------------------
    // S5: Invalid budget (zero) → InvalidBudget
    // -------------------------------------------------------------------------

    #[test]
    fn task_context_zero_budget_error() {
        let repo = FakeRepo::new();

        let result = compile_task_context(&repo, "test", 0, 10);
        assert!(matches!(result, Err(ContextError::InvalidBudget)));
    }

    // -------------------------------------------------------------------------
    // S6: Empty graph → empty report (exit 0)
    // -------------------------------------------------------------------------

    #[test]
    fn task_context_empty_graph() {
        let repo = FakeRepo::new();

        let result = compile_task_context(&repo, "anything", 4000, 10).unwrap();

        assert!(result.elements.is_empty());
        assert!(result.relations.is_empty());
        assert!(!result.budget.truncated);
    }

    // -------------------------------------------------------------------------
    // S7: Determinism — two calls produce byte-equal JSON
    // -------------------------------------------------------------------------

    #[test]
    fn task_context_determinism() {
        let repo = FakeRepo::new()
            .with_element("c4:container:a", "A", 0.9, "c4")
            .with_element("c4:container:b", "B", 0.8, "c4");

        let json1 =
            serde_json::to_string(&compile_task_context(&repo, "a", 4000, 10).unwrap()).unwrap();
        let json2 =
            serde_json::to_string(&compile_task_context(&repo, "a", 4000, 10).unwrap()).unwrap();

        assert_eq!(json1, json2);
    }

    // -------------------------------------------------------------------------
    // S8: Schema version and capability fields
    // -------------------------------------------------------------------------

    #[test]
    fn task_context_schema_version_and_capability() {
        let repo = FakeRepo::new().with_element("c4:container:srv", "OrderService", 0.9, "c4");

        let result = compile_task_context(&repo, "Order", 4000, 10).unwrap();

        assert_eq!(result.schema_version, "1.0");
        assert_eq!(result.capability, "architecture-task-context-mvp");
    }

    // -------------------------------------------------------------------------
    // S9: Evidence batch resolved per retained subject
    // -------------------------------------------------------------------------

    #[test]
    fn task_context_evidence_per_retained_element() {
        let repo = FakeRepo::new()
            .with_element("c4:container:a", "A", 0.9, "c4")
            .with_element("c4:container:b", "B", 0.8, "c4")
            .with_evidence("c4:container:a-v1", make_evidence("ev:a1", "accepted"))
            .with_evidence("c4:container:b-v1", make_evidence("ev:b1", "accepted"));

        // Tiny budget that only fits A
        let result = compile_task_context(&repo, "a", 100, 10).unwrap();

        // A should have evidence
        let a_elem = result.elements.iter().find(|e| e.id == "c4:container:a");
        assert!(a_elem.is_some());
        assert!(!a_elem.unwrap().evidence.is_empty());
    }

    // -------------------------------------------------------------------------
    // Budget estimate sanity: estimatedTokens <= requestedTokens
    // -------------------------------------------------------------------------

    #[test]
    fn task_context_budget_estimate_sanity() {
        let repo = FakeRepo::new()
            .with_element("c4:container:a", "A", 0.9, "c4")
            .with_element("c4:container:b", "B", 0.8, "c4");

        let result = compile_task_context(&repo, "a", 4000, 10).unwrap();

        assert!(result.budget.estimated_tokens <= result.budget.requested_tokens);
    }
}
