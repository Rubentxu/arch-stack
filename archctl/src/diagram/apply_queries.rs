//! Cypher template functions for the apply pipeline.
//!
//! Pure functions returning `String` — no I/O. Each query is validated
//! for structure but not executed here (execution lives in `apply.rs`).
//!
//! ## MERGE-on-REL fallback pattern (ADR-017 §"Nota técnica")
//!
//! lbug 0.18.3 rejects `MERGE` on REL TABLEs. For each edge-writing
//! operation we emit two queries: a primary `MERGE` and a fallback
//! `MATCH ... CREATE`. The caller tries the primary; on `BinderException`
//! it retries with the fallback. This makes edge writes idempotent:
//! if the edge already exists the second `CREATE` is a no-op.

use crate::diagram::view_types::{ViewGroup, ViewMember};

/// Return the current `revision` of a Diagram node by `id`.
///
/// Used by `apply::run` to compare `changeset.base_revision` against
/// the persisted `Diagram.revision` before applying any mutations.
pub fn get_diagram_for_update(diagram_id: &str) -> String {
    format!(
        "MATCH (d:Diagram {{id: '{diagram_id}'}}) \
         RETURN d.id, d.revision, d.selector, d.props, d.created_at, d.updated_at;"
    )
}

/// Cypher to upsert a ViewMember node (MERGE on `id`).
///
/// The `diagram_id` and `element_id` columns are set unconditionally
/// so the member is correctly associated even on re-apply.
pub fn upsert_view_member(member: &ViewMember) -> String {
    let safe_label = member.label.replace('\'', "\\'");
    let safe_props = serde_json::to_string(&member.props)
        .expect("ViewMember.props is always serializable")
        .replace('\'', "\\'");
    let now = chrono::Utc::now().to_rfc3339();

    format!(
        "MERGE (vm:ViewMember {{id: '{id}'}}) SET \
         vm.diagram_id = '{diagram_id}', \
         vm.element_id = '{element_id}', \
         vm.label = '{safe_label}', \
         vm.x = {x}, \
         vm.y = {y}, \
         vm.collapsed = {collapsed}, \
         vm.props = '{safe_props}', \
         vm.updated_at = timestamp('{now}'), \
         vm.created_at = COALESCE(vm.created_at, timestamp('{now}'));",
        id = member.id,
        diagram_id = member.diagram_id,
        element_id = member.element_id,
        safe_label = safe_label,
        x = member.x,
        y = member.y,
        collapsed = member.collapsed,
        safe_props = safe_props,
        now = now,
    )
}

/// Primary query: MERGE a MEMBER_OF edge (ViewMember → Diagram).
///
/// Falls back to `member_of_fallback` if lbug rejects MERGE on REL TABLE.
pub fn create_member_of(member_id: &str, diagram_id: &str) -> String {
    format!(
        "MATCH (vm:ViewMember {{id: '{mid}'}}), (d:Diagram {{id: '{did}'}}) \
         MERGE (vm)-[:MEMBER_OF]->(d);",
        mid = member_id,
        did = diagram_id,
    )
}

/// Fallback: MATCH + CREATE for MEMBER_OF (idempotent when edge exists).
pub fn member_of_fallback(member_id: &str, diagram_id: &str) -> String {
    format!(
        "MATCH (vm:ViewMember {{id: '{mid}'}}), (d:Diagram {{id: '{did}'}}) \
         CREATE (vm)-[:MEMBER_OF]->(d);",
        mid = member_id,
        did = diagram_id,
    )
}

/// Primary query: MERGE a RENDERS edge (ViewMember → Element).
///
/// Falls back to `renders_fallback` if lbug rejects MERGE on REL TABLE.
pub fn create_renders(member_id: &str, element_id: &str) -> String {
    format!(
        "MATCH (vm:ViewMember {{id: '{mid}'}}), (e:Element {{id: '{eid}'}}) \
         MERGE (vm)-[:RENDERS]->(e);",
        mid = member_id,
        eid = element_id,
    )
}

/// Fallback: MATCH + CREATE for RENDERS (idempotent when edge exists).
pub fn renders_fallback(member_id: &str, element_id: &str) -> String {
    format!(
        "MATCH (vm:ViewMember {{id: '{mid}'}}), (e:Element {{id: '{eid}'}}) \
         CREATE (vm)-[:RENDERS]->(e);",
        mid = member_id,
        eid = element_id,
    )
}

/// Cypher to upsert a ViewGroup node (MERGE on `id`).
pub fn upsert_view_group(group: &ViewGroup) -> String {
    let safe_label = group.label.replace('\'', "\\'");
    let safe_props = serde_json::to_string(&group.props)
        .expect("ViewGroup.props is always serializable")
        .replace('\'', "\\'");
    let now = chrono::Utc::now().to_rfc3339();

    format!(
        "MERGE (vg:ViewGroup {{id: '{id}'}}) SET \
         vg.diagram_id = '{diagram_id}', \
         vg.label = '{safe_label}', \
         vg.collapsed = {collapsed}, \
         vg.props = '{safe_props}', \
         vg.updated_at = timestamp('{now}'), \
         vg.created_at = COALESCE(vg.created_at, timestamp('{now}'));",
        id = group.id,
        diagram_id = group.diagram_id,
        safe_label = safe_label,
        collapsed = group.collapsed,
        safe_props = safe_props,
        now = now,
    )
}

/// Primary query: MERGE a GROUP_CONTAINS edge (ViewGroup → ViewMember).
///
/// Falls back to `group_contains_fallback` if lbug rejects MERGE on REL TABLE.
pub fn link_group_contains(group_id: &str, member_id: &str) -> String {
    format!(
        "MATCH (vg:ViewGroup {{id: '{gid}'}}), (vm:ViewMember {{id: '{mid}'}}) \
         MERGE (vg)-[:GROUP_CONTAINS]->(vm);",
        gid = group_id,
        mid = member_id,
    )
}

/// Fallback: MATCH + CREATE for GROUP_CONTAINS (idempotent when edge exists).
pub fn group_contains_fallback(group_id: &str, member_id: &str) -> String {
    format!(
        "MATCH (vg:ViewGroup {{id: '{gid}'}}), (vm:ViewMember {{id: '{mid}'}}) \
         CREATE (vg)-[:GROUP_CONTAINS]->(vm);",
        gid = group_id,
        mid = member_id,
    )
}

/// Bump the `Diagram.revision` to the new content-hash after applying commands.
///
/// Must be the final step of `apply_changeset` so the revision only changes
/// when all commands succeeded.
pub fn bump_diagram_revision(diagram_id: &str, new_revision: &str) -> String {
    let safe_revision = new_revision.replace('\'', "\\'");
    let now = chrono::Utc::now().to_rfc3339();
    format!(
        "MATCH (d:Diagram {{id: '{id}'}}) \
         SET d.revision = '{safe_revision}', d.updated_at = timestamp('{now}');",
        id = diagram_id,
        safe_revision = safe_revision,
        now = now,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::view_types::ViewMember;

    #[test]
    fn upsert_view_member_query_contains_merge_and_set() {
        let member = ViewMember {
            id: "vm:test:1".into(),
            diagram_id: "diagram:container:test".into(),
            element_id: "el:1".into(),
            label: "Test Node".into(),
            x: 100,
            y: 200,
            collapsed: false,
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        };
        let q = upsert_view_member(&member);
        assert!(q.contains("MERGE (vm:ViewMember"), "query must contain MERGE");
        assert!(q.contains("SET"), "query must contain SET");
        assert!(q.contains("vm.x = 100"), "x coordinate must be interpolated");
        assert!(q.contains("vm.y = 200"), "y coordinate must be interpolated");
    }

    #[test]
    fn escaping_handles_special_chars() {
        // O'Brien's Node has two single quotes — both must be escaped as \'
        let member = ViewMember {
            id: "vm:test:2".into(),
            diagram_id: "diagram:container:test".into(),
            element_id: "el:2".into(),
            label: "O'Brien's Node".into(),
            x: 0,
            y: 0,
            collapsed: false,
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        };
        let q = upsert_view_member(&member);
        // The escaped version O\'Brien must appear in the query
        assert!(q.contains("O\\'Brien"), "escaped O\\'Brien must appear in query: {}", q);
        // The double-escaped form O\\'Brien must NOT appear (only single escape)
        assert!(!q.contains("\\\\'Brien"), "double-escape must not appear: {}", q);
    }

    #[test]
    fn upsert_view_group_query_contains_collapse() {
        let group = ViewGroup {
            id: "view-group:container:test:backend".into(),
            diagram_id: "diagram:container:test".into(),
            label: "Backend".into(),
            collapsed: true,
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        };
        let q = crate::diagram::apply_queries::upsert_view_group(&group);
        assert!(q.contains("MERGE (vg:ViewGroup"), "query must contain MERGE");
        assert!(q.contains("vg.collapsed = true"), "collapsed flag must be set");
    }

    #[test]
    fn bump_diagram_revision_query_sets_revision() {
        let q = bump_diagram_revision("diagram:container:test", "blake3:abc123def456");
        assert!(q.contains("SET d.revision = 'blake3:abc123def456'"), "revision must be set");
    }
}
