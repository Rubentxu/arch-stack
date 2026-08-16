//! Source: C4 discovery strategies (cargo, npm, dockerfile, helm, components).
//!
//! Mirrors `code::strategies::register_strategies()` into the capability registry.
//! Each strategy is a distinct container-inference capability.

use crate::capability::{Availability, Capability, Category, Maturity, Provider};

/// All C4 container-inference strategies.
///
/// Order matches `code::strategies::register_strategies()`.
#[allow(dead_code)]
pub fn all() -> Vec<Capability> {
    vec![
        // S1: Cargo workspace detection.
        Capability::new(
            "code.strategy.cargo-workspace",
            Category::Code,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        )
        .with_requirement("cargo"),
        // S2: npm/yarn/pnpm workspace detection.
        Capability::new(
            "code.strategy.npm-workspace",
            Category::Code,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        )
        .with_requirement("npm"),
        // S6: npm single-package detection.
        Capability::new(
            "code.strategy.npm-single",
            Category::Code,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        // S5: Dockerfile per service.
        Capability::new(
            "code.strategy.dockerfile",
            Category::Code,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        // S6: Helm chart detection.
        Capability::new(
            "code.strategy.helm",
            Category::Code,
            Maturity::Experimental,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Experimental)],
        ),
        // G5: Internal component detection (ADR-029).
        Capability::new(
            "code.strategy.components",
            Category::Code,
            Maturity::Experimental,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Experimental)],
        ),
    ]
}
