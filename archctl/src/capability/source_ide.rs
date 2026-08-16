//! Source: IDE adapters.
//!
//! Mirrors `ide::builtin_adapters()` into the capability registry.

use crate::capability::{Availability, Capability, Category, Maturity, Provider};

/// All built-in IDE adapters (from `ide::builtin_adapters()`).
///
/// Order matches the stable adapter order: OpenCode, ZCode, Claude Code, Codex.
pub const ALL: &[Capability] = &[
    Capability::new(
        "ide.opencode",
        Category::Ide,
        Maturity::Stable,
        true,
        Availability::Available,
        vec![Provider::new("any", Maturity::Stable)],
    ),
    Capability::new(
        "ide.zcode",
        Category::Ide,
        Maturity::Experimental,
        true,
        Availability::Available,
        vec![Provider::new("any", Maturity::Experimental)],
    ),
    Capability::new(
        "ide.claude_code",
        Category::Ide,
        Maturity::Stable,
        true,
        Availability::Available,
        vec![Provider::new("any", Maturity::Stable)],
    ),
    Capability::new(
        "ide.codex",
        Category::Ide,
        Maturity::Experimental,
        true,
        Availability::Available,
        vec![Provider::new("any", Maturity::Experimental)],
    ),
];
