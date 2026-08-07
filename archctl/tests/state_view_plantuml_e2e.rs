//! End-to-end test for the state machine view PlantUML rendering pipeline
//! (M49).
//!
//! Mirrors M43 (`usecase_view_plantuml_e2e.rs`) and M48
//! (`sequence_view_plantuml_e2e.rs`) for the state view.
//!
//! Pre-M49, no test verified that the state view PlantUML projector output
//! (state blocks + transition joins via behavior.source_state /
//! behavior.target_state) rendered cleanly through a real PlantUML backend.
//! M49 adds this as a regression lock.
//!
//! **SKIPS if no PlantUML backend is installed** (CI-friendly).

use archctl::diagram::project::plantuml::project as project_plantuml;
use archctl::diagram::project_selector::ProjectSelector;
use archctl::diagram::queries::{ElementRow, SemanticEdgeRow};
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

use archctl::test_helpers::plantuml::backend_available;

/// M49: state machine projector + M40 PlantUML backend → SVG.
///
/// Per ADR-026, transitions join source_state → transition → target_state.
/// So we need 3 elements per transition (source, transition, target) +
/// 2 edges (source_state, target_state) connecting them.
#[test]
fn state_view_plantuml_to_svg() {
    if !backend_available() {
        eprintln!("SKIP: no PlantUML backend (plantuml CLI or docker plantuml/plantuml) installed");
        return;
    }

    let elements = vec![
        make_element("e1", "uml.state", "Idle", "uml"),
        make_element("e2", "uml.state", "Active", "uml"),
        make_element("e3", "uml.state", "Suspended", "uml"),
        // Transition elements (named after the action they perform).
        make_element("e4", "uml.transition", "activate", "uml"),
        make_element("e5", "uml.transition", "suspend", "uml"),
    ];
    // Per ADR-026: source_state edge = (source_state_element) -> (transition_element);
    //              target_state edge = (transition_element) -> (target_state_element).
    let edges = vec![
        make_edge("r1", "behavior.source_state", "e1", "e4"), // Idle -> activate
        make_edge("r2", "behavior.target_state", "e4", "e2"), // activate -> Active
        make_edge("r3", "behavior.source_state", "e2", "e5"), // Active -> suspend
        make_edge("r4", "behavior.target_state", "e5", "e3"), // suspend -> Suspended
    ];
    let selector = ProjectSelector::parse("state:*").unwrap();

    let dsl = project_plantuml(&elements, &edges, &selector);
    assert!(
        dsl.contains("@startuml") && dsl.contains("@enduml"),
        "projector DSL must wrap content in @startuml/@enduml; got:\n{dsl}"
    );
    // PlantUML state blocks: `state Name { }`.
    assert!(
        dsl.contains("state Idle"),
        "state 'Idle' should be emitted as PlantUML state block; got:\n{dsl}"
    );
    assert!(
        dsl.contains("state Active"),
        "state 'Active' should be emitted as PlantUML state block; got:\n{dsl}"
    );
    assert!(
        dsl.contains("state Suspended"),
        "state 'Suspended' should be emitted as PlantUML state block; got:\n{dsl}"
    );

    let svg = render_plantuml::render(&dsl)
        .unwrap_or_else(|e| panic!("backend should render the state machine: {e:?}"));

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

/// M49: empty state bundle (just the @startuml/@enduml wrapper) still
/// renders to valid SVG.
#[test]
fn state_view_plantuml_empty_to_svg() {
    if !backend_available() {
        eprintln!("SKIP: no PlantUML backend installed");
        return;
    }

    let elements: Vec<ElementRow> = Vec::new();
    let edges: Vec<SemanticEdgeRow> = Vec::new();
    let selector = ProjectSelector::parse("state:*").unwrap();

    let dsl = project_plantuml(&elements, &edges, &selector);
    let svg = render_plantuml::render(&dsl).expect("backend should render empty state diagram");

    assert!(
        svg.contains("<svg"),
        "empty state SVG must contain <svg; got len {}",
        svg.len()
    );
}
