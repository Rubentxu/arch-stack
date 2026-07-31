//! Serde-able types for the diagram bundle export.
//!
//! These structs mirror the JSON schema exactly. The `EvidenceEntry` shape
//! is derived from the `Evidence` struct — single source of truth, eliminating
//! meaning-connascence between Rust types and JSON shapes.

use serde::{Deserialize, Serialize};

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
