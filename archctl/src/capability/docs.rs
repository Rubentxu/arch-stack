//! Markdown emitter for the capability registry.
//!
//! Output is deterministic: sorted by `id`, then `provider.language`.
//! Byte-identical across runs (no HashMap, no time-based values).

use std::collections::BTreeMap;

use crate::capability::{Capability, CapabilityRegistry};

/// Render the registry as a Markdown table.
///
/// Sorting rules:
/// - Primary: `id` (ascending)
/// - Secondary: `provider.language` (ascending within each capability)
/// - BTreeMap provides deterministic iteration order.
pub fn render_markdown(reg: &CapabilityRegistry) -> String {
    // Group providers by capability, sorted by id.
    let mut lines: Vec<String> = Vec::with_capacity(reg.len() * 8 + 4);

    lines.push(String::from("# Capability Registry"));
    lines.push(String::from(""));
    lines.push(String::from(
        "| ID | Category | Maturity | Deterministic | Availability | Providers |",
    ));
    lines.push(String::from(
        "|----|----------|----------|---------------|--------------|-----------|",
    ));

    // BTreeMap for deterministic sort by id.
    let mut caps: BTreeMap<&str, &Capability> = BTreeMap::new();
    for cap in reg.iter() {
        caps.insert(&cap.id, cap);
    }

    for (_id, cap) in caps {
        // Providers sorted by language.
        let mut providers: BTreeMap<&str, &str> = BTreeMap::new();
        for p in &cap.providers {
            providers.insert(p.language.as_str(), p.maturity.label());
        }
        let provider_str: String = providers
            .iter()
            .map(|(lang, mat)| format!("{lang} ({mat})"))
            .collect::<Vec<_>>()
            .join(", ");

        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            cap.id,
            cap.category.label(),
            cap.maturity.label(),
            cap.deterministic,
            cap.availability.label(),
            provider_str,
        ));
    }

    lines.push(String::new());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{Availability, Category, Maturity, Provider};

    fn tiny_reg() -> CapabilityRegistry {
        let mut reg = CapabilityRegistry::new();
        reg.add(Capability::new(
            "render.plantuml",
            Category::Render,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![Provider::new("java", Maturity::Stable)],
        ));
        reg.add(Capability::new(
            "code.call_graph",
            Category::Code,
            Maturity::Stable,
            true,
            Availability::Available,
            vec![
                Provider::new("rust", Maturity::Stable),
                Provider::new("go", Maturity::Beta),
            ],
        ));
        reg
    }

    #[test]
    fn markdown_sorted_by_id() {
        let reg = tiny_reg();
        let md = render_markdown(&reg);
        let lines: Vec<&str> = md.lines().collect();
        // "code.call_graph" < "render.plantuml" alphabetically.
        let id_line = lines
            .iter()
            .find(|l| l.contains("code.call_graph"))
            .unwrap();
        let render_line = lines
            .iter()
            .find(|l| l.contains("render.plantuml"))
            .unwrap();
        let id_idx = lines.iter().position(|l| *l == *id_line).unwrap();
        let render_idx = lines.iter().position(|l| *l == *render_line).unwrap();
        assert!(
            id_idx < render_idx,
            "code.call_graph should appear before render.plantuml"
        );
    }

    #[test]
    fn markdown_byte_identical_runs() {
        let reg = tiny_reg();
        let first = render_markdown(&reg);
        let second = render_markdown(&reg);
        assert_eq!(
            first, second,
            "markdown output must be byte-identical across runs"
        );
    }

    #[test]
    fn markdown_provider_language_sorted() {
        let reg = tiny_reg();
        let md = render_markdown(&reg);
        let line = md.lines().find(|l| l.contains("code.call_graph")).unwrap();
        // go (beta) should appear before rust (stable) alphabetically.
        let go_pos = line.find("go").unwrap();
        let rust_pos = line.find("rust").unwrap();
        assert!(
            go_pos < rust_pos,
            "go provider should appear before rust in the providers column"
        );
    }
}
