//! Source: C4 diagram kinds and UML view kinds.
//!
//! Mirrors `diagram::selector::C4Kind` and `diagram::project_selector::ViewKind`
//! into the capability registry.

use crate::capability::{Availability, Capability, Category, Maturity, Provider};

/// All C4 diagram kinds (from `diagram::selector::C4Kind`).
#[allow(dead_code)]
pub fn c4_kinds() -> Vec<Capability> {
    vec![
        Capability::new(
            "diagram.c4.context",
            Category::Diagram,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "diagram.c4.container",
            Category::Diagram,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "diagram.c4.component",
            Category::Diagram,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "diagram.c4.dynamic",
            Category::Diagram,
            Maturity::Experimental,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Experimental)],
        ),
        Capability::new(
            "diagram.c4.deployment",
            Category::Diagram,
            Maturity::Experimental,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Experimental)],
        ),
    ]
}

/// All UML/view kinds (from `diagram::project_selector::ViewKind`).
#[allow(dead_code)]
pub fn view_kinds() -> Vec<Capability> {
    vec![
        Capability::new(
            "diagram.view.c4_context",
            Category::Diagram,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "diagram.view.c4_container",
            Category::Diagram,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "diagram.view.c4_component",
            Category::Diagram,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "diagram.view.class",
            Category::Diagram,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "diagram.view.sequence",
            Category::Diagram,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "diagram.view.state",
            Category::Diagram,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "diagram.view.usecase",
            Category::Diagram,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
    ]
}

/// All diagram kinds (C4 + UML views).
#[allow(dead_code)]
pub fn all() -> Vec<Capability> {
    let mut caps = c4_kinds();
    caps.extend(view_kinds());
    caps
}
