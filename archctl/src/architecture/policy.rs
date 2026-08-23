//! Architecture policy evaluator.
//!
//! Evaluates a set of `PolicyRule`s against the live architecture graph
//! via the `DiagramRepository` port. Produces a `PolicyReport` with
//! violations, waivers, and a summary. Waivers suppress matching
//! violations until expiry.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::graph::ElementRow;
use crate::store::DiagramRepository;

/// Severity level for a policy violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// A subject identified by id and kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySubject {
    /// Canonical element or relation id (e.g. "c4:container:a").
    pub id: String,
    /// Kind discriminator: "element" or "relation".
    #[serde(rename = "kind")]
    pub kind: String,
}

/// Parameters specific to each rule type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PolicyParams {
    /// Parameters for `forbid_dependency` and `require_dependency`.
    Dependency {
        /// The target element id pattern.
        target: String,
    },
    /// Parameters for `forbid_cycle`.
    Cycle {},
    /// Parameters for `max_fanout`.
    MaxFanout {
        /// Maximum allowed out-degree.
        max: usize,
    },
    /// Parameters for `evidence_required`.
    EvidenceRequired {},
    /// Parameters for `confidence_min`.
    ConfidenceMin {
        /// Minimum confidence value (0.0–1.0).
        min: f64,
    },
}

/// The six architecture policy rules.
///
/// Serialised with `#[serde(tag = "rule")]` so the JSON representation is:
///
/// ```json
/// { "rule": "forbid_dependency", "selector": "c4:container:a", "severity": "error", "params": { "target": "c4:container:b" } }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "rule")]
pub enum PolicyRule {
    /// Forbid a dependency from source to target.
    #[serde(rename = "forbid_dependency")]
    ForbidDependency {
        selector: String,
        severity: Severity,
        #[serde(flatten)]
        params: PolicyParams,
    },
    /// Require that source depends on target.
    #[serde(rename = "require_dependency")]
    RequireDependency {
        selector: String,
        severity: Severity,
        #[serde(flatten)]
        params: PolicyParams,
    },
    /// Forbid any cycle within the selected scope.
    #[serde(rename = "forbid_cycle")]
    ForbidCycle {
        selector: String,
        severity: Severity,
    },
    /// Enforce a maximum fan-out (out-degree) per element.
    #[serde(rename = "max_fanout")]
    MaxFanout {
        selector: String,
        severity: Severity,
        #[serde(flatten)]
        params: PolicyParams,
    },
    /// Require that every matched element has at least one evidence reference.
    #[serde(rename = "evidence_required")]
    EvidenceRequired {
        selector: String,
        severity: Severity,
    },
    /// Require that every matched element/relation has confidence >= min.
    #[serde(rename = "confidence_min")]
    ConfidenceMin {
        selector: String,
        severity: Severity,
        #[serde(flatten)]
        params: PolicyParams,
    },
}

/// A waiver suppresses a specific violation until it expires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Waiver {
    /// The rule id this waiver applies to.
    pub rule: String,
    /// The subject id this waiver applies to.
    pub subject_id: String,
    /// Human-readable justification.
    pub reason: String,
    /// UTC timestamp after which this waiver is ineffective.
    pub expires_at: DateTime<Utc>,
    /// Set to `true` when `expires_at` is in the past (computed at evaluation time).
    #[serde(default)]
    pub expired: bool,
}

/// A single policy violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// Rule that triggered this violation.
    pub rule: String,
    /// Severity level.
    pub severity: Severity,
    /// The subject that violated the policy.
    pub subject: PolicySubject,
    /// Rule-specific parameters at the time of violation.
    pub params: PolicyParams,
    /// Human-readable explanation.
    pub message: String,
}

/// Summary statistics for a policy evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySummary {
    /// Total number of rules evaluated.
    pub total: usize,
    /// Number of rules that passed (no violations).
    pub passed: usize,
    /// Number of rules that produced at least one violation.
    pub failed: usize,
    /// Number of violations suppressed by active waivers.
    pub waived: usize,
    /// The `--fail-on` threshold used for this evaluation.
    pub fail_on: String,
}

/// The full architecture policy evaluation report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyReport {
    /// Schema version for this report format.
    pub schema_version: String,
    /// Capability identifier.
    pub capability: String,
    /// Policy identifier (usually the policy file name).
    pub policy_id: String,
    /// UTC timestamp of evaluation.
    pub evaluated_at: DateTime<Utc>,
    /// All violations found (after waivers applied).
    #[serde(default)]
    pub violations: Vec<Violation>,
    /// All waivers provided in the policy input.
    #[serde(default)]
    pub waivers: Vec<Waiver>,
    /// Summary statistics.
    pub summary: PolicySummary,
    /// Non-fatal warnings (e.g. malformed selectors).
    #[serde(default)]
    pub warnings: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// PolicyError
// ─────────────────────────────────────────────────────────────────────────────

/// Errors that can occur during policy evaluation.
#[derive(Debug, Clone)]
pub enum PolicyError {
    /// Unknown or malformed rule type.
    UnknownRule(String),
    /// The policy file could not be parsed.
    MalformedPolicy(String),
    /// Failed to read from the graph store.
    RepoRead(String),
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyError::UnknownRule(id) => write!(f, "unknown policy rule type: {id}"),
            PolicyError::MalformedPolicy(msg) => write!(f, "malformed policy: {msg}"),
            PolicyError::RepoRead(msg) => write!(f, "graph store read error: {msg}"),
        }
    }
}

impl std::error::Error for PolicyError {}

// ─────────────────────────────────────────────────────────────────────────────
// Selector matching
// ─────────────────────────────────────────────────────────────────────────────

/// Returns `true` if `id` matches the given selector pattern.
///
/// Glob rules:
/// - If `pattern` ends with `*`, match the prefix (up to a `:` or `.` separator)
///   followed by anything.
/// - Otherwise, exact string equality.
pub fn matches_selector(selector: &str, id: &str) -> bool {
    if let Some(stripped) = selector.strip_suffix('*') {
        if stripped.is_empty() {
            return true;
        }
        if !id.starts_with(stripped) {
            return false;
        }
        let remainder = &id[stripped.len()..];
        // Prefix already ends with a separator: the remainder can be anything
        // (`c4:container:*` matches `c4:container:a`).
        if stripped.ends_with(':') || stripped.ends_with('.') {
            return true;
        }
        // Prefix has no trailing separator: require one so that
        // `c4:container*` does NOT match `c4:containers:a`.
        remainder.starts_with(':') || remainder.starts_with('.')
    } else {
        selector == id
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Evaluation helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Build a violation for a dependency rule.
fn dependency_violation(
    rule: &str,
    severity: Severity,
    source_id: &str,
    target_id: &str,
    message: String,
) -> Violation {
    Violation {
        rule: rule.to_string(),
        severity,
        subject: PolicySubject {
            id: source_id.to_string(),
            kind: "element".to_string(),
        },
        params: PolicyParams::Dependency {
            target: target_id.to_string(),
        },
        message,
    }
}

/// Evaluate `forbid_dependency` rules against the provided edges.
fn eval_forbid_dependency(rules: &[&PolicyRule], edges: &[(String, String)]) -> Vec<Violation> {
    let mut violations = Vec::new();
    for rule in rules {
        let PolicyRule::ForbidDependency {
            selector,
            severity,
            params,
        } = rule
        else {
            continue;
        };
        let PolicyParams::Dependency { target } = params else {
            continue;
        };
        for (source, target_id) in edges {
            if matches_selector(selector, source) && *target_id == *target {
                violations.push(dependency_violation(
                    "forbid_dependency",
                    *severity,
                    source,
                    target_id,
                    format!("{} is forbidden to depend on {}", source, target_id),
                ));
            }
        }
    }
    violations
}

/// Evaluate `require_dependency` rules against the provided edges.
fn eval_require_dependency(
    rules: &[&PolicyRule],
    elements: &[&ElementRow],
    edges: &[(String, String)],
) -> Vec<Violation> {
    let mut violations = Vec::new();
    for rule in rules {
        let PolicyRule::RequireDependency {
            selector,
            severity,
            params,
        } = rule
        else {
            continue;
        };
        let PolicyParams::Dependency { target } = params else {
            continue;
        };

        // Build a set of existing targets per source
        let mut outgoing: HashMap<&str, HashSet<&str>> = HashMap::new();
        for (source, tgt) in edges {
            outgoing
                .entry(source.as_str())
                .or_default()
                .insert(tgt.as_str());
        }

        for elem in elements {
            if !matches_selector(selector, &elem.id) {
                continue;
            }
            let has_dep = outgoing
                .get(elem.id.as_str())
                .map(|tgt| tgt.contains(target.as_str()))
                .unwrap_or(false);
            if !has_dep {
                violations.push(dependency_violation(
                    "require_dependency",
                    *severity,
                    &elem.id,
                    target,
                    format!("{} must depend on {} but does not", elem.id, target),
                ));
            }
        }
    }
    violations
}

/// Tarjan DFS for cycle detection on a filtered subgraph (iterative).
fn eval_forbid_cycle(
    rules: &[&PolicyRule],
    elements: &[&ElementRow],
    edges: &[(String, String)],
) -> Vec<Violation> {
    let mut violations = Vec::new();
    for rule in rules {
        let PolicyRule::ForbidCycle { selector, severity } = rule else {
            continue;
        };

        // Build element set matching the selector
        let elem_set: HashSet<&str> = elements
            .iter()
            .filter(|e| matches_selector(selector, &e.id))
            .map(|e| e.id.as_str())
            .collect();

        if elem_set.is_empty() {
            continue;
        }

        // Build adjacency for the filtered subgraph
        let adj: HashMap<&str, Vec<&str>> = edges
            .iter()
            .filter(|(s, t)| elem_set.contains(s.as_str()) && elem_set.contains(t.as_str()))
            .fold(HashMap::new(), |mut acc, (s, t)| {
                acc.entry(s.as_str()).or_default().push(t.as_str());
                acc
            });

        // Iterative Tarjan SCC using an explicit stack of (node, iterator_state)
        let mut index: i32 = 0;
        let mut indices: HashMap<&str, i32> = HashMap::new();
        let mut lowlinks: HashMap<&str, i32> = HashMap::new();
        let mut on_stack: HashSet<&str> = HashSet::new();
        let mut stack: Vec<&str> = Vec::new();
        let mut sccs: Vec<Vec<&str>> = Vec::new();

        // State machine for iterative DFS
        #[derive(Clone)]
        enum DfsState<'a> {
            Start(&'a str),
            AfterNeighbor(Vec<&'a str>),
        }
        let mut dfs_stack: Vec<DfsState<'_>> = Vec::new();

        for &node in &elem_set {
            if indices.contains_key(node) {
                continue;
            }

            dfs_stack.push(DfsState::Start(node));
            while let Some(state) = dfs_stack.pop() {
                match state {
                    DfsState::Start(n) => {
                        if indices.contains_key(n) {
                            continue;
                        }
                        indices.insert(n, index);
                        lowlinks.insert(n, index);
                        index += 1;
                        stack.push(n);
                        on_stack.insert(n);

                        // Push the "pop SCC" marker first (to run after children)
                        dfs_stack.push(DfsState::AfterNeighbor(
                            adj.get(n).cloned().unwrap_or_default(),
                        ));
                        // Push each neighbor as a new start
                        if let Some(neighbors) = adj.get(n) {
                            for &neighbor in neighbors.iter().rev() {
                                if !indices.contains_key(neighbor) {
                                    dfs_stack.push(DfsState::Start(neighbor));
                                }
                            }
                        }
                    }
                    DfsState::AfterNeighbor(neighbors) => {
                        // The node for this state is the top of the call stack minus the children we pushed
                        // We need to find the node we're returning to.
                        // Actually, we track the current node via the stack.
                        // Find the node at the current depth - it's the most recent unprocessed node on stack.
                        let Some(&current_node) = stack.last() else {
                            continue;
                        };

                        // Update lowlink from each processed neighbor
                        for neighbor in neighbors {
                            if !indices.contains_key(neighbor) {
                                continue;
                            }
                            if on_stack.contains(neighbor) {
                                let lw = *indices.get(neighbor).unwrap();
                                *lowlinks
                                    .entry(current_node)
                                    .or_insert(indices[current_node]) = (*lowlinks
                                    .entry(current_node)
                                    .or_insert(indices[current_node]))
                                .min(lw);
                            }
                        }

                        // Check if current node is root of SCC
                        if lowlinks[current_node] == indices[current_node] {
                            let mut scc: Vec<&str> = Vec::new();
                            loop {
                                let w = stack.pop().unwrap();
                                on_stack.remove(w);
                                scc.push(w);
                                if w == current_node {
                                    break;
                                }
                            }
                            sccs.push(scc);
                        }
                    }
                }
            }
        }

        // Each SCC with >1 node is a cycle; single-node SCCs with self-loop also cycle
        for scc in sccs {
            let is_cycle = scc.len() > 1
                || scc.iter().any(|&n| {
                    adj.get(n)
                        .map(|neighbors| neighbors.contains(&n))
                        .unwrap_or(false)
                });
            if is_cycle {
                let entry = scc.first().unwrap();
                violations.push(Violation {
                    rule: "forbid_cycle".to_string(),
                    severity: *severity,
                    subject: PolicySubject {
                        id: (*entry).to_string(),
                        kind: "element".to_string(),
                    },
                    params: PolicyParams::Cycle {},
                    message: format!(
                        "cycle detected involving {} (SCC: {:?})",
                        entry,
                        scc.iter().map(|s| s.to_string()).collect::<Vec<_>>()
                    ),
                });
            }
        }
    }
    violations
}

/// Evaluate `max_fanout` rules: matched elements must have out-degree <= max.
fn eval_max_fanout(
    rules: &[&PolicyRule],
    elements: &[&ElementRow],
    edges: &[(String, String)],
) -> Vec<Violation> {
    let mut violations = Vec::new();
    // Pre-compute out-degree per source
    let mut out_degree: HashMap<&str, usize> = HashMap::new();
    for (source, _) in edges {
        *out_degree.entry(source.as_str()).or_insert(0) += 1;
    }
    for rule in rules {
        let PolicyRule::MaxFanout {
            selector,
            severity,
            params,
        } = rule
        else {
            continue;
        };
        let PolicyParams::MaxFanout { max } = params else {
            continue;
        };
        for elem in elements {
            if !matches_selector(selector, &elem.id) {
                continue;
            }
            let degree = out_degree.get(elem.id.as_str()).copied().unwrap_or(0);
            if degree > *max {
                violations.push(Violation {
                    rule: "max_fanout".to_string(),
                    severity: *severity,
                    subject: PolicySubject {
                        id: elem.id.clone(),
                        kind: "element".to_string(),
                    },
                    params: PolicyParams::MaxFanout { max: *max },
                    message: format!(
                        "{} has fan-out {} which exceeds max {}",
                        elem.id, degree, max
                    ),
                });
            }
        }
    }
    violations
}

/// Evaluate `evidence_required` rules: every matched element must have evidence.
fn eval_evidence_required(
    rules: &[&PolicyRule],
    elements: &[&ElementRow],
    evidenced_version_ids: &HashSet<String>,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    for rule in rules {
        let PolicyRule::EvidenceRequired { selector, severity } = rule else {
            continue;
        };
        for elem in elements {
            if !matches_selector(selector, &elem.id) {
                continue;
            }
            let has_evidence = !elem.current_version_id.is_empty()
                && evidenced_version_ids.contains(&elem.current_version_id);
            if !has_evidence {
                violations.push(Violation {
                    rule: "evidence_required".to_string(),
                    severity: *severity,
                    subject: PolicySubject {
                        id: elem.id.clone(),
                        kind: "element".to_string(),
                    },
                    params: PolicyParams::EvidenceRequired {},
                    message: format!("{} has no supporting evidence", elem.id),
                });
            }
        }
    }
    violations
}

/// Evaluate `confidence_min` rules: matched elements must have confidence >= min.
fn eval_confidence_min(rules: &[&PolicyRule], elements: &[&ElementRow]) -> Vec<Violation> {
    let mut violations = Vec::new();
    for rule in rules {
        let PolicyRule::ConfidenceMin {
            selector,
            severity,
            params,
        } = rule
        else {
            continue;
        };
        let PolicyParams::ConfidenceMin { min } = params else {
            continue;
        };
        for elem in elements {
            if !matches_selector(selector, &elem.id) {
                continue;
            }
            if elem.current_confidence < *min {
                violations.push(Violation {
                    rule: "confidence_min".to_string(),
                    severity: *severity,
                    subject: PolicySubject {
                        id: elem.id.clone(),
                        kind: "element".to_string(),
                    },
                    params: PolicyParams::ConfidenceMin { min: *min },
                    message: format!(
                        "{} confidence {:.2} below minimum {:.2}",
                        elem.id, elem.current_confidence, min
                    ),
                });
            }
        }
    }
    violations
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Evaluate a policy against the live graph.
///
/// `policy` is the parsed list of rules (no waivers — those are in `waivers`).
/// `waivers` is the list of waivers from the policy file.
/// `repo` is the graph store to query.
/// `fail_on` is the severity threshold string ("error", "warning", "info").
/// `now` is the evaluation timestamp (used to check waiver expiry).
pub fn check_policy(
    policy: &[PolicyRule],
    waivers: &[Waiver],
    repo: &dyn DiagramRepository,
    fail_on: &str,
    now: DateTime<Utc>,
) -> Result<PolicyReport, PolicyError> {
    // Read graph data across all known categories (list_* filters by
    // category in Cypher, so an empty category would match nothing).
    let mut elements: Vec<ElementRow> = Vec::new();
    let mut edges: Vec<(String, String)> = Vec::new();
    for category in &["c4", "uml", "behavior"] {
        let elems = repo
            .list_elements(category, None, None)
            .map_err(|e| PolicyError::RepoRead(e.to_string()))?;
        elements.extend(elems);
        let semantic_edges = repo
            .list_semantic_edges(category)
            .map_err(|e| PolicyError::RepoRead(e.to_string()))?;
        edges.extend(
            semantic_edges
                .iter()
                .map(|e| (e.source_id.clone(), e.target_id.clone())),
        );
    }

    let elem_refs: Vec<_> = elements.iter().collect();
    let edge_tuples: Vec<(String, String)> = edges;

    // Partition rules by type
    let forbid_dep: Vec<_> = policy
        .iter()
        .filter(|r| matches_rule_name(r, "forbid_dependency"))
        .collect();
    let require_dep: Vec<_> = policy
        .iter()
        .filter(|r| matches_rule_name(r, "require_dependency"))
        .collect();
    let forbid_cycle: Vec<_> = policy
        .iter()
        .filter(|r| matches_rule_name(r, "forbid_cycle"))
        .collect();
    let max_fanout: Vec<_> = policy
        .iter()
        .filter(|r| matches_rule_name(r, "max_fanout"))
        .collect();
    let evidence_required: Vec<_> = policy
        .iter()
        .filter(|r| matches_rule_name(r, "evidence_required"))
        .collect();
    let confidence_min: Vec<_> = policy
        .iter()
        .filter(|r| matches_rule_name(r, "confidence_min"))
        .collect();

    // Collect evidenced version ids (for evidence_required).
    // Query per version id: EvidenceEntry carries no version back-reference,
    // so a batched call cannot be mapped back to its version.
    let version_ids: Vec<String> = elements
        .iter()
        .filter(|e| !e.current_version_id.is_empty())
        .map(|e| e.current_version_id.clone())
        .collect();
    let mut evidenced_version_ids: HashSet<String> = HashSet::new();
    if !evidence_required.is_empty() {
        for vid in &version_ids {
            if let Ok(entries) = repo.list_evidence_for_versions(std::slice::from_ref(vid))
                && !entries.is_empty()
            {
                evidenced_version_ids.insert(vid.clone());
            }
        }
    }

    // Evaluate
    let mut violations = Vec::new();
    violations.extend(eval_forbid_dependency(&forbid_dep, &edge_tuples));
    violations.extend(eval_require_dependency(
        &require_dep,
        &elem_refs,
        &edge_tuples,
    ));
    violations.extend(eval_forbid_cycle(&forbid_cycle, &elem_refs, &edge_tuples));
    violations.extend(eval_max_fanout(&max_fanout, &elem_refs, &edge_tuples));
    violations.extend(eval_evidence_required(
        &evidence_required,
        &elem_refs,
        &evidenced_version_ids,
    ));
    violations.extend(eval_confidence_min(&confidence_min, &elem_refs));

    // Sort violations by (rule, subject.id, severity)
    violations.sort_by_key(|v| (v.rule.clone(), v.subject.id.clone(), v.severity));

    // Mark expired waivers and apply active waivers
    let mut waivers_out: Vec<Waiver> = waivers.to_vec();
    for w in &mut waivers_out {
        w.expired = w.expires_at < now;
    }

    // Suppress violations with matching active waivers
    let active_waivers: HashSet<_> = waivers_out
        .iter()
        .filter(|w| !w.expired)
        .map(|w| (w.rule.clone(), w.subject_id.clone()))
        .collect();

    let original_count = violations.len();
    violations.retain(|v| !active_waivers.contains(&(v.rule.clone(), v.subject.id.clone())));
    let waived_count = original_count.saturating_sub(violations.len());

    // Summary
    let total = policy.len();
    let passed = total.saturating_sub(violations.len());
    let failed = violations.len();
    let summary = PolicySummary {
        total,
        passed,
        failed,
        waived: waived_count,
        fail_on: fail_on.to_string(),
    };

    Ok(PolicyReport {
        schema_version: "1.0".to_string(),
        capability: "architecture-policy-mvp".to_string(),
        policy_id: "policy.json".to_string(),
        evaluated_at: now,
        violations,
        waivers: waivers_out,
        summary,
        warnings: vec![],
    })
}

fn matches_rule_name(rule: &PolicyRule, name: &str) -> bool {
    matches!(
        (rule, name),
        (PolicyRule::ForbidDependency { .. }, "forbid_dependency")
            | (PolicyRule::RequireDependency { .. }, "require_dependency")
            | (PolicyRule::ForbidCycle { .. }, "forbid_cycle")
            | (PolicyRule::MaxFanout { .. }, "max_fanout")
            | (PolicyRule::EvidenceRequired { .. }, "evidence_required")
            | (PolicyRule::ConfidenceMin { .. }, "confidence_min")
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{ElementRow, SemanticEdgeRow};
    use crate::store::{ElementRepository, GraphStore, LbugStore, SemanticEdgeRepository};
    use chrono::Utc;

    /// Open a real `LbugStore` in `project_dir` and seed it with the
    /// given elements + edges. Replaces the previous MockRepo shadow
    /// of DiagramRepository — the production Cypher filter now runs
    /// against the production store, so tests exercise the same path
    /// as `archctl policy check`.
    fn seeded_store(
        project_dir: &std::path::Path,
        elements: Vec<ElementRow>,
        edges: Vec<SemanticEdgeRow>,
    ) -> LbugStore {
        let mut store = LbugStore::open(project_dir).expect("LbugStore::open");
        store.init().expect("LbugStore::init");
        for e in &elements {
            let v = crate::graph::ElementVersion {
                id: e.current_version_id.clone(),
                element_id: e.id.clone(),
                name: e.current_name.clone(),
                status: "accepted".to_string(),
                origin: "test".to_string(),
                confidence: e.current_confidence,
                props: Default::default(),
            };
            store
                .upsert_element_version(&v)
                .expect("upsert_element_version");
            store
                .link_current_version(&e.id, &e.current_version_id)
                .expect("link_current_version");
            let elem = crate::graph::Element {
                id: e.id.clone(),
                kind_id: e.kind_id.clone(),
                category: e.category.clone(),
                canonical_key: e.canonical_key.clone(),
                current_name: e.current_name.clone(),
                current_status: e.current_status.clone(),
                current_confidence: e.current_confidence,
                current_version_id: e.current_version_id.clone(),
            };
            store.upsert_element(&elem).expect("upsert_element");
        }
        for ed in &edges {
            store
                .link_semantic_edge(
                    &ed.source_id,
                    &ed.target_id,
                    &ed.relation_id,
                    &ed.predicate_id,
                    &ed.props,
                    true,
                )
                .expect("link_semantic_edge");
        }
        store
    }

    fn elem(id: &str) -> ElementRow {
        ElementRow {
            id: id.to_string(),
            kind_id: "container".to_string(),
            category: "c4".to_string(),
            canonical_key: id.to_string(),
            current_name: id.to_string(),
            current_status: "active".to_string(),
            current_confidence: 0.9,
            current_version_id: format!("{}-v1", id),
        }
    }

    fn edge(source: &str, target: &str) -> SemanticEdgeRow {
        SemanticEdgeRow {
            relation_id: format!("rel-{}-{}", source, target),
            predicate_id: "depends_on".to_string(),
            source_id: source.to_string(),
            target_id: target.to_string(),
            order_key: "0".to_string(),
            props: serde_json::Map::new(),
        }
    }

    #[test]
    fn test_no_violations_on_clean_graph() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = seeded_store(
            tmp.path(),
            vec![elem("c4:container:a"), elem("c4:container:b")],
            vec![edge("c4:container:a", "c4:container:b")],
        );
        let policy = vec![PolicyRule::ForbidDependency {
            selector: "c4:container:other*".to_string(),
            severity: Severity::Error,
            params: PolicyParams::Dependency {
                target: "c4:container:b".to_string(),
            },
        }];
        let report = check_policy(&policy, &[], &repo, "error", Utc::now()).unwrap();
        assert_eq!(report.summary.failed, 0);
        assert_eq!(report.summary.passed, 1);
    }

    #[test]
    fn test_forbid_dependency_violation() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = seeded_store(
            tmp.path(),
            vec![elem("c4:container:a"), elem("c4:container:b")],
            vec![edge("c4:container:a", "c4:container:b")],
        );
        let policy = vec![PolicyRule::ForbidDependency {
            selector: "c4:container:a".to_string(),
            severity: Severity::Error,
            params: PolicyParams::Dependency {
                target: "c4:container:b".to_string(),
            },
        }];
        let report = check_policy(&policy, &[], &repo, "error", Utc::now()).unwrap();
        assert_eq!(report.summary.failed, 1);
        assert!(!report.violations.is_empty());
        assert_eq!(report.violations[0].subject.id, "c4:container:a");
    }

    #[test]
    fn test_valid_waiver_suppresses_violation() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = seeded_store(
            tmp.path(),
            vec![elem("c4:container:a"), elem("c4:container:b")],
            vec![edge("c4:container:a", "c4:container:b")],
        );
        let policy = vec![PolicyRule::ForbidDependency {
            selector: "c4:container:a".to_string(),
            severity: Severity::Error,
            params: PolicyParams::Dependency {
                target: "c4:container:b".to_string(),
            },
        }];
        let waiver = Waiver {
            rule: "forbid_dependency".to_string(),
            subject_id: "c4:container:a".to_string(),
            reason: "intentional shared kernel".to_string(),
            expires_at: Utc::now() + chrono::Duration::days(30),
            expired: false,
        };
        let report = check_policy(&policy, &[waiver], &repo, "error", Utc::now()).unwrap();
        assert_eq!(report.summary.failed, 0);
        assert_eq!(report.summary.waived, 1);
        let active = report.waivers.iter().find(|w| !w.expired).unwrap();
        assert_eq!(active.reason, "intentional shared kernel");
    }

    #[test]
    fn test_expired_waiver_keeps_violation() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = seeded_store(
            tmp.path(),
            vec![elem("c4:container:a"), elem("c4:container:b")],
            vec![edge("c4:container:a", "c4:container:b")],
        );
        let policy = vec![PolicyRule::ForbidDependency {
            selector: "c4:container:a".to_string(),
            severity: Severity::Error,
            params: PolicyParams::Dependency {
                target: "c4:container:b".to_string(),
            },
        }];
        let waiver = Waiver {
            rule: "forbid_dependency".to_string(),
            subject_id: "c4:container:a".to_string(),
            reason: "intentional shared kernel".to_string(),
            expires_at: Utc::now() - chrono::Duration::days(1),
            expired: false, // will be recomputed
        };
        let report = check_policy(&policy, &[waiver], &repo, "error", Utc::now()).unwrap();
        assert_eq!(report.summary.failed, 1);
        assert_eq!(report.summary.waived, 0);
        let expired = report.waivers.iter().find(|w| w.expired).unwrap();
        assert!(expired.expired);
    }

    #[test]
    fn test_matches_selector_glob() {
        assert!(matches_selector("c4:container:*", "c4:container:a"));
        assert!(matches_selector("c4:container:*", "c4:container:b"));
        assert!(!matches_selector("c4:container:*", "c4:context:a"));
        assert!(!matches_selector("c4:container:*", "c4:container"));
        assert!(matches_selector("c4:*", "c4:container:a"));
        assert!(matches_selector("c4:*", "c4:context:x"));
        assert!(matches_selector("c4:container:a", "c4:container:a"));
        assert!(!matches_selector("c4:container:a", "c4:container:b"));
    }

    #[test]
    fn test_max_fanout_violation() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = seeded_store(
            tmp.path(),
            vec![
                elem("c4:container:a"),
                elem("c4:container:b"),
                elem("c4:container:c"),
                elem("c4:container:d"),
            ],
            vec![
                edge("c4:container:a", "c4:container:b"),
                edge("c4:container:a", "c4:container:c"),
                edge("c4:container:a", "c4:container:d"),
            ],
        );
        let policy = vec![PolicyRule::MaxFanout {
            selector: "c4:*".to_string(),
            severity: Severity::Warning,
            params: PolicyParams::MaxFanout { max: 2 },
        }];
        let report = check_policy(&policy, &[], &repo, "error", Utc::now()).unwrap();
        assert_eq!(report.summary.failed, 1);
        assert_eq!(report.violations[0].rule, "max_fanout");
        assert_eq!(report.violations[0].subject.id, "c4:container:a");
    }

    #[test]
    fn test_max_fanout_passes_within_limit() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = seeded_store(
            tmp.path(),
            vec![elem("c4:container:a"), elem("c4:container:b")],
            vec![edge("c4:container:a", "c4:container:b")],
        );
        let policy = vec![PolicyRule::MaxFanout {
            selector: "c4:*".to_string(),
            severity: Severity::Warning,
            params: PolicyParams::MaxFanout { max: 2 },
        }];
        let report = check_policy(&policy, &[], &repo, "error", Utc::now()).unwrap();
        assert_eq!(report.summary.failed, 0);
    }

    #[test]
    fn test_evidence_required_violation_without_evidence() {
        // MockRepo returns no evidence for any version
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = seeded_store(tmp.path(), vec![elem("c4:container:a")], vec![]);
        let policy = vec![PolicyRule::EvidenceRequired {
            selector: "c4:*".to_string(),
            severity: Severity::Error,
        }];
        let report = check_policy(&policy, &[], &repo, "error", Utc::now()).unwrap();
        assert_eq!(report.summary.failed, 1);
        assert_eq!(report.violations[0].rule, "evidence_required");
    }

    #[test]
    fn test_confidence_min_violation() {
        let mut low = elem("c4:container:low");
        low.current_confidence = 0.4;
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = seeded_store(tmp.path(), vec![low, elem("c4:container:high")], vec![]);
        let policy = vec![PolicyRule::ConfidenceMin {
            selector: "c4:*".to_string(),
            severity: Severity::Warning,
            params: PolicyParams::ConfidenceMin { min: 0.7 },
        }];
        let report = check_policy(&policy, &[], &repo, "error", Utc::now()).unwrap();
        assert_eq!(report.summary.failed, 1);
        assert_eq!(report.violations[0].rule, "confidence_min");
        assert_eq!(report.violations[0].subject.id, "c4:container:low");
    }

    #[test]
    fn test_confidence_min_passes_when_all_high() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = seeded_store(tmp.path(), vec![elem("c4:container:a")], vec![]);
        let policy = vec![PolicyRule::ConfidenceMin {
            selector: "c4:*".to_string(),
            severity: Severity::Warning,
            params: PolicyParams::ConfidenceMin { min: 0.7 },
        }];
        let report = check_policy(&policy, &[], &repo, "error", Utc::now()).unwrap();
        assert_eq!(report.summary.failed, 0);
    }
}
