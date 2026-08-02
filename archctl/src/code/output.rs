//! JSON serialization + human table formatting for discover reports.

use std::collections::BTreeMap;

use crate::code::c4_discover::DiscoverReport;
use crate::code::call_graph::{CallGraphReport, FunctionNode, Language};
use crate::code::class_diagram::{ClassDiagramReport, ClassNode, ClassEdgeKind, Language as CdLanguage, TypeKind};
use crate::code::sequence::SequenceReport;

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

/// Print a human-readable class-diagram table.
pub fn print_class_diagram_table(report: &ClassDiagramReport) {
    println!(
        "Class diagram ({} nodes, {} edges, {} errors, {} ms)",
        report.nodes.len(),
        report.edges.len(),
        report.errors.len(),
        report.project.duration_ms
    );

    // Group nodes by language and kind
    let mut by_lang_kind: BTreeMap<(CdLanguage, TypeKind), Vec<&ClassNode>> = BTreeMap::new();
    for node in &report.nodes {
        by_lang_kind
            .entry((node.language, node.kind))
            .or_default()
            .push(node);
    }

    for ((lang, kind), nodes) in &by_lang_kind {
        println!("  {lang:?} {kind:?}: {} type(s)", nodes.len());
        for node in &**nodes {
            println!(
                "    ✓ {}  {}:{}  (conf {:.2}){}",
                node.name,
                node.file,
                node.line,
                node.confidence,
                if node.members.is_empty() {
                    String::new()
                } else {
                    format!("  ({} members)", node.members.len())
                }
            );
        }
    }

    if !report.edges.is_empty() {
        println!("\nEdges ({}):", report.edges.len());
        for edge in &report.edges {
            let pred = match edge.predicate {
                ClassEdgeKind::Extends => "extends",
                ClassEdgeKind::Implements => "implements",
                ClassEdgeKind::Composes => "composes",
            };
            // Extract short names from canonical keys
            let src_name = edge.source.split(':').nth(3).unwrap_or(&edge.source);
            let tgt_name = edge.target.split(':').nth(3).unwrap_or(&edge.target);
            println!("    {} --{pred}--> {}  (conf {:.2})", src_name, tgt_name, edge.confidence);
        }
    }

    if !report.errors.is_empty() {
        println!("\nErrors ({}):", report.errors.len());
        for err in &report.errors {
            println!("  {}: {}", err.path, err.message);
        }
    }
}

/// Print a human-readable sequence table.
pub fn print_sequence_table(report: &SequenceReport) {
    println!(
        "Sequence from {:?} ({} interactions, {} ms)",
        report.from,
        report.interactions.len(),
        report.duration_ms
    );
    if report.cyclic {
        println!("  ⚠ cyclic (some callees were already visited)");
    }
    if report.truncated {
        println!(
            "  ⚠ truncated (hit max-interactions cap; {} total reachable)",
            report.total_reachable
        );
    }
    for i in &report.interactions {
        println!(
            "  {:3}. [d{}] {} → {} ({:?})",
            i.order_key,
            i.depth,
            i.sender,
            i.receiver,
            i.message_kind
        );
        if let (Some(f), Some(l)) = (&i.file, &i.line) {
            println!("        at {}:{}", f.display(), l);
        }
    }
}
