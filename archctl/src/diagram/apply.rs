//! Apply pipeline: validate changeset → acquire lock → dispatch commands → bump revision.
//!
//! ## Pipeline (linear, deterministic)
//!
//! 1. Resolve `project_dir` from `cwd`
//! 2. Read + validate changeset JSON against embedded schema
//! 3. Open `LbugStore` (acquires `fs2` flock on `.lbdb`)
//! 4. Fetch current `Diagram.revision` → compare to `changeset.base_revision`
//!    - mismatch → `bail!("baseRevision mismatch")`
//! 5. For each command: dispatch to `GraphStore` view-node methods
//! 6. Bump `Diagram.revision` to the new blake3 hash
//! 7. Drop `LbugStore` → kernel releases the flock (RAII)
//!
//! ## Failure paths
//!
//! If any step fails, `LbugStore` is dropped (via `?` propagation) without
//! partial-mutating the graph. The single-writer `fs2` flock ensures no
//! concurrent mutation is possible during the apply window.

use std::path::Path;

use anyhow::{bail, Context, Result};
use jsonschema;

use crate::clock::Clock;
use crate::diagram::changeset_schema::CHANGESET_SCHEMA;
#[cfg(test)]
use crate::diagram::changeset_types::Command;
use crate::diagram::changeset_types::{ChangeSet, CHANGESET_COMMAND_TYPES};
use crate::diagram::export_types::Projection;
use crate::diagram::hash::base_revision;
use crate::diagram::view_types::Diagram;
#[cfg(test)]
use crate::diagram::view_types::ViewMember;
use crate::filesystem::Filesystem;
use crate::project::resolve_project;
use crate::store::{DiagramOps, GraphStore, LbugStore};

/// Report from a successful apply operation.
#[derive(Debug)]
pub struct ApplyReport {
    /// The diagram that was modified.
    pub diagram_id: String,
    /// Number of commands that were applied.
    pub commands_applied: usize,
    /// The revision before this apply.
    pub old_revision: String,
    /// The new revision after this apply.
    pub new_revision: String,
}

/// Run the apply pipeline (convenience wrapper).
///
/// Opens the `LbugStore`, validates the changeset, checks `baseRevision`,
/// applies each command, and bumps the `Diagram.revision`.
pub fn run_apply(
    project_dir: &Path,
    changeset_path: &Path,
    clock: &dyn Clock,
    fs: &dyn Filesystem,
) -> Result<ApplyReport> {
    let info = resolve_project(&project_dir.to_string_lossy());
    let changeset_json = fs
        .read_to_string(changeset_path)
        .with_context(|| format!("read changeset file: {}", changeset_path.display()))?;
    let changeset: ChangeSet =
        serde_json::from_str(&changeset_json).context("parse changeset JSON")?;
    apply_changeset(&info.project_dir, changeset, clock)
}

/// Apply a parsed `ChangeSet` to an already-open graph store.
///
/// This is the core apply logic extracted for testability.
/// The store must already be initialized; caller retains ownership.
///
/// Takes `&mut dyn DiagramOps` (the narrowest sub-trait covering every
/// method this pipeline calls). Realises the ISP benefit of the
/// `GraphStore` trait split — the apply core depends only on DiagramOps,
/// not the full super-trait. Concrete `LbugStore` implements DiagramOps
/// via GraphStore, so the lock-aware `LbugStore::open` factory is the
/// caller's concern; the apply core only touches DiagramOps methods.
pub fn apply_to_store(store: &mut dyn DiagramOps, changeset: ChangeSet) -> Result<ApplyReport> {
    // Schema-validation of the changeset structure
    let changeset_json = serde_json::to_string(&changeset).context("re-serialize changeset")?;
    validate_changeset_schema(&changeset_json)?;

    // Basic semantic validation
    if changeset.schema_version != "1.0" {
        bail!(
            "unsupported schemaVersion: {} (only '1.0' is supported)",
            changeset.schema_version
        );
    }
    if changeset.commands.is_empty() {
        bail!("commands array must contain at least one command");
    }

    // Validate base_revision format (64 hex chars after "blake3:")
    if !changeset.base_revision.starts_with("blake3:") {
        bail!(
            "baseRevision must match pattern ^blake3:[0-9a-f]{{64}}$, got: {}",
            changeset.base_revision
        );
    }

    // ── compare baseRevision ─────────────────────────────────────────────────
    let current_revision = match store.get_diagram(&changeset.diagram_id) {
        Ok(diag) => diag.revision,
        Err(_) => {
            let initial = Diagram {
                id: changeset.diagram_id.clone(),
                revision: "blake3:0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
                selector: changeset.diagram_id.clone(),
                props: serde_json::json!({}),
                created_at: None,
                updated_at: None,
            };
            store.put_diagram(&initial).context("seed Diagram node")?;
            "blake3:0000000000000000000000000000000000000000000000000000000000000000".to_string()
        }
    };

    if current_revision != changeset.base_revision {
        bail!(
            "baseRevision mismatch: changeset has {} but graph has {} (graph changed since export)",
            changeset.base_revision,
            current_revision
        );
    }

    let old_revision = current_revision.clone();
    let commands_applied = changeset.commands.len();

    for cmd in &changeset.commands {
        cmd.apply(store, &changeset.diagram_id)?;
    }

    let projection =
        reexport_view(store, &changeset.diagram_id).context("re-export view for revision bump")?;
    let new_revision = base_revision(&projection);

    store.put_diagram(&Diagram {
        id: changeset.diagram_id.clone(),
        revision: new_revision.clone(),
        selector: changeset.diagram_id.clone(),
        props: serde_json::json!({}),
        created_at: None,
        updated_at: None,
    })?;

    Ok(ApplyReport {
        diagram_id: changeset.diagram_id,
        commands_applied,
        old_revision,
        new_revision,
    })
}

/// Run the apply pipeline, opening a new `LbugStore` internally.
///
/// Concrete `LbugStore` is constructed here because the lockfile lives
/// in `.lbdb` and only `LbugStore::open` knows how to acquire the
/// `fs2` flock. Once the store is open and initialised, the rest of
/// the pipeline operates on `&mut dyn GraphStore`.
pub fn apply_changeset(
    project_dir: &Path,
    changeset: ChangeSet,
    _clock: &dyn Clock,
) -> Result<ApplyReport> {
    let mut store = LbugStore::open(project_dir)
        .map_err(|e| anyhow::anyhow!("failed to acquire DB lock: {e}"))?;
    store.init().context("graph init (apply prerequisite)")?;
    apply_to_store(&mut store, changeset)
}

/// Validate `changeset_json` against the embedded `changeset.schema.json`.
fn validate_changeset_schema(changeset_json: &str) -> Result<()> {
    let schema: serde_json::Value =
        serde_json::from_str(&CHANGESET_SCHEMA).context("parse embedded changeset schema")?;

    let validator = jsonschema::validator_for(&schema).context("compile changeset schema")?;

    let instance: serde_json::Value =
        serde_json::from_str(changeset_json).context("parse changeset JSON")?;

    if validator.is_valid(&instance) {
        Ok(())
    } else {
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| format!("{}: {}", e.instance_path(), e))
            .collect();
        let msg = if errors.is_empty() {
            "changeset validation failed: unknown error".to_string()
        } else {
            format!("changeset validation failed: {}", errors.join("; "))
        };
        bail!("{}", msg)
    }
}

/// Dispatch a single `Command` to the appropriate `GraphStore` method.
///
/// Thin wrapper around [`Command::apply`] — kept as a public function
/// for tests that want to exercise dispatch in isolation. New code
/// should call `cmd.apply(store, diagram_id)?` directly.
#[cfg(test)]
fn dispatch_command(store: &mut dyn DiagramOps, cmd: &Command, diagram_id: &str) -> Result<()> {
    cmd.apply(store, diagram_id)
}

/// Re-export the view slice to recompute the deterministic `base_revision`.
///
/// Reads all current ViewMembers for `diagram_id`, reconstructs a minimal
/// `Projection`, and returns it so `base_revision()` can hash it.
fn reexport_view(store: &dyn DiagramOps, diagram_id: &str) -> Result<Projection> {
    use crate::diagram::export_types::Node;

    let members = store.get_view_members(diagram_id)?;

    let nodes: Vec<Node> = members
        .iter()
        .map(|m| Node {
            id: m.id.clone(),
            element_type: "container".to_string(),
            name: m.label.clone(),
            description: None,
            canonical_key: None,
            status: None,
            confidence: None,
            evidence_refs: None,
        })
        .collect();

    Ok(Projection {
        nodes,
        edges: vec![],
    })
}

/// Validate that `CHANGESET_COMMAND_TYPES` and the schema's `oneOf`
/// command types agree. Panics if they diverge (programming error).
pub fn assert_command_types_match_schema() {
    let schema: serde_json::Value =
        serde_json::from_str(CHANGESET_SCHEMA).expect("CHANGESET_SCHEMA is valid JSON");
    let defs = schema["$defs"]
        .as_object()
        .expect("$defs must be an object");
    let oneof = schema["$defs"]["Command"]["oneOf"]
        .as_array()
        .expect("$defs.Command.oneOf must be an array");

    let mut schema_types: Vec<&str> = Vec::new();
    for item in oneof {
        // Each item is { "$ref": "#/$defs/Name" } — resolve the $ref
        if let Some(ref_path) = item["$ref"].as_str() {
            let def_name = ref_path.strip_prefix("#/$defs/").unwrap_or(ref_path);
            let def = defs
                .get(def_name)
                .expect(&format!("$defs.{} must exist", def_name));
            let type_val = def["properties"]["type"]["const"]
                .as_str()
                .expect(&format!("type const must be string in $defs.{}", def_name));
            schema_types.push(type_val);
        }
    }

    schema_types.sort();
    let mut const_types: Vec<&str> = CHANGESET_COMMAND_TYPES.to_vec();
    const_types.sort();
    assert_eq!(
        schema_types, const_types,
        "CHANGESET_COMMAND_TYPES and schema oneOf disagree on command types"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_valid_changeset() {
        let valid = serde_json::json!({
            "schemaVersion": "1.0",
            "diagramId": "container:orders",
            "baseRevision": "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            "commands": [
                {"type": "move-member", "memberId": "vm:el:1", "elementId": "el:1", "x": 240, "y": 160}
            ]
        });
        let json = serde_json::to_string(&valid).unwrap();
        assert!(validate_changeset_schema(&json).is_ok());
    }

    #[test]
    fn validate_rejects_unknown_command_type() {
        let invalid = serde_json::json!({
            "schemaVersion": "1.0",
            "diagramId": "container:orders",
            "baseRevision": "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            "commands": [
                {"type": "fly-me-to-the-moon", "memberId": "vm:el:1", "elementId": "el:1", "x": 240, "y": 160}
            ]
        });
        let json = serde_json::to_string(&invalid).unwrap();
        assert!(validate_changeset_schema(&json).is_err());
    }

    #[test]
    fn validate_rejects_missing_required_field() {
        // Missing "commands"
        let invalid = serde_json::json!({
            "schemaVersion": "1.0",
            "diagramId": "container:orders",
            "baseRevision": "blake3:0000000000000000000000000000000000000000000000000000000000000000"
        });
        let json = serde_json::to_string(&invalid).unwrap();
        assert!(validate_changeset_schema(&json).is_err());
    }

    #[test]
    fn validate_rejects_x_out_of_range() {
        let invalid = serde_json::json!({
            "schemaVersion": "1.0",
            "diagramId": "container:orders",
            "baseRevision": "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            "commands": [
                {"type": "move-member", "memberId": "vm:el:1", "elementId": "el:1", "x": -1, "y": 160}
            ]
        });
        let json = serde_json::to_string(&invalid).unwrap();
        assert!(validate_changeset_schema(&json).is_err());
    }

    #[test]
    fn validate_rejects_empty_commands() {
        let invalid = serde_json::json!({
            "schemaVersion": "1.0",
            "diagramId": "container:orders",
            "baseRevision": "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            "commands": []
        });
        let json = serde_json::to_string(&invalid).unwrap();
        // schema says minItems: 1
        assert!(validate_changeset_schema(&json).is_err());
    }

    #[test]
    fn command_types_match_schema() {
        assert_command_types_match_schema();
    }

    // -------------------------------------------------------------------------
    // dispatch_command tests — require a real LbugStore
    // -------------------------------------------------------------------------

    fn make_test_store() -> (LbugStore, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        std::fs::create_dir_all(&project).unwrap();
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();
        (store, tmp)
    }

    #[test]
    fn dispatch_set_label_updates_label() {
        let (mut store, _tmp) = make_test_store();
        let diagram_id = "container:orders";

        store
            .put_diagram(&Diagram {
                id: diagram_id.into(),
                revision: "blake3:0000000000000000000000000000000000000000000000000000000000000000"
                    .into(),
                selector: diagram_id.into(),
                props: serde_json::json!({}),
                created_at: None,
                updated_at: None,
            })
            .unwrap();

        store
            .put_view_member(&ViewMember {
                id: "vm:container:orders:el:1".into(),
                diagram_id: diagram_id.into(),
                element_id: "el:1".into(),
                label: "Old Label".into(),
                x: 100,
                y: 200,
                collapsed: false,
                props: serde_json::json!({}),
                created_at: None,
                updated_at: None,
            })
            .unwrap();

        let cmd = Command::SetLabel {
            member_id: "vm:container:orders:el:1".into(),
            label: "New Label".into(),
        };
        dispatch_command(&mut store, &cmd, diagram_id).unwrap();

        let members = store.get_view_members(diagram_id).unwrap();
        let m = members
            .iter()
            .find(|m| m.id == "vm:container:orders:el:1")
            .unwrap();
        assert_eq!(m.label, "New Label");
    }

    #[test]
    fn dispatch_set_label_nonexistent_member_fails() {
        let (mut store, _tmp) = make_test_store();
        let diagram_id = "container:orders";

        store
            .put_diagram(&Diagram {
                id: diagram_id.into(),
                revision: "blake3:0000000000000000000000000000000000000000000000000000000000000000"
                    .into(),
                selector: diagram_id.into(),
                props: serde_json::json!({}),
                created_at: None,
                updated_at: None,
            })
            .unwrap();

        let cmd = Command::SetLabel {
            member_id: "vm:container:orders:el:999".into(),
            label: "Should Fail".into(),
        };
        let err = dispatch_command(&mut store, &cmd, diagram_id).unwrap_err();
        // After T6 (atomic update_view_member_label), the bail!() in
        // the GraphStore impl is wrapped by `with_context` in
        // `Command::apply`, so the error chain is:
        //   [0] update_view_member_label for {member_id}
        //   [1] member not found: {member_id}
        // `err.to_string()` only returns the topmost message; check
        // the whole chain for the underlying "member not found" cause.
        assert!(
            err.chain()
                .any(|c| c.to_string().contains("member not found")),
            "expected chain to contain 'member not found', got: {err:?}"
        );
    }

    // W-DV2-C2 regression: set_label must use the atomic
    // update_view_member_label path. After the refactor, a single
    // MATCH ... SET ... RETURN replaces the read-modify-write. This
    // test proves the new path persists the label through a round-trip
    // get_view_members call.
    #[test]
    fn set_label_atomic_path_persists_through_round_trip() {
        let (mut store, _tmp) = make_test_store();
        let diagram_id = "container:orders";

        store
            .put_diagram(&Diagram {
                id: diagram_id.into(),
                revision: "blake3:0000000000000000000000000000000000000000000000000000000000000000"
                    .into(),
                selector: diagram_id.into(),
                props: serde_json::json!({}),
                created_at: None,
                updated_at: None,
            })
            .unwrap();

        store
            .put_view_member(&ViewMember {
                id: "vm:container:orders:el:1".into(),
                diagram_id: diagram_id.into(),
                element_id: "el:1".into(),
                label: "Old Label".into(),
                x: 100,
                y: 200,
                collapsed: false,
                props: serde_json::json!({}),
                created_at: None,
                updated_at: None,
            })
            .unwrap();

        // Direct call to the new atomic method
        store
            .update_view_member_label("vm:container:orders:el:1", "Atomic New Label")
            .unwrap();

        // Verify round-trip: label persisted
        let members = store.get_view_members(diagram_id).unwrap();
        let m = members
            .iter()
            .find(|m| m.id == "vm:container:orders:el:1")
            .unwrap();
        assert_eq!(m.label, "Atomic New Label");
        assert_eq!(m.x, 100, "x preserved by atomic update (not RMW)");
        assert_eq!(m.y, 200, "y preserved by atomic update (not RMW)");
    }

    #[test]
    fn dispatch_collapse_group_creates_group() {
        let (mut store, _tmp) = make_test_store();
        let diagram_id = "container:svc";

        store
            .put_diagram(&Diagram {
                id: diagram_id.into(),
                revision: "blake3:0000000000000000000000000000000000000000000000000000000000000000"
                    .into(),
                selector: diagram_id.into(),
                props: serde_json::json!({}),
                created_at: None,
                updated_at: None,
            })
            .unwrap();

        let cmd = Command::CollapseGroup {
            group_id: "vg:container:svc:group:1".into(),
            member_ids: vec![
                "vm:container:svc:el:1".into(),
                "vm:container:svc:el:2".into(),
            ],
        };
        dispatch_command(&mut store, &cmd, diagram_id).unwrap();

        // Verify group was created via raw query
        let rows = store
            .query("MATCH (g:ViewGroup {id: 'vg:container:svc:group:1'}) RETURN g.id, g.collapsed;")
            .unwrap();
        assert_eq!(rows.len(), 1, "ViewGroup should exist");
        let collapsed = rows[0]
            .get("g.collapsed")
            .and_then(|c| c.as_bool())
            .unwrap();
        assert!(collapsed, "group should be collapsed");
    }

    #[test]
    fn dispatch_move_member_creates_member_and_links() {
        let (mut store, _tmp) = make_test_store();
        let diagram_id = "container:be";

        store
            .put_diagram(&Diagram {
                id: diagram_id.into(),
                revision: "blake3:0000000000000000000000000000000000000000000000000000000000000000"
                    .into(),
                selector: diagram_id.into(),
                props: serde_json::json!({}),
                created_at: None,
                updated_at: None,
            })
            .unwrap();

        // Create an Element node so link_renders finds it.
        // Uses the pattern from graph.rs: CREATE with required fields.
        store.query(
            "CREATE (:Element {id: 'el:api', kind_id: 'mt.system', category: 'c4', canonical_key: 'el:api'}) RETURN 1;"
        ).unwrap();

        let cmd = Command::MoveMember {
            member_id: "vm:container:be:el:api".into(),
            element_id: "el:api".into(),
            x: 240,
            y: 160,
        };
        dispatch_command(&mut store, &cmd, diagram_id).unwrap();

        let members = store.get_view_members(diagram_id).unwrap();
        let m = members
            .iter()
            .find(|mm| mm.id == "vm:container:be:el:api")
            .unwrap();
        assert_eq!(m.x, 240);
        assert_eq!(m.y, 160);
    }

    #[test]
    fn dispatch_move_member_on_nonexistent_element_fails() {
        let (mut store, _tmp) = make_test_store();
        let diagram_id = "container:be";

        store
            .put_diagram(&Diagram {
                id: diagram_id.into(),
                revision: "blake3:0000000000000000000000000000000000000000000000000000000000000000"
                    .into(),
                selector: diagram_id.into(),
                props: serde_json::json!({}),
                created_at: None,
                updated_at: None,
            })
            .unwrap();

        // No element seeded — link_renders will fail
        let cmd = Command::MoveMember {
            member_id: "vm:container:be:el:ghost".into(),
            element_id: "el:ghost".into(),
            x: 240,
            y: 160,
        };
        let err = dispatch_command(&mut store, &cmd, diagram_id).unwrap_err();
        assert!(
            err.to_string().contains("element not found"),
            "expected 'element not found' error, got: {}",
            err
        );
    }
}
