//! Arrows.app export serializer (v0.8 shape).
//!
//! Consumes `BundleEnvelope` fields (`projection`, `styles`) and produces a
//! single `.arrows` JSON document deterministically. No I/O, no lbug access.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::diagram::export_types::{Edge as ExportEdge, ElementColors, Projection, Styles};

/// Arrows document root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrowsDocument {
    /// Global style block.
    pub style: ArrowsStyle,
    /// Diagram nodes.
    pub nodes: Vec<ArrowsNode>,
    /// Diagram relationships.
    pub relationships: Vec<ArrowsRelationship>,
}

/// Arrows global style (currently static — per-node colours live on each node).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrowsStyle {}

/// A node in the Arrows document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrowsNode {
    /// Canonical element id (mirrors `properties["archctl:element"]`).
    pub id: String,
    /// Display label: `label_override` if present, else `name`.
    pub caption: String,
    /// 2-D position from ViewMember; defaults `{x:0, y:0}`.
    pub position: Position,
    /// PascalCase C4 kind labels, e.g. `["Container"]`.
    pub labels: Vec<String>,
    ///archctl:element + archctl:kind pockets.
    pub properties: BTreeMap<String, serde_json::Value>,
    /// Per-node style: `{"node-color": "#438dd5"}`.
    pub style: BTreeMap<String, String>,
}

/// A relationship in the Arrows document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrowsRelationship {
    /// Stable id derived from the edge canonical id.
    pub id: String,
    #[serde(rename = "fromId")]
    pub from_id: String,
    #[serde(rename = "toId")]
    pub to_id: String,
    #[serde(rename = "type")]
    pub predicate: String,
    ///archctl:relation pocket.
    pub properties: BTreeMap<String, serde_json::Value>,
    /// Per-edge style: `{"arrow-color": "#707070"}`.
    pub style: BTreeMap<String, String>,
}

/// 2-D node position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: i64,
    pub y: i64,
}

impl Position {
    pub const ZERO: Position = Position { x: 0, y: 0 };
}

// ─────────────────────────────────────────────────────────────────────────────
// Serialiser
// ─────────────────────────────────────────────────────────────────────────────

/// Map `mt.container` → `"Container"` etc.
fn kind_to_pascal(kind_id: &str) -> String {
    match kind_id.rsplit('.').next().unwrap_or(kind_id) {
        "context" => "Context".to_string(),
        "container" => "Container".to_string(),
        "component" => "Component".to_string(),
        "dynamic" => "Dynamic".to_string(),
        "deployment" => "Deployment".to_string(),
        other => {
            let lower = other.to_lowercase();
            let mut chars = lower.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + chars.as_str(),
            }
        }
    }
}

/// Retrieve the node-color hex for a given element kind_id.
fn node_color(kind_id: &str, colors: &ElementColors) -> String {
    match kind_id.rsplit('.').next().unwrap_or(kind_id) {
        "context" => colors.context.clone(),
        "container" => colors.container.clone(),
        "component" => colors.component.clone(),
        "dynamic" => colors.dynamic.clone(),
        "deployment" => colors.deployment.clone(),
        _ => colors.container.clone(),
    }
}

/// Convert an `ExportEdge` to an `ArrowsRelationship`.
///
/// Note: the edge `predicate` field is used (not `predicate_id` — the actual
/// field name in `export_types::Edge` is `predicate`).
fn serialize_edge(edge: &ExportEdge, style: &BTreeMap<String, String>) -> ArrowsRelationship {
    ArrowsRelationship {
        id: edge.id.clone(),
        from_id: edge.source.clone(),
        to_id: edge.target.clone(),
        predicate: edge.predicate.clone(),
        properties: {
            let mut m = BTreeMap::new();
            m.insert(
                "archctl:relation".to_string(),
                serde_json::Value::String(edge.id.clone()),
            );
            m
        },
        style: style.clone(),
    }
}

/// Convert an `Node` (export_types) to an `ArrowsNode`.
///
/// Position defaults to `{x:0, y:0}` when no ViewMember row matched (i.e.
/// when `x == 0 && y == 0` and `label_override` is `None`). This is a proxy
/// for "no ViewMember" since the projection schema does not expose that
/// directly; the real canonical indicator would be a separate ViewMember
/// existence flag, but that would require a schema change.
fn serialize_node(node: &crate::diagram::export_types::Node, colors: &ElementColors) -> ArrowsNode {
    let labels = vec![kind_to_pascal(&node.element_type)];
    let caption = node
        .label_override
        .clone()
        .unwrap_or_else(|| node.name.clone());
    // Use provided position when non-zero; serde(default) means x/y=0 is
    // skipped in the JSON, but the Rust struct always has the value.
    let position = if node.x == 0 && node.y == 0 {
        Position::ZERO
    } else {
        Position {
            x: node.x,
            y: node.y,
        }
    };
    let mut properties = BTreeMap::new();
    properties.insert(
        "archctl:element".to_string(),
        serde_json::Value::String(node.id.clone()),
    );
    properties.insert(
        "archctl:kind".to_string(),
        serde_json::Value::String(node.element_type.clone()),
    );
    let mut style = BTreeMap::new();
    style.insert(
        "node-color".to_string(),
        node_color(&node.element_type, colors),
    );

    ArrowsNode {
        id: node.id.clone(),
        caption,
        position,
        labels,
        properties,
        style,
    }
}

/// Serialize the projection + styles into an Arrows document.
pub fn serialize(projection: &Projection, styles: &Styles) -> ArrowsDocument {
    let arrow_color = styles.edge_colors.default.clone();
    let edge_style = {
        let mut s = BTreeMap::new();
        s.insert("arrow-color".to_string(), arrow_color);
        s
    };

    let nodes: Vec<ArrowsNode> = projection
        .nodes
        .iter()
        .map(|n| serialize_node(n, &styles.element_colors))
        .collect();

    let relationships: Vec<ArrowsRelationship> = projection
        .edges
        .iter()
        .map(|e| serialize_edge(e, &edge_style))
        .collect();

    ArrowsDocument {
        style: ArrowsStyle {},
        nodes,
        relationships,
    }
}

/// Derive a CWD-relative `.arrows` filename from a selector.
///
/// Replaces every `:` and `/` with `_` and appends `.arrows`:
///   `"container:orders"` → `"container_orders.arrows"`
///   `"c4:domain/orders"` → `"c4_domain_orders.arrows"`
///   `"system"`            → `"system.arrows"`
pub fn derive_default_path(selector: &str) -> PathBuf {
    let sanitized = selector.replace([':', '/'], "_");
    PathBuf::from(format!("{sanitized}.arrows"))
}

/// Count nodes that appear to have no ViewMember placement.
///
/// We proxy "unplaced" as: `x == 0 && y == 0 && label_override.is_none()`.
/// This correctly identifies nodes without a ViewMember row in the projection
/// since serde(default) means absent ViewMember → x=0,y=0 and
/// label_override=None. Note: a node genuinely at position 0,0 with no label
/// would also be counted — this is an acceptable false-positive for the MVP;
/// a proper solution would require a schema flag on `Node`.
pub fn count_unplaced(projection: &Projection) -> usize {
    projection
        .nodes
        .iter()
        .filter(|n| n.x == 0 && n.y == 0 && n.label_override.is_none())
        .count()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::export_types::{
        Edge as ExportEdge, EdgeColors, ElementColors, Node as ExportNode, Projection, Styles,
    };

    fn sample_styles() -> Styles {
        Styles {
            theme: "default".to_string(),
            version: "1.0".to_string(),
            element_colors: ElementColors {
                context: "#555555".to_string(),
                container: "#438dd5".to_string(),
                component: "#57b583".to_string(),
                dynamic: "#ecb256".to_string(),
                deployment: "#a43b47".to_string(),
            },
            edge_colors: EdgeColors {
                default: "#707070".to_string(),
            },
        }
    }

    fn sample_projection() -> Projection {
        Projection {
            nodes: vec![
                ExportNode {
                    id: "el:api".to_string(),
                    element_type: "mt.container".to_string(),
                    name: "API".to_string(),
                    description: None,
                    canonical_key: None,
                    status: None,
                    confidence: None,
                    evidence_refs: None,
                    x: 240,
                    y: 160,
                    collapsed: false,
                    label_override: Some("Display".to_string()),
                },
                ExportNode {
                    id: "el:db".to_string(),
                    element_type: "mt.container".to_string(),
                    name: "Database".to_string(),
                    description: None,
                    canonical_key: None,
                    status: None,
                    confidence: None,
                    evidence_refs: None,
                    // No ViewMember → x=0,y=0, label_override=None
                    x: 0,
                    y: 0,
                    collapsed: false,
                    label_override: None,
                },
            ],
            edges: vec![ExportEdge {
                id: "rel:1".to_string(),
                source: "el:api".to_string(),
                target: "el:db".to_string(),
                predicate: "calls".to_string(),
                label: None,
            }],
        }
    }

    #[test]
    fn roundtrip_via_json() {
        let proj = sample_projection();
        let sty = sample_styles();
        let doc = serialize(&proj, &sty);
        // Re-deserialise to prove structural validity
        let json = serde_json::to_value(&doc).unwrap();
        let reparsed: ArrowsDocument = serde_json::from_value(json).unwrap();
        assert_eq!(reparsed.nodes.len(), 2);
        assert_eq!(reparsed.relationships.len(), 1);
        assert_eq!(reparsed.nodes[0].caption, "Display");
        assert_eq!(reparsed.nodes[1].caption, "Database");
    }

    #[test]
    fn pascal_case_labels() {
        let cases = [
            ("mt.context", "Context"),
            ("mt.container", "Container"),
            ("mt.component", "Component"),
            ("mt.dynamic", "Dynamic"),
            ("mt.deployment", "Deployment"),
            ("mt.unknown", "Unknown"),
        ];
        for (kind, expected) in cases {
            assert_eq!(kind_to_pascal(kind), expected, "{kind}");
        }
    }

    #[test]
    fn derive_default_path_cases() {
        assert_eq!(
            derive_default_path("container:orders"),
            PathBuf::from("container_orders.arrows")
        );
        assert_eq!(
            derive_default_path("c4:domain/orders"),
            PathBuf::from("c4_domain_orders.arrows")
        );
        assert_eq!(
            derive_default_path("system"),
            PathBuf::from("system.arrows")
        );
        assert_eq!(derive_default_path("a:b:c"), PathBuf::from("a_b_c.arrows"));
    }

    #[test]
    fn count_unplaced_and_caption_fallback() {
        let proj = sample_projection();
        assert_eq!(count_unplaced(&proj), 1); // el:db is unplaced
        // el:api has label_override so not counted; el:db has x=0,y=0,no label → counted
        let doc = serialize(&proj, &sample_styles());
        assert_eq!(doc.nodes[0].caption, "Display"); // from label_override
        assert_eq!(doc.nodes[1].caption, "Database"); // fallback to name
        assert_eq!(doc.nodes[1].position, Position::ZERO);
        assert_eq!(doc.nodes[0].position, Position { x: 240, y: 160 });
    }

    #[test]
    fn node_has_archctl_pockets() {
        let proj = sample_projection();
        let doc = serialize(&proj, &sample_styles());
        let n = &doc.nodes[0];
        assert_eq!(
            n.properties
                .get("archctl:element")
                .unwrap()
                .as_str()
                .unwrap(),
            "el:api"
        );
        assert_eq!(
            n.properties.get("archctl:kind").unwrap().as_str().unwrap(),
            "mt.container"
        );
        assert!(n.style.contains_key("node-color"));
    }

    #[test]
    fn relationship_archctl_relation() {
        let proj = sample_projection();
        let doc = serialize(&proj, &sample_styles());
        let r = &doc.relationships[0];
        assert_eq!(r.predicate, "calls");
        assert_eq!(r.from_id, "el:api");
        assert_eq!(r.to_id, "el:db");
        assert_eq!(
            r.properties
                .get("archctl:relation")
                .unwrap()
                .as_str()
                .unwrap(),
            "rel:1"
        );
        assert_eq!(r.style.get("arrow-color").unwrap(), "#707070");
    }

    #[test]
    fn node_labels_pascal() {
        let proj = Projection {
            nodes: vec![ExportNode {
                id: "el:x".to_string(),
                element_type: "mt.component".to_string(),
                name: "X".to_string(),
                description: None,
                canonical_key: None,
                status: None,
                confidence: None,
                evidence_refs: None,
                x: 0,
                y: 0,
                collapsed: false,
                label_override: None,
            }],
            edges: vec![],
        };
        let doc = serialize(&proj, &sample_styles());
        assert_eq!(doc.nodes[0].labels, vec!["Component"]);
    }
}
