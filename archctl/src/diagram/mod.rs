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
pub mod project;
pub mod project_selector;
pub mod queries;
pub mod schema_embed;
pub mod selector;
pub mod validate;
pub mod view_types;

// Public re-exports
pub use apply::{ApplyReport, run_apply};
pub use assets::icon_for;
pub use changeset_types::{CHANGESET_COMMAND_TYPES, ChangeSet, Command};
pub use export::{BundleEnvelope, ExportReport, build_bundle, build_export_envelope, run_export};
pub use hash::base_revision;
pub use project::{OutputFormat, ProjectReport, project_dsl};
pub use project_selector::{ProjectSelector, ScopeFilter, ViewKind};
pub use selector::{C4Kind, ViewSelector, parse};
pub use validate::{ValidationError, ValidationReport, run_validate};
