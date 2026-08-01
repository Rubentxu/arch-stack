//! JSON serialization + human table formatting for discover reports.

use std::collections::BTreeMap;

use crate::code::c4_discover::DiscoverReport;
use crate::code::call_graph::{CallGraphReport, FunctionNode, Language};

/// Print a human-readable table of discovered Containers grouped by strategy.
pub fn print_human_table(report: &DiscoverReport) {
    if report.discovered.is_empty() {
        println!("No C4 Container boundaries detected.");
        return;
    }

    // Group by strategy
    let mut by_strategy: std::collections::BTreeMap<&str, Vec<&crate::code::c4_discover::Container>> =
        std::collections::BTreeMap::new();

    for container in &report.discovered {
        by_strategy
            .entry(container.strategy.as_str())
            .or_default()
            .push(container);
    }

    let total = report.discovered.len();
    println!("Container candidates ({} strategies, {} candidates):\n",
        by_strategy.len(), total);

    for (strategy, containers) in &by_strategy {
        let confidence = containers.first().map(|c| c.confidence).unwrap_or(0.0);
        println!("  {} ({} candidates, confidence {:.2})",
            strategy, containers.len(), confidence);
        for container in containers {
            let line = container.evidences.first()
                .map(|e| format!("{}:{}", e.file, e.line))
                .unwrap_or_default();
            println!("    ✓ {}    {}", container.canonical_key, line);
        }
        println!();
    }
}

/// Print a human-readable table of call-graph nodes grouped by language.
pub fn print_call_graph_table(report: &CallGraphReport) {
    println!(
        "Call graph ({} nodes, {} edges, {} errors, {} ms)",
        report.nodes.len(),
        report.edges.len(),
        report.errors.len(),
        report.project.duration_ms
    );

    // Group nodes by language
    let mut by_lang: BTreeMap<Language, Vec<&FunctionNode>> = BTreeMap::new();
    for node in &report.nodes {
        by_lang
            .entry(node.language)
            .or_default()
            .push(node);
    }

    for (lang, nodes) in &by_lang {
        println!("  {lang:?}: {} function(s)", nodes.len());
        for node in &**nodes {
            let fq = if node.fq_name.is_empty() {
                &node.name
            } else {
                &node.fq_name
            };
            println!("    ✓ {}  {}:{}  (conf {:.2})", fq, node.file, node.line, node.confidence);
        }
    }

    if !report.errors.is_empty() {
        println!("\nErrors ({}):", report.errors.len());
        for err in &report.errors {
            println!("  [{}] {}: {}", err.strategy, err.path, err.message);
        }
    }
}
