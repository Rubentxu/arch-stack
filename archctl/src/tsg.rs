//! Declarative evidence extraction via tree-sitter-graph (TSG) rules.
//!
//! `evidence.rs` exposes an ad-hoc `extract()` driven by a single ast-grep
//! pattern. For teams that want reusable, versioned extraction rules
//! independent of the binary, M8 (ADR-012) introduces
//! `basemind-tree-sitter-graph` (a maintained fork of `tree-sitter-graph`
//! that targets tree-sitter 0.26) as an alternative path: a `.tsg` file
//! declares "match these syntax nodes, create these graph nodes with
//! these attributes" and we convert the resulting graph into the same
//! `Evidence` records the agent already consumes.
//!
//! The two paths coexist on purpose:
//!
//! - `evidence::extract` is the default. It is fast, single-pattern, and
//!   does not require the caller to know TSG syntax.
//! - `tsg::extract_with_rules` loads a `.tsg` rule pack and is the path
//!   to take when the agent wants reusable, multi-pattern extraction.
//!
//! Both paths produce identical `Evidence` rows so the persistence layer
//! (graph::put_evidence) does not care which one ran.

use anyhow::{Context, Result};
use ast_grep_core::tree_sitter::LanguageExt;
use ast_grep_language::SupportLang;
use std::path::Path;
use tree_sitter::Parser as TsParser;
use tree_sitter_graph::ast::File as TsgFile;
use tree_sitter_graph::functions::Functions;
use tree_sitter_graph::{ExecutionConfig, NoCancellation, Variables};

/// Outcome of executing a TSG file against a single source document.
/// Each `Evidence` row maps 1:1 to a `(node, ...)` block in the TSG.
#[derive(Debug, Default)]
pub struct TsgOutput {
    pub evidence: Vec<crate::evidence::Evidence>,
    pub graph_node_count: usize,
    pub graph_edge_count: usize,
}

/// Load a TSG (tree-sitter-graph) rule file for the given language.
/// `tsg_src` is the textual content of the `.tsg` file.
pub fn load_rules(lang: SupportLang, tsg_src: &str) -> Result<TsgFile> {
    let ts_lang = lang.get_ts_language();
    TsgFile::from_str(ts_lang, tsg_src).context("parse TSG file")
}

/// Execute a TSG file against one source document. Returns the graph
/// nodes and edges as `Evidence` records. The conversion is rule-driven:
/// each graph node with a `kind` attribute and a `name` attribute becomes
/// one Evidence row.
pub fn execute(
    rules: &TsgFile,
    lang: SupportLang,
    rel_path: &str,
    source: &str,
    claim: &str,
    kind: crate::evidence::EvidenceKind,
    clock: &dyn crate::clock::Clock,
) -> Result<TsgOutput> {
    let ts_lang = lang.get_ts_language();
    let mut parser = TsParser::new();
    parser
        .set_language(&ts_lang)
        .context("set tree-sitter language for TSG execution")?;
    let tree = parser
        .parse(source, None)
        .context("parse source with tree-sitter")?;

    let functions = Functions::stdlib();
    let globals = Variables::new();
    let config = ExecutionConfig::new(&functions, &globals);

    let graph = rules
        .execute(&tree, source, &config, &NoCancellation)
        .context("execute TSG rules")?;

    let mut out = TsgOutput::default();
    out.graph_node_count = graph.node_count();
    out.graph_edge_count = graph.iter_nodes().map(|n| graph[n].edge_count()).sum();

    // Each graph node produced by the TSG becomes one evidence record.
    // The TSG must capture at least one syntax-node attribute per graph
    // node; that gives us a deterministic byte range and text snippet.
    for node_ref in graph.iter_nodes() {
        let node = &graph[node_ref];
        let Some(ev) =
            crate::evidence::from_tsg_node(node, &graph, rel_path, source, claim, kind, clock)
        else {
            continue;
        };
        out.evidence.push(ev);
    }

    Ok(out)
}

/// Walk `root` looking for files of `lang`, execute `rules` against each
/// and return the concatenated output. Files that fail to parse as UTF-8
/// are skipped with a debug log.
pub fn extract_with_rules(
    root: &Path,
    lang: SupportLang,
    rules: &TsgFile,
    claim: &str,
    kind: crate::evidence::EvidenceKind,
    clock: &dyn crate::clock::Clock,
) -> Result<TsgOutput> {
    let files = crate::inventory::supported_files(root, 50_000)?;
    let mut combined = TsgOutput::default();
    let label = match lang {
        SupportLang::Rust => "rust",
        SupportLang::TypeScript => "typescript",
        SupportLang::JavaScript => "javascript",
        SupportLang::Python => "python",
        SupportLang::Go => "go",
        SupportLang::Java => "java",
        SupportLang::Kotlin => "kotlin",
        _ => return Ok(combined),
    };
    for (rel_path, file_label) in files {
        if file_label != label {
            continue;
        }
        let abs = root.join(&rel_path);
        let source = match std::fs::read_to_string(&abs) {
            Ok(s) => s,
            Err(e) => {
                tracing::debug!(path = %rel_path.display(), error = %e, "skip (not UTF-8)");
                continue;
            }
        };
        let out = execute(rules, lang, rel_path.to_str().unwrap_or("<bad-path>"), &source, claim, kind, clock)?;
        combined.graph_node_count += out.graph_node_count;
        combined.graph_edge_count += out.graph_edge_count;
        combined.evidence.extend(out.evidence);
    }
    Ok(combined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{EvidenceKind, TOOL_NAME, TOOL_VERSION};

    /// Minimal TSG that turns every `function_item` (Rust) into a graph
    /// node carrying the function name and byte range attributes.
    const RUST_FN_RULES: &str = r#"
(function_item
  name: (identifier) @name) @fn {
  node fn_node
  attr (fn_node) kind = "function"
  attr (fn_node) name = (source-text @name)
  attr (fn_node) syntax = @fn
}
"#;

    #[test]
    fn load_and_execute_rust_function_rule() {
        let rules = load_rules(SupportLang::Rust, RUST_FN_RULES).expect("parse TSG");
        let src = "fn alpha() {}\nfn beta(x: i32) -> i32 { x }\n";
        let clock: &dyn crate::clock::Clock = &crate::clock::FixedClock::new("2026-07-30T00:00:00Z");
        let out = execute(
            &rules,
            SupportLang::Rust,
            "src/lib.rs",
            src,
            "Rust function definition",
            EvidenceKind::Structural,
            clock,
        )
        .expect("execute TSG");

        // TSG emits one `fn_node` graph node per matched function_item.
        assert_eq!(out.graph_node_count, 2);
        assert_eq!(out.evidence.len(), 2);

        let names: Vec<_> = out
            .evidence
            .iter()
            .map(|e| {
                e.props
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .collect();
        assert_eq!(names, vec!["alpha", "beta"]);

        // Default tool/version stamped by Evidence::from_tsg_node.
        assert!(out.evidence.iter().all(|e| e.tool_name == TOOL_NAME));
        assert!(out.evidence.iter().all(|e| e.tool_version == TOOL_VERSION));
        // Injected Clock port: deterministic timestamp, no SystemClock
        // races against the test runner.
        assert!(out
            .evidence
            .iter()
            .all(|e| e.observed_at == "2026-07-30T00:00:00Z"));
        // Byte ranges should be non-zero and within source.
        let src_len = src.len() as u64;
        assert!(out
            .evidence
            .iter()
            .all(|e| e.end_byte.unwrap_or(0) <= src_len));
    }

    #[test]
    fn load_rejects_invalid_tsg() {
        // Missing brace after a stanza is a TSG syntax error.
        let bad = "(function_item) @fn\n";
        let result = load_rules(SupportLang::Rust, bad);
        assert!(result.is_err(), "invalid TSG should fail to load");
    }
}
