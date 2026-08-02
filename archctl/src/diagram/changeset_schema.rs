//! Embed the ChangeSet JSON Schema at compile time.
//!
//! Mirrors the pattern in `schema_embed.rs` for the projection schema.

pub const CHANGESET_SCHEMA: &str = include_str!("../../../schemas/changeset.schema.json");
