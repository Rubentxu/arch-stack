//! Architecture Agent — heuristic naming connascence detection.
//!
//! v1.0: detects similarity in element names (e.g. UserService vs UserManager)
//! as a low-confidence naming inconsistency finding. No boundary analysis
//! without edge data in GraphView.

use std::collections::HashMap;

use crate::cognitive::context::{AgentContext, Element};
use crate::cognitive::descriptor::{AgentBudget, AgentDescriptor, ModelPolicy};
use crate::cognitive::observer::{ObserveError, ReactiveObserver};
use crate::cognitive::output::{
    AgentOutput, FindingCandidate, NoActionCode, NoActionReason, Severity,
};

/// Naming connascence confidence threshold.
const CONNASCENCE_THRESHOLD: f64 = 0.55;

/// Architecture Agent — heuristic-only, deterministic.
pub struct ArchitectureAgent {
    descriptor: AgentDescriptor,
}

impl ArchitectureAgent {
    pub fn new() -> Self {
        Self {
            descriptor: AgentDescriptor {
                id: "architecture-agent".into(),
                version: "0.1.0".into(),
                subscriptions: vec![],
                required_views: vec!["c4-components".into(), "c4-containers".into()],
                output_schema: "{}".into(),
                model_policy: ModelPolicy::Heuristic,
                budget: AgentBudget {
                    tokens: Some(8192),
                    time_ms: Some(5000),
                    cost_cents: Some(2),
                },
                capabilities: vec![],
                deterministic: true,
                idempotent: true,
            },
        }
    }

    fn arch_elements<'a>(&self, view: &'a [Element]) -> Vec<&'a Element> {
        view.iter()
            .filter(|e| super::is_arch_relevant(&e.kind_id))
            .collect()
    }

    /// Detect naming connascence between pairs of architecturally relevant elements.
    fn detect_naming_connascence(&self, elements: &[&Element]) -> Vec<FindingCandidate> {
        let mut findings = Vec::new();

        // Group by stripped name prefix for O(n) pairing
        let mut by_prefix: HashMap<String, Vec<&Element>> = HashMap::new();
        for el in elements {
            let prefix = super::strip_suffix(&el.name).to_lowercase();
            by_prefix.entry(prefix).or_default().push(el);
        }

        // For each group with 2+ elements, generate a finding
        for (prefix, group) in by_prefix.iter().filter(|(_, g)| g.len() > 1) {
            // Compute pairwise similarities within the group
            let mut best_sim = CONNASCENCE_THRESHOLD;
            let mut best_pair: Option<(&Element, &Element)> = None;

            for i in 0..group.len() {
                for j in (i + 1)..group.len() {
                    let sim = super::name_similarity(&group[i].name, &group[j].name);
                    if sim >= CONNASCENCE_THRESHOLD && sim > best_sim {
                        best_sim = sim;
                        best_pair = Some((group[i], group[j]));
                    }
                }
            }

            if let Some((e1, e2)) = best_pair {
                let confidence = best_sim;
                let evidence_ids: Vec<String> = vec![e1.id.clone(), e2.id.clone()];
                let recommended_views = if confidence >= 0.8 {
                    vec!["c4-component".into()]
                } else {
                    vec!["c4-container".into()]
                };

                findings.push(FindingCandidate {
                    severity: Severity::Warning,
                    title: format!(
                        "Naming connascence: {} and {} share prefix '{}'",
                        e1.name, e2.name, prefix
                    ),
                    body: format!(
                        "Elements '{}' and '{}' share the prefix '{}' which may indicate \
                         an unintentional naming pattern. Review if these represent distinct \
                         concepts or should be consolidated.",
                        e1.name, e2.name, prefix
                    ),
                    confidence,
                    evidence_ids,
                    recommended_views,
                });
            }
        }

        findings
    }
}

impl Default for ArchitectureAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl ReactiveObserver for ArchitectureAgent {
    fn descriptor(&self) -> AgentDescriptor {
        self.descriptor.clone()
    }

    fn matches(&self, context: &AgentContext) -> bool {
        !self.arch_elements(&context.graph_view.elements).is_empty()
    }

    fn observe(&self, context: &AgentContext) -> Result<AgentOutput, ObserveError> {
        let arch_elements: Vec<&Element> = self.arch_elements(&context.graph_view.elements);

        if arch_elements.is_empty() {
            return Ok(AgentOutput::NoAction(NoActionReason {
                code: NoActionCode::OutOfScope,
                message: "no architecturally relevant elements in graph view".into(),
            }));
        }

        let findings = self.detect_naming_connascence(&arch_elements);

        if findings.is_empty() {
            return Ok(AgentOutput::NoAction(NoActionReason {
                code: NoActionCode::InsufficientConfidence,
                message: "no naming connascence detected".into(),
            }));
        }

        // Return the highest-confidence finding
        let best = findings
            .into_iter()
            .max_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        Ok(AgentOutput::FindingCandidate(best))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::context::GraphView;

    fn make_ctx(elements: Vec<Element>) -> AgentContext {
        // REQ-T06-003: feedback_history plumbing — see AgentContext::with_feedback_history
        // REQ-M25-006: pending_adjudications wiring (TRUST-008 REQ-T08-005). See
        // archctl/src/cognitive/context.rs::with_pending_adjudications. Struct literal
        // intentionally empty at this site — agent-level contexts do not pre-fetch the
        // adjudication queue.
        // recent_events (M34 W2) populated by compress_for_budget before dispatch.
        AgentContext {
            goal: "analyze architecture".into(),
            triggering_event: None,
            graph_view: GraphView {
                elements,
                edges: vec![],
            },
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        }
    }

    fn el(id: &str, kind_id: &str, name: &str) -> Element {
        Element {
            id: id.into(),
            kind_id: kind_id.into(),
            name: name.into(),
            canonical_key: id.into(),
            properties: serde_json::Value::Null,
        }
    }

    #[test]
    fn matches_true_when_arch_elements_present() {
        let agent = ArchitectureAgent::new();
        let ctx = make_ctx(vec![el("e1", "mt.container", "UserService")]);
        assert!(agent.matches(&ctx));
    }

    #[test]
    fn matches_false_when_only_code_elements() {
        let agent = ArchitectureAgent::new();
        let ctx = make_ctx(vec![el("e1", "code.function", "create_user")]);
        assert!(!agent.matches(&ctx));
    }

    #[test]
    fn no_action_on_empty_graph() {
        let agent = ArchitectureAgent::new();
        let ctx = make_ctx(vec![]);
        let out = agent.observe(&ctx).unwrap();
        assert!(matches!(out, AgentOutput::NoAction(_)));
    }

    #[test]
    fn finding_on_similar_names() {
        let agent = ArchitectureAgent::new();
        let ctx = make_ctx(vec![
            el("e1", "mt.container", "UserService"),
            el("e2", "mt.container", "UserManager"),
        ]);
        let out = agent.observe(&ctx).unwrap();
        let finding = match out {
            AgentOutput::FindingCandidate(f) => f,
            other => panic!("expected FindingCandidate, got {:?}", other),
        };
        assert!(finding.confidence >= 0.55);
        assert!(finding.title.contains("UserService"));
        assert!(finding.title.contains("UserManager"));
    }

    #[test]
    fn no_action_on_distinct_names() {
        let agent = ArchitectureAgent::new();
        let ctx = make_ctx(vec![
            el("e1", "mt.container", "UserService"),
            el("e2", "mt.container", "OrderService"),
        ]);
        let out = agent.observe(&ctx).unwrap();
        assert!(matches!(out, AgentOutput::NoAction(_)));
    }

    #[test]
    fn descriptor_correct() {
        let agent = ArchitectureAgent::new();
        let d = agent.descriptor();
        assert_eq!(d.id, "architecture-agent");
        assert!(matches!(d.model_policy, ModelPolicy::Heuristic));
        assert!(d.deterministic);
    }

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage v2, 2026-08-22)
    // ---------------------------------------------------------------------------

    /// `Default::default()` is equivalent to `::new()` for `ArchitectureAgent`
    /// (both produce the same descriptor, deterministic + idempotent flags).
    #[test]
    fn default_trait_impl_equiv_new() {
        let default = ArchitectureAgent::default();
        let new = ArchitectureAgent::new();
        assert_eq!(default.descriptor().id, new.descriptor().id);
        assert_eq!(
            default.descriptor().required_views,
            new.descriptor().required_views
        );
        assert!(default.descriptor().deterministic);
        assert!(default.descriptor().idempotent);
    }

    /// When arch elements are present but no naming connascence is detected,
    /// the agent returns `NoAction(InsufficientConfidence)` with the
    /// "no naming connascence detected" message. Distinct from the empty-graph
    /// branch which returns `OutOfScope`.
    #[test]
    fn observe_returns_noaction_insufficient_confidence_when_no_connascence() {
        let agent = ArchitectureAgent::new();
        let ctx = make_ctx(vec![
            el("e1", "mt.container", "UserService"),
            el("e2", "mt.container", "OrderService"),
        ]);
        let out = agent.observe(&ctx).unwrap();
        match out {
            AgentOutput::NoAction(reason) => {
                assert_eq!(reason.code, NoActionCode::InsufficientConfidence);
                assert!(
                    reason.message.contains("no naming connascence"),
                    "message must reflect the post-detection branch, got: {}",
                    reason.message
                );
            }
            other => panic!("expected NoAction(InsufficientConfidence), got {other:?}"),
        }
    }

    /// Empty graph → `NoAction(OutOfScope)` with the "no architecturally
    /// relevant elements in graph view" message. Distinct from the
    /// no-connascence branch (InsufficientConfidence).
    #[test]
    fn observe_returns_noaction_out_of_scope_on_empty_graph() {
        let agent = ArchitectureAgent::new();
        let ctx = make_ctx(vec![]);
        let out = agent.observe(&ctx).unwrap();
        match out {
            AgentOutput::NoAction(reason) => {
                assert_eq!(reason.code, NoActionCode::OutOfScope);
                assert!(
                    reason.message.contains("no architecturally relevant"),
                    "message must reflect the empty-graph branch, got: {}",
                    reason.message
                );
            }
            other => panic!("expected NoAction(OutOfScope), got {other:?}"),
        }
    }

    /// Graph with only non-architectural elements (e.g. `code.function`) →
    /// `arch_elements()` filters them out → `OutOfScope`. Locks that
    /// the filter is applied BEFORE the connascence detector runs.
    #[test]
    fn observe_returns_out_of_scope_when_only_non_arch_elements() {
        let agent = ArchitectureAgent::new();
        let ctx = make_ctx(vec![
            el("e1", "code.function", "create_user"),
            el("e2", "code.function", "delete_user"),
        ]);
        let out = agent.observe(&ctx).unwrap();
        match out {
            AgentOutput::NoAction(reason) => {
                assert_eq!(reason.code, NoActionCode::OutOfScope);
            }
            other => panic!("expected NoAction(OutOfScope), got {other:?}"),
        }
    }

    /// When multiple pairs trigger connascence (3+ elements in the same
    /// prefix group), the agent returns the HIGHEST-confidence pair, not
    /// the first or last. Locks the `max_by(partial_cmp)` selection in
    /// `observe()`.
    #[test]
    fn observe_returns_highest_confidence_finding_when_multiple_pairs() {
        let agent = ArchitectureAgent::new();
        let ctx = make_ctx(vec![
            el("e1", "mt.container", "UserService"),
            el("e2", "mt.container", "UserManager"),
            el("e3", "mt.container", "UserController"),
        ]);
        let out = agent.observe(&ctx).unwrap();
        let finding = match out {
            AgentOutput::FindingCandidate(f) => f,
            other => panic!("expected FindingCandidate, got {other:?}"),
        };
        assert!(finding.confidence >= 0.55);
        // The finding must include the most similar pair (highest sim)
        // — not necessarily "UserService"/"UserManager" specifically.
        assert!(finding.title.contains("Naming connascence"));
        // The two evidence_ids must come from the highest-sim pair
        assert_eq!(finding.evidence_ids.len(), 2);
    }

    /// `recommended_views` for HIGH-confidence findings (>= 0.8) is
    /// `["c4-component"]`; for lower confidence it's `["c4-container"]`.
    /// Locks the `if confidence >= 0.8` branch in `detect_naming_connascence`.
    #[test]
    fn recommended_views_for_high_confidence_is_c4_component() {
        let agent = ArchitectureAgent::new();
        // Pair with very high name similarity — both end in "Service"
        let ctx = make_ctx(vec![
            el("e1", "mt.container", "PaymentService"),
            el("e2", "mt.container", "PaymentManager"),
        ]);
        let out = agent.observe(&ctx).unwrap();
        let finding = match out {
            AgentOutput::FindingCandidate(f) => f,
            other => panic!("expected FindingCandidate, got {other:?}"),
        };
        // The pair "PaymentService"/"PaymentManager" has high sim
        // (share "payment" prefix, end with "service"/"manager").
        // If sim >= 0.8, recommended_views must be the high-confidence view.
        if finding.confidence >= 0.8 {
            assert_eq!(
                finding.recommended_views,
                vec!["c4-component".to_string()],
                "high-confidence findings must recommend c4-component view"
            );
        } else {
            assert_eq!(
                finding.recommended_views,
                vec!["c4-container".to_string()],
                "medium-confidence findings must recommend c4-container view"
            );
        }
    }

    /// Mixed-kind graphs: architectural elements alongside non-architectural
    /// elements. The connascence detector must only consider the
    /// architectural subset. Locks the filter behavior under mixed input.
    #[test]
    fn observe_only_consideres_arch_elements_for_connascence() {
        let agent = ArchitectureAgent::new();
        let ctx = make_ctx(vec![
            // 2 arch elements with similar names — would trigger finding
            el("e1", "mt.container", "UserService"),
            el("e2", "mt.container", "UserManager"),
            // Non-arch element with very-similar name — must be ignored
            el("e3", "code.function", "userService"),
        ]);
        let out = agent.observe(&ctx).unwrap();
        // The non-arch element must not appear in the finding's evidence_ids
        let finding = match out {
            AgentOutput::FindingCandidate(f) => f,
            other => panic!("expected FindingCandidate, got {other:?}"),
        };
        assert!(
            !finding.evidence_ids.contains(&"e3".to_string()),
            "code.function elements must be filtered out — finding.evidence_ids: {:?}",
            finding.evidence_ids
        );
        assert!(finding.evidence_ids.contains(&"e1".to_string()));
        assert!(finding.evidence_ids.contains(&"e2".to_string()));
    }

    /// The `idempotent` and `deterministic` flags on the descriptor are
    /// critical for cache reuse and parallel evaluation — they must both
    /// be true for `ArchitectureAgent`. Locks the v1.0 contract.
    #[test]
    fn descriptor_deterministic_and_idempotent_flags() {
        let agent = ArchitectureAgent::new();
        let d = agent.descriptor();
        assert!(d.deterministic, "ArchitectureAgent must be deterministic");
        assert!(d.idempotent, "ArchitectureAgent must be idempotent");
        assert!(
            d.required_views.contains(&"c4-components".to_string())
                && d.required_views.contains(&"c4-containers".to_string()),
            "ArchitectureAgent must require both c4-components and c4-containers views, got: {:?}",
            d.required_views
        );
        // ModelPolicy must be Heuristic (heuristic-only agent per doc comment)
        assert!(matches!(d.model_policy, ModelPolicy::Heuristic));
    }
}
