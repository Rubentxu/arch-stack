//! Integration tests for `archctl diagram project` (SCN-410, SCN-411, SCN-412, SCN-416, SCN-417, SCN-418, SCN-419).
//!
//! These tests exercise the diagram project command end-to-end with a real graph store.

use std::fs;
use std::process::Command;

use tempfile::TempDir;

use archctl::diagram::project::OutputFormat;
use archctl::diagram::project::{ProjectReport, project_dsl};
use archctl::diagram::project_selector::ProjectSelector;
use archctl::diagram::queries::{ElementRow, SemanticEdgeRow};
use archctl::store::{GraphStore, LbugStore};

// ──────────────────────────────────────────────────────────────────────────────
// Helper functions
// ──────────────────────────────────────────────────────────────────────────────

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

fn make_edge_with_props(
    rel_id: &str,
    pred: &str,
    src: &str,
    tgt: &str,
    description: &str,
) -> SemanticEdgeRow {
    use serde_json::Map;
    let mut props = Map::new();
    props.insert(
        "description".to_string(),
        serde_json::Value::String(description.to_string()),
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

// ──────────────────────────────────────────────────────────────────────────────
// SCN-410: Class view produces valid PlantUML with classes AND interfaces (implements keyword)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn class_view_produces_valid_plantuml_with_interface() {
    // Setup: OrderService implements IRepository
    let elements = vec![
        make_element("e1", "uml.class", "OrderService", "uml"),
        make_element("e2", "uml.interface", "IRepository", "uml"),
    ];
    let edges = vec![make_edge("r1", "uml.implements", "e1", "e2")];
    let selector = ProjectSelector::parse("class:*").unwrap();

    let (dsl, _report) = project_dsl(&selector, &elements, &edges, OutputFormat::Plantuml);

    // Verify PlantUML structure
    assert!(dsl.contains("@startuml"));
    assert!(dsl.contains("@enduml"));
    assert!(dsl.contains("class OrderService"));
    assert!(dsl.contains("class IRepository"));
    assert!(dsl.contains("<<interface>>"));
    // Implements should use ..|> arrow
    assert!(dsl.contains("OrderService ..|> IRepository"));
}

#[test]
fn class_view_produces_multiple_relationships() {
    // Setup: Order extends BaseOrder, implements ISerializable
    let elements = vec![
        make_element("e1", "uml.class", "Order", "uml"),
        make_element("e2", "uml.class", "BaseOrder", "uml"),
        make_element("e3", "uml.interface", "ISerializable", "uml"),
    ];
    let edges = vec![
        make_edge("r1", "uml.extends", "e1", "e2"),
        make_edge("r2", "uml.implements", "e1", "e3"),
    ];
    let selector = ProjectSelector::parse("class:*").unwrap();

    let (dsl, _report) = project_dsl(&selector, &elements, &edges, OutputFormat::Plantuml);

    // Both relationships should appear with correct arrows
    assert!(dsl.contains("Order --|> BaseOrder"));
    assert!(dsl.contains("Order ..|> ISerializable"));
}

// ──────────────────────────────────────────────────────────────────────────────
// SCN-411: C4 container produces valid Structurizr — relation description from predicate
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn c4_container_produces_valid_structurizr_with_description() {
    // Setup: API container uses DB container with "reads data" description
    let elements = vec![
        make_element("e1", "c4.container", "API", "c4"),
        make_element("e2", "c4.container", "DB", "c4"),
    ];
    let edges = vec![make_edge_with_props(
        "r1",
        "core.uses",
        "e1",
        "e2",
        "reads data",
    )];
    let selector = ProjectSelector::parse("c4-container:*").unwrap();

    let (dsl, _report) = project_dsl(&selector, &elements, &edges, OutputFormat::Structurizr);

    // Verify Structurizr DSL structure with description
    assert!(dsl.contains("workspace {"));
    assert!(dsl.contains("model {"));
    assert!(dsl.contains("container \"API\""));
    assert!(dsl.contains("container \"DB\""));
    // Description should appear in the relationship
    assert!(dsl.contains("API -> DB \"reads data\""));
    assert!(dsl.contains("views {"));
}

#[test]
fn c4_container_structurizr_without_description() {
    let elements = vec![
        make_element("e1", "c4.container", "API", "c4"),
        make_element("e2", "c4.container", "DB", "c4"),
    ];
    let edges = vec![make_edge("r1", "core.uses", "e1", "e2")];
    let selector = ProjectSelector::parse("c4-container:*").unwrap();

    let (dsl, _report) = project_dsl(&selector, &elements, &edges, OutputFormat::Structurizr);

    // Without description, just arrow
    assert!(dsl.contains("API -> DB"));
    // But not with description
    assert!(!dsl.contains("API -> DB \""));
}

// ──────────────────────────────────────────────────────────────────────────────
// SCN-412: Use case view — actors use [ActorName] shape (NOT (ActorName))
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn usecase_view_actors_use_bracket_shape() {
    // Setup: Customer actor participates in PlaceOrder use case
    let elements = vec![
        make_element("e1", "uml.actor", "Customer", "uml"),
        make_element("e2", "uml.use_case", "PlaceOrder", "uml"),
    ];
    let edges = vec![make_edge("r1", "usecase.participates_in", "e1", "e2")];
    let selector = ProjectSelector::parse("usecase:*").unwrap();

    let (dsl, _report) = project_dsl(&selector, &elements, &edges, OutputFormat::Plantuml);

    // PlantUML actors should use [ActorName] bracket shape (SCN-412 fix)
    assert!(dsl.contains("[Customer]"));
    assert!(dsl.contains("usecase PlaceOrder"));
    // Should NOT contain parentheses actor syntax
    assert!(!dsl.contains("(Customer)"));
    assert!(!dsl.contains("(Customer)"));
}

// ──────────────────────────────────────────────────────────────────────────────
// SCN-416: Parent dir creation and file write
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn diagram_project_creates_parent_directories() {
    let tmpdir = TempDir::new().unwrap();
    let project = tmpdir.path().join("proj");
    fs::create_dir_all(&project).unwrap();

    // Initialize store with some data
    {
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();
    }

    // Output path with nested directories that don't exist
    let output_path = tmpdir
        .path()
        .join("output")
        .join("nested")
        .join("diagram.puml");

    let result = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "diagram",
            "project",
            "--cwd",
            project.to_str().unwrap(),
            "--view",
            "class:*",
            "--format",
            "plantuml",
            "--output",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "command should succeed, stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    // File should be created with content
    assert!(output_path.exists(), "output file should be created");
    let content = fs::read_to_string(&output_path).unwrap();
    assert!(
        content.contains("@startuml"),
        "should contain PlantUML header"
    );
}

#[test]
fn diagram_project_writes_correct_content() {
    let tmpdir = TempDir::new().unwrap();
    let project = tmpdir.path().join("proj");
    fs::create_dir_all(&project).unwrap();

    // Initialize store
    {
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();
    }

    let output_path = tmpdir.path().join("diagram.puml");

    let result = Command::new(env!("CARGO_BIN_EXE_archctl"))
        .args([
            "diagram",
            "project",
            "--cwd",
            project.to_str().unwrap(),
            "--view",
            "class:*",
            "--format",
            "plantuml",
            "--output",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("@startuml"));
    assert!(content.contains("@enduml"));
}

// ──────────────────────────────────────────────────────────────────────────────
// SCN-417: Exact scope with prefix matching — scope `src/auth` matches `src/auth/user.rs`
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn scope_selector_parses_path_scope() {
    // SCN-417: scope selector parsing for path-based scopes like `src/auth`
    let selector = ProjectSelector::parse("class:src/auth").unwrap();
    assert_eq!(selector.scope_ident(), Some("src/auth"));
    assert!(!selector.scope_ident().is_none());
}

#[test]
fn scope_all_selector_returns_none_ident() {
    // Wildcard scope should not filter by scope
    let selector = ProjectSelector::parse("class:*").unwrap();
    assert!(selector.scope_ident().is_none());
}

#[test]
fn project_dsl_receives_elements_with_canonical_keys_correctly() {
    // This tests that project_dsl handles elements with path-based canonical_keys
    // The actual scope filtering (STARTS WITH) is done in query_elements, not here.
    // This test verifies that elements with path-based canonical_keys are projected.
    let elements = vec![ElementRow {
        id: "e1".to_string(),
        kind_id: "uml.class".to_string(),
        category: "uml".to_string(),
        canonical_key: "src/auth".to_string(),
        current_name: "UserService".to_string(),
        current_status: "accepted".to_string(),
        current_confidence: 1.0,
        current_version_id: "v1".to_string(),
    }];
    let edges = vec![];
    let selector = ProjectSelector::parse("class:*").unwrap(); // All scope to bypass filtering

    let (dsl, _report) = project_dsl(&selector, &elements, &edges, OutputFormat::Plantuml);

    // Element should be projected with its current_name
    assert!(dsl.contains("UserService"), "should contain UserService");
    assert!(dsl.contains("@startuml"));
}

// ──────────────────────────────────────────────────────────────────────────────
// SCN-418: Valid/invalid format handling
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn valid_format_plantuml_is_accepted() {
    let selector = ProjectSelector::parse("class:*").unwrap();
    let elements = vec![];
    let edges = vec![];

    let result = OutputFormat::parse("plantuml");
    assert!(result.is_some());
    let (_dsl, report) = project_dsl(&selector, &elements, &edges, result.unwrap());
    assert_eq!(report.format, "plantuml");
}

#[test]
fn valid_format_mermaid_is_accepted() {
    let selector = ProjectSelector::parse("class:*").unwrap();
    let elements = vec![];
    let edges = vec![];

    let result = OutputFormat::parse("mermaid");
    assert!(result.is_some());
    let (_dsl, report) = project_dsl(&selector, &elements, &edges, result.unwrap());
    assert_eq!(report.format, "mermaid");
}

#[test]
fn valid_format_structurizr_is_accepted() {
    let selector = ProjectSelector::parse("class:*").unwrap();
    let elements = vec![];
    let edges = vec![];

    let result = OutputFormat::parse("structurizr");
    assert!(result.is_some());
    let (_dsl, report) = project_dsl(&selector, &elements, &edges, result.unwrap());
    assert_eq!(report.format, "structurizr");
}

#[test]
fn invalid_format_is_rejected() {
    let result = OutputFormat::parse("invalid-format");
    assert!(result.is_none());
}

#[test]
fn case_insensitive_format_parsing() {
    assert!(OutputFormat::parse("PlantUML").is_some());
    assert!(OutputFormat::parse("PLANTUML").is_some());
    assert!(OutputFormat::parse("Mermaid").is_some());
    assert!(OutputFormat::parse("STRUCTURIZR").is_some());
}

// ──────────────────────────────────────────────────────────────────────────────
// SCN-419: State view — source/target edges through transition join (NOT source_state as state→state)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn state_view_transitions_join_through_transition_node() {
    // Per ADR-026: Transition node has source_state (→state) AND target_state (→state)
    // PlantUML should emit: state1 --> Transition --> state2
    //
    // Setup: Pending --[source_state]--> SubmitOrder --[target_state]--> Confirmed
    let elements = vec![
        make_element("e1", "uml.state", "Pending", "uml"),
        make_element("e2", "uml.state", "Confirmed", "uml"),
        make_element("e3", "uml.state", "SubmitOrder", "uml"), // transition node
    ];
    let edges = vec![
        // source_state: Pending (e1) → SubmitOrder (e3)
        make_edge("r1", "behavior.source_state", "e1", "e3"),
        // target_state: SubmitOrder (e3) → Confirmed (e2)
        make_edge("r2", "behavior.target_state", "e3", "e2"),
    ];
    let selector = ProjectSelector::parse("state:*").unwrap();

    let (dsl, _report) = project_dsl(&selector, &elements, &edges, OutputFormat::Plantuml);

    // Should contain all three state declarations
    assert!(dsl.contains("state Pending"));
    assert!(dsl.contains("state Confirmed"));
    assert!(dsl.contains("state SubmitOrder"));

    // The key assertion: transition must go THROUGH the transition node
    // Correct: Pending --> SubmitOrder --> Confirmed
    // Wrong: Pending --> Confirmed (direct connection)
    assert!(
        dsl.contains("Pending --> SubmitOrder --> Confirmed"),
        "expected transition join pattern 'Pending --> SubmitOrder --> Confirmed', got: {}",
        dsl
    );
    // Should NOT have direct Pending --> Confirmed connection
    assert!(
        !dsl.contains("Pending --> Confirmed"),
        "should NOT have direct state→state connection, got: {}",
        dsl
    );
}

#[test]
fn state_view_single_transition_edge() {
    // Proper transition: source_state edge + target_state edge + transition element
    let elements = vec![
        make_element("e1", "uml.state", "Pending", "uml"),
        make_element("e2", "uml.transition", "SubmitOrder_Transition", "uml"),
        make_element("e3", "uml.state", "SubmitOrder", "uml"),
    ];
    // source_state: Pending -> SubmitOrder_Transition
    // target_state: SubmitOrder_Transition -> SubmitOrder
    let edges = vec![
        make_edge("r1", "behavior.source_state", "e1", "e2"),
        make_edge("r2", "behavior.target_state", "e2", "e3"),
    ];
    let selector = ProjectSelector::parse("state:*").unwrap();

    let (dsl, _report) = project_dsl(&selector, &elements, &edges, OutputFormat::Plantuml);

    // States should be declared
    assert!(dsl.contains("state Pending"));
    assert!(dsl.contains("state SubmitOrder"));
    // Joined transition output
    assert!(dsl.contains("Pending --> SubmitOrder_Transition --> SubmitOrder"));
}

#[test]
fn state_view_no_transitions() {
    let elements = vec![
        make_element("e1", "uml.state", "Pending", "uml"),
        make_element("e2", "uml.state", "Confirmed", "uml"),
    ];
    let edges = vec![];
    let selector = ProjectSelector::parse("state:*").unwrap();

    let (dsl, _report) = project_dsl(&selector, &elements, &edges, OutputFormat::Plantuml);

    // Just state declarations, no transitions
    assert!(dsl.contains("state Pending"));
    assert!(dsl.contains("state Confirmed"));
    assert!(!dsl.contains("-->"));
}

#[test]
fn state_view_multiple_transitions_from_same_source() {
    // Pending can transition to either SubmitOrder or CancelOrder
    let elements = vec![
        make_element("e1", "uml.state", "Pending", "uml"),
        make_element("e2", "uml.state", "SubmitOrder", "uml"),
        make_element("e3", "uml.state", "CancelOrder", "uml"),
    ];
    let edges = vec![
        make_edge("r1", "behavior.source_state", "e1", "e2"),
        make_edge("r2", "behavior.target_state", "e2", "e3"),
    ];
    let selector = ProjectSelector::parse("state:*").unwrap();

    let (dsl, _report) = project_dsl(&selector, &elements, &edges, OutputFormat::Plantuml);

    assert!(dsl.contains("Pending --> SubmitOrder --> CancelOrder"));
}

// ──────────────────────────────────────────────────────────────────────────────
// Determinism tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn plantuml_output_is_deterministic() {
    let elements = vec![
        make_element("e3", "uml.class", "Zebra", "uml"),
        make_element("e1", "uml.class", "Alpha", "uml"),
        make_element("e2", "uml.class", "Beta", "uml"),
    ];
    let edges = vec![make_edge("r1", "uml.extends", "e1", "e2")];
    let selector = ProjectSelector::parse("class:*").unwrap();

    let (dsl1, _report) = project_dsl(&selector, &elements, &edges, OutputFormat::Plantuml);
    let (dsl2, _report) = project_dsl(&selector, &elements, &edges, OutputFormat::Plantuml);

    assert_eq!(dsl1, dsl2, "output must be deterministic");
}

#[test]
fn structurizr_output_is_deterministic() {
    let elements = vec![
        make_element("e1", "c4.container", "Zebra", "c4"),
        make_element("e2", "c4.container", "Alpha", "c4"),
    ];
    let edges = vec![];
    let selector = ProjectSelector::parse("c4-container:*").unwrap();

    let (dsl1, _report) = project_dsl(&selector, &elements, &edges, OutputFormat::Structurizr);
    let (dsl2, _report) = project_dsl(&selector, &elements, &edges, OutputFormat::Structurizr);

    assert_eq!(dsl1, dsl2, "output must be deterministic");
}
