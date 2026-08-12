//! Serde-able types for the diagram bundle export.
//!
//! These structs mirror the JSON schema exactly. The `EvidenceEntry` shape
//! is derived from the `Evidence` struct — single source of truth, eliminating
//! meaning-connascence between Rust types and JSON shapes.

use serde::{Deserialize, Serialize};

/// Helper for `#[serde(skip_serializing_if)]` — returns true when x == 0.
fn is_zero_i64(x: &i64) -> bool {
    *x == 0
}

/// Helper for `#[serde(skip_serializing_if)]` — returns true when b == false.
fn is_false_bool(b: &bool) -> bool {
    !*b
}

/// A node in the projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(rename = "type")]
    pub element_type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "canonicalKey")]
    pub canonical_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "evidenceRefs")]
    pub evidence_refs: Option<Vec<String>>,
    // ─── M81: projection schema 1.1 ───────────────────────────────────────────
    /// View-level x coordinate. Default 0 if no ViewMember row matches.
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub x: i64,
    /// View-level y coordinate. Default 0 if no ViewMember row matches.
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub y: i64,
    /// Whether this member is collapsed in the view. Default false.
    #[serde(default, skip_serializing_if = "is_false_bool")]
    pub collapsed: bool,
    /// Per-view display label. None → renderer falls back to `name`.
    /// Emitted only when non-empty so pre-set-label bundles are byte-identical
    /// to v1.0 (backward-compat).
    #[serde(skip_serializing_if = "Option::is_none", rename = "labelOverride")]
    pub label_override: Option<String>,
}

/// An edge in the projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub predicate: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A projection bundle (nodes + edges).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projection {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

/// An evidence entry in the bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceEntry {
    pub id: String,
    pub kind: String,
    pub claim: String,
    pub path: String,
    #[serde(rename = "startLine")]
    pub start_line: u64,
    #[serde(rename = "endLine")]
    pub end_line: u64,
    #[serde(rename = "toolName")]
    pub tool_name: String,
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    #[serde(rename = "contentHash")]
    pub content_hash: String,
    #[serde(rename = "observedAt")]
    pub observed_at: String,
}

/// The evidence bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub evidence: Vec<EvidenceEntry>,
}

/// Element color definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementColors {
    pub context: String,
    pub container: String,
    pub component: String,
    pub dynamic: String,
    pub deployment: String,
}

/// Edge color definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeColors {
    pub default: String,
}

/// Color styles for a diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Styles {
    pub theme: String,
    pub version: String,
    #[serde(rename = "elementColors")]
    pub element_colors: ElementColors,
    #[serde(rename = "edgeColors")]
    pub edge_colors: EdgeColors,
}

/// The manifest metadata for a bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub format: String,
    #[serde(rename = "viewSelector")]
    pub view_selector: String,
    #[serde(rename = "baseRevision")]
    pub base_revision: String,
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    #[serde(rename = "elementCount")]
    pub element_count: usize,
    #[serde(rename = "edgeCount")]
    pub edge_count: usize,
    #[serde(rename = "evidenceCount")]
    pub evidence_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M81 D2: verify cosmetic fields serialize correctly.
    /// Fields x/y/collapsed use serde(default) so 0/false are NOT emitted.
    /// label_override is skipped when None (skip_serializing_if).
    #[test]
    fn node_cosmetic_fields_with_values() {
        let node = Node {
            id: "el:1".into(),
            element_type: "container".into(),
            name: "API".into(),
            description: None,
            canonical_key: None,
            status: None,
            confidence: None,
            evidence_refs: None,
            x: 240,
            y: 160,
            collapsed: true,
            label_override: Some("DisplayLabel".into()),
        };
        let json = serde_json::to_string(&node).unwrap();
        // Must emit the cosmetic fields
        assert!(json.contains(r#""x":240"#), "x must be emitted: {json}");
        assert!(json.contains(r#""y":160"#), "y must be emitted: {json}");
        assert!(
            json.contains(r#""collapsed":true"#),
            "collapsed must be emitted: {json}"
        );
        assert!(
            json.contains(r#""labelOverride":"DisplayLabel""#),
            "labelOverride must be emitted: {json}"
        );
        // Must NOT emit labelOverride when None
        let node_no_label = Node {
            label_override: None,
            ..node.clone()
        };
        let json_no_label = serde_json::to_string(&node_no_label).unwrap();
        assert!(
            !json_no_label.contains("labelOverride"),
            "labelOverride must be skipped when None: {json_no_label}"
        );
    }

    /// M81 D2: verify defaults (0/false/None) are NOT emitted for optional fields.
    #[test]
    fn node_cosmetic_defaults_not_emitted() {
        let node = Node {
            id: "el:1".into(),
            element_type: "container".into(),
            name: "API".into(),
            description: None,
            canonical_key: None,
            status: None,
            confidence: None,
            evidence_refs: None,
            x: 0,
            y: 0,
            collapsed: false,
            label_override: None,
        };
        let json = serde_json::to_string(&node).unwrap();
        // x/y/collapsed use #[serde(default, skip_serializing_if)] so 0/false are NOT emitted
        assert!(
            !json.contains(r#""x":0"#),
            "x=0 must not be emitted: {json}"
        );
        assert!(
            !json.contains(r#""y":0"#),
            "y=0 must not be emitted: {json}"
        );
        assert!(
            !json.contains(r#""collapsed":false"#),
            "collapsed=false must not be emitted: {json}"
        );
        assert!(
            !json.contains("labelOverride"),
            "labelOverride=None must not be emitted: {json}"
        );
    }

    /// M81 D2: verify backward-compat — JSON without cosmetic fields parses with defaults.
    #[test]
    fn node_cosmetic_parses_from_1_0_bundle() {
        let json = r#"{"id":"el:1","type":"container","name":"API"}"#;
        let node: Node = serde_json::from_str(json).unwrap();
        assert_eq!(node.id, "el:1");
        assert_eq!(node.x, 0, "x defaults to 0");
        assert_eq!(node.y, 0, "y defaults to 0");
        assert!(!node.collapsed, "collapsed defaults to false");
        assert_eq!(node.label_override, None, "label_override defaults to None");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Export format (arrows / viewer-bundle)
// ─────────────────────────────────────────────────────────────────────────────

/// Output format for `archctl diagram export`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    ViewerBundle,
    Arrows,
}

impl ExportFormat {
    /// Parse a string into an `ExportFormat`, case-insensitively.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "viewer-bundle" => Some(Self::ViewerBundle),
            "arrows" => Some(Self::Arrows),
            _ => None,
        }
    }
}

#[cfg(test)]
mod export_format_tests {
    use super::*;

    #[test]
    fn parse_case_insensitive() {
        assert_eq!(ExportFormat::parse("arrows"), Some(ExportFormat::Arrows));
        assert_eq!(ExportFormat::parse("ARROWS"), Some(ExportFormat::Arrows));
        assert_eq!(ExportFormat::parse("Arrows"), Some(ExportFormat::Arrows));
        assert_eq!(
            ExportFormat::parse("viewer-bundle"),
            Some(ExportFormat::ViewerBundle)
        );
        assert_eq!(
            ExportFormat::parse("VIEWER-BUNDLE"),
            Some(ExportFormat::ViewerBundle)
        );
        assert_eq!(
            ExportFormat::parse("Viewer-Bundle"),
            Some(ExportFormat::ViewerBundle)
        );
        assert_eq!(ExportFormat::parse("unknown"), None);
    }
}
