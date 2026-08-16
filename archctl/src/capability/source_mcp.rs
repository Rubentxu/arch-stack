//! Source: MCP tools (read-only mirror of `cognitive::mcp::gateway::ALLOWED_TOOLS`).
//!
//! Per ADR-021: the runtime `ALLOWED_TOOLS` remains the source of truth for
//! what is executable. This registry entry is a READ-ONLY metadata mirror.
//! The runtime gateway is NOT modified by the registry.

use crate::capability::{Availability, Capability, Category, Maturity, Provider};

/// MCP tool surface (read-only mirror of `cognitive::mcp::gateway::ALLOWED_TOOLS`).
///
/// DO NOT add, remove, or modify entries here to change runtime behaviour.
/// The runtime gateway (`cognitive::mcp::gateway`) is the authoritative gate.
#[allow(dead_code)]
pub fn all() -> Vec<Capability> {
    vec![
        Capability::new(
            "mcp.tool.graph_query",
            Category::Mcp,
            Maturity::Stable,
            true,
            Availability::OptIn,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "mcp.tool.schema_validate",
            Category::Mcp,
            Maturity::Stable,
            true,
            Availability::OptIn,
            vec![Provider::new("any", Maturity::Stable)],
        ),
        Capability::new(
            "mcp.tool.run_tests_local",
            Category::Mcp,
            Maturity::Experimental,
            true,
            Availability::OptIn,
            vec![Provider::new("any", Maturity::Experimental)],
        ),
    ]
}
