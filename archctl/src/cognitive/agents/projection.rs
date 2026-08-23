//! Projection Agent — heuristic goal-to-projection mapping.
//!
//! v1.0: hardcoded keyword → view_kind map. No LLM needed.

use crate::cognitive::context::{AgentContext, Element};
use crate::cognitive::descriptor::{AgentBudget, AgentDescriptor, ModelPolicy};
use crate::cognitive::observer::{ObserveError, ReactiveObserver};
use crate::cognitive::output::{
    AgentOutput, DiagramFormat, LayoutHints, NoActionCode, NoActionReason, ProjectionSpec, ViewKind,
};

/// Keywords that trigger the Projection Agent.
const TRIGGER_KEYWORDS: &[&str] = &[
    "diagram",
    "view",
    "project",
    "render",
    "export",
    "sequence",
    "c4",
    "component",
    "container",
    "context",
    "class",
    "structure",
];

/// Keyword → view_kind mapping.
fn keyword_to_view_kind(keyword: &str) -> Option<ViewKind> {
    match keyword.to_lowercase().as_str() {
        "sequence" => Some(ViewKind::Sequence),
        "class" | "structure" => Some(ViewKind::Class),
        "component" => Some(ViewKind::C4Component),
        "container" => Some(ViewKind::C4Container),
        "context" | "c4" => Some(ViewKind::C4Context),
        "state" => Some(ViewKind::State),
        "usecase" | "use-case" => Some(ViewKind::UseCase),
        _ => None,
    }
}

/// Keyword → diagram format mapping.
fn keyword_to_format(keyword: &str) -> Option<DiagramFormat> {
    match keyword.to_lowercase().as_str() {
        "mermaid" => Some(DiagramFormat::Mermaid),
        "structurizr" => Some(DiagramFormat::Structurizr),
        "plantuml" => Some(DiagramFormat::PlantUML),
        _ => None,
    }
}

/// Projection Agent — heuristic-only, deterministic.
pub struct ProjectionAgent {
    descriptor: AgentDescriptor,
}

impl ProjectionAgent {
    pub fn new() -> Self {
        Self {
            descriptor: AgentDescriptor {
                id: "projection-agent".into(),
                version: "0.1.0".into(),
                subscriptions: vec![],
                required_views: vec![],
                output_schema: "{}".into(),
                model_policy: ModelPolicy::Heuristic,
                budget: AgentBudget {
                    tokens: Some(2048),
                    time_ms: Some(1000),
                    cost_cents: Some(0),
                },
                capabilities: vec![],
                deterministic: true,
                idempotent: true,
            },
        }
    }

    /// Extract focus element IDs by matching goal terms against element labels.
    fn focus_elements(&self, goal: &str, elements: &[Element]) -> Vec<String> {
        let goal_lower = goal.to_lowercase();
        let mut ids: Vec<String> = elements
            .iter()
            .filter(|e| {
                goal_lower.contains(&e.name.to_lowercase())
                    || goal_lower.contains(&e.canonical_key.to_lowercase())
            })
            .map(|e| e.id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }
}

impl Default for ProjectionAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl ReactiveObserver for ProjectionAgent {
    fn descriptor(&self) -> AgentDescriptor {
        self.descriptor.clone()
    }

    fn matches(&self, context: &AgentContext) -> bool {
        context
            .goal
            .to_lowercase()
            .split_whitespace()
            .any(|word| TRIGGER_KEYWORDS.contains(&word))
    }

    fn observe(&self, context: &AgentContext) -> Result<AgentOutput, ObserveError> {
        let goal_lower = context.goal.to_lowercase();

        // Find the first matching keyword for view_kind
        let view_kind = goal_lower
            .split_whitespace()
            .filter_map(keyword_to_view_kind)
            .next();

        // Check for explicit format hint
        let format = goal_lower
            .split_whitespace()
            .filter_map(keyword_to_format)
            .next()
            .unwrap_or(DiagramFormat::PlantUML);

        let view_kind = match view_kind {
            Some(v) => v,
            None => {
                return Ok(AgentOutput::NoAction(NoActionReason {
                    code: NoActionCode::OutOfScope,
                    message: "no projection keyword found in goal".into(),
                }));
            }
        };

        let focus_elements = self.focus_elements(&context.goal, &context.graph_view.elements);

        Ok(AgentOutput::ProjectionSpec(ProjectionSpec {
            view_kind,
            format,
            focus_elements,
            layout_hints: LayoutHints {
                direction: None,
                ranksep: None,
                nodesep: None,
            },
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::context::GraphView;

    fn make_ctx(goal: &str, elements: Vec<Element>) -> AgentContext {
        // REQ-T06-003: feedback_history plumbing — see AgentContext::with_feedback_history
        // REQ-M25-006: pending_adjudications wiring (TRUST-008 REQ-T08-005). Struct literal
        // intentionally empty at this site.
        // recent_events (M34 W2) populated by compress_for_budget before dispatch.
        AgentContext {
            goal: goal.into(),
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

    fn el(id: &str, name: &str, canonical_key: &str) -> Element {
        Element {
            id: id.into(),
            kind_id: "mt.container".into(),
            name: name.into(),
            canonical_key: canonical_key.into(),
            properties: serde_json::Value::Null,
        }
    }

    #[test]
    fn matches_on_diagram_keyword() {
        let agent = ProjectionAgent::new();
        let ctx = make_ctx("show me a component diagram", vec![]);
        assert!(agent.matches(&ctx));
    }

    #[test]
    fn matches_on_view_keyword() {
        let agent = ProjectionAgent::new();
        let ctx = make_ctx("render the container view", vec![]);
        assert!(agent.matches(&ctx));
    }

    #[test]
    fn matches_false_on_non_projection_goal() {
        let agent = ProjectionAgent::new();
        let ctx = make_ctx("analyze coupling between A and B", vec![]);
        assert!(!agent.matches(&ctx));
    }

    #[test]
    fn maps_to_c4_component() {
        let agent = ProjectionAgent::new();
        let ctx = make_ctx("component diagram for auth", vec![]);
        let out = agent.observe(&ctx).unwrap();
        let spec = match out {
            AgentOutput::ProjectionSpec(s) => s,
            other => panic!("expected ProjectionSpec, got {:?}", other),
        };
        assert!(matches!(spec.view_kind, ViewKind::C4Component));
        assert!(matches!(spec.format, DiagramFormat::PlantUML));
    }

    #[test]
    fn maps_to_mermaid_format() {
        let agent = ProjectionAgent::new();
        let ctx = make_ctx("sequence in mermaid", vec![]);
        let out = agent.observe(&ctx).unwrap();
        let spec = match out {
            AgentOutput::ProjectionSpec(s) => s,
            other => panic!("expected ProjectionSpec, got {:?}", other),
        };
        assert!(matches!(spec.view_kind, ViewKind::Sequence));
        assert!(matches!(spec.format, DiagramFormat::Mermaid));
    }

    #[test]
    fn focus_elements_from_goal_terms() {
        let agent = ProjectionAgent::new();
        let ctx = make_ctx(
            "show me the UserService component",
            vec![
                el("e1", "UserService", "user-svc"),
                el("e2", "OrderService", "order-svc"),
            ],
        );
        let out = agent.observe(&ctx).unwrap();
        let spec = match out {
            AgentOutput::ProjectionSpec(s) => s,
            other => panic!("expected ProjectionSpec, got {:?}", other),
        };
        assert!(spec.focus_elements.contains(&"e1".into()));
        assert!(!spec.focus_elements.contains(&"e2".into()));
    }

    #[test]
    fn no_action_when_no_keyword() {
        let agent = ProjectionAgent::new();
        let ctx = make_ctx("analyze the codebase", vec![]);
        let out = agent.observe(&ctx).unwrap();
        assert!(matches!(out, AgentOutput::NoAction(_)));
    }

    #[test]
    fn descriptor_correct() {
        let agent = ProjectionAgent::new();
        let d = agent.descriptor();
        assert_eq!(d.id, "projection-agent");
        assert!(matches!(d.model_policy, ModelPolicy::Heuristic));
        assert!(d.deterministic);
    }

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage v3, 2026-08-22)
    // ---------------------------------------------------------------------------

    /// `ProjectionAgent::default()` is equivalent to `ProjectionAgent::new()`.
    /// Locks the `Default` impl contract.
    #[test]
    fn projection_agent_default_equiv_new() {
        let via_default = ProjectionAgent::default();
        let via_new = ProjectionAgent::new();
        assert_eq!(
            via_default.descriptor().id,
            via_new.descriptor().id,
            "default() and new() must yield identical descriptors"
        );
        assert_eq!(via_default.descriptor().id, "projection-agent");
    }

    /// "container" keyword maps to `ViewKind::C4Container`. Locks the
    /// distinguishability from "component" (which maps to C4Component).
    #[test]
    fn maps_to_c4_container_keyword() {
        let agent = ProjectionAgent::new();
        let ctx = make_ctx("show me the container diagram", vec![]);
        let out = agent.observe(&ctx).unwrap();
        let spec = match out {
            AgentOutput::ProjectionSpec(s) => s,
            other => panic!("expected ProjectionSpec, got {:?}", other),
        };
        assert!(matches!(spec.view_kind, ViewKind::C4Container));
    }

    /// Both "context" and "c4" keywords map to `ViewKind::C4Context`. Distinct
    /// from `maps_to_c4_component` (component → C4Component) and
    /// `maps_to_c4_container_keyword` (container → C4Container).
    #[test]
    fn maps_to_c4_context_keyword() {
        let agent = ProjectionAgent::new();
        for keyword in ["context", "c4"] {
            let ctx = make_ctx(&format!("generate a {keyword} diagram"), vec![]);
            let out = agent.observe(&ctx).unwrap();
            let spec = match out {
                AgentOutput::ProjectionSpec(s) => s,
                other => panic!("expected ProjectionSpec for {keyword}, got {:?}", other),
            };
            assert!(
                matches!(spec.view_kind, ViewKind::C4Context),
                "keyword '{keyword}' must map to C4Context, got {:?}",
                spec.view_kind
            );
        }
    }

    /// Both "class" and "structure" keywords map to `ViewKind::Class`.
    #[test]
    fn maps_to_class_view_kind() {
        let agent = ProjectionAgent::new();
        for keyword in ["class", "structure"] {
            let ctx = make_ctx(&format!("render the {keyword} diagram"), vec![]);
            let out = agent.observe(&ctx).unwrap();
            let spec = match out {
                AgentOutput::ProjectionSpec(s) => s,
                other => panic!("expected ProjectionSpec for {keyword}, got {:?}", other),
            };
            assert!(
                matches!(spec.view_kind, ViewKind::Class),
                "keyword '{keyword}' must map to Class, got {:?}",
                spec.view_kind
            );
        }
    }

    /// "state" keyword maps to `ViewKind::State`. Note: state is NOT in the
    /// TRIGGER_KEYWORDS list, so `matches()` returns false — but if a goal
    /// contains "state" alongside another trigger keyword (like "diagram"),
    /// the keyword scan picks it up.
    #[test]
    fn maps_to_state_view_kind() {
        let agent = ProjectionAgent::new();
        let ctx = make_ctx("diagram the state machine", vec![]);
        let out = agent.observe(&ctx).unwrap();
        let spec = match out {
            AgentOutput::ProjectionSpec(s) => s,
            other => panic!("expected ProjectionSpec, got {:?}", other),
        };
        assert!(matches!(spec.view_kind, ViewKind::State));
    }

    /// Both "usecase" and "use-case" keywords map to `ViewKind::UseCase`.
    #[test]
    fn maps_to_usecase_keyword() {
        let agent = ProjectionAgent::new();
        for keyword in ["usecase", "use-case"] {
            let ctx = make_ctx(&format!("render the {keyword} diagram"), vec![]);
            let out = agent.observe(&ctx).unwrap();
            let spec = match out {
                AgentOutput::ProjectionSpec(s) => s,
                other => panic!("expected ProjectionSpec for {keyword}, got {:?}", other),
            };
            assert!(
                matches!(spec.view_kind, ViewKind::UseCase),
                "keyword '{keyword}' must map to UseCase, got {:?}",
                spec.view_kind
            );
        }
    }

    /// Explicit "structurizr" format keyword maps to `DiagramFormat::Structurizr`.
    /// Distinct from `maps_to_mermaid_format` which checks Mermaid.
    #[test]
    fn maps_to_structurizr_format() {
        let agent = ProjectionAgent::new();
        let ctx = make_ctx("class diagram in structurizr", vec![]);
        let out = agent.observe(&ctx).unwrap();
        let spec = match out {
            AgentOutput::ProjectionSpec(s) => s,
            other => panic!("expected ProjectionSpec, got {:?}", other),
        };
        assert!(matches!(spec.format, DiagramFormat::Structurizr));
    }

    /// Explicit "plantuml" format keyword maps to `DiagramFormat::PlantUML`.
    /// PlantUML is also the DEFAULT format when no explicit format is given.
    #[test]
    fn maps_to_plantuml_format() {
        let agent = ProjectionAgent::new();
        let ctx = make_ctx("sequence in plantuml", vec![]);
        let out = agent.observe(&ctx).unwrap();
        let spec = match out {
            AgentOutput::ProjectionSpec(s) => s,
            other => panic!("expected ProjectionSpec, got {:?}", other),
        };
        assert!(matches!(spec.format, DiagramFormat::PlantUML));
    }

    /// Default format (no explicit keyword) is `PlantUML`. Locks the
    /// `unwrap_or(DiagramFormat::PlantUML)` fallback.
    #[test]
    fn default_format_is_plantuml() {
        let agent = ProjectionAgent::new();
        let ctx = make_ctx("sequence diagram please", vec![]);
        let out = agent.observe(&ctx).unwrap();
        let spec = match out {
            AgentOutput::ProjectionSpec(s) => s,
            other => panic!("expected ProjectionSpec, got {:?}", other),
        };
        assert!(matches!(spec.format, DiagramFormat::PlantUML));
    }

    /// Focus elements match via `canonical_key` when `name` does not overlap
    /// with the goal. Distinct from `focus_elements_from_goal_terms` which
    /// matches via `name`.
    #[test]
    fn focus_elements_matches_via_canonical_key() {
        let agent = ProjectionAgent::new();
        let ctx = make_ctx(
            "show me the user-svc component",
            vec![
                el("e1", "UserService", "user-svc"),
                el("e2", "OrderService", "order-svc"),
            ],
        );
        let out = agent.observe(&ctx).unwrap();
        let spec = match out {
            AgentOutput::ProjectionSpec(s) => s,
            other => panic!("expected ProjectionSpec, got {:?}", other),
        };
        assert_eq!(spec.focus_elements, vec!["e1".to_string()]);
    }

    /// Focus elements are empty when there is no overlap between goal and
    /// element names/canonical_keys. Locks the `filter().map().collect()`
    /// pipeline returning an empty Vec.
    #[test]
    fn focus_elements_empty_when_no_overlap() {
        let agent = ProjectionAgent::new();
        let ctx = make_ctx(
            "show me the billing component",
            vec![
                el("e1", "UserService", "user-svc"),
                el("e2", "OrderService", "order-svc"),
            ],
        );
        let out = agent.observe(&ctx).unwrap();
        let spec = match out {
            AgentOutput::ProjectionSpec(s) => s,
            other => panic!("expected ProjectionSpec, got {:?}", other),
        };
        assert!(
            spec.focus_elements.is_empty(),
            "no element matches goal, focus must be empty"
        );
    }

    /// Focus elements are sorted alphabetically by id when multiple matches
    /// exist. Distinct from `focus_elements_from_goal_terms` which only
    /// checks inclusion of one specific id. Note: `dedup()` is invoked but
    /// ids are unique per Element, so it never removes anything — the
    /// dedup is dead code in practice (kept for forward-compat if Element
    /// ever allows duplicate ids).
    #[test]
    fn focus_elements_sorted_by_id() {
        let agent = ProjectionAgent::new();
        let ctx = make_ctx(
            "component view of UserService and AuthService",
            vec![
                el("z9", "UnrelatedThing", "no-match"), // no match
                el("a1", "UserService", "user-svc"),    // matches "UserService"
                el("b2", "AuthService", "auth-svc"),    // matches "AuthService"
                el("c3", "PaymentService", "pay-svc"),  // no match
            ],
        );
        let out = agent.observe(&ctx).unwrap();
        let spec = match out {
            AgentOutput::ProjectionSpec(s) => s,
            other => panic!("expected ProjectionSpec, got {:?}", other),
        };
        // Sort puts a1 < b2. z9 and c3 are excluded.
        assert_eq!(
            spec.focus_elements,
            vec!["a1".to_string(), "b2".to_string()]
        );
    }
}
