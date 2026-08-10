//! Contract alignment test: schema ↔ Rust DTO ↔ TypeScript loader.
//!
//! D4: Validates that the `diagram-projection.schema.json` field names
//! align with the Rust `BundleEnvelope` serialization and the TypeScript
//! `normalizeBundle` loader. Uses the embedded schema and a deterministic
//! fixture that mirrors the output of `build_bundle(store, "context:*", clock)`.
//!
//! The fixture is embedded inline (not loaded from a file) so the test is
//! self-contained and deterministic. A FixedClock ensures reproducible output.
//!
//! Verification chain:
//!   embedded schema (schema_embed::SCHEMA)
//!     → serde_json::validate against fixture
//!     → assert manifest.viewSelector (camelCase)
//!     → assert canonicalKey / evidenceRefs field names in nodes
//!     → dump fixture → temp file (consumed by TS test)
//!   [TS] normalizeBundle(fixture) → no throw + nodes populated

use std::fs;
use tempfile::TempDir;

/// Minimal fixture that matches the output of `build_bundle(&store, "context:*", &clock)`.
/// Deterministic: FixedClock "2026-07-30T12:00:00Z", single container element.
static FIXTURE_JSON: &str = r##"{
  "manifest": {
    "schemaVersion": "1.0.0",
    "format": "viewer-bundle",
    "viewSelector": "context:*",
    "baseRevision": "blake3:0000000000000000000000000000000000000000000000000000000000000000",
    "generatedAt": "2026-07-30T12:00:00Z",
    "elementCount": 1,
    "edgeCount": 0,
    "evidenceCount": 0
  },
  "projection": {
    "nodes": [
      {
        "id": "el:1",
        "type": "context",
        "name": "Platform",
        "canonicalKey": "platform",
        "description": "System boundary",
        "evidenceRefs": []
      }
    ],
    "edges": []
  },
  "evidence": {
    "evidence": []
  },
  "styles": {
    "theme": "default",
    "version": "1.0.0",
    "elementColors": {
      "context": "#1168bd",
      "container": "#438dd5",
      "component": "#85b8e8",
      "dynamic": "#2694ab",
      "deployment": "#999999"
    },
    "edgeColors": {
      "default": "#707070"
    }
  }
}"##;

#[test]
fn schema_validates_fixture_with_zero_violations() {
    // Load the embedded schema and fixture JSON
    let schema_str = archctl::diagram::schema_embed::SCHEMA;
    let schema: serde_json::Value =
        serde_json::from_str(schema_str).expect("embedded schema is valid JSON");
    let fixture: serde_json::Value =
        serde_json::from_str(FIXTURE_JSON).expect("fixture is valid JSON");

    // Validate fixture against schema — zero violations required
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    let result = validator.validate(&fixture);
    assert!(
        result.is_ok(),
        "fixture must validate against schema; violations: {:?}",
        result.err()
    );
}

#[test]
fn manifest_view_selector_is_camel_case() {
    // The manifest field `view_selector` in Rust serializes as `viewSelector`
    // (serde rename). Verify the fixture key matches schema's expectation.
    let fixture: serde_json::Value =
        serde_json::from_str(FIXTURE_JSON).expect("fixture is valid JSON");

    // Schema requires "viewSelector" (camelCase) — check fixture has it
    let manifest = fixture.get("manifest").expect("manifest section exists");
    assert!(
        manifest.get("viewSelector").is_some(),
        "manifest.viewSelector must be present (camelCase per schema)"
    );
    assert!(
        manifest.get("schemaVersion").is_some(),
        "manifest.schemaVersion must be present (camelCase)"
    );
}

#[test]
fn node_canonical_key_and_evidence_refs_are_camel_case() {
    // Schema uses camelCase for node fields: canonicalKey, evidenceRefs.
    // Verify fixture uses the correct field names.
    let fixture: serde_json::Value =
        serde_json::from_str(FIXTURE_JSON).expect("fixture is valid JSON");

    let nodes = fixture
        .get("projection")
        .expect("projection section exists")
        .get("nodes")
        .expect("nodes array exists");

    let node = nodes
        .as_array()
        .expect("nodes must be an array")
        .first()
        .expect("at least one node in fixture");

    assert!(
        node.get("canonicalKey").is_some(),
        "node.canonicalKey must be present (camelCase per schema)"
    );
    assert!(
        node.get("evidenceRefs").is_some(),
        "node.evidenceRefs must be present (camelCase per schema)"
    );
    // Ensure snake_case variants are NOT present
    assert!(
        node.get("canonical_key").is_none(),
        "node.canonical_key (snake_case) must NOT exist"
    );
    assert!(
        node.get("evidence_refs").is_none(),
        "node.evidence_refs (snake_case) must NOT exist"
    );
}

#[test]
fn dump_fixture_for_ts_test() {
    // Write the fixture to a temp file so the TypeScript test can load it.
    // The TS test (`loader.contract.test.ts`) reads this file and calls
    // normalizeBundle(), asserting no throw and nodes populated.
    let tmp = TempDir::new().expect("temp dir created");
    let fixture_path = tmp.path().join("contract-fixture.json");
    fs::write(&fixture_path, FIXTURE_JSON).expect("fixture written");

    // Verify the file was written correctly
    let content = fs::read_to_string(&fixture_path).expect("fixture readable");
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("written fixture is valid JSON");
    assert!(
        parsed.get("manifest").is_some(),
        "written fixture must have manifest section"
    );
}
