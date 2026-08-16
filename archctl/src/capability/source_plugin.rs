//! Source: plugin extension points.
//!
//! Per ADR-057 §4, the plugin system ships a tap model. Third-party plugin
//! enumeration is not yet wired; only the extension-point declaration exists.
//! `plugin.loadpoint` is experimental — real plugin loading is pending ADR-057 §4 v0.2.

use crate::capability::{Availability, Capability, Category, Maturity, Provider};

/// Plugin extension points (ADR-057 §4).
#[allow(dead_code)]
pub fn all() -> Vec<Capability> {
    vec![Capability::new(
        "plugin.loadpoint",
        Category::Plugin,
        Maturity::Experimental,
        true,
        Availability::OptIn,
        vec![Provider::new("any", Maturity::Experimental)],
    )]
}
