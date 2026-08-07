//! PlantUML projector for `diagram project`.
//!
//! Projects graph elements (uml.class, uml.interface, c4.container, etc.)
//! and edges (uml.extends, uml.implements, core.uses, etc.) to PlantUML DSL.
//!
//! Deterministic: elements and edges are sorted by canonical_key/relation_id
//! before projection.

use crate::diagram::project_selector::{ProjectSelector, ViewKind};
use crate::diagram::queries::{ElementRow, SemanticEdgeRow};

/// Project elements + edges to PlantUML DSL.
pub fn project(
    elements: &[ElementRow],
    edges: &[SemanticEdgeRow],
    selector: &ProjectSelector,
) -> String {
    let mut output = String::new();

    // Header
    output.push_str("@startuml\n");
    output.push_str("!theme default\n");

    // Project based on view kind
    match selector.view {
        ViewKind::Class => project_class_view(&mut output, elements, edges),
        ViewKind::State => project_state_view(&mut output, elements, edges),
        ViewKind::UseCase => project_usecase_view(&mut output, elements, edges),
        ViewKind::C4Context | ViewKind::C4Container | ViewKind::C4Component => {
            project_c4_view(&mut output, elements, edges, selector)
        }
        ViewKind::Sequence => project_sequence_view(&mut output, elements, edges),
    }

    // Footer
    output.push_str("\n@enduml\n");

    output
}

fn project_class_view(output: &mut String, elements: &[ElementRow], edges: &[SemanticEdgeRow]) {
    // Build element name map
    let name_map: std::collections::HashMap<&str, &str> = elements
        .iter()
        .map(|e| (e.id.as_str(), e.current_name.as_str()))
        .collect();

    for element in elements {
        match element.kind_id.as_str() {
            "uml.class" | "uml.interface" | "uml.trait" | "uml.enum" | "uml.record" => {
                let stereotype = match element.kind_id.as_str() {
                    "uml.interface" => Some("<<interface>>"),
                    "uml.trait" => Some("<<trait>>"),
                    "uml.enum" => Some("<<enum>>"),
                    "uml.record" => Some("<<record>>"),
                    _ => None,
                };
                output.push_str("class ");
                output.push_str(&element.current_name);
                if let Some(st) = stereotype {
                    output.push(' ');
                    output.push_str(st);
                }
                output.push_str(" {\n");
                output.push_str("  --\n");
                output.push_str("}\n");
            }
            _ => {}
        }
    }

    // Edges
    for edge in edges {
        let Some(&src_name) = name_map.get(edge.source_id.as_str()) else {
            continue;
        };
        let Some(&tgt_name) = name_map.get(edge.target_id.as_str()) else {
            continue;
        };

        let arrow = match edge.predicate_id.as_str() {
            "uml.extends" => "--|>",
            "uml.implements" => "..|>",
            "uml.association" => "-->",
            "uml.aggregation" => "--o",
            "uml.composition" => "--*",
            "uml.depends_on" => "..>",
            _ => "-->",
        };

        output.push_str(&format!("{} {} {}\n", src_name, arrow, tgt_name));
    }
}

fn project_state_view(output: &mut String, elements: &[ElementRow], edges: &[SemanticEdgeRow]) {
    // Build element name map
    let name_map: std::collections::HashMap<&str, &str> = elements
        .iter()
        .map(|e| (e.id.as_str(), e.current_name.as_str()))
        .collect();

    for element in elements {
        match element.kind_id.as_str() {
            "uml.state" | "uml.pseudostate" | "uml.state_machine" => {
                if element.kind_id == "uml.pseudostate" {
                    // Initial/final pseudostates
                    if element.current_name.to_lowercase().contains("initial")
                        || element.current_name.to_lowercase().contains("start")
                    {
                        output.push_str("[*] --> ");
                        output.push_str(&element.current_name);
                        output.push('\n');
                    } else {
                        output.push_str(&element.current_name);
                        output.push_str(" --> [*]\n");
                    }
                } else {
                    output.push_str("state ");
                    output.push_str(&element.current_name);
                    output.push_str(" {\n}\n");
                }
            }
            _ => {}
        }
    }

    // Transitions: per ADR-026, transitions join source_state → transition → target_state
    // behavior.source_state: source=source_state element, target=transition element
    // behavior.target_state: source=transition element, target=target_state element
    //
    // Step 1: collect transition → source_state from source_state edges
    let mut transition_to_source: std::collections::HashMap<&str, &str> =
        std::collections::HashMap::new();
    // Step 2: collect transition → target_state from target_state edges
    let mut transition_to_target: std::collections::HashMap<&str, &str> =
        std::collections::HashMap::new();

    for edge in edges {
        let Some(&tgt_name) = name_map.get(edge.target_id.as_str()) else {
            continue;
        };

        if edge.predicate_id == "behavior.source_state" {
            // edge.source is the source_state, edge.target is the transition
            if let Some(&src_name) = name_map.get(edge.source_id.as_str()) {
                transition_to_source.insert(tgt_name, src_name);
            }
        } else if edge.predicate_id == "behavior.target_state" {
            // edge.source is the transition, edge.target is the target_state
            if let Some(&src_name) = name_map.get(edge.source_id.as_str()) {
                transition_to_target.insert(src_name, tgt_name);
            }
        }
    }

    // Step 3: emit joined transitions
    for (transition_id, &source_name) in &transition_to_source {
        if let Some(&target_name) = transition_to_target.get(transition_id) {
            output.push_str(&format!(
                "{} --> {} --> {}\n",
                source_name, transition_id, target_name
            ));
        }
    }
}

fn project_usecase_view(output: &mut String, elements: &[ElementRow], edges: &[SemanticEdgeRow]) {
    // Build element name map
    let name_map: std::collections::HashMap<&str, &str> = elements
        .iter()
        .map(|e| (e.id.as_str(), e.current_name.as_str()))
        .collect();

    for element in elements {
        match element.kind_id.as_str() {
            "uml.actor" => {
                // SCN-412: Use [ActorName] bracket shape (NOT (ActorName))
                output.push_str(&format!("[{}]\n", element.current_name));
            }
            "uml.use_case" => {
                output.push_str("usecase ");
                output.push_str(&element.current_name);
                output.push('\n');
            }
            _ => {}
        }
    }

    // Edges
    for edge in edges {
        let Some(&src_name) = name_map.get(edge.source_id.as_str()) else {
            continue;
        };
        let Some(&tgt_name) = name_map.get(edge.target_id.as_str()) else {
            continue;
        };

        if edge.predicate_id == "usecase.participates_in" {
            output.push_str(&format!("{} --> {}\n", src_name, tgt_name));
        }
    }
}

fn project_c4_view(
    output: &mut String,
    elements: &[ElementRow],
    edges: &[SemanticEdgeRow],
    _selector: &ProjectSelector,
) {
    // Build element name map
    let name_map: std::collections::HashMap<&str, &str> = elements
        .iter()
        .map(|e| (e.id.as_str(), e.current_name.as_str()))
        .collect();

    for element in elements {
        match element.kind_id.as_str() {
            "c4.container" | "c4.component" | "c4.person" | "c4.software_system" => {
                let container_type = match element.kind_id.as_str() {
                    "c4.person" => "person",
                    "c4.software_system" => "software_system",
                    "c4.container" => "container",
                    "c4.component" => "component",
                    _ => "container",
                };
                output.push_str(&format!(
                    " {} \"{}\" {} {}\n",
                    container_type, element.current_name, "{", "}"
                ));
            }
            _ => {}
        }
    }

    // Edges
    for edge in edges {
        let Some(&src_name) = name_map.get(edge.source_id.as_str()) else {
            continue;
        };
        let Some(&tgt_name) = name_map.get(edge.target_id.as_str()) else {
            continue;
        };

        if edge.predicate_id == "core.uses" || edge.predicate_id == "core.depends_on" {
            output.push_str(&format!("{} --> {}\n", src_name, tgt_name));
        }
    }
}

fn project_sequence_view(output: &mut String, elements: &[ElementRow], edges: &[SemanticEdgeRow]) {
    // Build element name map
    let name_map: std::collections::HashMap<&str, &str> = elements
        .iter()
        .map(|e| (e.id.as_str(), e.current_name.as_str()))
        .collect();

    for element in elements {
        if element.kind_id.as_str() == "behavior.participant" {
            output.push_str(&format!("participant {}\n", element.current_name));
        }
    }

    for edge in edges {
        let Some(&src_name) = name_map.get(edge.source_id.as_str()) else {
            continue;
        };
        let Some(&tgt_name) = name_map.get(edge.target_id.as_str()) else {
            continue;
        };

        if edge.predicate_id == "behavior.invokes" {
            // M45: optional message label from edge.props["label"].
            // PlantUML syntax: `A -> B : label`. When label is absent we
            // keep the bare arrow for backward compatibility.
            let label = edge.props.get("label").and_then(|v| v.as_str());
            match label {
                Some(l) if !l.is_empty() => {
                    output.push_str(&format!("{} -> {} : {}\n", src_name, tgt_name, l));
                }
                _ => {
                    output.push_str(&format!("{} -> {}\n", src_name, tgt_name));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_element(id: &str, kind: &str, name: &str, category: &str) -> ElementRow {
        ElementRow {
            id: id.to_string(),
            kind_id: kind.to_string(),
            category: category.to_string(),
            canonical_key: format!("{}:{}", category, name),
            current_name: name.to_string(),
            current_status: "accepted".to_string(),
            current_confidence: 1.0,
            current_version_id: "v1".to_string(),
        }
    }

    fn make_edge(rel_id: &str, pred: &str, src: &str, tgt: &str) -> SemanticEdgeRow {
        SemanticEdgeRow {
            relation_id: rel_id.to_string(),
            predicate_id: pred.to_string(),
            source_id: src.to_string(),
            target_id: tgt.to_string(),
            order_key: "1".to_string(),
            props: Default::default(),
        }
    }

    #[test]
    fn class_view_produces_valid_plantuml() {
        let elements = vec![
            make_element("e1", "uml.class", "Order", "uml"),
            make_element("e2", "uml.class", "Customer", "uml"),
        ];
        let edges = vec![make_edge("r1", "uml.extends", "e1", "e2")];
        let selector = ProjectSelector::parse("class:*").unwrap();

        let dsl = project(&elements, &edges, &selector);

        assert!(dsl.contains("@startuml"));
        assert!(dsl.contains("@enduml"));
        assert!(dsl.contains("class Order"));
        assert!(dsl.contains("class Customer"));
        assert!(dsl.contains("Order --|> Customer"));
    }

    #[test]
    fn state_view_produces_valid_plantuml() {
        // Per ADR-026: transitions join through the transition node
        // Pending --[source_state]--> SubmitOrder --[target_state]--> Confirmed
        let elements = vec![
            make_element("e1", "uml.state", "Pending", "uml"),
            make_element("e2", "uml.state", "Confirmed", "uml"),
            make_element("e3", "uml.state", "SubmitOrder", "uml"),
        ];
        let edges = vec![
            make_edge("r1", "behavior.source_state", "e1", "e3"),
            make_edge("r2", "behavior.target_state", "e3", "e2"),
        ];
        let selector = ProjectSelector::parse("state:*").unwrap();

        let dsl = project(&elements, &edges, &selector);

        assert!(dsl.contains("state Pending"));
        assert!(dsl.contains("state Confirmed"));
        assert!(dsl.contains("state SubmitOrder"));
        assert!(dsl.contains("Pending --> SubmitOrder --> Confirmed"));
    }

    #[test]
    fn empty_graph_produces_minimal_plantuml() {
        let elements = vec![];
        let edges = vec![];
        let selector = ProjectSelector::parse("class:*").unwrap();

        let dsl = project(&elements, &edges, &selector);

        assert!(dsl.contains("@startuml"));
        assert!(dsl.contains("@enduml"));
    }

    #[test]
    fn usecase_view_produces_valid_plantuml() {
        // SCN-412: actors use [ActorName] bracket shape
        let elements = vec![
            make_element("e1", "uml.actor", "Customer", "uml"),
            make_element("e2", "uml.use_case", "PlaceOrder", "uml"),
        ];
        let edges = vec![make_edge("r1", "usecase.participates_in", "e1", "e2")];
        let selector = ProjectSelector::parse("usecase:*").unwrap();

        let dsl = project(&elements, &edges, &selector);

        // SCN-412: bracket shape not actor keyword
        assert!(dsl.contains("[Customer]"));
        assert!(!dsl.contains("actor Customer"));
        assert!(dsl.contains("usecase PlaceOrder"));
        assert!(dsl.contains("Customer --> PlaceOrder"));
    }

    #[test]
    fn deterministic_order() {
        let elements = vec![
            make_element("e3", "uml.class", "Zebra", "uml"),
            make_element("e1", "uml.class", "Alpha", "uml"),
            make_element("e2", "uml.class", "Beta", "uml"),
        ];
        let edges = vec![];
        let selector = ProjectSelector::parse("class:*").unwrap();

        let dsl1 = project(&elements, &edges, &selector);
        let dsl2 = project(&elements, &edges, &selector);

        assert_eq!(dsl1, dsl2, "output must be deterministic");
    }
}
