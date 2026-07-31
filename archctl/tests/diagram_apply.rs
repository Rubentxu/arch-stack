//! Integration tests for the apply pipeline.
//!
//! Strategy: since lbug sessions don't share uncommitted data across
//! `LbugStore::open` calls, tests use `apply_to_store` (which accepts a
//! pre-opened store) so seeding and apply share the same session.

use std::fs::OpenOptions;
use fs2::FileExt;

use tempfile::TempDir;

use archctl::diagram::changeset_schema::CHANGESET_SCHEMA;
use archctl::diagram::changeset_types::{ChangeSet, CHANGESET_COMMAND_TYPES};
use archctl::clock::FixedClock;
use archctl::diagram::view_types::Diagram;
use archctl::store::{GraphStore, LbugStore};

// ──────────────────────────────────────────────────────────────────────────────
// Schema consistency (pure unit test — no DB needed)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn changeset_command_types_match_schema_onedef() {
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
        if let Some(ref_path) = item["$ref"].as_str() {
            let def_name = ref_path
                .strip_prefix("#/$defs/")
                .unwrap_or(ref_path);
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
        "CHANGESET_COMMAND_TYPES and schema $defs.Command.oneOf must agree"
    );
    assert_eq!(schema_types.len(), 3);
    assert!(schema_types.contains(&"move-member"));
    assert!(schema_types.contains(&"collapse-group"));
    assert!(schema_types.contains(&"set-label"));
}

// ──────────────────────────────────────────────────────────────────────────────
// Schema validation (pure unit tests)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn schema_accepts_valid_changeset() {
    let changeset = serde_json::json!({
        "schemaVersion": "1.0",
        "diagramId": "container:orders",
        "baseRevision": "blake3:0000000000000000000000000000000000000000000000000000000000000000",
        "commands": [
            {"type": "set-label", "memberId": "vm:el:1", "label": "New Label"}
        ]
    });

    let schema: serde_json::Value =
        serde_json::from_str(CHANGESET_SCHEMA).expect("CHANGESET_SCHEMA is valid JSON");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let result = validator.validate(&changeset);
    assert!(result.is_ok(), "valid changeset should pass schema: {:?}", result.err());
}

#[test]
fn schema_rejects_unknown_command_type() {
    let changeset = serde_json::json!({
        "schemaVersion": "1.0",
        "diagramId": "container:orders",
        "baseRevision": "blake3:0000000000000000000000000000000000000000000000000000000000000000",
        "commands": [
            {"type": "fly-to-the-moon", "memberId": "vm:el:1", "elementId": "el:1", "x": 240, "y": 160}
        ]
    });

    let schema: serde_json::Value =
        serde_json::from_str(CHANGESET_SCHEMA).expect("CHANGESET_SCHEMA is valid JSON");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let result = validator.validate(&changeset);
    assert!(result.is_err(), "unknown command type should fail schema validation");
}

#[test]
fn schema_rejects_missing_commands_field() {
    let changeset = serde_json::json!({
        "schemaVersion": "1.0",
        "diagramId": "container:orders",
        "baseRevision": "blake3:0000000000000000000000000000000000000000000000000000000000000000"
    });

    let schema: serde_json::Value =
        serde_json::from_str(CHANGESET_SCHEMA).expect("CHANGESET_SCHEMA is valid JSON");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let result = validator.validate(&changeset);
    assert!(result.is_err(), "missing commands field should fail schema validation");
}

// ──────────────────────────────────────────────────────────────────────────────
// Test: stale baseRevision is rejected
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn stale_base_revision_is_rejected() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();

    let mut store = LbugStore::open(&project).unwrap();
    store.init().unwrap();

    // Seed diagram with a known revision
    let correct_revision = "blake3:abcd000000000000000000000000000000000000000000000000000000000000ab".to_string();
    store.put_diagram(&Diagram {
        id: "container:api".into(),
        revision: correct_revision.clone(),
        selector: "container:api".into(),
        props: serde_json::json!({}),
        created_at: None,
        updated_at: None,
    }).unwrap();

    // Apply with wrong baseRevision but a valid command structure.
    // We include a move-member command; it will fail with "element not found"
    // BEFORE the baseRevision check runs, so we check schema validation instead.
    let changeset = ChangeSet {
        schema_version: "1.0".to_string(),
        diagram_id: "container:api".into(),
        base_revision: "blake3:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        commands: vec![
            archctl::diagram::changeset_types::Command::MoveMember {
                member_id: "vm:container:api:el:ghost".into(),
                element_id: "el:ghost".into(),
                x: 100,
                y: 200,
            },
        ],
    };

    let result = archctl::diagram::apply::apply_to_store(&mut store, changeset);
    let err = result.expect_err("apply with stale baseRevision must fail");
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("baseRevision mismatch"),
        "error must mention 'baseRevision mismatch', got: {err_msg}"
    );
    assert!(
        err_msg.contains(&correct_revision[..16]),
        "error must include current revision prefix, got: {err_msg}"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Test: DB lock blocks apply
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn concurrent_apply_is_blocked_by_db_lock() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();

    // Seed diagram, release lock by dropping store
    {
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();
        store.put_diagram(&Diagram {
            id: "container:x".into(),
            revision: "blake3:0000000000000000000000000000000000000000000000000000000000000000".into(),
            selector: "container:x".into(),
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        }).unwrap();
    }

    // Hold an exclusive flock on the .lbdb file
    let lbdb_path = project.join("architecture.lbdb");
    let holder = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lbdb_path)
        .unwrap();
    holder.try_lock_exclusive().unwrap();

    // Attempt apply via apply_changeset (opens its own store) — must fail
    let changeset = ChangeSet {
        schema_version: "1.0".to_string(),
        diagram_id: "container:x".into(),
        base_revision: "blake3:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        commands: vec![],
    };

    let result = archctl::diagram::apply::apply_changeset(
        &project,
        changeset,
        &FixedClock::new("2026-07-31T12:01:00Z"),
    );

    drop(holder);

    let err = result.expect_err("apply with held flock must fail");
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("another archctl") || err_msg.contains("DB lock"),
        "error must mention DB lock, got: {err_msg}"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Test: unsupported schemaVersion is rejected
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn unsupported_schema_version_is_rejected() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();

    let mut store = LbugStore::open(&project).unwrap();
    store.init().unwrap();

    let changeset = ChangeSet {
        schema_version: "99.0".to_string(),
        diagram_id: "container:orders".into(),
        base_revision: "blake3:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        commands: vec![],
    };

    let err = archctl::diagram::apply::apply_to_store(&mut store, changeset)
        .expect_err("unsupported schemaVersion should fail");
    let err_msg = err.to_string();
    // Schema validation catches this first (schemaVersion is validated by jsonschema)
    assert!(
        err_msg.contains("changeset validation failed") && err_msg.contains("schemaVersion"),
        "error should mention changeset validation and schemaVersion: {err_msg}"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Test: empty commands array is rejected
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn empty_commands_array_is_rejected() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();

    let mut store = LbugStore::open(&project).unwrap();
    store.init().unwrap();

    let changeset = ChangeSet {
        schema_version: "1.0".to_string(),
        diagram_id: "container:orders".into(),
        base_revision: "blake3:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        commands: vec![],
    };

    let err = archctl::diagram::apply::apply_to_store(&mut store, changeset)
        .expect_err("empty commands should fail");
    let err_msg = err.to_string();
    // Schema validation catches empty commands before our semantic check
    assert!(
        err_msg.contains("changeset validation failed"),
        "error should mention changeset validation failed: {err_msg}"
    );
}
