//! Serde-able types for persisted view nodes (schema v3).
//!
//! These structs are the typed carriers for `GraphStore` view-node methods
//! (T5–T8). They mirror the NODE TABLE columns from
//! `docs/schema/003_view_nodes.cypher`.
//!
//! Naming follows the spec (Diagram, ViewMember) — not the design's
//! longer `DiagramNode` / `ViewMemberNode` aliases. `Node` suffix is
//! reserved for raw row types if they ever diverge from these structs.

use serde::{Deserialize, Serialize};

/// A persisted diagram view container.
///
/// `revision` is the blake3 content-hash of the exported bundle at apply-time,
/// used for `baseRevision` concurrency checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagram {
    /// Unique diagram identifier (project-scoped).
    pub id: String,
    /// blake3 content-hash of the bundle at apply-time.
    pub revision: String,
    /// JSON-encoded C4 view selector.
    pub selector: String,
    /// Arbitrary key-value metadata.
    pub props: serde_json::Value,
    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Last-update timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// A canonical Element placed inside a Diagram.
///
/// `diagram_id` is denormalised for indexed lookup.
/// `element_id` is the foreign key to Element.id.
/// `x`, `y`, `collapsed`: view-level position + collapse state
/// (per spec §Capability 1; ADR-007 §"Vista persistida"). Defaults via
/// `Default::default()` so construction sites can use `..Default::default()`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ViewMember {
    /// Unique view-member identifier.
    pub id: String,
    /// Parent Diagram id.
    pub diagram_id: String,
    /// Canonical Element id rendered by this member.
    pub element_id: String,
    /// Display label (may differ from Element.name in the view).
    pub label: String,
    /// View-level x coordinate. Default 0.
    #[serde(default)]
    pub x: i64,
    /// View-level y coordinate. Default 0.
    #[serde(default)]
    pub y: i64,
    /// Whether this member is collapsed in the view. Default false.
    #[serde(default)]
    pub collapsed: bool,
    /// Arbitrary key-value metadata.
    pub props: serde_json::Value,
    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Last-update timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// A directed edge between two ViewMembers within a Diagram.
///
/// Corresponds to a SemanticRelation override in the view layer.
/// `source_member_id` and `target_member_id` are ViewMember.id values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewEdge {
    /// Unique edge identifier.
    pub id: String,
    /// Parent Diagram id.
    pub diagram_id: String,
    /// Source ViewMember id.
    pub source_member_id: String,
    /// Target ViewMember id.
    pub target_member_id: String,
    /// Edge label (predicate name in the view).
    pub edge_label: String,
    /// Arbitrary key-value metadata.
    pub props: serde_json::Value,
    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Last-update timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// A named group of ViewMembers within a Diagram.
/// `collapsed`: view-level collapse state (per spec §Capability 5 SCN-101).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ViewGroup {
    /// Unique group identifier.
    pub id: String,
    /// Parent Diagram id.
    pub diagram_id: String,
    /// Display label.
    pub label: String,
    /// Whether this group is collapsed in the view. Default false.
    #[serde(default)]
    pub collapsed: bool,
    /// Arbitrary key-value metadata.
    pub props: serde_json::Value,
    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Last-update timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagram_serde_round_trip() {
        let diag = Diagram {
            id: "d1".into(),
            revision: "blake3:abc123".into(),
            selector: r#"{"kind":"container"}"#.into(),
            props: serde_json::json!({"foo": "bar"}),
            created_at: Some("2026-07-31T00:00:00Z".into()),
            updated_at: None,
        };
        let encoded = serde_json::to_string(&diag).unwrap();
        let decoded: Diagram = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.id, diag.id);
        assert_eq!(decoded.revision, diag.revision);
        assert_eq!(decoded.selector, diag.selector);
        assert_eq!(decoded.props, diag.props);
    }

    #[test]
    fn view_member_serde_round_trip() {
        let vm = ViewMember {
            id: "vm1".into(),
            diagram_id: "d1".into(),
            element_id: "el1".into(),
            label: "My Service".into(),
            x: 240,
            y: 160,
            collapsed: false,
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        };
        let encoded = serde_json::to_string(&vm).unwrap();
        let decoded: ViewMember = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.id, vm.id);
        assert_eq!(decoded.element_id, vm.element_id);
        assert_eq!(decoded.x, 240);
        assert_eq!(decoded.y, 160);
        assert!(!decoded.collapsed);
    }

    #[test]
    fn view_member_default_serde_round_trip() {
        // Construct with `..Default::default()` — proves x/y/collapsed
        // can be omitted on read (serde(default) attribute on each field).
        let vm = ViewMember {
            id: "vm-default".into(),
            diagram_id: "d1".into(),
            element_id: "el1".into(),
            label: "Default member".into(),
            ..Default::default()
        };
        let encoded = serde_json::to_string(&vm).unwrap();
        let decoded: ViewMember = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.x, 0);
        assert_eq!(decoded.y, 0);
        assert!(!decoded.collapsed);
    }

    #[test]
    fn view_edge_serde_round_trip() {
        let ve = ViewEdge {
            id: "ve1".into(),
            diagram_id: "d1".into(),
            source_member_id: "vm1".into(),
            target_member_id: "vm2".into(),
            edge_label: "calls".into(),
            props: serde_json::json!({"async": true}),
            created_at: Some("2026-07-31T00:00:00Z".into()),
            updated_at: None,
        };
        let encoded = serde_json::to_string(&ve).unwrap();
        let decoded: ViewEdge = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.id, ve.id);
        assert_eq!(decoded.edge_label, ve.edge_label);
    }

    #[test]
    fn view_group_serde_round_trip() {
        let vg = ViewGroup {
            id: "vg1".into(),
            diagram_id: "d1".into(),
            label: "Backend".into(),
            collapsed: false,
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        };
        let encoded = serde_json::to_string(&vg).unwrap();
        let decoded: ViewGroup = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.id, vg.id);
        assert_eq!(decoded.label, vg.label);
        assert!(!decoded.collapsed);
    }
}
