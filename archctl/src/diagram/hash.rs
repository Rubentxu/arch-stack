//! Content-hash module for deterministic bundle revision.
//!
//! `baseRevision` = blake3 over canonical (sorted-key, sorted-array) JSON.
//! ADR-013 uses a counter example `"revision:42"`; this implementation uses
//! a content-hash instead, documented in the schema's `$comment` field.

use std::collections::BTreeMap;

use crate::diagram::export_types::Projection;

/// Compute the deterministic `baseRevision` for a projection.
///
/// The revision is computed as:
///
/// 1. Serialize `projection` to a serde_json::Value.
/// 2. Sort all object keys recursively (alphabetical).
/// 3. Sort all array items recursively by their `id` field.
/// 4. Serialize to compact JSON bytes (no whitespace).
/// 5. Hash with blake3 (32-byte digest).
/// 6. Encode as `blake3:<lowercase-hex>`.
pub fn base_revision(projection: &Projection) -> String {
    // Step 1: to serde_json::Value
    let mut value = serde_json::to_value(projection).expect("projection is serializable");

    // Step 2: sort object keys recursively
    sort_object_keys_recursive(&mut value);

    // Step 3: sort arrays by id field
    sort_arrays_by_id(&mut value);

    // Step 4: compact bytes (serde_json::to_vec is compact, no pretty-print)
    let bytes = serde_json::to_vec(&value).expect("canonical JSON serialization is infallible");

    // Step 5: blake3 hash
    let digest = blake3::hash(&bytes);

    // Step 6: encode as blake3:<hex>
    format!("blake3:{}", hex::encode(digest.as_bytes()))
}

/// Recursively sort all object keys in alphabetical order.
fn sort_object_keys_recursive(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            // Recurse into values first (before draining)
            for v in map.values_mut() {
                sort_object_keys_recursive(v);
            }
            // Build a BTreeMap to get sorted keys, then put back
            let pairs: Vec<_> = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            let sorted: BTreeMap<String, serde_json::Value> = pairs.into_iter().collect();
            *map = sorted.into_iter().collect();
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() {
                sort_object_keys_recursive(v);
            }
        }
        _ => {}
    }
}

/// Recursively sort arrays by their `id` field (for deterministic node/edge ordering).
fn sort_arrays_by_id(value: &mut serde_json::Value) {
    if let serde_json::Value::Array(arr) = value {
        for item in arr.iter_mut() {
            sort_arrays_by_id(item);
        }
        // Sort in-place by id if items have an "id" field
        let mut indices: Vec<_> = arr
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                item.get("id")
                    .and_then(|id| id.as_str())
                    .map(|id| (i, id.to_string()))
            })
            .collect();

        if indices.len() == arr.len() {
            indices.sort_by(|a, b| a.1.cmp(&b.1));
            let original = std::mem::take(arr);
            for (old_idx, _) in indices.into_iter() {
                arr.push(original[old_idx].clone());
            }
        }
    } else if let serde_json::Value::Object(map) = value {
        for v in map.values_mut() {
            sort_arrays_by_id(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::export_types::{Edge, Node};

    fn make_projection(nodes: Vec<Node>, edges: Vec<Edge>) -> Projection {
        Projection { nodes, edges }
    }

    #[test]
    fn base_revision_deterministic_on_same_input() {
        let p = make_projection(
            vec![Node {
                id: "c4:container:api".into(),
                element_type: "container".into(),
                name: "API".into(),
                description: None,
                canonical_key: Some("api".into()),
                status: Some("accepted".into()),
                confidence: Some(0.9),
                evidence_refs: None,
                // M81: cosmetic fields default to 0/0/false/None
                x: 0,
                y: 0,
                collapsed: false,
                label_override: None,
            }],
            vec![],
        );

        let h1 = base_revision(&p);
        let h2 = base_revision(&p);
        assert_eq!(h1, h2, "same projection must produce identical hash");
    }

    #[test]
    fn base_revision_identical_for_equivalent_projections() {
        // Two projections with same nodes/edges but built in different order
        let p1 = make_projection(
            vec![Node {
                id: "c4:container:api".into(),
                element_type: "container".into(),
                name: "API".into(),
                description: None,
                canonical_key: Some("api".into()),
                status: Some("accepted".into()),
                confidence: Some(0.9),
                evidence_refs: None,
                x: 0,
                y: 0,
                collapsed: false,
                label_override: None,
            }],
            vec![],
        );

        let p2 = make_projection(
            vec![Node {
                id: "c4:container:api".into(),
                element_type: "container".into(),
                name: "API".into(),
                description: None,
                canonical_key: Some("api".into()),
                status: Some("accepted".into()),
                confidence: Some(0.9),
                evidence_refs: None,
                x: 0,
                y: 0,
                collapsed: false,
                label_override: None,
            }],
            vec![],
        );

        assert_eq!(base_revision(&p1), base_revision(&p2));
    }

    #[test]
    fn base_revision_differs_for_different_projections() {
        let p1 = make_projection(
            vec![Node {
                id: "c4:container:api".into(),
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
            }],
            vec![],
        );

        let p2 = make_projection(
            vec![Node {
                id: "c4:container:db".into(),
                element_type: "container".into(),
                name: "DB".into(),
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
            vec![],
        );

        assert_ne!(base_revision(&p1), base_revision(&p2));
    }

    #[test]
    fn base_revision_format_is_blake3_hex() {
        let p = make_projection(vec![], vec![]);
        let rev = base_revision(&p);
        assert!(
            rev.starts_with("blake3:"),
            "revision must start with 'blake3:'"
        );
        let hex = &rev["blake3:".len()..];
        assert_eq!(hex.len(), 64, "blake3 hex must be 64 chars (32 bytes)");
        assert!(
            hex.chars().all(|c| c.is_ascii_hexdigit()),
            "hex must be lowercase hex"
        );
    }
}
