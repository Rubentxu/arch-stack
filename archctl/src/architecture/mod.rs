//! Architecture bounded context — snapshot metadata management.
//!
//! `archctl architecture snapshot {create,list,gc,diff}` lives here. The module
//! exposes typed use cases (`create`, `list`, `gc`, `diff`) that consume
//! `SnapshotRepository` through the `GraphStore` port. Application code
//! must NOT construct `LbugStore` directly — use `CliContext` composition
//! (ADR-044/059 boundary).
//!
//! ## Public surface
//!
//! - `create` — create a snapshot, idempotent on the identity tuple.
//! - `list` — list all snapshots for a project.
//! - `gc` — garbage-collect old snapshots, preserving pinned and recent.
//! - `diff` — compare two snapshots and emit an `ArchitectureDiffReport`.
//! - `explain` — explain a subject (element or relation) by returning its provenance chain.
//! - `Snapshot` carrier — the domain struct for snapshot metadata.

pub mod diff;
pub mod digest;
pub mod errors;
pub mod explain;
pub mod snapshot;

// Public re-exports
pub use diff::{ArchitectureDiffReport, DiffError, diff_snapshots};
pub use errors::{SnapshotError, SnapshotGcReport};
pub use explain::{ExplainError, ExplainReport, explain};
pub use snapshot::{create, gc, list};
