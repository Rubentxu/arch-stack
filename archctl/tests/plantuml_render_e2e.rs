//! End-to-end test for the PlantUML render path (M40).
//!
//! Proves that `archctl::render::plantuml::render` works against an installed
//! PlantUML backend (Java CLI / docker / custom) and produces valid SVG.
//!
//! Tests SKIP if no backend is available — the binary must remain usable
//! even on machines without PlantUML installed (the render path emits a
//! clear "no PlantUML backend found" error instead of crashing).

use archctl::render::plantuml as render_plantuml;

const REAL_PUML: &str = r#"@startuml
title Use Case — PlaceOrder
actor Customer
usecase "Place Order" as PO
usecase "Pay" as P
Customer --> PO
PO .> P : <<include>>
@enduml
"#;

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

/// M40: when a backend is installed, render the real-world use case source
/// (matching the syntax emitted by `archctl diagram project --view usecase:*
/// --format plantuml` per M39) and verify the SVG is non-empty + contains
/// `<svg` + contains the use case name as text.
#[test]
fn plantuml_render_real_world_use_case_to_svg() {
    if !backend_available() {
        eprintln!("SKIP: no PlantUML backend (plantuml CLI or docker plantuml/plantuml) installed");
        return;
    }

    let svg = render_plantuml::render(REAL_PUML).expect("backend should render successfully");

    assert!(
        svg.starts_with("<?xml") || svg.starts_with("<svg"),
        "SVG should start with xml/svg"
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
        svg.contains("Place Order"),
        "use case name 'Place Order' must appear in rendered SVG"
    );
    assert!(
        svg.contains("Pay"),
        "second use case name 'Pay' must appear in rendered SVG"
    );
}

/// M40: minimal C4-style diagram should also render.
#[test]
fn plantuml_render_minimal_c4_container_to_svg() {
    if !backend_available() {
        eprintln!("SKIP: no PlantUML backend installed");
        return;
    }

    let src = r#"@startuml
!include <C4/Container>
Person(customer, "Customer", "A customer of the e-commerce platform")
Container(webapp, "Web App", "Java/Spring", "Serves the customer UI")
ContainerDb(db, "Database", "PostgreSQL", "Stores orders and customer data")
Rel(customer, webapp, "Uses", "HTTPS")
Rel(webapp, db, "Reads/writes", "JDBC")
@enduml
"#;
    let svg = render_plantuml::render(src).expect("backend should render C4 container");

    assert!(
        svg.contains("<svg"),
        "C4 container SVG must contain <svg; got len {}",
        svg.len()
    );
    assert!(svg.contains("Customer"), "Customer should appear in SVG");
}
