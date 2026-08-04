//! Structurizr DSL projector for `diagram project`.
//!
//! Projects C4 elements and edges to Structurizr DSL format.

use crate::diagram::project_selector::{ProjectSelector, ViewKind};
use crate::diagram::queries::{ElementRow, SemanticEdgeRow};

/// Project elements + edges to Structurizr DSL.
pub fn project(
    elements: &[ElementRow],
    edges: &[SemanticEdgeRow],
    selector: &ProjectSelector,
) -> String {
    let mut output = String::new();

    output.push_str("workspace {\n");
    output.push_str("    model {\n");

    // Model section
    match selector.view {
        ViewKind::C4Context => {
            for element in elements {
                match element.kind_id.as_str() {
                    "c4.person" => {
                        output.push_str(&format!(
                            "        person \"{}\"\n",
                            escape(&element.current_name)
                        ));
                    }
                    "c4.software_system" => {
                        output.push_str(&format!(
                            "        softwareSystem \"{}\"\n",
                            escape(&element.current_name)
                        ));
                    }
                    _ => {}
                }
            }
        }
        ViewKind::C4Container | ViewKind::C4Component => {
            for element in elements {
                match element.kind_id.as_str() {
                    "c4.container" | "c4.component" => {
                        let container_type = if element.kind_id == "c4.container" {
                            "container"
                        } else {
                            "component"
                        };
                        output.push_str(&format!(
                            "        {} \"{}\"\n",
                            container_type,
                            escape(&element.current_name)
                        ));
                    }
                    _ => {}
                }
            }
        }
        _ => {
            // For non-C4 views, emit whatever we have
            for element in elements {
                output.push_str(&format!(
                    "        softwareSystem \"{}\"\n",
                    escape(&element.current_name)
                ));
            }
        }
    }

    // Relationships
    let name_map: std::collections::HashMap<&str, &str> = elements
        .iter()
        .map(|e| (e.id.as_str(), e.current_name.as_str()))
        .collect();

    for edge in edges {
        let Some(&src) = name_map.get(edge.source_id.as_str()) else {
            continue;
        };
        let Some(&tgt) = name_map.get(edge.target_id.as_str()) else {
            continue;
        };

        // Get predicate description from props if available
        let desc = edge
            .props
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if desc.is_empty() {
            output.push_str(&format!("        {} -> {}\n", escape(src), escape(tgt)));
        } else {
            output.push_str(&format!(
                "        {} -> {} \"{}\"\n",
                escape(src),
                escape(tgt),
                escape(desc)
            ));
        }
    }

    output.push_str("    }\n");
    output.push_str("    views {\n");

    // Default view
    match selector.view {
        ViewKind::C4Context => {
            output.push_str("        systemContext * {\n");
            output.push_str("            include *\n");
            output.push_str("        }\n");
        }
        ViewKind::C4Container => {
            output.push_str("        container * {\n");
            output.push_str("            include *\n");
            output.push_str("        }\n");
        }
        ViewKind::C4Component => {
            output.push_str("        component * {\n");
            output.push_str("            include *\n");
            output.push_str("        }\n");
        }
        _ => {
            output.push_str("        systemContext * {\n");
            output.push_str("            include *\n");
            output.push_str("        }\n");
        }
    }

    output.push_str("    }\n");
    output.push_str("}\n");

    output
}

fn escape(s: &str) -> String {
    // Structurizr DSL uses double-quotes; escape backslashes and double-quotes
    s.replace('\\', "\\\\").replace('"', "\\\"")
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
    fn c4_context_produces_valid_structurizr() {
        let elements = vec![
            make_element("e1", "c4.person", "Customer", "c4"),
            make_element("e2", "c4.software_system", "Orders", "c4"),
        ];
        let edges = vec![make_edge("r1", "core.uses", "e1", "e2")];
        let selector = ProjectSelector::parse("c4-context:*").unwrap();

        let dsl = project(&elements, &edges, &selector);

        assert!(dsl.contains("workspace {"));
        assert!(dsl.contains("model {"));
        assert!(dsl.contains("person \"Customer\""));
        assert!(dsl.contains("softwareSystem \"Orders\""));
        assert!(dsl.contains("Customer -> Orders"));
        assert!(dsl.contains("views {"));
    }

    #[test]
    fn c4_container_produces_valid_structurizr() {
        let elements = vec![
            make_element("e1", "c4.container", "API", "c4"),
            make_element("e2", "c4.container", "DB", "c4"),
        ];
        let edges = vec![make_edge("r1", "core.uses", "e1", "e2")];
        let selector = ProjectSelector::parse("c4-container:*").unwrap();

        let dsl = project(&elements, &edges, &selector);

        assert!(dsl.contains("container \"API\""));
        assert!(dsl.contains("container \"DB\""));
        assert!(dsl.contains("API -> DB"));
    }

    #[test]
    fn empty_graph_produces_minimal_structurizr() {
        let elements = vec![];
        let edges = vec![];
        let selector = ProjectSelector::parse("c4-context:*").unwrap();

        let dsl = project(&elements, &edges, &selector);

        assert!(dsl.contains("workspace {"));
        assert!(dsl.contains("model {"));
        assert!(dsl.contains("views {"));
    }

    #[test]
    fn escape_handles_quotes() {
        assert_eq!(escape("foo\"bar"), "foo\\\"bar");
        assert_eq!(escape("a\\b"), "a\\\\b");
    }
}
