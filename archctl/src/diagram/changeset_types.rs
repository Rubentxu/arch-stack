//! Serde types for the ChangeSet format used by `archctl diagram apply`.
//!
//! These types mirror the JSON schema in `schemas/changeset.schema.json`.
//! The `Command` enum uses `#[serde(tag = "type")]` to emit the `type`
//! discriminator field, matching the schema's `oneOf` + `const` pattern.
//!
//! **Single source of truth**: `CHANGESET_COMMAND_TYPES` is the authoritative
//! list of command-type strings. It is cross-checked against the schema's
//! `$defs.Command.oneOf` by the round-trip test in `tests/diagram_apply.rs`.

use serde::{Deserialize, Serialize};

/// All valid ChangeSet command type strings.
/// Single source of truth shared by the apply parser and the schema
/// round-trip test. Adding a new command requires updating both this
/// const and the schema's `$defs.Command.oneOf`.
pub const CHANGESET_COMMAND_TYPES: &[&str] =
    &["move-member", "collapse-group", "set-label"];

/// A command in a ChangeSet.
///
/// The `#[serde(tag = "type")]` form emits `"type": "..."` as the
/// discriminator, matching the schema's `oneOf` + `const` pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Command {
    /// Move a projected member to a new (x, y) position.
    #[serde(rename = "move-member")]
    MoveMember {
        /// ViewMember id (e.g. `vm:container:orders:el:1`).
        #[serde(rename = "memberId")]
        member_id: String,
        /// Element id this member renders.
        #[serde(rename = "elementId")]
        element_id: String,
        /// New x coordinate.
        x: i64,
        /// New y coordinate.
        y: i64,
    },

    /// Collapse or expand a named group of members.
    #[serde(rename = "collapse-group")]
    CollapseGroup {
        /// ViewGroup id (e.g. `view-group:container:orders:backend`).
        #[serde(rename = "groupId")]
        group_id: String,
        /// Member ids that belong to this group.
        #[serde(rename = "memberIds")]
        member_ids: Vec<String>,
    },

    /// Set the display label for a projected member.
    #[serde(rename = "set-label")]
    SetLabel {
        /// ViewMember id to relabel.
        #[serde(rename = "memberId")]
        member_id: String,
        /// New display label (max 256 chars per schema).
        label: String,
    },
}

/// A batch of view-level edits to apply to a persisted Diagram.
///
/// Consumed by `diagram::apply::run`. Serialized from JSON, validated
/// against `schemas/changeset.schema.json`, then dispatched to the
/// `GraphStore` view-node methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeSet {
    /// Must be `"1.0"` for now. Adding a new command type is a minor bump.
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,

    /// The view selector this changeset targets
    /// (e.g. `container:orders`).
    #[serde(rename = "diagramId")]
    pub diagram_id: String,

    /// The blake3 content-hash of the projection at export time.
    /// Used for optimistic concurrency control: if the graph has changed
    /// since export, the current `Diagram.revision` will differ and
    /// apply rejects the changeset with `"baseRevision mismatch"`.
    #[serde(rename = "baseRevision")]
    pub base_revision: String,

    /// Ordered list of commands to apply atomically.
    pub commands: Vec<Command>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_round_trip_move_member() {
        let cmd = Command::MoveMember {
            member_id: "vm:el:1".into(),
            element_id: "el:1".into(),
            x: 240,
            y: 160,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let round_tripped: Command = serde_json::from_str(&json).unwrap();
        assert!(matches!(round_tripped, Command::MoveMember { member_id, element_id, x: 240, y: 160 }
            if member_id == "vm:el:1" && element_id == "el:1"
        ));
    }

    #[test]
    fn serde_round_trip_collapse_group() {
        let cmd = Command::CollapseGroup {
            group_id: "view-group:orders:backend".into(),
            member_ids: vec!["vm:el:1".into(), "vm:el:2".into()],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let round_tripped: Command = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            round_tripped,
            Command::CollapseGroup { group_id, member_ids }
            if group_id == "view-group:orders:backend" && member_ids.len() == 2
        ));
    }

    #[test]
    fn serde_round_trip_set_label() {
        let cmd = Command::SetLabel {
            member_id: "vm:el:1".into(),
            label: "Orders Backend".into(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let round_tripped: Command = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            round_tripped,
            Command::SetLabel { member_id, label }
            if member_id == "vm:el:1" && label == "Orders Backend"
        ));
    }

    #[test]
    fn changeset_deserializes_from_json() {
        let json = r#"{
            "schemaVersion": "1.0",
            "diagramId": "container:orders",
            "baseRevision": "blake3:abc123",
            "commands": [
                {"type": "move-member", "memberId": "vm:el:1", "elementId": "el:1", "x": 100, "y": 200}
            ]
        }"#;
        let cs: ChangeSet = serde_json::from_str(json).unwrap();
        assert_eq!(cs.schema_version, "1.0");
        assert_eq!(cs.diagram_id, "container:orders");
        assert_eq!(cs.base_revision, "blake3:abc123");
        assert!(matches!(
            &cs.commands[0],
            Command::MoveMember { member_id, element_id, x: 100, y: 200 }
            if *member_id == "vm:el:1" && *element_id == "el:1"
        ));
    }

    #[test]
    fn changeset_serializes_to_json() {
        let cs = ChangeSet {
            schema_version: "1.0".into(),
            diagram_id: "container:orders".into(),
            base_revision: "blake3:abc123".into(),
            commands: vec![Command::SetLabel {
                member_id: "vm:el:2".into(),
                label: "DB Layer".into(),
            }],
        };
        let json = serde_json::to_string(&cs).unwrap();
        assert!(json.contains(r#""schemaVersion":"1.0""#));
        assert!(json.contains(r#""diagramId":"container:orders""#));
        assert!(json.contains(r#""baseRevision":"blake3:abc123""#));
        assert!(json.contains(r#""type":"set-label""#));
    }
}
