//! End-to-end test for the use case view PlantUML rendering pipeline (M43).
//!
//! Closes the verification loop from M39 (use case projector emits PlantUML
//! text) through M40 (PlantUML rendering via user-installed backend) to a
//! real SVG output. Mirrors the M39 `usecase_view_e2e.rs` (Mermaid) and
//! M40 `plantuml_render_e2e.rs` (PlantUML render from hand-crafted source)
//! patterns.
//!
//! Pre-M43, no test exercised the full projector → PlantUML → backend → SVG
//! chain. M43 adds this as a regression lock.
//!
//! **SKIPS if no PlantUML backend is installed** (CI-friendly). The test is
//! the canonical "does the M39 + M40 wiring actually work end-to-end?" check.

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

fn backend_available() -> bool {
    std::process::Command::new("plantuml")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        || std::process::Command::new("docker")
            .args(["image", "inspect", "plantuml/plantuml"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
}

/// M43: full chain — use case projector (M39) emits PlantUML DSL, M40
/// backend renders it to SVG. Asserts SVG is non-empty + contains `<svg`
/// + contains both actor and use case names as text nodes.
#[test]
fn usecase_view_plantuml_projector_to_svg() {
    if !backend_available() {
        eprintln!("SKIP: no PlantUML backend (plantuml CLI or docker plantuml/plantuml) installed");
        return;
    }

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

    // Step 1: projector emits PlantUML DSL.
    let dsl = project_plantuml(&elements, &edges, &selector);
    assert!(
        dsl.contains("@startuml") && dsl.contains("@enduml"),
        "projector DSL must wrap content in @startuml/@enduml; got:\n{dsl}"
    );
    // M39 emit: actor = [Name] bracket (SCN-412), use case = usecase Name.
    assert!(
        dsl.contains("[Customer]"),
        "actor should use [Name] bracket shape; got:\n{dsl}"
    );
    assert!(
        dsl.contains("usecase PlaceOrder"),
        "use case should use native PlantUML usecase syntax; got:\n{dsl}"
    );
    assert!(
        dsl.contains("usecase CancelOrder"),
        "second use case missing; got:\n{dsl}"
    );

    // Step 2: M40 backend renders the DSL to SVG.
    let svg = render_plantuml::render(&dsl)
        .unwrap_or_else(|e| panic!("backend should render the use case bundle: {e:?}"));

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

/// M43: minimal projector → SVG round-trip — even a single-element bundle
/// (one actor) should render to a valid SVG.
#[test]
fn usecase_view_plantuml_minimal_to_svg() {
    if !backend_available() {
        eprintln!("SKIP: no PlantUML backend installed");
        return;
    }

    let elements = vec![make_element("e1", "uml.actor", "Admin", "uml")];
    let edges: Vec<SemanticEdgeRow> = vec![];
    let selector = ProjectSelector::parse("usecase:*").unwrap();

    let dsl = project_plantuml(&elements, &edges, &selector);
    let svg = render_plantuml::render(&dsl).expect("backend should render minimal bundle");

    assert!(
        svg.contains("<svg"),
        "minimal SVG must contain <svg; got len {}",
        svg.len()
    );
    assert!(
        svg.contains("Admin"),
        "minimal actor 'Admin' must appear in SVG"
    );
}
