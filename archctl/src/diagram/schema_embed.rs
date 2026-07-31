//! Embed the JSON Schema at compile time.
//!
//! The schema is included as a string constant so the validate module
//! can validate bundles without requiring the schema file to exist at runtime.

pub const SCHEMA: &str = include_str!("../../../schemas/diagram-projection.schema.json");
