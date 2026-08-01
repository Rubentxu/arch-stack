//! TSG rule packs for call-graph extraction.
//!
//! Each pack is compiled in via `include_str!` so there is no runtime file I/O.
//! Adding a language = add one `.tsg` file.

/// Rule pack for Rust function/method/closure + call edges.
pub const RUST_TSG: &str = include_str!("rust.tsg");

/// Rule pack for TypeScript function/arrow/method + call edges.
pub const TYPESCRIPT_TSG: &str = include_str!("typescript.tsg");

/// Rule pack for Python function/method/lambda + call edges.
pub const PYTHON_TSG: &str = include_str!("python.tsg");
