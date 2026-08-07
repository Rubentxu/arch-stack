//! Test-only helpers shared across integration test files.
//!
//! M56: extracted `backend_available()` (skip-on-missing-backend) from
//! 5 e2e tests into `plantuml::backend_available`. Kept in the main
//! lib (not gated behind `#[cfg(test)]`) because integration tests in
//! `tests/` are separate crates and need to import via
//! `archctl::test_helpers`.

pub mod plantuml;
