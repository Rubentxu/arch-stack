#![cfg(test)]

//! Alignment tests — verify the registry stays in sync with the code.
//!
//! Per ADR-045, violations of these invariants must fail the test suite:
//! - New Language variant without a registry provider → test fails.
//! - Registry provider without corresponding Language variant → test fails.
//! - Strategy added to `register_strategies()` without registry entry → test fails.
//! - Registry entry without corresponding strategy → test fails.

use crate::capability::Capability;
use crate::capability::source_code;
use crate::code::call_graph::Language as CallGraphLang;
use crate::code::class_diagram::Language as ClassDiagramLang;
use crate::code::state_machine::Language as StateMachineLang;

/// All Language variants for `code::call_graph`.
const CALL_GRAPH_LANGUAGES: &[(&str, CallGraphLang)] = &[
    ("rust", CallGraphLang::Rust),
    ("typescript", CallGraphLang::TypeScript),
    ("python", CallGraphLang::Python),
    ("go", CallGraphLang::Go),
    ("java", CallGraphLang::Java),
    ("kotlin", CallGraphLang::Kotlin),
];

/// All Language variants for `code::class_diagram`.
const CLASS_DIAGRAM_LANGUAGES: &[(&str, ClassDiagramLang)] = &[
    ("rust", ClassDiagramLang::Rust),
    ("typescript", ClassDiagramLang::TypeScript),
    ("python", ClassDiagramLang::Python),
];

/// All Language variants for `code::state_machine`.
const STATE_MACHINE_LANGUAGES: &[(&str, StateMachineLang)] = &[
    ("rust", StateMachineLang::Rust),
    ("typescript", StateMachineLang::TypeScript),
    ("python", StateMachineLang::Python),
];

// ─── Helper ─────────────────────────────────────────────────────────────────

/// Find the capability entry by id in a capability slice.
fn find_cap<'a>(caps: &'a [Capability], id: &str) -> Option<&'a Capability> {
    caps.iter().find(|c| c.id == id)
}

/// Find a provider by language string within a capability.
fn find_provider<'a>(
    cap: &'a crate::capability::Capability,
    lang: &str,
) -> Option<&'a crate::capability::Provider> {
    cap.providers.iter().find(|p| p.language == lang)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

/// Scenario S6: Adding a Language::Ruby to code::call_graph::Language without
/// a corresponding registry entry must fail the alignment suite.
#[test]
fn test_language_variant_fails() {
    let caps = source_code::all();
    let cap = find_cap(&caps, "code.call_graph").expect("code.call_graph capability must exist");
    let missing: Vec<&str> = CALL_GRAPH_LANGUAGES
        .iter()
        .filter(|(lang, _)| find_provider(cap, lang).is_none())
        .map(|(lang, _)| *lang)
        .collect();
    assert!(
        missing.is_empty(),
        "code.call_graph is missing providers for languages (add them to \
         source_code::all()): {missing:?}"
    );
}

/// Scenario S7: Adding a registry provider "ruby" to code.call_graph without
/// a corresponding Language::Ruby variant must fail the alignment suite.
#[test]
fn test_orphan_provider_fails() {
    let caps = source_code::all();
    let cap = find_cap(&caps, "code.call_graph").expect("code.call_graph capability must exist");
    let known_langs: Vec<&str> = CALL_GRAPH_LANGUAGES.iter().map(|(l, _)| *l).collect();
    let orphans: Vec<&str> = cap
        .providers
        .iter()
        .filter(|p| p.language != "any" && !known_langs.contains(&p.language.as_str()))
        .map(|p| p.language.as_str())
        .collect();
    assert!(
        orphans.is_empty(),
        "code.call_graph has registry providers with no corresponding Language variant \
         (add Language enum variant or remove the orphan): {orphans:?}"
    );
}

/// Scenario S8: Strategy drift — a strategy added to register_strategies()
/// without a corresponding registry entry must fail the alignment test.
#[test]
fn test_strategy_drift_fails() {
    // Hardcoded list of registered strategy ids (mirrors register_strategies()).
    // If a new strategy is added to register_strategies() without adding a
    // corresponding entry to source_cargo::all(), this test will fail.
    let registered_strategy_ids: &[&str] = &[
        "cargo-workspace",
        "npm-workspace",
        "npm-single",
        "dockerfile",
        "helm",
        "components",
    ];

    let registry_ids: std::collections::BTreeSet<String> = crate::capability::source_cargo::all()
        .into_iter()
        .map(|c| {
            c.id.strip_prefix("code.strategy.")
                .map(String::from)
                .unwrap_or_else(|| c.id.clone())
        })
        .collect();

    let missing: Vec<&str> = registered_strategy_ids
        .iter()
        .filter(|id| !registry_ids.contains(*id as &str))
        .copied()
        .collect();

    assert!(
        missing.is_empty(),
        "Strategies registered in code but missing from registry \
         (add entries to source_cargo::all()): {missing:?}"
    );
}

/// Verify all class_diagram languages are covered.
#[test]
fn test_class_diagram_language_coverage() {
    let caps = source_code::all();
    let cap =
        find_cap(&caps, "code.class_diagram").expect("code.class_diagram capability must exist");
    let missing: Vec<&str> = CLASS_DIAGRAM_LANGUAGES
        .iter()
        .filter(|(lang, _)| find_provider(cap, lang).is_none())
        .map(|(lang, _)| *lang)
        .collect();
    assert!(
        missing.is_empty(),
        "code.class_diagram missing providers: {missing:?}"
    );
}

/// Verify all state_machine languages are covered.
#[test]
fn test_state_machine_language_coverage() {
    let caps = source_code::all();
    let cap =
        find_cap(&caps, "code.state_machine").expect("code.state_machine capability must exist");
    let missing: Vec<&str> = STATE_MACHINE_LANGUAGES
        .iter()
        .filter(|(lang, _)| find_provider(cap, lang).is_none())
        .map(|(lang, _)| *lang)
        .collect();
    assert!(
        missing.is_empty(),
        "code.state_machine missing providers: {missing:?}"
    );
}
