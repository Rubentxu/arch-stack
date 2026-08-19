//! End-to-end test for the sequence view pipeline (M45).
//!
//! Proves the full chain: project a `sequence:*` bundle → Mermaid DSL →
//! render via merman → assert valid SVG with participants and labels visible.
//!
//! Sequence diagrams without message labels are useless. M45 adds label
//! support via edge.props["label"]. This test verifies labels flow through
//! to the rendered SVG.

use archctl::diagram::project::mermaid::project as project_mermaid;
use archctl::diagram::project_selector::ProjectSelector;
use archctl::graph::{ElementRow, SemanticEdgeRow};
use archctl::render::mermaid as render_mermaid;

fn make_element(id: &str, kind: &str, name: &str, category: &str) -> ElementRow {
    ElementRow {
        id: id.to_string(),
        kind_id: kind.to_string(),
        category: category.to_string(),
        canonical_key: format!("{category}:{name}"),
        current_name: name.to_string(),
        current_status: "accepted".to_string(),
        current_confidence: 1.0,
        current_version_id: "v1".to_string(),
    }
}

fn make_edge_with_label(
    rel_id: &str,
    pred: &str,
    src: &str,
    tgt: &str,
    label: &str,
) -> SemanticEdgeRow {
    let mut props = serde_json::Map::new();
    props.insert(
        "label".to_string(),
        serde_json::Value::String(label.to_string()),
    );
    SemanticEdgeRow {
        relation_id: rel_id.to_string(),
        predicate_id: pred.to_string(),
        source_id: src.to_string(),
        target_id: tgt.to_string(),
        order_key: "1".to_string(),
        props,
    }
}

/// M45: labeled sequence edge flows through to rendered SVG.
#[test]
fn sequence_view_with_label_renders_to_svg() {
    let elements = vec![
        make_element("e1", "behavior.participant", "Client", "behavior"),
        make_element("e2", "behavior.participant", "Server", "behavior"),
    ];
    let edges = vec![make_edge_with_label(
        "r1",
        "behavior.invokes",
        "e1",
        "e2",
        "placeOrder()",
    )];
    let selector = ProjectSelector::parse("sequence:checkout").unwrap();

    let dsl = project_mermaid(&elements, &edges, &selector);
    assert!(
        dsl.contains("Client->>+Server: placeOrder()"),
        "DSL should contain labeled arrow; got:\n{dsl}"
    );

    let svg = render_mermaid::render(&dsl)
        .unwrap_or_else(|e| panic!("merman should render labeled sequence: {e:?}"));

    assert!(
        svg.contains("<svg"),
        "SVG must contain <svg root; got len {}",
        svg.len()
    );
    assert!(
        svg.contains("Client"),
        "participant 'Client' must appear in SVG"
    );
    assert!(
        svg.contains("Server"),
        "participant 'Server' must appear in SVG"
    );
    assert!(
        svg.contains("placeOrder()"),
        "label 'placeOrder()' must appear in rendered SVG"
    );
}

/// M45: sequence diagram with one participant and no edges renders to valid SVG.
#[test]
fn sequence_view_single_participant_renders_to_svg() {
    let elements = vec![make_element(
        "e1",
        "behavior.participant",
        "Lone",
        "behavior",
    )];
    let edges: Vec<SemanticEdgeRow> = vec![];
    let selector = ProjectSelector::parse("sequence:lone").unwrap();

    let dsl = project_mermaid(&elements, &edges, &selector);
    let svg = render_mermaid::render(&dsl).expect("merman should render single-participant");

    assert!(
        svg.contains("<svg"),
        "single-participant SVG must contain <svg"
    );
    assert!(svg.contains("Lone"), "participant 'Lone' must appear");
}
