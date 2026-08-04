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
}
