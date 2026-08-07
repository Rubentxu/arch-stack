//! End-to-end test for the state machine view pipeline (M41).
//!
//! Proves the full chain: project a `state:*` bundle → Mermaid DSL →
//! render via merman → assert valid SVG with state names visible.
//!
//! Mirrors the M39 `usecase_view_e2e.rs` pattern. Pre-M41 the bare
//! `[Name]:::state` syntax was rejected by merman; M41 fixes it by emitting
//! `id([Name]):::state`.

use archctl::diagram::project::mermaid::project as project_mermaid;
use archctl::diagram::project_selector::ProjectSelector;
use archctl::diagram::queries::{ElementRow, SemanticEdgeRow};
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

/// M41: state bundle projects to merman-parseable Mermaid and renders to SVG.
#[test]
fn state_view_renders_to_svg_with_names_visible() {
    let elements = vec![
        make_element("e1", "uml.state", "Idle", "uml"),
        make_element("e2", "uml.state", "Active", "uml"),
        make_element("e3", "uml.state", "Suspended", "uml"),
    ];
    let edges = vec![
        make_edge("r1", "behavior.source_state", "e1", "e2"),
        make_edge("r2", "behavior.source_state", "e2", "e3"),
    ];
    let selector = ProjectSelector::parse("state:*").unwrap();

    let dsl = project_mermaid(&elements, &edges, &selector);
    assert!(
        dsl.contains("e1([Idle]):::state"),
        "state projection must use id([Name]):::state; got:\n{dsl}"
    );

    let svg = render_mermaid::render(&dsl).unwrap_or_else(|e| {
        panic!("merman render failed for state DSL:\n---\n{dsl}\n---\nerror: {e:?}")
    });

    assert!(
        svg.contains("<svg"),
        "SVG must contain <svg root; got len {}",
        svg.len()
    );
    assert!(
        svg.contains("Idle"),
        "state 'Idle' must appear in rendered SVG"
    );
    assert!(
        svg.contains("Active"),
        "state 'Active' must appear in rendered SVG"
    );
    assert!(
        svg.contains("Suspended"),
        "state 'Suspended' must appear in rendered SVG"
    );
}

/// M41: empty state bundle still renders valid (minimal) SVG.
#[test]
fn state_view_empty_bundle_renders_to_svg() {
    let elements: Vec<ElementRow> = Vec::new();
    let edges: Vec<SemanticEdgeRow> = Vec::new();
    let selector = ProjectSelector::parse("state:*").unwrap();

    let dsl = project_mermaid(&elements, &edges, &selector);
    let svg = render_mermaid::render(&dsl).expect("merman should render empty bundle");

    assert!(
        svg.contains("<svg"),
        "empty bundle SVG must contain <svg; got len {}",
        svg.len()
    );
}
