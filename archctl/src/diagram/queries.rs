//! Graph read queries for diagram bundle export.
//!
//! P1-04: the four `query_*` functions have been removed — callers should
//! use [`crate::store::DiagramRepository`] directly (import `graph::ElementRow`
//! etc. from [`crate::graph`]).

// Re-export types from graph for backward compatibility (P1-04).
// These types are identical to the ones that were previously defined here.
#[deprecated(since = "1.43.0", note = "use crate::graph::ElementRow directly")]
pub use crate::graph::ElementRow;

#[deprecated(since = "1.43.0", note = "use crate::graph::SemanticEdgeRow directly")]
pub use crate::graph::SemanticEdgeRow;

#[deprecated(since = "1.43.0", note = "use crate::graph::VersionPropsRow directly")]
pub use crate::graph::VersionPropsRow;
