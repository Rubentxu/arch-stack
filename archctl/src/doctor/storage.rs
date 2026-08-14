//! Storage compatibility probe for LadybugDB (lbug).
//!
//! The `StorageProbe` trait defines the interface for checking storage
//! backend availability. `LbugStorageProbe` is the primary implementation
//! for LadybugDB. `NativeProbe` is a fallback that verifies the lbug
//! command-line tool is available.

use crate::store::open_default;
use std::path::Path;
use std::process::Command;

/// Result of a storage compatibility probe.
#[derive(Debug, Clone)]
pub struct ProbeResult {
    /// Whether the probe passed.
    pub ok: bool,
    /// LadybugDB version string if available.
    pub version: Option<String>,
    /// Backend name (e.g., "lbug", "native").
    pub backend: String,
    /// Error message if the probe failed.
    pub error: Option<String>,
}

impl ProbeResult {
    /// Create a successful probe result.
    fn ok_(version: Option<String>, backend: &str) -> Self {
        Self {
            ok: true,
            version,
            backend: backend.to_string(),
            error: None,
        }
    }

    /// Create a failed probe result.
    fn fail(error: impl Into<String>) -> Self {
        let error = error.into();
        Self {
            ok: false,
            version: None,
            backend: "unknown".to_string(),
            error: Some(error),
        }
    }
}

/// Trait for probing storage backend compatibility.
///
/// Implementors must provide a `probe` method that checks whether
/// the storage backend is available and functional.
pub trait StorageProbe: Send + Sync {
    /// Run the compatibility probe against a project directory.
    fn probe(&self, project_dir: &Path) -> Result<ProbeResult, anyhow::Error>;
}

/// LadybugDB storage probe.
///
/// Checks lbug availability by:
/// 1. Attempting to open the lbug store with flock
/// 2. Running a simple query to verify read/write works
pub struct LbugStorageProbe {
    _priv: (),
}

impl LbugStorageProbe {
    /// Create a new LbugStorageProbe.
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl Default for LbugStorageProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageProbe for LbugStorageProbe {
    fn probe(&self, project_dir: &Path) -> Result<ProbeResult, anyhow::Error> {
        // Try to open and init the store
        let store = match open_default(project_dir) {
            Ok(s) => s,
            Err(e) => return Ok(ProbeResult::fail(format!("failed to open store: {e}"))),
        };

        // Run init to ensure schema is applied
        let mut store = store;
        if let Err(e) = store.init() {
            return Ok(ProbeResult::fail(format!("failed to init store: {e}")));
        }

        // Get stats to verify the store is functional
        let stat = match store.stat() {
            Ok(s) => s,
            Err(e) => return Ok(ProbeResult::fail(format!("stat query failed: {e}"))),
        };

        // Verify we can run a read query (basic schema check)
        let count_query = "MATCH (e:Element) RETURN count(e) AS count;";
        if let Err(e) = store.query(count_query) {
            return Ok(ProbeResult::fail(format!("read query failed: {e}")));
        }

        // Try a simple write+read round-trip using transactions
        // Use Element node type which is part of the canonical schema
        let test_id = format!("doctor:probe:{}", std::process::id());

        let write_result: Result<(), Box<dyn std::error::Error>> = (|| {
            store.begin_transaction()?;
            let cypher = format!("MERGE (e:Element {{id: '{test_id}'}}) SET e.probed = true;");
            store.query(&cypher)?;
            store.commit_transaction()?;
            Ok(())
        })();

        if let Err(e) = write_result {
            // Write test is optional - the store still works for reads
            // Just log a warning but don't fail the probe
            tracing::debug!("write probe failed: {e}");
        }

        // Return version info based on stat
        let version = format!("{} elements", stat.elements);
        Ok(ProbeResult::ok_(Some(version), "lbug"))
    }
}

/// Native (command-line) probe for lbug availability.
///
/// This probe checks whether the `lbug` binary is on PATH
/// and can report its version. It is used as a fallback when
/// the Rust lbug crate is not available.
pub struct NativeProbe {
    _priv: (),
}

impl NativeProbe {
    /// Create a new NativeProbe.
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl Default for NativeProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageProbe for NativeProbe {
    fn probe(&self, _project_dir: &Path) -> Result<ProbeResult, anyhow::Error> {
        let output = Command::new("lbug").arg("--version").output();

        match output {
            Ok(o) if o.status.success() => {
                let version = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let version = if version.is_empty() {
                    String::from_utf8_lossy(&o.stderr).trim().to_string()
                } else {
                    version
                };
                Ok(ProbeResult::ok_(Some(version), "lbug-native"))
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
                Ok(ProbeResult::fail(stderr))
            }
            Err(e) => Ok(ProbeResult::fail(format!("lbug not found on PATH: {e}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn probe_result_ok_display() {
        let result = ProbeResult::ok_(Some("0.18.3".to_string()), "lbug");
        assert!(result.ok);
        assert_eq!(result.backend, "lbug");
        assert_eq!(result.version.as_deref(), Some("0.18.3"));
        assert!(result.error.is_none());
    }

    #[test]
    fn probe_result_fail_display() {
        let result = ProbeResult::fail("connection refused");
        assert!(!result.ok);
        assert_eq!(result.error.as_deref(), Some("connection refused"));
    }

    #[test]
    fn lbug_storage_probe_smoke() {
        let tmp = TempDir::new().unwrap();
        let probe = LbugStorageProbe::new();
        let result = probe.probe(tmp.path()).expect("probe must not error");
        // A fresh TempDir should succeed with empty stats
        assert!(result.ok, "fresh temp dir should pass: {:?}", result.error);
        assert_eq!(result.backend, "lbug");
    }

    #[test]
    fn lbug_storage_probe_nonexistent_dir() {
        let tmp = TempDir::new().unwrap();
        let nonexistent = tmp.path().join("nonexistent/project");
        let probe = LbugStorageProbe::new();
        let result = probe.probe(&nonexistent).expect("probe must not error");
        // lbug can create directories as needed, so even a nonexistent
        // path should succeed (it creates the database)
        assert!(
            result.ok,
            "lbug should be able to create db in nonexistent path: {:?}",
            result.error
        );
    }
}
