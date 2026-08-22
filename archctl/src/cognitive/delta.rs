//! Graph delta types for the reactive runtime.
//!
//! PR1 provides element-level deltas only — edges are deferred to M18.1.

use serde::{Deserialize, Serialize};

use crate::cognitive::context::Element;

// ---------------------------------------------------------------------------
// DeltaChange
// ---------------------------------------------------------------------------

/// The kind of change applied to a graph element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeltaChange {
    /// The element is new in the graph.
    Added,
    /// The element existed and was modified.
    Modified,
}

// ---------------------------------------------------------------------------
// DeltaElement
// ---------------------------------------------------------------------------

/// A single element change pairing the element with its change kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaElement {
    /// The element that changed.
    pub element: Element,
    /// What kind of change occurred.
    pub change: DeltaChange,
}

// ---------------------------------------------------------------------------
// GraphDelta
// ---------------------------------------------------------------------------

/// A collection of element-level changes to the cognitive graph.
///
/// Edges (relationships between elements) are NOT included in this struct —
/// they are deferred to M18.1 when the replay engine is implemented.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphDelta {
    /// Elements that are new in the graph.
    pub added: Vec<DeltaElement>,
    /// Elements that existed and were modified.
    pub modified: Vec<DeltaElement>,
}

impl GraphDelta {
    /// Returns `true` if there are no changes.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty()
    }

    /// Returns the total number of changed elements.
    pub fn len(&self) -> usize {
        self.added.len() + self.modified.len()
    }

    /// Merge another delta into this one, adding its changes.
    ///
    /// This is a shallow concatenation — duplicate elements are NOT deduplicated.
    pub fn merge(&mut self, other: GraphDelta) {
        self.added.extend(other.added);
        self.modified.extend(other.modified);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::context::Element;

    fn make_element(id: &str) -> Element {
        Element {
            id: id.into(),
            kind_id: "component".into(),
            name: id.into(),
            canonical_key: format!("component:{}", id),
            properties: serde_json::json!({}),
        }
    }

    #[test]
    fn graph_delta_empty_by_default() {
        let delta = GraphDelta::default();
        assert!(delta.is_empty());
        assert_eq!(delta.len(), 0);
    }

    #[test]
    fn graph_delta_with_added_elements() {
        let mut delta = GraphDelta::default();
        delta.added.push(DeltaElement {
            element: make_element("e1"),
            change: DeltaChange::Added,
        });
        delta.added.push(DeltaElement {
            element: make_element("e2"),
            change: DeltaChange::Added,
        });
        assert!(!delta.is_empty());
        assert_eq!(delta.len(), 2);
    }

    #[test]
    fn graph_delta_with_modified_elements() {
        let mut delta = GraphDelta::default();
        delta.modified.push(DeltaElement {
            element: make_element("e1"),
            change: DeltaChange::Modified,
        });
        assert!(!delta.is_empty());
        assert_eq!(delta.len(), 1);
    }

    #[test]
    fn graph_delta_merge() {
        let mut delta1 = GraphDelta::default();
        delta1.added.push(DeltaElement {
            element: make_element("e1"),
            change: DeltaChange::Added,
        });

        let delta2 = GraphDelta {
            added: vec![DeltaElement {
                element: make_element("e2"),
                change: DeltaChange::Added,
            }],
            modified: vec![DeltaElement {
                element: make_element("e3"),
                change: DeltaChange::Modified,
            }],
        };

        delta1.merge(delta2);
        assert_eq!(delta1.len(), 3);
        assert_eq!(delta1.added.len(), 2);
        assert_eq!(delta1.modified.len(), 1);
    }

    #[test]
    fn delta_element_serialize() {
        let el = DeltaElement {
            element: make_element("auth-svc"),
            change: DeltaChange::Added,
        };
        let json = serde_json::to_string(&el).unwrap();
        let back: DeltaElement = serde_json::from_str(&json).unwrap();
        assert_eq!(back.element.id, "auth-svc");
        assert_eq!(back.change, DeltaChange::Added);
    }

    #[test]
    fn graph_delta_serialize() {
        let delta = GraphDelta {
            added: vec![DeltaElement {
                element: make_element("svc1"),
                change: DeltaChange::Added,
            }],
            modified: vec![DeltaElement {
                element: make_element("svc2"),
                change: DeltaChange::Modified,
            }],
        };
        let json = serde_json::to_string(&delta).unwrap();
        let back: GraphDelta = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 2);
    }

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage v4, 2026-08-22)
    // ---------------------------------------------------------------------------

    /// `DeltaChange::Added` round-trips through serde.
    #[test]
    fn delta_change_added_serde_roundtrip() {
        let original = DeltaChange::Added;
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, "\"Added\"", "variant serializes as PascalCase string");
        let back: DeltaChange = serde_json::from_str(&json).unwrap();
        assert_eq!(back, original);
    }

    /// `DeltaChange::Modified` round-trips through serde. Distinct from
    /// `delta_change_added_serde_roundtrip` to cover both variants.
    #[test]
    fn delta_change_modified_serde_roundtrip() {
        let original = DeltaChange::Modified;
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, "\"Modified\"");
        let back: DeltaChange = serde_json::from_str(&json).unwrap();
        assert_eq!(back, original);
    }

    /// `merge()` with an empty `other` is a no-op (existing `added` and
    /// `modified` are preserved).
    #[test]
    fn graph_delta_merge_with_empty_other_is_noop() {
        let mut delta = GraphDelta::default();
        delta.added.push(DeltaElement {
            element: make_element("e1"),
            change: DeltaChange::Added,
        });
        delta.modified.push(DeltaElement {
            element: make_element("e2"),
            change: DeltaChange::Modified,
        });

        let before_len = delta.len();
        delta.merge(GraphDelta::default());
        assert_eq!(delta.len(), before_len, "merge with empty is no-op");
        assert_eq!(delta.added.len(), 1);
        assert_eq!(delta.modified.len(), 1);
    }

    /// `merge()` into an empty self appends all entries from `other`.
    #[test]
    fn graph_delta_merge_into_empty_appends_all() {
        let mut delta = GraphDelta::default();
        let other = GraphDelta {
            added: vec![
                DeltaElement {
                    element: make_element("e1"),
                    change: DeltaChange::Added,
                },
                DeltaElement {
                    element: make_element("e2"),
                    change: DeltaChange::Added,
                },
            ],
            modified: vec![DeltaElement {
                element: make_element("e3"),
                change: DeltaChange::Modified,
            }],
        };
        delta.merge(other);
        assert_eq!(delta.added.len(), 2);
        assert_eq!(delta.modified.len(), 1);
        assert_eq!(delta.len(), 3);
    }

    /// `merge()` keeps the `added` and `modified` lists separate — an
    /// `added` element in `other` is NOT moved to `modified` in self.
    /// (The merge is a shallow concatenation, not a kind-aware blend.)
    #[test]
    fn graph_delta_merge_preserves_change_kinds() {
        let mut delta = GraphDelta::default();
        let other = GraphDelta {
            added: vec![DeltaElement {
                element: make_element("e1"),
                change: DeltaChange::Added,
            }],
            modified: vec![],
        };
        delta.merge(other);
        // e1 stays in `added`, NOT promoted to `modified`
        assert_eq!(delta.added.len(), 1);
        assert_eq!(delta.modified.len(), 0);
        assert!(matches!(delta.added[0].change, DeltaChange::Added));
    }

    /// `merge()` does NOT deduplicate duplicate elements. Two deltas with
    /// the same element-id produce two entries in the result.
    /// (Locks the comment at delta.rs:64 "duplicate elements are NOT
    /// deduplicated".)
    #[test]
    fn graph_delta_merge_does_not_dedup() {
        let mut delta = GraphDelta {
            added: vec![DeltaElement {
                element: make_element("e1"),
                change: DeltaChange::Added,
            }],
            modified: vec![],
        };
        let other = GraphDelta {
            added: vec![DeltaElement {
                element: make_element("e1"),
                change: DeltaChange::Added,
            }],
            modified: vec![],
        };
        delta.merge(other);
        // Both e1 entries preserved (no dedup)
        assert_eq!(delta.added.len(), 2);
        assert_eq!(delta.len(), 2);
    }
}
