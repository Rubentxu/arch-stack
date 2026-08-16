//! Capability registry — single source of truth for archctl feature surface.
//!
//! Replaces nine drift sources (README.md, MANUAL.md, schemas/call-graph-report.schema.json
//! language enum, STATE.md, ROADMAP.md, docs/specs/index.md, ADRs) with one
//! introspectable registry. Language support is derived from the registry itself.
//!
//! Per ADR-045: every strategy, renderer, view kind, doctor scope, IDE
//! adapter, MCP tool, and CLI subcommand declared in code has a matching
//! registry entry, and vice versa. Violations fail `alignment.rs` tests.

mod alignment;
mod docs;
mod source_cargo;
mod source_cli;
mod source_code;
mod source_diagram;
mod source_doctor;
mod source_ide;
mod source_mcp;
mod source_plugin;
mod source_render;

/// JSON Schema for the capability registry (JSON Schema 2020-12).
pub const CAPABILITY_REGISTRY_SCHEMA: &str =
    include_str!("../../../schemas/capability-registry.schema.json");

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Capability maturity level.
///
/// Used to gate user-facing features and communicate stability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Maturity {
    Stable,
    Beta,
    Experimental,
    Proposed,
}

impl Maturity {
    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Maturity::Stable => "stable",
            Maturity::Beta => "beta",
            Maturity::Experimental => "experimental",
            Maturity::Proposed => "proposed",
        }
    }
}

/// Runtime availability of a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Availability {
    #[serde(rename = "available")]
    Available,
    #[serde(rename = "opt_in")]
    OptIn,
    #[serde(rename = "experimental")]
    Experimental,
}

impl Availability {
    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Availability::Available => "available",
            Availability::OptIn => "opt_in",
            Availability::Experimental => "experimental",
        }
    }
}

/// High-level category for grouping capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Code,
    Render,
    Diagram,
    Ide,
    Mcp,
    Doctor,
    Cli,
    Plugin,
}

impl Category {
    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Category::Code => "code",
            Category::Render => "render",
            Category::Diagram => "diagram",
            Category::Ide => "ide",
            Category::Mcp => "mcp",
            Category::Doctor => "doctor",
            Category::Cli => "cli",
            Category::Plugin => "plugin",
        }
    }
}

/// A single language-specific implementation of a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provider {
    /// Lowercase language id (e.g. "rust", "typescript").
    pub language: String,
    /// Maturity of this specific language provider.
    pub maturity: Maturity,
    /// Optional schema id this provider emits (e.g. "call-graph-report/1").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
}

impl Provider {
    /// Build a new provider.
    pub fn new(language: &'static str, maturity: Maturity) -> Self {
        Self {
            language: String::from(language),
            maturity,
            schema: None,
        }
    }

    /// Build a new provider with a schema id.
    pub fn with_schema(language: &'static str, maturity: Maturity, schema: &'static str) -> Self {
        Self {
            language: String::from(language),
            maturity,
            schema: Some(String::from(schema)),
        }
    }
}

/// A registered capability entry.
///
/// Each entry carries a stable id (`<domain>.<kind>`), maturity, availability,
/// deterministic flag, requirements, and per-language providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    /// Stable id in `<domain>.<kind>` form (e.g. `"code.call_graph"`).
    pub id: String,
    /// High-level category.
    pub category: Category,
    /// Overall maturity of this capability.
    pub maturity: Maturity,
    /// Whether the output is deterministic for the same input.
    pub deterministic: bool,
    /// List of feature flags or runtime requirements.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub requirements: BTreeSet<String>,
    /// Runtime availability.
    pub availability: Availability,
    /// Per-language providers for this capability.
    pub providers: Vec<Provider>,
}

impl Capability {
    /// Build a new capability entry.
    ///
    /// The id is not validated at const-construction time; call `validate_id`
    /// separately in non-const init or tests.
    pub fn new(
        id: &'static str,
        category: Category,
        maturity: Maturity,
        deterministic: bool,
        availability: Availability,
        providers: Vec<Provider>,
    ) -> Self {
        Self::validate_id(id);
        Self {
            id: String::from(id),
            category,
            maturity,
            deterministic,
            requirements: BTreeSet::new(),
            availability,
            providers,
        }
    }

    /// Add a requirement string.
    pub fn with_requirement(mut self, req: &'static str) -> Self {
        self.requirements.insert(String::from(req));
        self
    }

    /// Validate that `id` is a well-formed registry identifier.
    ///
    /// Format: `category.technology` or `category.technology.variant`
    /// - Must contain at least one `.` separator
    /// - No empty segments (no leading, trailing, or consecutive dots)
    #[track_caller]
    pub fn validate_id(id: &str) {
        assert!(
            !id.is_empty()
                && !id.starts_with('.')
                && !id.ends_with('.')
                && !id.contains("..")
                && id.split('.').count() >= 2
                && id.split('.').all(|s| !s.is_empty()),
            "capability id must be 'category.technology' or 'category.technology.variant': got {id}"
        );
    }
}

/// The capability registry.
///
/// Capabilities are kept sorted by `id` to ensure deterministic iteration
/// (BTreeMap-style ordering, byte-identical across runs).
#[derive(Debug, Clone, Default)]
pub struct CapabilityRegistry {
    entries: Vec<Capability>,
}

impl CapabilityRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Register a capability entry. Entries are kept sorted by `id`.
    pub fn add(&mut self, cap: Capability) {
        // Insert in sorted order (id is unique by design).
        let pos = self
            .entries
            .binary_search_by(|e| e.id.cmp(&cap.id))
            .unwrap_or_else(|i| i);
        self.entries.insert(pos, cap);
    }

    /// Iterate over all entries in deterministic (sorted by id) order.
    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        self.entries.iter()
    }

    /// Number of registered capabilities.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when the registry has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Build the capability registry from all registered sources.
///
/// Entries are sorted by `id` (deterministic iteration order).
pub fn registry() -> CapabilityRegistry {
    let mut reg = CapabilityRegistry::new();
    // Gather all capability sources; each `all()` function returns `Vec<Capability>`.
    let sources: Vec<Vec<Capability>> = vec![
        source_cargo::all(),
        source_cli::all(),
        source_code::all(),
        source_diagram::all(),
        source_doctor::all(),
        source_ide::all(),
        source_mcp::all(),
        source_plugin::all(),
        source_render::all(),
    ];
    for source in sources {
        for cap in source {
            reg.add(cap.clone());
        }
    }
    reg
}

/// Render the registry as a versioned JSON object.
///
/// Output is deterministic (sorted by id) and validates against
/// `CAPABILITY_REGISTRY_SCHEMA`.
pub fn render_json(reg: &CapabilityRegistry) -> String {
    #[derive(serde::Serialize)]
    struct RegistryOutput<'a> {
        #[serde(rename = "schemaVersion")]
        schema_version: &'a str,
        capabilities: &'a [Capability],
    }
    let output = RegistryOutput {
        schema_version: "1",
        capabilities: &reg.entries,
    };
    serde_json::to_string_pretty(&output).expect("capability registry serializes to JSON")
}

// Re-export for convenience.
pub use docs::render_markdown;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_id_format_enforced() {
        // Valid: exactly one dot
        let r = Capability::new(
            "code.call_graph",
            Category::Code,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![],
        );
        assert_eq!(r.id, "code.call_graph");

        // Invalid: no dot — should panic
        let result = std::panic::catch_unwind(|| {
            Capability::new(
                "invalid",
                Category::Code,
                Maturity::Stable,
                true,
                Availability::Available,
                vec![],
            )
        });
        assert!(result.is_err());

        // Invalid: empty segment (consecutive dots) — should panic
        let result = std::panic::catch_unwind(|| {
            Capability::new(
                "code..call_graph",
                Category::Code,
                Maturity::Stable,
                true,
                Availability::Available,
                vec![],
            )
        });
        assert!(result.is_err());
    }

    #[test]
    fn provider_maturity_downgrade_keeps_entry() {
        // A provider can be downgraded from beta → experimental without
        // removing the entry from the registry.
        let provider = Provider::new("kotlin", Maturity::Beta);
        let mut cap = Capability::new(
            "code.call_graph",
            Category::Code,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![provider],
        );
        assert_eq!(cap.providers.len(), 1);
        assert_eq!(cap.providers[0].maturity, Maturity::Beta);

        // Downgrade the provider maturity in-place
        cap.providers[0] = Provider::new("kotlin", Maturity::Experimental);
        assert_eq!(cap.providers[0].maturity, Maturity::Experimental);
        // Entry still present
        assert_eq!(cap.providers.len(), 1);
    }

    #[test]
    fn registry_deterministic_sort() {
        let mut reg = CapabilityRegistry::new();
        reg.add(Capability::new(
            "render.plantuml",
            Category::Render,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![],
        ));
        reg.add(Capability::new(
            "code.call_graph",
            Category::Code,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![],
        ));
        reg.add(Capability::new(
            "diagram.c4",
            Category::Diagram,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![],
        ));

        let ids: Vec<_> = reg.iter().map(|c| c.id.clone()).collect();
        assert_eq!(
            ids,
            vec!["code.call_graph", "diagram.c4", "render.plantuml"]
        );
    }

    #[test]
    fn capability_with_requirements() {
        let cap = Capability::new(
            "code.call_graph",
            Category::Code,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![],
        )
        .with_requirement("ast-grep")
        .with_requirement("tree-sitter");

        assert!(cap.requirements.contains("ast-grep"));
        assert!(cap.requirements.contains("tree-sitter"));
        assert_eq!(cap.requirements.len(), 2);
    }
}
