//! JSON serialization + human table formatting for discover reports.

use crate::code::c4_discover::DiscoverReport;

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
