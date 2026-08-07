//! End-to-end test for the C4 view PlantUML rendering pipeline (M50).
//!
//! Mirrors M43 (`usecase_view_plantuml_e2e.rs`), M48
//! (`sequence_view_plantuml_e2e.rs`), and M49 (`state_view_plantuml_e2e.rs`)
//! for the C4 view. Closes the verification triangle (use case, sequence,
//! state, c4 — all four views with full projector + backend e2e coverage).
//!
//! Pre-M50, the C4 PlantUML projector emitted lowercase Structurizr
//! keywords (`person "X" { }`, `container "Y" { }`) inside
//! `@startuml`/`@enduml` — syntax rejected by vanilla Java PlantUML unless
//! the C4-PlantUML stdlib is loaded. M50 fixes the projector to emit native
//! PlantUML shapes (`actor "X" as X`, `rectangle "Y" as Y`) and adds this
//! e2e test as the regression lock.
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

/// M50: C4 container projector (fixed to emit vanilla PlantUML) +
/// M40 PlantUML backend → SVG. Closes the verification triangle.
#[test]
fn c4_view_plantuml_to_svg() {
    if !backend_available() {
        eprintln!("SKIP: no PlantUML backend (plantuml CLI or docker plantuml/plantuml) installed");
        return;
    }

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

    // Step 1: projector emits valid PlantUML (actor + rectangle syntax).
    let dsl = project_plantuml(&elements, &edges, &selector);
    assert!(
        dsl.contains("@startuml") && dsl.contains("@enduml"),
        "projector DSL must wrap content in @startuml/@enduml; got:\n{dsl}"
    );
    // M50 fix: vanilla PlantUML shapes, NOT lowercase Structurizr keywords.
    assert!(
        dsl.contains("actor \"Customer\""),
        "person should emit `actor \"Name\"`; got:\n{dsl}"
    );
    assert!(
        dsl.contains("rectangle \"Orders\""),
        "software_system should emit `rectangle \"Name\"`; got:\n{dsl}"
    );
    assert!(
        dsl.contains("rectangle \"WebApp\""),
        "container should emit `rectangle \"Name\"`; got:\n{dsl}"
    );

    // Step 2: M40 backend renders the DSL to SVG.
    let svg = render_plantuml::render(&dsl)
        .unwrap_or_else(|e| panic!("backend should render the C4 bundle: {e:?}"));

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
        "person name 'Customer' must appear in SVG"
    );
    assert!(
        svg.contains("Orders"),
        "system name 'Orders' must appear in SVG"
    );
    assert!(
        svg.contains("WebApp"),
        "container 'WebApp' must appear in SVG"
    );
    assert!(
        svg.contains("Database"),
        "container 'Database' must appear in SVG"
    );
}

/// M50: empty C4 bundle still renders valid SVG.
#[test]
fn c4_view_plantuml_empty_to_svg() {
    if !backend_available() {
        eprintln!("SKIP: no PlantUML backend installed");
        return;
    }

    let elements: Vec<ElementRow> = Vec::new();
    let edges: Vec<SemanticEdgeRow> = Vec::new();
    let selector = ProjectSelector::parse("c4-container:empty").unwrap();

    let dsl = project_plantuml(&elements, &edges, &selector);
    let svg = render_plantuml::render(&dsl).expect("backend should render empty C4 bundle");

    assert!(
        svg.contains("<svg"),
        "empty C4 SVG must contain <svg; got len {}",
        svg.len()
    );
}
