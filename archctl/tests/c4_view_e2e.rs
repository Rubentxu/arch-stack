//! End-to-end test for the C4 container view pipeline (M41).
//!
//! Proves the full chain: project a `c4-container:*` bundle → Mermaid DSL →
//! render via merman → assert valid SVG with container + person names visible.
//!
//! Mirrors the M39 `usecase_view_e2e.rs` pattern. Pre-M41 the bare `[Name]`
//! syntax for containers was rejected by merman; M41 fixes it by emitting
//! `id([Name])`.

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

/// M41: C4 container bundle projects to merman-parseable Mermaid and renders
/// to SVG with the person + container names visible.
#[test]
fn c4_container_view_renders_to_svg_with_names_visible() {
    let elements = vec![
        make_element("e1", "c4.person", "Customer", "c4"),
        make_element("e2", "c4.software_system", "Orders", "c4"),
        make_element("e3", "c4.container", "WebApp", "c4"),
        make_element("e4", "c4.container", "Database", "c4"),
    ];
    let edges = vec![
        make_edge("r1", "core.uses", "e1", "e2"),
        make_edge("r2", "core.uses", "e2", "e3"),
        make_edge("r3", "core.depends_on", "e3", "e4"),
    ];
    let selector = ProjectSelector::parse("c4-container:orders").unwrap();

    let dsl = project_mermaid(&elements, &edges, &selector);
    assert!(
        dsl.contains("e1(Customer)"),
        "person should use id(Name) rounded rect; got:\n{dsl}"
    );
    assert!(
        dsl.contains("e2([Orders])"),
        "software_system should use id([Name]) rectangle; got:\n{dsl}"
    );
    assert!(
        dsl.contains("e3([WebApp])"),
        "container should use id([Name]) rectangle; got:\n{dsl}"
    );

    let svg = render_mermaid::render(&dsl).unwrap_or_else(|e| {
        panic!("merman render failed for C4 DSL:\n---\n{dsl}\n---\nerror: {e:?}")
    });

    assert!(
        svg.contains("<svg"),
        "SVG must contain <svg root; got len {}",
        svg.len()
    );
    assert!(
        svg.contains("Customer"),
        "person name 'Customer' must appear"
    );
    assert!(
        svg.contains("Orders"),
        "software system 'Orders' must appear"
    );
    assert!(svg.contains("WebApp"), "container 'WebApp' must appear");
    assert!(svg.contains("Database"), "container 'Database' must appear");
}

/// M41: empty C4 container bundle still renders valid SVG.
#[test]
fn c4_container_view_empty_bundle_renders_to_svg() {
    let elements: Vec<ElementRow> = Vec::new();
    let edges: Vec<SemanticEdgeRow> = Vec::new();
    let selector = ProjectSelector::parse("c4-container:orders").unwrap();

    let dsl = project_mermaid(&elements, &edges, &selector);
    let svg = render_mermaid::render(&dsl).expect("merman should render empty bundle");

    assert!(
        svg.contains("<svg"),
        "empty bundle SVG must contain <svg; got len {}",
        svg.len()
    );
}
