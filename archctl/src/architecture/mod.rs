//! Architecture bounded context — snapshot metadata management.
//!
//! `archctl architecture snapshot {create,list,gc}` lives here. The module
//! exposes typed use cases (`create`, `list`, `gc`) that consume
//! `SnapshotRepository` through the `GraphStore` port. Application code
//! must NOT construct `LbugStore` directly — use `CliContext` composition
//! (ADR-044/059 boundary).
//!
//! ## Public surface
//!
//! - `create` — create a snapshot, idempotent on the identity tuple.
//! - `list` — list all snapshots for a project.
//! - `gc` — garbage-collect old snapshots, preserving pinned and recent.
//! - `Snapshot` carrier — the domain struct for snapshot metadata.

pub mod digest;
pub mod errors;
pub mod snapshot;

// Public re-exports
pub use errors::{SnapshotError, SnapshotGcReport};
pub use snapshot::{create, gc, list};
