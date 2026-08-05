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
}
