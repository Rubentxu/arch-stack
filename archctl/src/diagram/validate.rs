//! Validate a diagram bundle: schema check + internal consistency.
//!
//! Validates a bundle directory against the JSON Schema 2020-12 document
//! plus internal-consistency rules:
//! - evidence IDs referenced in nodes exist in evidence.json
//! - icon files referenced exist in assets/
//!
//! Idempotent: no side effects, can be run multiple times.

use std::collections::HashSet;
use std::path::Path;

use anyhow::Context;
use serde_json::Value;

use crate::diagram::schema_embed::SCHEMA;
use crate::diagram::export_types::{EvidenceBundle, Projection};
use crate::filesystem::Filesystem;

/// Validation error with context.
#[derive(Debug)]
pub struct ValidationError {
    pub file: String,
    pub error: String,
}

/// A validation report: list of errors (empty = valid).
#[derive(Debug)]
pub struct ValidationReport {
    pub errors: Vec<ValidationError>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validate a bundle directory.
///
/// Loads and validates each of the 5 required files, then checks internal
/// consistency (evidence refs resolve, asset files present).
pub fn run_validate(
    bundle_dir: &Path,
    fs: &dyn Filesystem,
) -> anyhow::Result<ValidationReport> {
    let manifest_path = bundle_dir.join("manifest.json");
    let projection_path = bundle_dir.join("projection.json");
    let evidence_path = bundle_dir.join("evidence.json");
    let styles_path = bundle_dir.join("styles.json");
    let assets_dir = bundle_dir.join("assets");

    let mut errors = Vec::new();

    // 1. Load schema
    let schema: Value = serde_json::from_str(SCHEMA)
        .context("failed to parse embedded schema")?;

    // 2. Validate manifest.json
    if !fs.exists(&manifest_path) {
        errors.push(ValidationError {
            file: "manifest.json".into(),
            error: "file not found".into(),
        });
    } else {
        if let Err(e) = validate_file_against_def(fs, &manifest_path, &schema, "Manifest") {
            errors.push(ValidationError {
                file: "manifest.json".into(),
                error: e,
            });
        }
    }

    // 3. Validate projection.json
    let projection = if !fs.exists(&projection_path) {
        errors.push(ValidationError {
            file: "projection.json".into(),
            error: "file not found".into(),
        });
        None
    } else {
        match validate_file_against_def(fs, &projection_path, &schema, "Projection") {
            Ok(_) => load_projection(fs, &projection_path).ok(),
            Err(e) => {
                errors.push(ValidationError {
                    file: "projection.json".into(),
                    error: e,
                });
                None
            }
        }
    };

    // 4. Validate evidence.json
    let evidence_bundle = if !fs.exists(&evidence_path) {
        errors.push(ValidationError {
            file: "evidence.json".into(),
            error: "file not found".into(),
        });
        None
    } else {
        match validate_file_against_def(fs, &evidence_path, &schema, "EvidenceBundle") {
            Ok(_) => load_evidence_bundle(fs, &evidence_path).ok(),
            Err(e) => {
                errors.push(ValidationError {
                    file: "evidence.json".into(),
                    error: e,
                });
                None
            }
        }
    };

    // 5. Validate styles.json
    if !fs.exists(&styles_path) {
        errors.push(ValidationError {
            file: "styles.json".into(),
            error: "file not found".into(),
        });
    } else {
        if let Err(e) = validate_file_against_def(fs, &styles_path, &schema, "Styles") {
            errors.push(ValidationError {
                file: "styles.json".into(),
                error: e,
            });
        }
    }

    // 6. Consistency: evidence IDs referenced in nodes exist in evidence.json
    if let (Some(proj), Some(ev_bundle)) = (&projection, &evidence_bundle) {
        let evidence_ids: HashSet<&str> = ev_bundle.evidence.iter().map(|e| e.id.as_str()).collect();
        for node in &proj.nodes {
            if let Some(ref refs) = node.evidence_refs {
                for ref_id in refs {
                    if !evidence_ids.contains(ref_id.as_str()) {
                        errors.push(ValidationError {
                            file: "projection.json".into(),
                            error: format!(
                                "dangling evidence-ref '{}' in node '{}': not found in evidence.json",
                                ref_id, node.id
                            ),
                        });
                    }
                }
            }
        }
    }

    // 7. Consistency: required icon files exist in assets/
    let required_icons = ["context.png", "container.png", "component.png", "dynamic.png", "deployment.png"];
    if fs.exists(&assets_dir) {
        for icon in required_icons {
            let icon_path = assets_dir.join(icon);
            if !fs.exists(&icon_path) {
                errors.push(ValidationError {
                    file: format!("assets/{}", icon),
                    error: "icon file not found".into(),
                });
            }
        }
    } else {
        errors.push(ValidationError {
            file: "assets/".into(),
            error: "assets directory not found".into(),
        });
    }

    Ok(ValidationReport { errors })
}

/// Validate a file against a named schema definition.
fn validate_file_against_def(
    fs: &dyn Filesystem,
    path: &Path,
    schema: &Value,
    def_name: &str,
) -> anyhow::Result<(), String> {
    let content = fs.read_to_string(path)
        .map_err(|e| format!("read error: {}", e))?;

    let instance: Value = serde_json::from_str(&content)
        .map_err(|e| format!("JSON parse error: {}", e))?;

    // Build a schema that validates against the named $def
    let def_schema = build_schema_for_def(schema, def_name)
        .ok_or_else(|| format!("definition '{}' not found in schema", def_name))?;

    validate_instance(&instance, &def_schema)
}

/// Build a JSON Schema that validates against a specific $def.
fn build_schema_for_def(root: &Value, def_name: &str) -> Option<Value> {
    let defs = root.get("$defs")?.as_object()?;
    if !defs.contains_key(def_name) {
        return None;
    }
    Some(serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$ref": format!("#/$defs/{}", def_name), // e.g., "#/$defs/Manifest" resolves to $defs.Manifest
        "$defs": root.get("$defs")
    }))
}

fn validate_instance(instance: &Value, schema: &Value) -> Result<(), String> {
    // Use jsonschema crate for validation
    let validator = jsonschema::validator_for(schema)
        .map_err(|e| format!("schema compilation error: {}", e))?;

    if validator.is_valid(instance) {
        Ok(())
    } else {
        let errors: Vec<String> = validator
            .iter_errors(instance)
            .map(|e| format!("{}: {}", e.instance_path(), e))
            .collect();
        Err(if errors.is_empty() {
            "schema validation failed".into()
        } else {
            errors.join("; ")
        })
    }
}

fn load_projection(fs: &dyn Filesystem, path: &Path) -> anyhow::Result<Projection> {
    let content = fs.read_to_string(path)?;
    serde_json::from_str(&content).context("failed to parse projection.json")
}

fn load_evidence_bundle(fs: &dyn Filesystem, path: &Path) -> anyhow::Result<EvidenceBundle> {
    let content = fs.read_to_string(path)?;
    serde_json::from_str(&content).context("failed to parse evidence.json")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::filesystem::MemoryFilesystem;

    use super::*;

    #[test]
    fn validate_missing_manifest() {
        let fs = MemoryFilesystem::new();
        let report = run_validate(PathBuf::from("/tmp/bundle").as_path(), &fs).unwrap();
        assert!(!report.is_valid());
        assert!(report.errors.iter().any(|e| e.file == "manifest.json" && e.error.contains("not found")));
    }

    #[test]
    fn validate_idempotent() {
        // Same bundle validated twice should produce identical results
        // (no side effects in validate)
        let fs = MemoryFilesystem::new();
        let dir = PathBuf::from("/tmp/bundle");
        let r1 = run_validate(&dir, &fs).unwrap();
        let r2 = run_validate(&dir, &fs).unwrap();
        assert_eq!(r1.errors.len(), r2.errors.len());
    }

    #[test]
    fn build_schema_for_def_compiles_all_def_names() {
        // Regression: ensure $ref paths like "#/$defs/Manifest" are resolvable.
        // Previously build_schema_for_def used "#/Manifest" which JSON Schema
        // resolves as a top-level key (not under $defs), producing
        // "Pointer '/Manifest' does not exist" at schema compilation time.
        let schema: Value = serde_json::from_str(SCHEMA).unwrap();
        for def_name in ["Manifest", "Projection", "EvidenceBundle", "Styles"] {
            let def_schema = build_schema_for_def(&schema, def_name);
            assert!(def_schema.is_some(), "definition '{}' should be found in schema", def_name);
            let compiled = jsonschema::validator_for(def_schema.as_ref().unwrap());
            assert!(compiled.is_ok(), "schema for '{}' should compile without errors", def_name);
        }
    }
}
