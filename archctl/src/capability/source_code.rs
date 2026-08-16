//! Source: code extractors (c4_discover, call_graph, class_diagram, state_machine, sequence).
//!
//! Mirrors each code extractor into the capability registry.
//! For extractors with a Language enum, each variant is a provider.

use crate::capability::{Availability, Capability, Category, Maturity, Provider};

/// All code extractors.
pub const ALL: &[Capability] = &[
    // ── c4_discover (strategy-based, no Language enum) ────────────────────────
    // Covered by source_cargo.rs (code.strategy.* entries).
    // This entry documents the inference engine itself.
    Capability::new(
        "code.c4_discover",
        Category::Code,
        Maturity::Stable,
        true,
        Availability::Available,
        vec![Provider::new("any", Maturity::Stable)],
    )
    .with_requirement("strategies"),
    // ── call_graph ────────────────────────────────────────────────────────────
    Capability::new(
        "code.call_graph",
        Category::Code,
        Maturity::Stable,
        true,
        Availability::Available,
        vec![
            Provider::with_schema("rust", Maturity::Stable, "call-graph-report/1"),
            Provider::with_schema("typescript", Maturity::Stable, "call-graph-report/1"),
            Provider::with_schema("python", Maturity::Stable, "call-graph-report/1"),
            Provider::with_schema("go", Maturity::Beta, "call-graph-report/1"),
            Provider::with_schema("java", Maturity::Beta, "call-graph-report/1"),
            Provider::with_schema("kotlin", Maturity::Beta, "call-graph-report/1"),
        ],
    ),
    // ── class_diagram ─────────────────────────────────────────────────────────
    Capability::new(
        "code.class_diagram",
        Category::Code,
        Maturity::Stable,
        true,
        Availability::Available,
        vec![
            Provider::with_schema("rust", Maturity::Stable, "class-diagram-report/1"),
            Provider::with_schema("typescript", Maturity::Stable, "class-diagram-report/1"),
            Provider::with_schema("python", Maturity::Stable, "class-diagram-report/1"),
        ],
    ),
    // ── state_machine ─────────────────────────────────────────────────────────
    Capability::new(
        "code.state_machine",
        Category::Code,
        Maturity::Stable,
        true,
        Availability::Available,
        vec![
            Provider::with_schema("rust", Maturity::Stable, "state-machine-report/1"),
            Provider::with_schema("typescript", Maturity::Stable, "state-machine-report/1"),
            Provider::with_schema("python", Maturity::Stable, "state-machine-report/1"),
        ],
    ),
    // ── sequence (graph projection, READ-ONLY) ─────────────────────────────────
    Capability::new(
        "code.sequence",
        Category::Code,
        Maturity::Stable,
        true,
        Availability::Available,
        vec![Provider::with_schema("any", Maturity::Stable, "sequence-report/1")],
    ),
];
