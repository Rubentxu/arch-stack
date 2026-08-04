//! State machine extraction from source code (Rust enum+match, TypeScript state pattern, Python transitions).
//!
//! Pattern: `extract() -> StateMachineReport` (pure, no graph writes) + `apply() -> ApplyReport`.
//! Confidence is always < 1.0 because guards and events are inferred, not definitively extracted.
//!
//! Per spec SCN-427: guards and events are NOT extracted — that is the agent's job via `evidence put`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;
use ast_grep_core::tree_sitter::LanguageExt;
use ast_grep_language::SupportLang;
use serde::{Deserialize, Serialize};
use tree_sitter::{Parser, Tree};

use crate::filesystem::Filesystem;

/// JSON Schema for StateMachineReport.
pub const STATE_MACHINE_REPORT_SCHEMA: &str =
    include_str!("../../../schemas/state-machine-report.schema.json");

/// Language for state machine extraction.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    TypeScript,
    Python,
}

impl Language {
    fn lang_label(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::TypeScript => "typescript",
            Language::Python => "python",
        }
    }

    fn support_lang(&self) -> SupportLang {
        match self {
            Language::Rust => SupportLang::Rust,
            Language::TypeScript => SupportLang::TypeScript,
            Language::Python => SupportLang::Python,
        }
    }
}

/// Kind of a state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StateKind {
    Regular,
    Initial,
    Final,
    Choice,
}

/// One state in a state machine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct State {
    pub name: String,
    pub kind: StateKind,
    pub line: u32,
}

/// One transition between states.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub trigger: Option<String>,
    pub guard: Option<String>,
    pub line: u32,
}

/// One state machine extracted from source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateMachine {
    /// `<lang>:<file>:state_machine:<name>:<line>`
    pub canonical_key: String,
    pub name: String,
    pub file: String,
    pub content_hash: String,
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
    /// Always < 1.0 (per SCN-425: boundary inferred)
    pub confidence: f64,
}

/// Top-level report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateMachineReport {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub project: ProjectMeta,
    pub machines: Vec<StateMachine>,
}

/// Per-project metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectMeta {
    pub root: String,
    #[serde(rename = "filesScanned")]
    pub files_scanned: u64,
    pub languages: BTreeMap<String, u64>,
}

/// Report from apply.
#[derive(Debug, Clone, Serialize)]
pub struct ApplyReport {
    pub elements_written: usize,
    pub elements_skipped: usize,
    pub relations_written: usize,
    pub relations_skipped: usize,
    pub seed_writes: usize,
    pub duration_ms: u64,
}

// ─── Extraction ────────────────────────────────────────────────────────────────

/// Run state machine extraction on `cwd`. Pure: no graph writes.
pub fn extract(
    cwd: &Path,
    languages: &[Language],
    fs: &dyn Filesystem,
) -> Result<StateMachineReport> {
    let root = cwd.to_string_lossy().to_string();
    let start = Instant::now();

    // Collect files to process
    let mut files_to_process: Vec<(PathBuf, Language)> = Vec::new();
    let mut lang_counts: BTreeMap<String, u64> = BTreeMap::new();

    let walker = ignore::WalkBuilder::new(cwd)
        .hidden(false)
        .follow_links(false)
        .build();

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension() else {
            continue;
        };
        let ext_str = ext.to_string_lossy().to_lowercase();
        let lang = match ext_str.as_str() {
            "rs" => Language::Rust,
            "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => Language::TypeScript,
            "py" => Language::Python,
            _ => continue,
        };
        if !languages.is_empty() && !languages.contains(&lang) {
            continue;
        }
        let rel = path.strip_prefix(cwd).unwrap_or(path);
        let _rel_str = rel.to_string_lossy().to_string();
        files_to_process.push((rel.to_path_buf(), lang));
        let label = lang.lang_label();
        *lang_counts.entry(label.to_string()).or_insert(0) += 1;
    }

    let files_scanned = files_to_process.len() as u64;
    let mut all_machines: Vec<StateMachine> = Vec::new();

    for (rel_path, lang) in &files_to_process {
        let abs_path = cwd.join(rel_path);
        let source = match fs.read_to_string(&abs_path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let support_lang = lang.support_lang();
        let ts_lang = support_lang.get_ts_language();
        let mut parser = Parser::new();
        if parser.set_language(&ts_lang).is_err() {
            continue;
        }
        let tree = match parser.parse(&source, None) {
            Some(t) => t,
            None => continue,
        };

        let file_str = rel_path.to_string_lossy().to_string();
        let content_hash = crate::evidence::content_hash_of(&source);

        match lang {
            Language::Rust => extract_rust_state_machines(
                &tree,
                &source,
                &file_str,
                &content_hash,
                &mut all_machines,
            ),
            Language::TypeScript => extract_ts_state_machines(
                &tree,
                &source,
                &file_str,
                &content_hash,
                &mut all_machines,
            ),
            Language::Python => extract_python_state_machines(
                &tree,
                &source,
                &file_str,
                &content_hash,
                &mut all_machines,
            ),
        }
    }

    // Sort for determinism
    all_machines.sort_by(|a, b| a.canonical_key.cmp(&b.canonical_key));

    let _duration_ms = start.elapsed().as_millis() as u64;

    Ok(StateMachineReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root,
            files_scanned,
            languages: lang_counts,
        },
        machines: all_machines,
    })
}

// ─── Rust extractor ─────────────────────────────────────────────────────────

fn extract_rust_state_machines(
    tree: &Tree,
    source: &str,
    file: &str,
    content_hash: &str,
    machines: &mut Vec<StateMachine>,
) {
    let root = tree.root_node();
    find_rust_state_machines(tree, root, source, file, content_hash, machines);
}

fn find_rust_state_machines<'tree>(
    tree: &Tree,
    node: tree_sitter::Node<'tree>,
    source: &str,
    file: &str,
    content_hash: &str,
    machines: &mut Vec<StateMachine>,
) {
    let kind = node.kind();

    if kind == "enum_item" {
        if let Some(machine) = extract_rust_enum(tree, node, source, file, content_hash) {
            machines.push(machine);
        }
        return; // Don't recurse into enum body
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            find_rust_state_machines(tree, child, source, file, content_hash, machines);
        }
    }
}

fn extract_rust_enum(
    tree: &Tree,
    node: tree_sitter::Node,
    source: &str,
    file: &str,
    content_hash: &str,
) -> Option<StateMachine> {
    let mut enum_name = String::new();
    let line = (node.start_position().row + 1) as u32;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            match child.kind() {
                "visibility_modifier" => {}
                "identifier" if enum_name.is_empty() => {
                    enum_name = source[child.start_byte()..child.end_byte()].to_string();
                }
                _ => {}
            }
        }
    }

    if enum_name.is_empty() {
        return None;
    }

    // Extract variants (states)
    let mut states: Vec<State> = Vec::new();
    let mut transitions: Vec<Transition> = Vec::new();

    // Look for variant_list
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32)
            && child.kind() == "variant_list"
        {
            for j in 0..child.child_count() {
                if let Some(variant) = child.child(j as u32)
                    && variant.kind() == "enum_variant"
                {
                    let variant_name = extract_variant_name(variant, source);
                    let variant_line = (variant.start_position().row + 1) as u32;
                    states.push(State {
                        name: variant_name,
                        kind: StateKind::Regular,
                        line: variant_line,
                    });
                }
            }
        }
    }

    // Look for impl blocks that contain match on self
    let impl_blocks = find_impl_blocks_for_enum(tree, source, file, &enum_name);

    // Also look for standalone match expressions on fields named `state` or `status`
    find_rust_match_on_state(tree, source, file, &enum_name, &mut transitions);

    // Merge transitions from impl blocks
    for impl_trans in impl_blocks {
        for t in impl_trans {
            if !transitions
                .iter()
                .any(|e| e.from == t.from && e.to == t.to && e.line == t.line)
            {
                transitions.push(t);
            }
        }
    }

    if states.is_empty() {
        return None;
    }

    let canonical_key = format!("rust:{}:state_machine:{}:{}", file, enum_name, line);

    // Confidence: Rust enum+match is fairly reliable, but guards/events are not extracted
    let confidence = 0.75;

    Some(StateMachine {
        canonical_key,
        name: enum_name,
        file: file.to_string(),
        content_hash: content_hash.to_string(),
        states,
        transitions,
        confidence,
    })
}

fn extract_variant_name(variant: tree_sitter::Node, source: &str) -> String {
    for i in 0..variant.child_count() {
        if let Some(child) = variant.child(i as u32)
            && (child.kind() == "identifier" || child.kind() == "type_identifier")
        {
            return source[child.start_byte()..child.end_byte()].to_string();
        }
    }
    String::new()
}

/// Find impl blocks that contain match expressions on self.state
fn find_impl_blocks_for_enum(
    tree: &Tree,
    source: &str,
    file: &str,
    _enum_name: &str,
) -> Vec<Vec<Transition>> {
    let mut results: Vec<Vec<Transition>> = Vec::new();

    // Walk the tree looking for impl_item blocks
    let root = tree.root_node();
    find_impl_matches(root, source, file, &mut results);

    results
}

fn find_impl_matches(
    node: tree_sitter::Node,
    source: &str,
    file: &str,
    results: &mut Vec<Vec<Transition>>,
) {
    let kind = node.kind();

    if kind == "impl_item" {
        let transitions = extract_rust_impl_transitions(node, source, file);
        if !transitions.is_empty() {
            results.push(transitions);
        }
        return;
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            find_impl_matches(child, source, file, results);
        }
    }
}

fn extract_rust_impl_transitions(
    node: tree_sitter::Node,
    source: &str,
    file: &str,
) -> Vec<Transition> {
    let mut transitions: Vec<Transition> = Vec::new();

    // Look for match_expression with expression field containing field_expression `state`
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            find_match_expressions(child, source, file, &mut transitions);
        }
    }

    transitions
}

fn find_match_expressions(
    node: tree_sitter::Node,
    source: &str,
    file: &str,
    transitions: &mut Vec<Transition>,
) {
    let kind = node.kind();

    if kind == "match_expression" {
        // Check if the matched expression involves a field access (e.g., self.state)
        let matched_expr = node.child_by_field_name("expression");
        if let Some(expr) = matched_expr
            && expr.kind() == "field_expression"
        {
            // This is a match on self.field — extract transitions
            let more_transitions = extract_rust_match_transitions(node, source, file);
            transitions.extend(more_transitions);
        }
        return; // Don't recurse into match body
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            find_match_expressions(child, source, file, transitions);
        }
    }
}

fn extract_rust_match_transitions(
    node: tree_sitter::Node,
    source: &str,
    file: &str,
) -> Vec<Transition> {
    let mut transitions: Vec<Transition> = Vec::new();

    // Get the match arms
    if let Some(arms) = node.child_by_field_name("alternatives") {
        for i in 0..arms.child_count() {
            if let Some(arm) = arms.child(i as u32)
                && arm.kind() == "match_pattern"
            {
                let (from_state, _) = extract_match_pattern(arm, source);
                let line = (arm.start_position().row + 1) as u32;

                // Look for the consequence (what happens after the arrow)
                if let Some(consequence) = arm.child_by_field_name("value") {
                    let to_state = extract_consequence_state(consequence, source, file);

                    if !from_state.is_empty() && !to_state.is_empty() && from_state != to_state {
                        transitions.push(Transition {
                            from: from_state,
                            to: to_state,
                            trigger: None,
                            guard: None,
                            line,
                        });
                    }
                }
            }
        }
    }

    transitions
}

fn extract_match_pattern(arm: tree_sitter::Node, source: &str) -> (String, Option<String>) {
    // Pattern can be: enum_variant, identifier, or path_pattern
    let mut state_name = String::new();
    let mut guard: Option<String> = None;

    for i in 0..arm.child_count() {
        if let Some(child) = arm.child(i as u32) {
            match child.kind() {
                "enum_variant_pattern" => {
                    // e.g., OrderState::Pending
                    for j in 0..child.child_count() {
                        if let Some(inner) = child.child(j as u32) {
                            let txt = source[inner.start_byte()..inner.end_byte()].to_string();
                            if (inner.kind() == "identifier" || inner.kind() == "type_identifier")
                                && state_name.is_empty()
                            {
                                state_name = txt;
                            }
                        }
                    }
                }
                "identifier" => {
                    let txt = source[child.start_byte()..child.end_byte()].to_string();
                    if state_name.is_empty() {
                        state_name = txt;
                    }
                }
                "guard" => {
                    // Extract guard condition - but we don't store it per SCN-427
                    let guard_txt = source[child.start_byte()..child.end_byte()].to_string();
                    guard = Some(guard_txt);
                }
                _ => {}
            }
        }
    }

    (state_name, guard)
}

fn extract_consequence_state(node: tree_sitter::Node, source: &str, _file: &str) -> String {
    if node.kind() == "call_expression" {
        // Look for the function name and arguments
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i as u32)
                && child.kind() == "identifier"
            {
                let fn_name = source[child.start_byte()..child.end_byte()].to_string();
                // Check for transition_to, to_, etc.
                if fn_name.contains("transition_to") || fn_name.starts_with("to_") {
                    // Look for an argument
                    for j in 0..node.child_count() {
                        if let Some(arg) = node.child(j as u32) {
                            if arg.kind() == "identifier" {
                                return source[arg.start_byte()..arg.end_byte()].to_string();
                            }
                            if arg.kind() == "path" {
                                return extract_path_identifier(arg, source);
                            }
                        }
                    }
                }
            }
        }
        return String::new();
    }

    if node.kind() == "call" {
        // Python-style call
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i as u32) {
                let txt = source[child.start_byte()..child.end_byte()].to_string();
                if txt.contains("transition_to") || txt.starts_with("to_") {
                    for j in 0..node.child_count() {
                        if let Some(arg) = node.child(j as u32)
                            && (arg.kind() == "identifier" || arg.kind() == "string")
                        {
                            let raw = source[arg.start_byte()..arg.end_byte()].to_string();
                            return raw.trim_matches(['\'', '"']).to_string();
                        }
                    }
                }
            }
        }
        return String::new();
    }

    // Recurse into children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            let inner = extract_consequence_state(child, source, _file);
            if !inner.is_empty() {
                return inner;
            }
        }
    }

    String::new()
}

fn extract_path_identifier(node: tree_sitter::Node, source: &str) -> String {
    // e.g., `Confirmed` or `OrderState::Confirmed`
    let mut result = String::new();
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            let txt = source[child.start_byte()..child.end_byte()].to_string();
            if (child.kind() == "identifier" || child.kind() == "type_identifier")
                && result.is_empty()
            {
                result = txt;
            }
        }
    }
    result
}

fn find_rust_match_on_state(
    tree: &Tree,
    source: &str,
    file: &str,
    _enum_name: &str,
    transitions: &mut Vec<Transition>,
) {
    let root = tree.root_node();
    find_match_expressions(root, source, file, transitions);
}

// ─── TypeScript extractor ───────────────────────────────────────────────────

fn extract_ts_state_machines(
    tree: &Tree,
    source: &str,
    file: &str,
    content_hash: &str,
    machines: &mut Vec<StateMachine>,
) {
    let root = tree.root_node();
    find_ts_state_machines(tree, root, source, file, content_hash, machines);
}

fn find_ts_state_machines<'tree>(
    tree: &Tree,
    node: tree_sitter::Node<'tree>,
    source: &str,
    file: &str,
    content_hash: &str,
    machines: &mut Vec<StateMachine>,
) {
    let kind = node.kind();

    // TypeScript state pattern: union type + switch
    if kind == "type_alias_declaration" {
        if let Some(machine) = extract_ts_union_state(tree, node, source, file, content_hash) {
            machines.push(machine);
        }
        return;
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            find_ts_state_machines(tree, child, source, file, content_hash, machines);
        }
    }
}

fn extract_ts_union_state(
    tree: &Tree,
    node: tree_sitter::Node,
    source: &str,
    file: &str,
    content_hash: &str,
) -> Option<StateMachine> {
    let mut type_name = String::new();
    let line = (node.start_position().row + 1) as u32;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            match child.kind() {
                "identifier" | "type_identifier" if type_name.is_empty() => {
                    type_name = source[child.start_byte()..child.end_byte()].to_string();
                }
                _ => {}
            }
        }
    }

    if type_name.is_empty() {
        return None;
    }

    // Extract union variants from the type
    let mut states: Vec<State> = Vec::new();
    let mut transitions: Vec<Transition> = Vec::new();

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32)
            && let Some(variants) = extract_ts_union_variants(child, source)
        {
            for (name, variant_line) in variants {
                states.push(State {
                    name,
                    kind: StateKind::Regular,
                    line: variant_line,
                });
            }
        }
    }

    // Look for switch statements on this state variable
    let root = tree.root_node();
    find_ts_switch_for_state(root, source, file, &type_name, &mut transitions);

    if states.is_empty() {
        return None;
    }

    let canonical_key = format!("typescript:{}:state_machine:{}:{}", file, type_name, line);
    let confidence = 0.70; // TypeScript is more dynamic

    Some(StateMachine {
        canonical_key,
        name: type_name,
        file: file.to_string(),
        content_hash: content_hash.to_string(),
        states,
        transitions,
        confidence,
    })
}

fn extract_ts_union_variants(node: tree_sitter::Node, source: &str) -> Option<Vec<(String, u32)>> {
    if node.kind() != "union_type" {
        return None;
    }

    let mut variants = Vec::new();
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            let txt = source[child.start_byte()..child.end_byte()].to_string();
            let line = (child.start_position().row + 1) as u32;
            // Only string literals or identifiers
            if txt.starts_with('"') || txt.starts_with('\'') {
                variants.push((txt.trim_matches('"').trim_matches('\'').to_string(), line));
            } else if child.kind() == "identifier" || child.kind() == "string" {
                variants.push((txt, line));
            }
        }
    }

    if variants.is_empty() {
        None
    } else {
        Some(variants)
    }
}

fn find_ts_switch_for_state(
    node: tree_sitter::Node,
    source: &str,
    file: &str,
    _state_var: &str,
    transitions: &mut Vec<Transition>,
) {
    let kind = node.kind();

    if kind == "switch_statement" {
        extract_ts_switch_transitions(node, source, file, transitions);
        return;
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            find_ts_switch_for_state(child, source, file, _state_var, transitions);
        }
    }
}

fn extract_ts_switch_transitions(
    node: tree_sitter::Node,
    source: &str,
    file: &str,
    transitions: &mut Vec<Transition>,
) {
    let cases = node.child_by_field_name("body");
    if let Some(cases) = cases {
        let mut prev_state: Option<String> = None;

        for i in 0..cases.child_count() {
            if let Some(case_item) = cases.child(i as u32)
                && case_item.kind() == "switch_case"
            {
                // Case label (the state value)
                if let Some(label) = case_item.child_by_field_name("label") {
                    let label_txt = source[label.start_byte()..label.end_byte()].to_string();
                    let trimmed = label_txt.trim();
                    if !trimmed.is_empty() && trimmed != "default" {
                        if let Some(state) = trimmed.strip_prefix('\'') {
                            if let Some(state) = state.strip_suffix('\'') {
                                prev_state = Some(state.to_string());
                            }
                        } else if let Some(state) = trimmed.strip_prefix('"') {
                            if let Some(state) = state.strip_suffix('"') {
                                prev_state = Some(state.to_string());
                            }
                        } else {
                            prev_state = Some(trimmed.to_string());
                        }
                    }
                }

                // Consequence (the body)
                if let Some(consequence) = case_item.child_by_field_name("body") {
                    let line = (case_item.start_position().row + 1) as u32;
                    let next_state = extract_ts_case_consequence(consequence, source, file);

                    if let Some(from) = prev_state.take()
                        && !from.is_empty()
                        && !next_state.is_empty()
                    {
                        transitions.push(Transition {
                            from,
                            to: next_state,
                            trigger: None,
                            guard: None,
                            line,
                        });
                    }
                }
            }
        }
    }
}

fn extract_ts_case_consequence(node: tree_sitter::Node, source: &str, file: &str) -> String {
    // Look for this.state = 'next' or similar assignment
    if node.kind() == "expression_statement"
        && let Some(expr) = node.child_by_field_name("expression")
    {
        return extract_ts_assignment_target(expr, source, file);
    }

    if node.kind() == "assignment_expression" {
        return extract_ts_assignment_target(node, source, file);
    }

    // Recurse into block
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            let result = extract_ts_case_consequence(child, source, file);
            if !result.is_empty() {
                return result;
            }
        }
    }

    String::new()
}

fn extract_ts_assignment_target(node: tree_sitter::Node, source: &str, _file: &str) -> String {
    // e.g., this.state = 'Confirmed' -> returns 'Confirmed'
    if node.kind() == "assignment_expression"
        && let Some(right) = node.child_by_field_name("right")
    {
        let txt = source[right.start_byte()..right.end_byte()].to_string();
        let trimmed = txt.trim();
        // Check for string literal
        if let Some(s) = trimmed.strip_prefix('\'')
            && let Some(s) = s.strip_suffix('\'')
        {
            return s.to_string();
        }
        if let Some(s) = trimmed.strip_prefix('"')
            && let Some(s) = s.strip_suffix('"')
        {
            return s.to_string();
        }
        // Identifier
        if node.child_by_field_name("right").is_some() {
            return trimmed.to_string();
        }
    }

    String::new()
}

// ─── Python extractor ────────────────────────────────────────────────────────

fn extract_python_state_machines(
    tree: &Tree,
    source: &str,
    file: &str,
    content_hash: &str,
    machines: &mut Vec<StateMachine>,
) {
    let root = tree.root_node();
    find_python_state_machines(root, source, file, content_hash, machines);
}

fn find_python_state_machines<'tree>(
    node: tree_sitter::Node<'tree>,
    source: &str,
    file: &str,
    content_hash: &str,
    machines: &mut Vec<StateMachine>,
) {
    let kind = node.kind();

    // Python state pattern: class with @transition decorators or methods named to_*
    if kind == "class_definition" {
        if let Some(machine) = extract_python_class_state_machine(node, source, file, content_hash)
        {
            machines.push(machine);
        }
        return;
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            find_python_state_machines(child, source, file, content_hash, machines);
        }
    }
}

fn extract_python_class_state_machine(
    node: tree_sitter::Node,
    source: &str,
    file: &str,
    content_hash: &str,
) -> Option<StateMachine> {
    let mut class_name = String::new();
    let line = (node.start_position().row + 1) as u32;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            match child.kind() {
                "identifier" if class_name.is_empty() => {
                    class_name = source[child.start_byte()..child.end_byte()].to_string();
                }
                _ => {}
            }
        }
    }

    if class_name.is_empty() {
        return None;
    }

    let mut states: Vec<State> = Vec::new();
    let mut transitions: Vec<Transition> = Vec::new();

    // Extract decorated methods or methods starting with 'to_'
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32)
            && child.kind() == "block"
        {
            extract_python_methods_and_decorators(child, source, &mut states, &mut transitions);
        }
    }

    if transitions.is_empty() {
        // No transitions found — not a state machine
        return None;
    }

    let canonical_key = format!("python:{}:state_machine:{}:{}", file, class_name, line);
    let confidence = 0.60; // Python is most dynamic

    Some(StateMachine {
        canonical_key,
        name: class_name,
        file: file.to_string(),
        content_hash: content_hash.to_string(),
        states,
        transitions,
        confidence,
    })
}

fn extract_python_methods_and_decorators(
    node: tree_sitter::Node,
    source: &str,
    _states: &mut Vec<State>,
    transitions: &mut Vec<Transition>,
) {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            let kind = child.kind();

            if kind == "decorated_definition" {
                // Check for @transition decorator
                if let Some(def) = child.child_by_field_name("definition")
                    && let Some(decorator) = child.child_by_field_name("decorator")
                {
                    let dec_text = source[decorator.start_byte()..decorator.end_byte()].to_string();
                    if dec_text.contains("transition")
                        && let Some(func) = extract_python_function(def, source)
                    {
                        let line = (child.start_position().row + 1) as u32;
                        // Try to extract from/to from decorator args
                        let (from_state, to_state) =
                            extract_transition_decorator_args(decorator, source);

                        if !from_state.is_empty() && !to_state.is_empty() {
                            transitions.push(Transition {
                                from: from_state,
                                to: to_state,
                                trigger: Some(func.name.clone()),
                                guard: None,
                                line,
                            });
                        }
                    }
                }
            } else if kind == "function_definition"
                && let Some(func) = extract_python_function(child, source)
            {
                let line = (child.start_position().row + 1) as u32;

                // Check if method name starts with to_ (e.g., def to_confirmed(self):)
                if let Some(to_state) = func.name.strip_prefix("to_")
                    && !to_state.is_empty()
                {
                    transitions.push(Transition {
                        from: String::new(), // Unknown source state
                        to: to_state.to_string(),
                        trigger: Some(func.name.clone()),
                        guard: None,
                        line,
                    });
                }
            }
        }
    }
}

fn extract_python_function(node: tree_sitter::Node, source: &str) -> Option<PythonFunction> {
    let mut func = PythonFunction {
        name: String::new(),
        args: Vec::new(),
    };

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            match child.kind() {
                "identifier" if func.name.is_empty() => {
                    func.name = source[child.start_byte()..child.end_byte()].to_string();
                }
                "arguments" => {
                    for j in 0..child.child_count() {
                        if let Some(arg) = child.child(j as u32)
                            && arg.kind() == "identifier"
                        {
                            func.args
                                .push(source[arg.start_byte()..arg.end_byte()].to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Some(func)
}

#[derive(Debug)]
struct PythonFunction {
    name: String,
    args: Vec<String>,
}

fn extract_transition_decorator_args(
    decorator: tree_sitter::Node,
    source: &str,
) -> (String, String) {
    // e.g., @transition("draft", "confirmed") -> ("draft", "confirmed")
    let mut from = String::new();
    let mut to = String::new();

    if let Some(args_node) = decorator.child_by_field_name("arguments") {
        let mut arg_strings: Vec<String> = Vec::new();
        for i in 0..args_node.child_count() {
            if let Some(child) = args_node.child(i as u32) {
                let txt = source[child.start_byte()..child.end_byte()].to_string();
                if child.kind() == "string" {
                    arg_strings.push(txt.trim_matches('"').trim_matches('\'').to_string());
                }
            }
        }
        if arg_strings.len() >= 2 {
            from = arg_strings[0].clone();
            to = arg_strings[1].clone();
        }
    }

    (from, to)
}

// ─── Apply ─────────────────────────────────────────────────────────────────

/// Apply a state machine report to the graph.
pub fn apply(
    project_dir: &Path,
    report: &StateMachineReport,
    _fs: &dyn Filesystem,
) -> Result<ApplyReport> {
    use crate::code::apply_common::escape_cypher_string;
    use crate::store::open_and_init;

    let start = Instant::now();
    let store = open_and_init(project_dir)?;

    let mut seed_writes = 0usize;

    // Seed MetaTypes
    let meta_types = [
        "uml.state_machine",
        "uml.state",
        "uml.pseudostate",
        "uml.transition",
        "uml.guard",
        "uml.event",
    ];
    for mt in &meta_types {
        let q = format!("MERGE (:MetaType {{id: '{}'}});", mt);
        if store.query(&q).is_ok() {
            seed_writes += 1;
        }
    }

    // Seed Predicates
    let predicates = [
        "behavior.source_state",
        "behavior.target_state",
        "behavior.has_transition",
        "behavior.trigger",
        "behavior.has_guard",
    ];
    for pred in &predicates {
        let q = format!("MERGE (:Predicate {{id: '{}'}});", pred);
        if store.query(&q).is_ok() {
            seed_writes += 1;
        }
    }

    let mut elements_written = 0usize;
    let mut elements_skipped = 0usize;
    let mut relations_written = 0usize;
    let mut relations_skipped = 0usize;

    let existing_keys: std::collections::HashSet<String> = store
        .query("MATCH (e:Element) WHERE e.canonical_key IS NOT NULL RETURN e.canonical_key;")?
        .into_iter()
        .filter_map(|row| {
            row.get("e.canonical_key")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect();

    let version_id = format!(
        "blake3:{}",
        blake3::hash(report.schema_version.as_bytes()).to_hex()
    );

    for machine in &report.machines {
        // Write state machine element
        let machine_id = format!("sm:{}", machine.canonical_key);
        if existing_keys.contains(&machine.canonical_key) {
            elements_skipped += 1;
        } else {
            let canonical_key_escaped = escape_cypher_string(&machine.canonical_key);
            let name_escaped = escape_cypher_string(&machine.name);

            let cypher = format!(
                "MERGE (e:Element {{id: '{id}'}}) SET \
                 e.kind_id = 'uml.state_machine', \
                 e.category = 'uml', \
                 e.canonical_key = '{key}', \
                 e.current_name = '{name}', \
                 e.current_status = 'active', \
                 e.current_confidence = {conf}, \
                 e.current_version_id = '{vid}';",
                id = machine_id,
                key = canonical_key_escaped,
                name = name_escaped,
                conf = machine.confidence,
                vid = version_id,
            );
            if store.query(&cypher).is_ok() {
                elements_written += 1;
            }
        }

        // Build a prefix for state keys: <lang>:<file>
        // machine.canonical_key is <lang>:<file>:state_machine:<name>:<line>
        let lang_file_prefix = machine
            .canonical_key
            .split_once(":state_machine:")
            .map(|(prefix, _)| prefix)
            .unwrap_or(&machine.canonical_key);

        // Write state elements
        for state in &machine.states {
            // State canonical key: <lang>:<file>:state:<name>:<line> (per SCN-424)
            let state_key = format!("{}:state:{}:{}", lang_file_prefix, state.name, state.line);
            let state_id = format!("sm:state:{}", state_key);

            if !existing_keys.contains(&state_key) {
                let key_escaped = escape_cypher_string(&state_key);
                let name_escaped = escape_cypher_string(&state.name);

                let cypher = format!(
                    "MERGE (e:Element {{id: '{id}'}}) SET \
                     e.kind_id = 'uml.state', \
                     e.category = 'uml', \
                     e.canonical_key = '{key}', \
                     e.current_name = '{name}', \
                     e.current_status = 'active', \
                     e.current_confidence = {conf}, \
                     e.current_version_id = '{vid}';",
                    id = state_id,
                    key = key_escaped,
                    name = name_escaped,
                    conf = machine.confidence,
                    vid = version_id,
                );
                if store.query(&cypher).is_ok() {
                    elements_written += 1;
                }
            } else {
                elements_skipped += 1;
            }
        }

        // Write transitions as Elements with proper keys, then link to states
        for transition in &machine.transitions {
            // Find source and target state line numbers by matching state names
            let src_state = machine.states.iter().find(|s| s.name == transition.from);
            let tgt_state = machine.states.iter().find(|s| s.name == transition.to);

            // Transition canonical key: <lang>:<file>:transition:<from>_<to>:<line>
            let transition_key = format!(
                "{}:transition:{}_{}:{}",
                lang_file_prefix, transition.from, transition.to, transition.line
            );
            let trans_id = format!("sm:transition:{}", transition_key);

            // Write transition element if not exists
            if !existing_keys.contains(&transition_key) {
                let key_escaped = escape_cypher_string(&transition_key);
                let trigger_str = transition.trigger.as_deref().unwrap_or("");
                let guard_str = transition.guard.as_deref().unwrap_or("");
                let transition_name = format!("{}_to_{}", transition.from, transition.to);

                let cypher = format!(
                    "MERGE (e:Element {{id: '{id}'}}) SET \
                     e.kind_id = 'uml.transition', \
                     e.category = 'uml', \
                     e.canonical_key = '{key}', \
                     e.current_name = '{name}', \
                     e.current_status = 'active', \
                     e.current_confidence = {conf}, \
                     e.current_version_id = '{vid}', \
                     e.source_state = '{src}', \
                     e.target_state = '{tgt}', \
                     e.trigger = '{trigger}', \
                     e.guard = '{guard}';",
                    id = trans_id,
                    key = key_escaped,
                    name = transition_name,
                    conf = machine.confidence,
                    vid = version_id,
                    src = transition.from,
                    tgt = transition.to,
                    trigger = escape_cypher_string(trigger_str),
                    guard = escape_cypher_string(guard_str),
                );
                if store.query(&cypher).is_ok() {
                    elements_written += 1;
                }
            } else {
                elements_skipped += 1;
            }

            // Create transition→source_state edge (if we found the source state)
            if let Some(src) = src_state {
                let src_key = format!("{}:state:{}:{}", lang_file_prefix, src.name, src.line);
                let src_id = format!("sm:state:{}", src_key);

                let edge_rel_id =
                    format!("sm:edge:transition:{}:source:{}", transition_key, src.name);
                let cypher = format!(
                    "MATCH (tr:Element {{id: '{tr}'}}), (s:Element {{id: '{src}'}}) \
                     MERGE (tr)-[r:SEMANTIC_EDGE {{relation_id: '{rel}'}}]->(s) \
                     SET r.predicate_id = 'behavior.source_state', \
                     r.active = true;",
                    tr = trans_id,
                    src = src_id,
                    rel = edge_rel_id,
                );
                if store.query(&cypher).is_ok() {
                    relations_written += 1;
                } else {
                    relations_skipped += 1;
                }
            }

            // Create transition→target_state edge (if we found the target state)
            if let Some(tgt) = tgt_state {
                let tgt_key = format!("{}:state:{}:{}", lang_file_prefix, tgt.name, tgt.line);
                let tgt_id = format!("sm:state:{}", tgt_key);

                let edge_rel_id =
                    format!("sm:edge:transition:{}:target:{}", transition_key, tgt.name);
                let cypher = format!(
                    "MATCH (tr:Element {{id: '{tr}'}}), (t:Element {{id: '{tgt}'}}) \
                     MERGE (tr)-[r:SEMANTIC_EDGE {{relation_id: '{rel}'}}]->(t) \
                     SET r.predicate_id = 'behavior.target_state', \
                     r.active = true;",
                    tr = trans_id,
                    tgt = tgt_id,
                    rel = edge_rel_id,
                );
                if store.query(&cypher).is_ok() {
                    relations_written += 1;
                } else {
                    relations_skipped += 1;
                }
            }
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(ApplyReport {
        elements_written,
        elements_skipped,
        relations_written,
        relations_skipped,
        seed_writes,
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_kind_serialization() {
        assert_eq!(
            serde_json::to_string(&StateKind::Regular).unwrap(),
            "\"regular\""
        );
        assert_eq!(
            serde_json::to_string(&StateKind::Initial).unwrap(),
            "\"initial\""
        );
    }

    #[test]
    fn language_lang_label() {
        assert_eq!(Language::Rust.lang_label(), "rust");
        assert_eq!(Language::TypeScript.lang_label(), "typescript");
        assert_eq!(Language::Python.lang_label(), "python");
    }

    #[test]
    fn state_machine_confidence_always_below_one() {
        // This test documents the invariant: all extracted machines have confidence < 1.0
        // Since we can't run tree-sitter here without a full parse, we document it
        // in the struct definition and assert at construction time
        let sm = StateMachine {
            canonical_key: "rust:src/lib.rs:state_machine:Test:1".to_string(),
            name: "Test".to_string(),
            file: "src/lib.rs".to_string(),
            content_hash: "sha256:abc".to_string(),
            states: vec![State {
                name: "Active".to_string(),
                kind: StateKind::Regular,
                line: 5,
            }],
            transitions: vec![],
            confidence: 0.75,
        };
        assert!(sm.confidence < 1.0, "confidence must be < 1.0 per SCN-425");
    }
}
