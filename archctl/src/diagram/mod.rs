//! Diagram bundle export, validation, and apply.
//!
//! `archctl diagram export` produces a deterministic 5-file viewer-bundle
//! from a C4 view selector. `archctl diagram validate` checks a bundle
//! against the JSON Schema and internal-consistency rules.
//! `archctl diagram apply` applies a ChangeSet to the persisted graph.
//!
//! Apply acquires a shared-lock on the project DB (fs2) before mutating.

pub mod apply;
pub mod assets;
pub mod changeset_schema;
pub mod changeset_types;
pub mod export;
pub mod export_types;
pub mod hash;
pub mod queries;
pub mod schema_embed;
pub mod selector;
pub mod validate;
pub mod view_types;

// Public re-exports
pub use apply::{run_apply, ApplyReport};
pub use assets::icon_for;
pub use changeset_types::{ChangeSet, Command, CHANGESET_COMMAND_TYPES};
pub use export::{run_export, ExportReport};
pub use hash::base_revision;
pub use selector::{parse, C4Kind, ScopeFilter, ViewSelector};
pub use validate::{run_validate, ValidationError, ValidationReport};
