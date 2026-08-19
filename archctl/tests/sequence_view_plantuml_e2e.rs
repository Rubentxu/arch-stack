//! End-to-end test for the sequence view PlantUML rendering pipeline (M48).
//!
//! Mirrors M43 (`usecase_view_plantuml_e2e.rs`) for the sequence view.
//! Exercises: M45 sequence projector (with labels) → M40 PlantUML backend
//! delegation → SVG. Asserts participants and label text appear in the
//! rendered SVG.
//!
//! Pre-M48, no test verified that the M45 sequence projector output
//! rendered cleanly through a real PlantUML backend. M48 adds this as a
//! regression lock.
//!
//! **SKIPS if no PlantUML backend is installed** (CI-friendly).

use archctl::diagram::project::plantuml::project as project_plantuml;
use archctl::diagram::project_selector::ProjectSelector;
use archctl::graph::{ElementRow, SemanticEdgeRow};
use archctl::render::plantuml as render_plantuml;

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

use archctl::test_helpers::plantuml::backend_available;

/// M48: labeled sequence projector (M45) → PlantUML backend (M40) → SVG.
#[test]
fn sequence_view_plantuml_labeled_to_svg() {
    if !backend_available() {
        eprintln!("SKIP: no PlantUML backend (plantuml CLI or docker plantuml/plantuml) installed");
        return;
    }

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

    // Step 1: projector emits PlantUML DSL.
    let dsl = project_plantuml(&elements, &edges, &selector);
    assert!(
        dsl.contains("@startuml") && dsl.contains("@enduml"),
        "projector DSL must wrap content in @startuml/@enduml; got:\n{dsl}"
    );
    // M45 emit: `Client -> Server : placeOrder()`.
    assert!(
        dsl.contains("Client -> Server : placeOrder()"),
        "labeled sequence arrow should include ' : label'; got:\n{dsl}"
    );

    // Step 2: M40 backend renders the DSL to SVG.
    let svg = render_plantuml::render(&dsl)
        .unwrap_or_else(|e| panic!("backend should render the labeled sequence: {e:?}"));

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

/// M48: unlabeled sequence still renders (backward-compat).
#[test]
fn sequence_view_plantuml_unlabeled_to_svg() {
    if !backend_available() {
        eprintln!("SKIP: no PlantUML backend installed");
        return;
    }

    let elements = vec![
        make_element("e1", "behavior.participant", "A", "behavior"),
        make_element("e2", "behavior.participant", "B", "behavior"),
    ];
    let edges = vec![SemanticEdgeRow {
        relation_id: "r1".to_string(),
        predicate_id: "behavior.invokes".to_string(),
        source_id: "e1".to_string(),
        target_id: "e2".to_string(),
        order_key: "1".to_string(),
        props: serde_json::Map::new(),
    }];
    let selector = ProjectSelector::parse("sequence:test").unwrap();

    let dsl = project_plantuml(&elements, &edges, &selector);
    let svg = render_plantuml::render(&dsl).expect("backend should render unlabeled");

    assert!(
        svg.contains("<svg"),
        "SVG must contain <svg; got len {}",
        svg.len()
    );
    assert!(svg.contains("A"), "participant 'A' must appear in SVG");
    assert!(svg.contains("B"), "participant 'B' must appear in SVG");
}
