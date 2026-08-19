//! End-to-end test for the use case view pipeline (M39).
//!
//! Proves the full chain: project a `usecase:*` bundle to Mermaid DSL →
//! render the DSL via merman → assert valid SVG with actor/use case names
//! present.
//!
//! Pre-M39, no integration test asserted that the use case view produced
//! a renderable SVG. This is the regression test that locks in the M39
//! mermaid shape change (`(name)` for actors, `((name))` for use cases).

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

/// M39: the use case bundle, projected to Mermaid, MUST render to a
/// valid SVG with both the actor and use case names visible.
#[test]
fn usecase_view_renders_to_svg_with_names_visible() {
    let elements = vec![
        make_element("e1", "uml.actor", "Customer", "uml"),
        make_element("e2", "uml.use_case", "PlaceOrder", "uml"),
        make_element("e3", "uml.use_case", "CancelOrder", "uml"),
    ];
    let edges = vec![
        make_edge("r1", "usecase.participates_in", "e1", "e2"),
        make_edge("r2", "usecase.participates_in", "e1", "e3"),
    ];
    let selector = ProjectSelector::parse("usecase:*").unwrap();

    let dsl = project_mermaid(&elements, &edges, &selector);
    assert!(
        dsl.contains("((PlaceOrder))"),
        "projected DSL must use double-paren for use cases; got:\n{dsl}"
    );
    assert!(
        dsl.contains("((CancelOrder))"),
        "second use case also missing circle marker; got:\n{dsl}"
    );

    let svg = render_mermaid::render(&dsl)
        .unwrap_or_else(|e| panic!("merman render failed for DSL:\n---\n{dsl}\n---\nerror: {e:?}"));

    assert!(
        svg.starts_with("<?xml") || svg.starts_with("<svg"),
        "SVG should start with xml/svg; got first 80 chars: {}",
        svg.chars().take(80).collect::<String>()
    );
    assert!(
        svg.contains("<svg"),
        "SVG must contain <svg root; got len {}",
        svg.len()
    );
    // Merman renders text labels as <text> nodes; both names MUST appear.
    assert!(
        svg.contains("Customer"),
        "actor name 'Customer' must appear in rendered SVG"
    );
    assert!(
        svg.contains("PlaceOrder"),
        "use case name 'PlaceOrder' must appear in rendered SVG"
    );
    assert!(
        svg.contains("CancelOrder"),
        "second use case name 'CancelOrder' must appear in rendered SVG"
    );
}

/// M39: empty use case bundle should still render a valid (minimal) SVG.
#[test]
fn usecase_view_empty_bundle_renders_to_svg() {
    let elements: Vec<ElementRow> = Vec::new();
    let edges: Vec<SemanticEdgeRow> = Vec::new();
    let selector = ProjectSelector::parse("usecase:*").unwrap();

    let dsl = project_mermaid(&elements, &edges, &selector);
    let svg = render_mermaid::render(&dsl).expect("merman should render empty bundle");

    assert!(
        svg.contains("<svg"),
        "empty bundle SVG must still contain <svg; got len {}",
        svg.len()
    );
}
