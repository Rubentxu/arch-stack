//! Source: CLI subcommands.
//!
//! Mirrors `cli::Command` into the capability registry as `cli.<subcommand>`.

use crate::capability::{Availability, Capability, Category, Maturity, Provider};

/// All CLI subcommands (top-level `Command` variants).
///
/// Order matches the `cli::Command` enum declaration.
#[allow(dead_code)]
pub fn all() -> Vec<Capability> {
    vec![
        Capability::new(
            "cli.doctor",
            Category::Cli,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "cli.project",
            Category::Cli,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "cli.graph",
            Category::Cli,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "cli.inventory",
            Category::Cli,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "cli.diagram",
            Category::Cli,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "cli.evidence",
            Category::Cli,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "cli.render",
            Category::Cli,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "cli.code",
            Category::Cli,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "cli.skills",
            Category::Cli,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "cli.agent",
            Category::Cli,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "cli.mcp",
            Category::Cli,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "cli.plugin",
            Category::Cli,
            Maturity::Experimental,
            true,
            Availability::OptIn,
            vec![Provider::new("any", Maturity::Experimental)],
        ),
        Capability::new(
            "cli.ide",
            Category::Cli,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "cli.view",
            Category::Cli,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "cli.self",
            Category::Cli,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "cli.architecture",
            Category::Cli,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("any", Maturity::Stable)],
        ),
    ]
}
