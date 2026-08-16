//! Source: diagram renderers (structurizr, plantuml, mermaid).
//!
//! Mirrors `render.rs::RenderKind` into the capability registry.
//! Per ADR-011, all renderers are local-only (no network egress).

use crate::capability::{Availability, Capability, Category, Maturity, Provider};

/// All diagram renderers.
///
/// Order: structurizr, plantuml, mermaid (matches RenderKind declaration order).
#[allow(dead_code)]
pub fn all() -> Vec<Capability> {
    vec![
        Capability::new(
            "render.structurizr",
            Category::Render,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "render.plantuml",
            Category::Render,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "render.mermaid",
            Category::Render,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
    ]
}
