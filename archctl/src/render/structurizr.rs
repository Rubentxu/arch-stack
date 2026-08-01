//! Minimal Structurizr DSL → SVG renderer.
//!
//! Parses a C4-shaped subset of the Structurizr DSL:
//! ```text
//! workspace "name" "description" {
//!     model {
//!         person = person "User" "External user"
//!         softwareSystem = softwareSystem "Payments" "Handles payments"
//!         container = container "API" "REST API" "Kotlin, Spring"
//!         container = container "DB" "PostgreSQL" "PostgreSQL"
//!         container -> container "calls"
//!     }
//!     views {
//!         container payments {
//!             include person, *
//!         }
//!     }
//! }
//! ```
//!
//! ## Layout
//!
//! Sugiyama-style layered layout:
//! 1. Build a `petgraph::DiGraph<Node, Edge>` from the parsed model.
//! 2. Assign each node to a layer via longest-path-from-sources.
//! 3. Within a layer, distribute nodes horizontally with even spacing.
//! 4. Emit SVG with `<rect>` per node, `<line>` per edge, and
//!    `<text>` labels.
//!
//! ## Scope
//!
//! ADR-011 limits `archctl` to C4 Context/Container/Component; the
//! Structurizr-Lite full surface (deployment, dynamic, styles, groups,
//! icons, paper size) is out of scope and belongs to `archview`.
//! Anything outside the C4 subset yields an explicit parse error so
//! the failure mode is clear, not silent.

use anyhow::{anyhow, bail, Context, Result};
use petgraph::graph::DiGraph;
use petgraph::visit::{DfsPostOrder, EdgeRef};
use petgraph::Direction;
use svg::node::element::path::Data;
use svg::node::element::{Group, Marker, Path, Rectangle, Text};
use svg::Document;

/// One parsed Structurizr DSL statement.
///
/// Only the C4 subset is recognised. Anything else yields a parse error.
#[derive(Debug, Clone)]
pub enum Stmt {
    Person {
        id: String,
        name: String,
        description: String,
    },
    SoftwareSystem {
        id: String,
        name: String,
        description: String,
    },
    Container {
        id: String,
        name: String,
        description: String,
        technology: String,
    },
    #[allow(dead_code)]
    ContainerSystem {
        id: String,
        name: String,
        description: String,
        technology: String,
    },
    Rel {
        src: String,
        dst: String,
        description: String,
    },
}

/// Render a Structurizr DSL string to SVG.
pub fn render(dsl: &str) -> Result<String> {
    let stmts = parse(dsl)?;
    let mut graph: DiGraph<Stmt, String> = DiGraph::new();
    let mut ids: std::collections::HashMap<String, petgraph::graph::NodeIndex> =
        std::collections::HashMap::new();

    // First pass: register all nodes (so relations can reference them in any order).
    for stmt in &stmts {
        match stmt {
            Stmt::Person { id, .. }
            | Stmt::SoftwareSystem { id, .. }
            | Stmt::Container { id, .. }
            | Stmt::ContainerSystem { id, .. } => {
                if ids.contains_key(id) {
                    bail!("duplicate identifier in DSL: {id}");
                }
                let node = graph.add_node(stmt.clone());
                ids.insert(id.clone(), node);
            }
            Stmt::Rel { .. } => {}
        }
    }

    // Second pass: add edges.
    for stmt in &stmts {
        if let Stmt::Rel {
            src,
            dst,
            description,
        } = stmt
        {
            let s = ids
                .get(src)
                .ok_or_else(|| anyhow!("relation references unknown source: {src}"))?;
            let d = ids
                .get(dst)
                .ok_or_else(|| anyhow!("relation references unknown target: {dst}"))?;
            graph.add_edge(*s, *d, description.clone());
        }
    }

    layered_svg(&graph, &ids)
}

fn parse(dsl: &str) -> Result<Vec<Stmt>> {
    let mut stmts: Vec<Stmt> = Vec::new();
    for (lineno, raw) in dsl.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        // Skip blank lines and block headers/closers (`workspace … {`,
        // `model {`, `views { … {`, `}`, `*`).
        if line.is_empty()
            || line.starts_with("workspace")
            || line.starts_with("model")
            || line.starts_with("views")
            || line == "}"
            || line == "{"
        {
            continue;
        }
        // The `views { id { include ... } }` blocks contain nested braces
        // that the line-based tokenizer would split incorrectly. We
        // currently skip their contents (views are rendered by `archview`,
        // not `archctl` per ADR-011 scope).
        if line.starts_with("include")
            || line.starts_with("exclude")
            || line.starts_with("description")
            || line.ends_with('{')
            || line.ends_with('}')
        {
            continue;
        }
        let stmt = parse_line(line)
            .with_context(|| format!("parse line {}: `{}`", lineno + 1, raw.trim()))?;
        stmts.push(stmt);
    }
    Ok(stmts)
}

fn parse_line(line: &str) -> Result<Stmt> {
    // Tokenize by whitespace, respecting quoted strings.
    let tokens = tokenize(line);
    if tokens.is_empty() {
        bail!("empty line after tokenize");
    }

    // Relation line: `<src> -> <dst> ["description"]`.
    // Detection: a `->` token anywhere in the line.
    if let Some(arrow_pos) = tokens.iter().position(|t| t == "->") {
        if arrow_pos != 1 || tokens.len() < 4 {
            bail!(
                "relation syntax: `<src> -> <dst> [\"description\"]`, got `{}`",
                line
            );
        }
        let src = tokens[0].clone();
        let dst = tokens[2].clone();
        let description = if tokens.len() >= 4 {
            tokens[3].clone()
        } else {
            String::new()
        };
        return Ok(Stmt::Rel {
            src,
            dst,
            description,
        });
    }

    // Node declaration: `<id> = <statement_type> <args...>`.
    let eq_pos = tokens
        .iter()
        .position(|t| t == "=")
        .ok_or_else(|| anyhow!("DSL line missing `=` separator: `{}`", line))?;
    if eq_pos != 1 {
        bail!("expected `<id> = ...` but got {} tokens before `=`", eq_pos);
    }
    let id = tokens[0].clone();
    let rhs = &tokens[eq_pos + 1..];
    if rhs.is_empty() {
        bail!("empty RHS after `=`");
    }

    match rhs[0].as_str() {
        "person" => {
            let (name, description) = expect_two(rhs, "person")?;
            Ok(Stmt::Person {
                id,
                name,
                description,
            })
        }
        "softwareSystem" => {
            let (name, description) = expect_two(rhs, "softwareSystem")?;
            Ok(Stmt::SoftwareSystem {
                id,
                name,
                description,
            })
        }
        "container" => {
            let (name, description, technology) = expect_three(rhs, "container")?;
            Ok(Stmt::Container {
                id,
                name,
                description,
                technology,
            })
        }
        other => bail!("unsupported DSL statement type `{other}` (only C4 subset supported)"),
    }
}

/// Tokenize a DSL line, respecting double-quoted strings.
fn tokenize(line: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for c in line.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn expect_two(tokens: &[String], stmt: &str) -> Result<(String, String)> {
    if tokens.len() < 3 {
        bail!(
            "`{stmt}` requires 2 quoted args (name, description); got {} tokens",
            tokens.len() - 1
        );
    }
    Ok((tokens[1].clone(), tokens[2].clone()))
}

fn expect_three(tokens: &[String], stmt: &str) -> Result<(String, String, String)> {
    if tokens.len() < 4 {
        bail!(
            "`{stmt}` requires 3 quoted args (name, description, technology); got {} tokens",
            tokens.len() - 1
        );
    }
    Ok((tokens[1].clone(), tokens[2].clone(), tokens[3].clone()))
}

fn layered_svg(
    graph: &DiGraph<Stmt, String>,
    ids: &std::collections::HashMap<String, petgraph::graph::NodeIndex>,
) -> Result<String> {
    // 1. Assign each node to a layer via longest-path-from-sources.
    let layers = assign_layers(graph);
    // 2. Compute pixel positions.
    let (positions, width, height) = layout(graph, &layers, ids);

    // 3. Build SVG document.
    let mut doc = Document::new()
        .set("viewBox", (0, 0, width, height))
        .set("xmlns", "http://www.w3.org/2000/svg");

    // Edges first (so nodes overlap them visually).
    let mut edges_group = Group::new()
        .set("stroke", "#444")
        .set("stroke-width", "1.5");
    for edge in graph.edge_references() {
        let s = edge.source();
        let t = edge.target();
        if let (Some(s_pos), Some(t_pos)) = (positions.get(&s), positions.get(&t)) {
            let data = Data::new()
                .move_to((s_pos.0 + BOX_W / 2.0, s_pos.1 + BOX_H / 2.0))
                .line_to((t_pos.0 + BOX_W / 2.0, t_pos.1 + BOX_H / 2.0));
            edges_group = edges_group.add(
                Path::new()
                    .set("d", data)
                    .set("fill", "none")
                    .set("marker-end", "url(#arrow)"),
            );
        }
    }
    doc = doc.add(edges_group);

    // Arrow marker definition.
    doc = doc.add(
        Marker::new()
            .set("id", "arrow")
            .set("viewBox", "0 0 10 10")
            .set("refX", "9")
            .set("refY", "5")
            .set("markerWidth", "8")
            .set("markerHeight", "8")
            .set("orient", "auto-start-reverse")
            .add(
                Path::new()
                    .set("d", "M 0 0 L 10 5 L 0 10 z")
                    .set("fill", "#444"),
            ),
    );

    // Nodes on top.
    let mut nodes_group = Group::new()
        .set("font-family", "sans-serif")
        .set("font-size", "12");
    for (id_str, node_idx) in ids {
        if let (Some(stmt), Some((x, y))) = (graph.node_weight(*node_idx), positions.get(node_idx))
        {
            let x = *x;
            let y = *y;
            let (color, label, sub) = node_style(stmt);
            nodes_group = nodes_group.add(
                Rectangle::new()
                    .set("x", x)
                    .set("y", y)
                    .set("width", BOX_W)
                    .set("height", BOX_H)
                    .set("fill", color)
                    .set("stroke", "#222")
                    .set("rx", "4"),
            );
            nodes_group = nodes_group.add(
                Text::new()
                    .set("x", x + BOX_W / 2.0)
                    .set("y", y + 22.0)
                    .set("text-anchor", "middle")
                    .set("fill", "#111")
                    .add(svg::node::Text::new(label.clone())),
            );
            if !sub.is_empty() {
                nodes_group = nodes_group.add(
                    Text::new()
                        .set("x", x + BOX_W / 2.0)
                        .set("y", y + BOX_H - 8.0)
                        .set("text-anchor", "middle")
                        .set("fill", "#444")
                        .add(svg::node::Text::new(sub.clone())),
                );
            }
            let _ = id_str; // ids map kept for future view-filter support
        }
    }
    doc = doc.add(nodes_group);

    Ok(doc.to_string())
}

fn node_style(stmt: &Stmt) -> (&'static str, String, String) {
    match stmt {
        Stmt::Person {
            name, description, ..
        } => ("#e8f0fe", name.clone(), description.clone()),
        Stmt::SoftwareSystem {
            name, description, ..
        } => ("#fff3bf", name.clone(), description.clone()),
        Stmt::Container {
            name,
            description,
            technology,
            ..
        }
        | Stmt::ContainerSystem {
            name,
            description,
            technology,
            ..
        } => (
            "#d3f9d8",
            name.clone(),
            format!("[{technology}] {description}"),
        ),
        Stmt::Rel { .. } => ("#ffffff", String::new(), String::new()),
    }
}

const BOX_W: f32 = 160.0;
const BOX_H: f32 = 70.0;
const LAYER_GAP: f32 = 60.0;
const MARGIN: f32 = 30.0;

/// Assign each node to a layer using longest-path-from-sources.
fn assign_layers(graph: &DiGraph<Stmt, String>) -> Vec<Vec<petgraph::graph::NodeIndex>> {
    let n = graph.node_count();
    let mut layer_of: Vec<usize> = vec![0; n];

    // Iteratively relax: layer[v] = max(layer[u] + 1 for u in incoming(v))
    // or 0 if no incoming. Converges in at most n passes.
    for _ in 0..n {
        let mut changed = false;
        for v in graph.node_indices() {
            let max_pred = graph
                .neighbors_directed(v, Direction::Incoming)
                .map(|u| layer_of[u.index()] + 1)
                .max()
                .unwrap_or(0);
            if layer_of[v.index()] != max_pred {
                layer_of[v.index()] = max_pred;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let n_layers = layer_of.iter().copied().max().unwrap_or(0) + 1;
    let mut layers: Vec<Vec<petgraph::graph::NodeIndex>> = vec![Vec::new(); n_layers];
    for (i, &layer) in layer_of.iter().enumerate() {
        layers[layer].push(petgraph::graph::NodeIndex::new(i));
    }
    // Stable ordering within a layer: deterministic by node-index.
    for layer in &mut layers {
        layer.sort_by_key(|n| n.index());
    }
    layers
}

fn layout(
    graph: &DiGraph<Stmt, String>,
    layers: &[Vec<petgraph::graph::NodeIndex>],
    _ids: &std::collections::HashMap<String, petgraph::graph::NodeIndex>,
) -> (
    std::collections::HashMap<petgraph::graph::NodeIndex, (f32, f32)>,
    f32,
    f32,
) {
    let widest_layer = layers.iter().map(|l| l.len()).max().unwrap_or(0).max(1);
    let layer_width =
        widest_layer as f32 * BOX_W + (widest_layer as f32 - 1.0).max(0.0) * LAYER_GAP;
    let width = layer_width + 2.0 * MARGIN;
    let height = (layers.len() as f32).max(1.0) * BOX_H
        + ((layers.len() as f32).max(1.0) - 1.0) * LAYER_GAP
        + 2.0 * MARGIN;

    let mut positions: std::collections::HashMap<petgraph::graph::NodeIndex, (f32, f32)> =
        std::collections::HashMap::new();
    let _ = graph; // graph kept for potential future per-layer ordering
    for (i, layer) in layers.iter().enumerate() {
        let n = layer.len();
        let total_w = n as f32 * BOX_W + (n as f32 - 1.0).max(0.0) * LAYER_GAP;
        let start_x = (width - total_w) / 2.0;
        let y = MARGIN + i as f32 * (BOX_H + LAYER_GAP);
        for (j, node) in layer.iter().enumerate() {
            let x = start_x + j as f32 * (BOX_W + LAYER_GAP);
            positions.insert(*node, (x, y));
        }
    }
    (positions, width, height)
}

/// Re-exported for tests; not currently used outside.
#[allow(dead_code)]
pub(crate) fn _post_order_dfs_witness(
    graph: &DiGraph<Stmt, String>,
    root: petgraph::graph::NodeIndex,
) -> Vec<petgraph::graph::NodeIndex> {
    let mut visitor = DfsPostOrder::new(graph, root);
    let mut order = Vec::new();
    while let Some(n) = visitor.next(graph) {
        order.push(n);
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_minimal_dsl_to_svg() {
        let dsl = r#"
            user = person "User" "External user"
            sys = softwareSystem "Payments" "Handles payments"
            api = container "API" "REST API" "Kotlin"
            db = container "DB" "PostgreSQL" "PostgreSQL"
            api -> db "calls"
        "#;
        let svg = render(dsl).expect("render");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("User"));
        assert!(svg.contains("API"));
        assert!(svg.contains("DB"));
        assert!(svg.contains("Kotlin"));
        assert!(svg.contains("PostgreSQL"));
    }

    #[test]
    fn errors_on_unsupported_statement() {
        // `enterprise` is not a supported C4 statement type — should bail.
        let dsl = r#"acme = enterprise "Acme" "Holdings""#;
        let err = render(dsl).unwrap_err();
        // anyhow's to_string() only returns the topmost context, not the
        // cause. Check the full chain for the underlying error.
        assert!(
            err.chain()
                .any(|c| c.to_string().contains("unsupported DSL statement")),
            "expected chain to contain 'unsupported DSL statement', got: {err:?}"
        );
    }

    #[test]
    fn errors_on_relation_with_unknown_target() {
        let dsl = r#"
            user = person "User" "External user"
            api = container "API" "REST" "Kotlin"
            api -> ghost "calls"
        "#;
        let err = render(dsl).unwrap_err();
        assert!(err.to_string().contains("unknown target"));
    }
}
