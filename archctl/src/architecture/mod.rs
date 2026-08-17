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
//! - `coverage` — compute evidence quality coverage metrics over the live graph.
//! - `Snapshot` carrier — the domain struct for snapshot metadata.

pub mod coverage;
pub mod diff;
pub mod digest;
pub mod errors;
pub mod explain;
pub mod intent;
pub mod intent_loader;
pub mod policy;
pub mod relevance;
pub mod report_formats;
pub mod snapshot;
pub mod task_context;

// Public re-exports
pub use crate::observation_claim::{
    Claim, Observation, ObservationError, compat_claim_from_evidence, observation_from_evidence,
    observations_and_claims_for_version,
};
pub use coverage::{CoverageError, CoverageReport, coverage};
pub use diff::{ArchitectureDiffReport, DiffError, diff_snapshots};
pub use errors::{SnapshotError, SnapshotGcReport};
pub use explain::{ExplainError, ExplainReport, explain};
pub use intent::{IntentDeclaration, IntentDelta, IntentError, IntentReport, check_intent};
pub use intent_loader::load_intent;
pub use policy::{PolicyError, PolicyReport, PolicyRule, Waiver, check_policy};
pub use relevance::{
    RelevanceError, RelevanceOptions, RelevanceReport, ScoredElement, ScoredRelation,
    SelectionTrace, relevance,
};
pub use report_formats::{to_junit_xml, to_sarif};
pub use snapshot::{create, gc, list};
pub use task_context::{
    ContextElement, ContextError, ContextRelation, TaskContextReport, compile_task_context,
};
