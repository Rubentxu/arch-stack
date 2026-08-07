//! Mermaid projector for `diagram project`.
//!
//! Projects graph elements and edges to Mermaid flowchart/sequence DSL.

use crate::diagram::project_selector::{ProjectSelector, ViewKind};
use crate::diagram::queries::{ElementRow, SemanticEdgeRow};

/// Project elements + edges to Mermaid DSL.
pub fn project(
    elements: &[ElementRow],
    edges: &[SemanticEdgeRow],
    selector: &ProjectSelector,
) -> String {
    let mut output = String::new();

    // Header — use flowchart TD for most views, sequence for sequence view
    match selector.view {
        ViewKind::Sequence => {
            output.push_str("sequenceDiagram\n");
        }
        _ => {
            output.push_str("flowchart TD\n");
        }
    }

    match selector.view {
        ViewKind::Class => project_class_view(&mut output, elements, edges),
        ViewKind::State => project_state_view(&mut output, elements, edges),
        ViewKind::UseCase => project_usecase_view(&mut output, elements, edges),
        ViewKind::C4Context | ViewKind::C4Container | ViewKind::C4Component => {
            project_c4_view(&mut output, elements, edges)
        }
        ViewKind::Sequence => project_sequence_view(&mut output, elements, edges),
    }

    output
}

fn project_class_view(output: &mut String, elements: &[ElementRow], edges: &[SemanticEdgeRow]) {
    let name_map: std::collections::HashMap<&str, &str> = elements
        .iter()
        .map(|e| (e.id.as_str(), e.current_name.as_str()))
        .collect();

    for element in elements {
        if element.kind_id == "uml.class" || element.kind_id == "uml.interface" {
            output.push_str(&format!(
                "    {}(\"{}\")\n",
                element.current_name, element.current_name
            ));
        }
    }

    for edge in edges {
        let Some(&src) = name_map.get(edge.source_id.as_str()) else {
            continue;
        };
        let Some(&tgt) = name_map.get(edge.target_id.as_str()) else {
            continue;
        };

        let arrow = match edge.predicate_id.as_str() {
            "uml.extends" => "-->",
            "uml.implements" => "-->",
            _ => "-->",
        };
        output.push_str(&format!("    {} {} {}\n", src, arrow, tgt));
    }
}

fn project_state_view(output: &mut String, elements: &[ElementRow], edges: &[SemanticEdgeRow]) {
    // Mermaid node IDs MUST be alphanumeric identifiers; bare `[Label]` syntax
    // is rejected by merman's parser (same bug class as the pre-M39 use case
    // view). We use the element's id as the node ID and the name as the label.
    //
    // Mermaid shape mapping for UML state view (M41):
    // - `uml.state` → `id([Name]):::state` (rectangle with state class)
    for element in elements {
        if element.kind_id == "uml.state" {
            output.push_str(&format!(
                "    {}([{}]):::state\n",
                element.id, element.current_name
            ));
        }
    }

    // Edges reference node IDs (not names).
    let known_ids: std::collections::HashSet<&str> =
        elements.iter().map(|e| e.id.as_str()).collect();
    for edge in edges {
        if edge.predicate_id != "behavior.source_state" {
            continue;
        }
        if !known_ids.contains(edge.source_id.as_str())
            || !known_ids.contains(edge.target_id.as_str())
        {
            continue;
        }
        output.push_str(&format!("    {} --> {}\n", edge.source_id, edge.target_id));
    }

    // Add state style if any states exist
    if elements.iter().any(|e| e.kind_id == "uml.state") {
        output.push_str("    classDef state fill:#f9f,stroke:#333,stroke-width:2px\n");
    }
}

fn project_usecase_view(output: &mut String, elements: &[ElementRow], edges: &[SemanticEdgeRow]) {
    // Mermaid node IDs MUST be alphanumeric identifiers; bare `(`Label)` syntax
    // is rejected by merman's parser. We use the element's id as the node ID
    // and the name as the label. (Pre-M39 the projection emitted bare `(Label)`
    // which never rendered — only the unit test that did substring matches
    // passed, masking the bug. The M39 e2e test catches this.)
    //
    // Mermaid shape mapping for UML use case view (M39):
    // - `uml.actor`   → `id(Name)` (rounded rect — closest approximation of
    //                    UML stick figure, which Mermaid cannot draw natively)
    // - `uml.use_case` → `id((Name))` (circle — closest approximation of UML ellipse)
    for element in elements {
        match element.kind_id.as_str() {
            "uml.actor" => {
                output.push_str(&format!("    {}({})\n", element.id, element.current_name));
            }
            "uml.use_case" => {
                output.push_str(&format!("    {}(({}))\n", element.id, element.current_name));
            }
            _ => {}
        }
    }

    // Edges reference node IDs (not names) — Mermaid `A --> B` requires both
    // sides to be node IDs declared earlier in the diagram.
    let known_ids: std::collections::HashSet<&str> =
        elements.iter().map(|e| e.id.as_str()).collect();
    for edge in edges {
        if edge.predicate_id != "usecase.participates_in" {
            continue;
        }
        if !known_ids.contains(edge.source_id.as_str())
            || !known_ids.contains(edge.target_id.as_str())
        {
            continue;
        }
        output.push_str(&format!("    {} --> {}\n", edge.source_id, edge.target_id));
    }
}

fn project_c4_view(output: &mut String, elements: &[ElementRow], edges: &[SemanticEdgeRow]) {
    // Mermaid node IDs MUST be alphanumeric identifiers; bare `[Label]` / `(Label)`
    // syntax is rejected by merman's parser (same bug class as the pre-M39 use case
    // view). We use the element's id as the node ID and the name as the label.
    //
    // Mermaid shape mapping for C4 view (M41):
    // - `c4.person` → `id(name)` (rounded rect — actor approximation)
    // - `c4.software_system | c4.container | c4.component`
    //                → `id([name])` (rectangle)
    for element in elements {
        match element.kind_id.as_str() {
            "c4.person" => {
                output.push_str(&format!("    {}({})\n", element.id, element.current_name));
            }
            "c4.software_system" | "c4.container" | "c4.component" => {
                output.push_str(&format!("    {}([{}])\n", element.id, element.current_name));
            }
            _ => {}
        }
    }

    // Edges reference node IDs (not names).
    let known_ids: std::collections::HashSet<&str> =
        elements.iter().map(|e| e.id.as_str()).collect();
    for edge in edges {
        let matches = edge.predicate_id == "core.uses" || edge.predicate_id == "core.depends_on";
        if !matches {
            continue;
        }
        if !known_ids.contains(edge.source_id.as_str())
            || !known_ids.contains(edge.target_id.as_str())
        {
            continue;
        }
        output.push_str(&format!("    {} --> {}\n", edge.source_id, edge.target_id));
    }
}

fn project_sequence_view(output: &mut String, elements: &[ElementRow], edges: &[SemanticEdgeRow]) {
    let name_map: std::collections::HashMap<&str, &str> = elements
        .iter()
        .map(|e| (e.id.as_str(), e.current_name.as_str()))
        .collect();

    for element in elements {
        if element.kind_id == "behavior.participant" {
            output.push_str(&format!("    participant {}\n", element.current_name));
        }
    }

    for edge in edges {
        let Some(&src) = name_map.get(edge.source_id.as_str()) else {
            continue;
        };
        let Some(&tgt) = name_map.get(edge.target_id.as_str()) else {
            continue;
        };

        if edge.predicate_id == "behavior.invokes" {
            // M45: optional message label from edge.props["label"].
            // Mermaid syntax: `A->>B: label`. When label is absent we keep
            // the bare arrow for backward compatibility.
            let label = edge.props.get("label").and_then(|v| v.as_str());
            match label {
                Some(l) if !l.is_empty() => {
                    output.push_str(&format!("    {}->>+{}: {}\n", src, tgt, l));
                }
                _ => {
                    output.push_str(&format!("    {}->>+{}\n", src, tgt));
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
    fn usecase_view_produces_valid_mermaid() {
        let elements = vec![
            make_element("e1", "uml.actor", "Customer", "uml"),
            make_element("e2", "uml.use_case", "PlaceOrder", "uml"),
        ];
        let edges = vec![make_edge("r1", "usecase.participates_in", "e1", "e2")];
        let selector = ProjectSelector::parse("usecase:*").unwrap();

        let dsl = project(&elements, &edges, &selector);

        assert!(dsl.contains("flowchart TD"));
        // M39: actor = rounded rect, use case = circle (UML ellipse approximation)
        // Nodes use the element id as Mermaid node id (Mermaid requires a
        // node-id prefix — bare `(Label)` is rejected by merman).
        assert!(
            dsl.contains("e1(Customer)"),
            "actor should use id(Name) rounded rect; got: {dsl}"
        );
        assert!(
            dsl.contains("e2((PlaceOrder))"),
            "use case should use id((Name)) circle; got: {dsl}"
        );
        // Edges reference node IDs, not names.
        assert!(
            dsl.contains("e1 --> e2"),
            "edge should reference node ids; got: {dsl}"
        );
    }

    /// M39: ensure actors and use cases have visually distinct shapes even
    /// when names contain shared substrings (regression test for the Mermaid
    /// substring collision between `(Customer)` and `((PlaceOrder))`).
    #[test]
    fn usecase_view_actor_and_usecase_shapes_are_distinct() {
        let elements = vec![
            make_element("e1", "uml.actor", "Admin", "uml"),
            make_element("e2", "uml.use_case", "ManageUsers", "uml"),
        ];
        let edges = vec![make_edge("r1", "usecase.participates_in", "e1", "e2")];
        let selector = ProjectSelector::parse("usecase:*").unwrap();

        let dsl = project(&elements, &edges, &selector);

        // Use case MUST have the double-paren marker (Mermaid circle).
        assert!(
            dsl.contains("e2((ManageUsers))"),
            "use case missing id((…)) circle marker; got: {dsl}"
        );
        // Actor MUST use single-paren rounded rect.
        assert!(
            dsl.contains("e1(Admin)"),
            "actor should use id(Name) rounded rect; got: {dsl}"
        );
        // The use case must NOT appear with only single-paren markers around it.
        assert!(
            !dsl.contains("e2(ManageUsers)\n"),
            "use case must NOT be emitted as single-paren rounded rect; got: {dsl}"
        );
    }

    #[test]
    fn empty_graph_produces_minimal_mermaid() {
        let elements = vec![];
        let edges = vec![];
        let selector = ProjectSelector::parse("class:*").unwrap();

        let dsl = project(&elements, &edges, &selector);

        assert!(dsl.contains("flowchart TD"));
    }

    /// M41: state machine view emits `id([Name]):::state` (rectangle with
    /// class assignment) and edges reference node IDs. Pre-M41 bare
    /// `[Name]:::state` was rejected by merman.
    #[test]
    fn state_view_produces_valid_mermaid() {
        let elements = vec![
            make_element("e1", "uml.state", "Idle", "uml"),
            make_element("e2", "uml.state", "Active", "uml"),
        ];
        let edges = vec![make_edge("r1", "behavior.source_state", "e1", "e2")];
        let selector = ProjectSelector::parse("state:*").unwrap();

        let dsl = project(&elements, &edges, &selector);

        assert!(dsl.contains("flowchart TD"));
        assert!(
            dsl.contains("e1([Idle]):::state"),
            "state should use id([Name]):::state rectangle; got: {dsl}"
        );
        assert!(
            dsl.contains("e2([Active]):::state"),
            "second state missing rectangle marker; got: {dsl}"
        );
        // Edges reference node IDs.
        assert!(
            dsl.contains("e1 --> e2"),
            "edge should reference node ids; got: {dsl}"
        );
        // classDef style is appended.
        assert!(
            dsl.contains("classDef state"),
            "classDef state should be present; got: {dsl}"
        );
    }

    /// M45: sequence view emits `A->>B: label` when edge.props["label"] is set.
    #[test]
    fn sequence_view_with_edge_label() {
        let mut props = serde_json::Map::new();
        props.insert(
            "label".to_string(),
            serde_json::Value::String("placeOrder()".to_string()),
        );
        let edges = vec![SemanticEdgeRow {
            relation_id: "r1".to_string(),
            predicate_id: "behavior.invokes".to_string(),
            source_id: "e1".to_string(),
            target_id: "e2".to_string(),
            order_key: "1".to_string(),
            props,
        }];
        let elements = vec![
            make_element("e1", "behavior.participant", "Client", "behavior"),
            make_element("e2", "behavior.participant", "Server", "behavior"),
        ];
        let selector = ProjectSelector::parse("sequence:checkout").unwrap();

        let dsl = project(&elements, &edges, &selector);
        assert!(
            dsl.contains("Client->>+Server: placeOrder()"),
            "labeled sequence arrow should include ': label'; got:\n{dsl}"
        );
    }

    /// M45: sequence view without label preserves bare arrow (backward-compat).
    #[test]
    fn sequence_view_without_edge_label() {
        let edges = vec![SemanticEdgeRow {
            relation_id: "r1".to_string(),
            predicate_id: "behavior.invokes".to_string(),
            source_id: "e1".to_string(),
            target_id: "e2".to_string(),
            order_key: "1".to_string(),
            props: serde_json::Map::new(),
        }];
        let elements = vec![
            make_element("e1", "behavior.participant", "Client", "behavior"),
            make_element("e2", "behavior.participant", "Server", "behavior"),
        ];
        let selector = ProjectSelector::parse("sequence:checkout").unwrap();

        let dsl = project(&elements, &edges, &selector);
        assert!(
            dsl.contains("Client->>+Server\n"),
            "unlabeled sequence arrow should be bare; got:\n{dsl}"
        );
        assert!(
            !dsl.contains("Client->>+Server:"),
            "unlabeled arrow must NOT contain a label separator; got:\n{dsl}"
        );
    }

    /// M45: empty label string is treated as no label.
    #[test]
    fn sequence_view_with_empty_label_treated_as_unlabeled() {
        let mut props = serde_json::Map::new();
        props.insert(
            "label".to_string(),
            serde_json::Value::String(String::new()),
        );
        let edges = vec![SemanticEdgeRow {
            relation_id: "r1".to_string(),
            predicate_id: "behavior.invokes".to_string(),
            source_id: "e1".to_string(),
            target_id: "e2".to_string(),
            order_key: "1".to_string(),
            props,
        }];
        let elements = vec![
            make_element("e1", "behavior.participant", "A", "behavior"),
            make_element("e2", "behavior.participant", "B", "behavior"),
        ];
        let selector = ProjectSelector::parse("sequence:test").unwrap();

        let dsl = project(&elements, &edges, &selector);
        assert!(
            !dsl.contains("A->>+B:"),
            "empty label must NOT emit a label separator; got:\n{dsl}"
        );
    }

    /// M41: C4 container view emits `id(name)` for persons and `id([name])` for
    /// systems/containers/components, with edges referencing node IDs.
    #[test]
    fn c4_container_view_produces_valid_mermaid() {
        let elements = vec![
            make_element("e1", "c4.person", "Customer", "c4"),
            make_element("e2", "c4.container", "WebApp", "c4"),
            make_element("e3", "c4.container", "Database", "c4"),
        ];
        let edges = vec![
            make_edge("r1", "core.uses", "e1", "e2"),
            make_edge("r2", "core.uses", "e2", "e3"),
        ];
        let selector = ProjectSelector::parse("c4-container:orders").unwrap();

        let dsl = project(&elements, &edges, &selector);

        assert!(dsl.contains("flowchart TD"));
        assert!(
            dsl.contains("e1(Customer)"),
            "person should use id(Name) rounded rect; got: {dsl}"
        );
        assert!(
            dsl.contains("e2([WebApp])"),
            "container should use id([Name]) rectangle; got: {dsl}"
        );
        assert!(
            dsl.contains("e3([Database])"),
            "second container missing rectangle marker; got: {dsl}"
        );
        assert!(
            dsl.contains("e1 --> e2"),
            "edge should reference node ids; got: {dsl}"
        );
        assert!(dsl.contains("e2 --> e3"), "second edge missing; got: {dsl}");
    }

    #[test]
    fn deterministic_order() {
        let elements = vec![
            make_element("e3", "uml.actor", "Zebra", "uml"),
            make_element("e1", "uml.actor", "Alpha", "uml"),
        ];
        let edges = vec![];
        let selector = ProjectSelector::parse("usecase:*").unwrap();

        let dsl1 = project(&elements, &edges, &selector);
        let dsl2 = project(&elements, &edges, &selector);

        assert_eq!(dsl1, dsl2, "output must be deterministic");
    }
}
