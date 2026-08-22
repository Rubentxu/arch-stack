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

#[cfg(test)]
mod tests {
    use super::*;

    /// `all()` returns a non-empty Vec covering every CLI subcommand
    /// exposed by `cli::Command`. Locks the inventory count + non-empty
    /// invariant.
    #[test]
    fn all_returns_full_cli_inventory() {
        let caps = all();
        assert!(
            !caps.is_empty(),
            "CLI capability inventory must not be empty"
        );
        // Lock the exact count — adding a subcommand without updating
        // this source is a contract break that this test catches.
        assert_eq!(caps.len(), 16, "expected 16 CLI subcommands");
    }

    /// All `id`s are unique — registry consumers dedup by id and silently
    // lose duplicates otherwise. Locks the unique-id contract.
    #[test]
    fn all_ids_are_unique() {
        let caps = all();
        let mut seen = std::collections::BTreeSet::new();
        for c in &caps {
            assert!(
                seen.insert(c.id.clone()),
                "duplicate capability id: {}",
                c.id
            );
        }
        assert_eq!(seen.len(), caps.len());
    }

    /// All CLI source entries use `Category::Cli` — locks the category
    /// contract for this source.
    #[test]
    fn all_entries_use_category_cli() {
        for c in all() {
            assert_eq!(c.category, Category::Cli, "{} must use Category::Cli", c.id);
        }
    }

    /// All CLI ids start with `"cli."` — locks the naming convention.
    #[test]
    fn all_ids_start_with_cli_prefix() {
        for c in all() {
            assert!(
                c.id.starts_with("cli."),
                "CLI source id '{}' must start with 'cli.'",
                c.id
            );
            // Also: well-formed (no leading/trailing dots, no empty segments)
            Capability::validate_id(&c.id);
        }
    }

    /// Every entry is `deterministic=true` — CLI invocations of the
    // same arguments must produce the same registry entry.
    #[test]
    fn all_entries_are_deterministic() {
        for c in all() {
            assert!(c.deterministic, "{} must be deterministic=true", c.id);
        }
    }

    /// Every entry has at least one provider — locks the "no orphan
    // capability" contract.
    #[test]
    fn all_entries_have_at_least_one_provider() {
        for c in all() {
            assert!(
                !c.providers.is_empty(),
                "{} must have at least one provider",
                c.id
            );
        }
    }

    /// All entries are `Stable`/`Available` EXCEPT `cli.plugin` which
    // is `Experimental`/`OptIn`. Locks the single-exception contract
    // so a future addition can't slip through with the wrong combo.
    #[test]
    fn cli_plugin_is_only_experimental_optin() {
        let caps = all();
        for c in &caps {
            if c.id == "cli.plugin" {
                assert_eq!(c.maturity, Maturity::Experimental);
                assert_eq!(c.availability, Availability::OptIn);
            } else {
                assert_eq!(
                    c.maturity,
                    Maturity::Stable,
                    "{} must be Stable (only cli.plugin is Experimental)",
                    c.id
                );
                assert_eq!(
                    c.availability,
                    Availability::Available,
                    "{} must be Available (only cli.plugin is OptIn)",
                    c.id
                );
            }
        }
    }
}
