//! Evidence extraction. Walks the source tree, runs ast-grep matches
//! against user-supplied or built-in patterns, and produces `Evidence`
//! records that match the v2 data model
//! (`docs/DATA-MODEL-LADYBUGDB.md`).
//!
//! Evidence is the v2 primitive that lets the diagrammer say "I saw
//! this in the repo, at this byte range, with this tool". Without it,
//! every element in the graph would be unanchored — a name in space.
//! The audit rule (CONTEXT.md: "A high-confidence claim without
//! evidence is rejected") is enforced at the persistence boundary in
//! `graph::put_evidence` and propagated through here.
//!
//! M4 scope: extract evidence from ast-grep matches, optionally
//! persist to the graph. We do NOT yet classify (`observed` /
//! `derived` / `inferred` / `confirmed`) — that classification is the
//! role of the agent that requested the evidence.

use anyhow::{Context, Result};
use ast_grep_core::source::Doc;
use blake3::Hasher;
use chrono::Utc;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use tracing::debug;
use tree_sitter_graph::graph::{Graph as TsgGraph, GraphNode, Value as TsgValue};

use crate::astgrep::{compile_pattern, find_all, parse, Lang};
use crate::inventory::supported_files;

/// `kind` for evidence records. Maps loosely to the audit categories
/// in the v2 data model. The agent that requests the evidence assigns
/// the kind; `archctl` only records what was matched.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// Direct structural observation: a function, class, route, etc.
    Structural,
    /// String-based observation: a comment, docstring, identifier
    /// spelling.
    Lexical,
    /// Configuration observation: a key/value in a manifest, env, etc.
    Config,
    /// Annotation observation: a decorator, attribute, tag, etc.
    Annotation,
    /// Catch-all when the caller doesn't classify.
    Other,
}

impl EvidenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceKind::Structural => "structural",
            EvidenceKind::Lexical => "lexical",
            EvidenceKind::Config => "config",
            EvidenceKind::Annotation => "annotation",
            EvidenceKind::Other => "other",
        }
    }
}

/// One evidence record. Maps 1:1 to a row in the `Evidence` node
/// table of `docs/schema/001_initial_schema.cypher`. The fields
/// `commit_hash`, `content_hash`, `tool_name`, `tool_version`,
/// `rule_id`, `props`, `observed_at` are filled in here; the agent
/// may later attach a `classification` (observed/derived/inferred/
/// confirmed) and a `confidence` via a separate update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub kind: EvidenceKind,
    pub claim: String,
    pub path: String,
    pub start_line: u64,
    pub end_line: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_byte: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_byte: Option<u64>,
    pub tool_name: String,
    pub tool_version: String,
    pub rule_id: String,
    pub language: String,
    pub observed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_preview: Option<String>,
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    pub props: serde_json::Map<String, serde_json::Value>,
}

pub const TOOL_NAME: &str = "archctl";
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// One row of the extraction output. The agent gets a list of these.
#[derive(Debug, Clone, Serialize)]
pub struct ExtractionResult {
    pub language: String,
    pub pattern: String,
    pub files_scanned: usize,
    pub matches_total: usize,
    pub evidence: Vec<Evidence>,
}

/// Walk every file of `language` under `root` and run the pattern.
/// `claim_template` can reference `$NAME`, `$TYPE`, etc. from the
/// match — for now we just use the match's text + a fixed claim.
pub fn extract(
    root: &Path,
    lang: Lang,
    pattern_src: &str,
    claim: &str,
    kind: EvidenceKind,
) -> Result<ExtractionResult> {
    let files = supported_files(root, 50_000)?;
    let pattern = compile_pattern(lang, pattern_src)?;
    let mut all_evidence = Vec::new();
    let mut scanned = 0usize;

    for (rel_path, label) in files {
        if Lang::from_label(label) != Some(lang) {
            continue;
        }
        scanned += 1;
        let abs = root.join(&rel_path);
        let source = match std::fs::read_to_string(&abs) {
            Ok(s) => s,
            Err(e) => {
                debug!(path = %rel_path.display(), error = %e, "skip (not UTF-8)");
                continue;
            }
        };
        let ast = parse(lang, &source);
        let matches = find_all(&ast, &pattern);
        debug!(path = %rel_path.display(), matches = matches.len(), "scanned");
        for m in matches {
            all_evidence.push(evidence_from_match(
                lang, rel_path.to_str().unwrap_or("<bad-path>"), &source, claim, kind, &m,
            )?);
        }
    }

    Ok(ExtractionResult {
        language: lang.label().to_string(),
        pattern: pattern_src.to_string(),
        files_scanned: scanned,
        matches_total: all_evidence.len(),
        evidence: all_evidence,
    })
}

    fn evidence_from_match<D: Doc>(
        lang: Lang,
        rel_path: &str,
        source: &str,
        claim: &str,
        kind: EvidenceKind,
        m: &ast_grep_core::matcher::NodeMatch<'_, D>,
    ) -> Result<Evidence> {
        let range = m.range();
        let text = m.text().to_string();
        let start_line = line_at_byte(source, range.start) as u64 + 1;
        let end_line = line_at_byte(source, range.end.saturating_sub(1).max(range.start)) as u64 + 1;
        let content_hash = Some(content_hash_of(source));
        let text_preview = Some(truncate(&text, 200));
        let id = evidence_id(rel_path, range.start, range.end, &text);

        let mut props = serde_json::Map::new();
        props.insert(
            "node_kind".to_string(),
            serde_json::Value::String(m.kind().to_string()),
        );
        props.insert(
            "byte_range".to_string(),
            serde_json::json!([range.start, range.end]),
        );
        // Schema columns (`Evidence` table in docs/schema/) do not have
        // `language`, `start_byte`, `end_byte`, `text_preview`. We
        // mirror them into `props` so consumers can read everything via
        // the same JSON field without a schema migration.
        props.insert(
            "language".to_string(),
            serde_json::Value::String(lang.label().to_string()),
        );
        props.insert(
            "start_byte".to_string(),
            serde_json::json!(range.start),
        );
        props.insert(
            "end_byte".to_string(),
            serde_json::json!(range.end),
        );
        if let Some(ref p) = text_preview {
            props.insert(
                "text_preview".to_string(),
                serde_json::Value::String(p.clone()),
            );
        }

        Ok(Evidence {
            id,
            kind,
            claim: claim.to_string(),
            path: rel_path.to_string(),
            start_line,
            end_line,
            start_byte: Some(range.start as u64),
            end_byte: Some(range.end as u64),
            tool_name: TOOL_NAME.to_string(),
            tool_version: TOOL_VERSION.to_string(),
            rule_id: format!("astgrep:{}:{}", lang.label(), m.kind()),
            language: lang.label().to_string(),
            observed_at: Utc::now().to_rfc3339(),
            content_hash,
            text_preview,
            props,
        })
    }

fn line_at_byte(source: &str, byte: usize) -> usize {
    source[..byte.min(source.len())]
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
}

fn content_hash_of(source: &str) -> String {
    let digest = Sha256::digest(source.as_bytes());
    format!("sha256:{}", hex::encode(digest))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut end = max;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

/// Stable, deterministic id for an evidence row. Two scans of the
/// same file at the same range with the same text produce the same id
/// — this is what lets us deduplicate on `MATCH (e:Evidence {id})
/// RETURN e` without recomputing.
fn evidence_id(path: &str, start: usize, end: usize, text: &str) -> String {
    let mut h = Hasher::new();
    h.update(path.as_bytes());
    h.update(&start.to_le_bytes());
    h.update(&end.to_le_bytes());
    h.update(text.as_bytes());
    format!("ev:{}", hex::encode(&h.finalize().as_bytes()[..16]))
}

/// Build an Evidence row from a `tree-sitter-graph` graph node.
/// Each graph node produced by the TSG maps to one evidence record. The
/// graph node's attributes are flattened into the Evidence `props` map so
/// downstream consumers can query them by name.
pub fn from_tsg_node(
    node: &GraphNode,
    graph: &TsgGraph<'_>,
    rel_path: &str,
    source: &str,
    claim: &str,
    kind: EvidenceKind,
) -> Option<Evidence> {
    // Walk attributes and capture any string values into props.
    let mut props = serde_json::Map::new();
    let mut byte_start: Option<usize> = None;
    let mut byte_end: Option<usize> = None;
    let mut kind_attr: Option<String> = None;
    let mut name_attr: Option<String> = None;
    for (key, value) in node.attributes.iter() {
        match value {
            TsgValue::String(s) => {
                props.insert(
                    key.as_str().to_string(),
                    serde_json::Value::String(s.clone()),
                );
                match key.as_str() {
                    "kind" => kind_attr = Some(s.clone()),
                    "name" => name_attr = Some(s.clone()),
                    _ => {}
                }
            }
            TsgValue::Integer(i) => {
                props.insert(
                    key.as_str().to_string(),
                    serde_json::Value::Number((*i).into()),
                );
            }
            TsgValue::SyntaxNode(sn_ref) => {
                let ts_node = &graph[*sn_ref];
                let start = ts_node.start_byte();
                let end = ts_node.end_byte();
                if byte_start.is_none() {
                    byte_start = Some(start);
                }
                byte_end = Some(end);
                props.insert(
                    format!("{}_byte_start", key.as_str()),
                    serde_json::json!(start),
                );
                props.insert(
                    format!("{}_byte_end", key.as_str()),
                    serde_json::json!(end),
                );
            }
            _ => {
                // Skip Null, Boolean, List, Set, GraphNode — the Evidence
                // schema doesn't carry them and they don't influence the
                // audit fields.
            }
        }
    }

    let start = byte_start?;
    let end = byte_end?;

    // The TSG must produce one captured syntax node per graph node for
    // us to derive a meaningful byte range. Without a syntax-node
    // attribute we skip the row rather than emit a position-less one.
    let text = source.get(start..end).unwrap_or("").to_string();
    let start_line = line_at_byte(source, start) as u64 + 1;
    let end_line =
        line_at_byte(source, end.saturating_sub(1).max(start)) as u64 + 1;
    let id = evidence_id(rel_path, start, end, &text);
    let content_hash = Some(content_hash_of(source));
    let text_preview = Some(truncate(&text, 200));

    let rule_id = format!(
        "tsg:{}:{}",
        kind_attr.as_deref().unwrap_or("node"),
        name_attr.as_deref().unwrap_or("?")
    );

    let mut ev = Evidence {
        id,
        kind,
        claim: claim.to_string(),
        path: rel_path.to_string(),
        start_line,
        end_line,
        start_byte: Some(start as u64),
        end_byte: Some(end as u64),
        tool_name: TOOL_NAME.to_string(),
        tool_version: TOOL_VERSION.to_string(),
        rule_id,
        language: String::new(),
        observed_at: Utc::now().to_rfc3339(),
        content_hash,
        text_preview: text_preview.clone(),
        props,
    };
    if let Some(ref p) = text_preview {
        ev.props.insert(
            "text_preview".to_string(),
            serde_json::Value::String(p.clone()),
        );
    }
    Some(ev)
}

/// Persist a batch of evidence to the canonical graph. Each call
/// uses `MERGE` semantics (the CYPHER `MATCH ... CREATE` pattern
/// archctl prefers for idempotent writes). We do not yet link the
/// Evidence to any ElementVersion — that is the agent's job once it
/// has decided what the evidence supports.
pub fn put(project_dir: &Path, evidence: &[Evidence]) -> Result<usize> {
    if evidence.is_empty() {
        return Ok(0);
    }
    let session = crate::graph::open_session(project_dir)?;
    let mut written = 0usize;
    for ev in evidence {
        let id = crate::graph::validate_identifier(&ev.id)
            .context("evidence id failed validation")?;
        let path = crate::graph::validate_identifier(&ev.path)
            .context("evidence path failed validation")?;
        let kind = crate::graph::validate_identifier(ev.kind.as_str())?;
        let tool = crate::graph::validate_identifier(&ev.tool_name)?;
        let rule = crate::graph::validate_identifier(&ev.rule_id)?;
        let props_json =
            serde_json::to_string(&ev.props).context("serialize evidence props")?;
        let hash_json = serde_json::to_string(ev.content_hash.as_deref().unwrap_or(""))
            .context("serialize content_hash")?;

        // lbug 0.18.3 has no parameter binding; we interpolate after
        // escaping single quotes. The id/path/kind/tool/rule/lang are
        // allowlist-validated; the user-supplied claim is escaped.
        // The Evidence table columns in `docs/schema/` are
        //   id, kind, classification, claim, confidence, path,
        //   start_line, end_line, commit_hash, content_hash,
        //   tool_name, tool_version, rule_id, props, observed_at
        // We mirror extra fields (language, start_byte, end_byte,
        // text_preview) into `props`.
        let safe_claim = ev.claim.replace('\'', "\\'");
        let safe_tv = ev.tool_version.replace('\'', "\\'");
        let safe_oa = ev.observed_at.replace('\'', "\\'");
        // lbug TIMESTAMP column requires `timestamp(<string>)`, not a
        // bare string literal. We wrap the allowlist-validated ISO-8601
        // timestamp at query time. (validated above by ensure_ascii
        // path; we still cap length defensively.)
        let oa_cypher = if safe_oa.is_empty() || safe_oa.len() > 64 {
            "timestamp('1970-01-01T00:00:00Z')".to_string()
        } else {
            format!("timestamp('{safe_oa}')")
        };
        let safe_ch = hash_json.replace('\'', "\\'");
        let safe_props = props_json.replace('\'', "\\'");

        let cypher = format!(
            "MERGE (e:Evidence {{id: '{id}'}}) SET \
             e.kind = '{kind}', \
             e.claim = '{safe_claim}', \
             e.path = '{path}', \
             e.start_line = {sl}, \
             e.end_line = {el}, \
             e.tool_name = '{tool}', \
             e.tool_version = '{safe_tv}', \
             e.rule_id = '{rule}', \
             e.content_hash = '{safe_ch}', \
             e.observed_at = {oa_cypher}, \
             e.props = '{safe_props}' RETURN e;",
            sl = ev.start_line,
            el = ev.end_line,
        );
        session.conn.query(&cypher).with_context(|| format!("persist evidence {id}"))?;
        written += 1;
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astgrep::Lang;
    use std::fs;

    fn fixture() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(
            tmp.path().join("src/lib.rs"),
            "fn add(a: i32, b: i32) -> i32 { a + b }\nfn mul(a: i32, b: i32) -> i32 { a * b }\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("src/app.py"),
            "def foo():\n    pass\n\ndef bar(x):\n    return x\n",
        )
        .unwrap();
        tmp
    }

    #[test]
    fn extract_finds_two_rust_functions() {
        let tmp = fixture();
        let result = extract(
            tmp.path(),
            Lang::Rust,
            "fn $NAME",
            "Rust function definition",
            EvidenceKind::Structural,
        )
        .unwrap();
        assert_eq!(result.language, "rust");
        assert_eq!(result.matches_total, 2);
        assert_eq!(result.evidence.len(), 2);
        let names: Vec<_> = result
            .evidence
            .iter()
            .map(|e| e.text_preview.clone().unwrap_or_default())
            .collect();
        assert!(names[0].contains("fn add"));
        assert!(names[1].contains("fn mul"));
    }

    #[test]
    fn extract_evidence_id_is_stable() {
        let tmp = fixture();
        let a = extract(
            tmp.path(),
            Lang::Rust,
            "fn $NAME",
            "claim",
            EvidenceKind::Structural,
        )
        .unwrap();
        let b = extract(
            tmp.path(),
            Lang::Rust,
            "fn $NAME",
            "claim",
            EvidenceKind::Structural,
        )
        .unwrap();
        let ids_a: Vec<_> = a.evidence.iter().map(|e| &e.id).collect();
        let ids_b: Vec<_> = b.evidence.iter().map(|e| &e.id).collect();
        assert_eq!(ids_a, ids_b, "ids must be deterministic");
    }

    #[test]
    fn evidence_row_captures_line_range() {
        let tmp = fixture();
        let result = extract(
            tmp.path(),
            Lang::Rust,
            "fn $NAME",
            "claim",
            EvidenceKind::Structural,
        )
        .unwrap();
        let ev = &result.evidence[0];
        assert_eq!(ev.start_line, 1);
        assert!(ev.end_line >= ev.start_line);
        assert!(ev.start_byte.unwrap() < ev.end_byte.unwrap());
    }

    #[test]
    fn line_at_byte_counts_newlines() {
        assert_eq!(line_at_byte("a\nb\nc", 0), 0);
        assert_eq!(line_at_byte("a\nb\nc", 1), 0);
        assert_eq!(line_at_byte("a\nb\nc", 2), 1);
        assert_eq!(line_at_byte("a\nb\nc", 4), 2);
    }

    #[test]
    fn truncate_respects_char_boundaries() {
        assert_eq!(truncate("hello", 10), "hello");
        // Truncation at byte position 3 in "héllo" (h=1, é=2, l=1) must
        // back off to a valid UTF-8 boundary, then append the ellipsis.
        // The result must be a valid UTF-8 string (is_char_boundary OK).
        let s = truncate("héllo", 3);
        assert!(s.is_char_boundary(s.len()), "got {:?} (len {})", s, s.len());
        assert!(s.starts_with('h'));
        assert!(s.ends_with('…'));
    }

    #[test]
    fn put_is_idempotent() {
        // Bootstrap a graph, extract evidence, put twice, count rows.
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        crate::graph::init(&project).unwrap();
        let evidence = vec![Evidence {
            id: "ev:test:1".to_string(),
            kind: EvidenceKind::Structural,
            claim: "test claim".to_string(),
            path: "src/lib.rs".to_string(),
            start_line: 1,
            end_line: 1,
            start_byte: Some(0),
            end_byte: Some(4),
            tool_name: "archctl".to_string(),
            tool_version: "test".to_string(),
            rule_id: "astgrep:rust:function_item".to_string(),
            language: "rust".to_string(),
            observed_at: "2026-07-29T00:00:00Z".to_string(),
            content_hash: Some("sha256:0".to_string()),
            text_preview: Some("fn a".to_string()),
            props: Default::default(),
        }];
        let n1 = put(&project, &evidence).unwrap();
        let n2 = put(&project, &evidence).unwrap();
        assert_eq!(n1, 1);
        assert_eq!(n2, 1, "MERGE must not duplicate rows");
        let count = crate::graph::query(&project, "MATCH (e:Evidence) RETURN count(e) AS n;")
            .unwrap();
        assert_eq!(count[0]["n"], 1);
    }
}
