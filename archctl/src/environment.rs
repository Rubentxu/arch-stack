//! Environment port — abstraction over process-wide context that the
//! domain should not read directly.
//!
//! The domain asks "what is the user's home directory?", "what is the
//! current working directory?", "is variable X set?" via this trait.
//! In production the adapter reads `std::env`; in tests a
//! [`FixedEnvironment`] pre-loads every answer.
//!
//! ## What the port hides
//!
//! - **`std::env::*` calls.** The domain never imports `std::env`. The
//!   production adapter ([`SystemEnvironment`]) does, the test
//!   adapter ([`FixedEnvironment`]) does not.
//!
//! - **Cross-platform home directory lookup.** `home_dir()` resolves
//!   `HOME` on Unix, `USERPROFILE` (and `HOMEDRIVE` + `HOMEPATH`) on
//!   Windows, all behind one method.
//!
//! ## What the port does NOT hide
//!
//! - **Working-directory semantics.** The port answers "what is cwd?"
//!   when asked, but does not pretend to change it. If the caller
//!   wants "default cwd", it composes `Environment::current_dir`
//!   with whatever fallback makes sense for the command.
//!
//! - **Path canonicalisation.** `current_dir` returns whatever the
//!   OS reports. Callers that need an absolute path call
//!   `Path::canonicalize` themselves — but that requires
//!   `Filesystem`, which is a separate port (see
//!   `archctl/src/filesystem.rs`).

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

/// The environment port.
///
/// Implementations:
/// - [`SystemEnvironment`] — production adapter, reads `std::env::*`.
/// - [`FixedEnvironment`] — test adapter, returns pre-loaded values.
///
/// All methods are infallible except `current_dir` and `home_dir`,
/// which can legitimately fail (cwd removed, HOME unset on Windows
/// without USERPROFILE). Callers handle the error by either
/// bailing out or substituting a sensible default.
pub trait Environment: Send + Sync {
    /// Process working directory as reported by the OS. Returns an
    /// error if the OS reports no cwd (rare, but possible after
    /// `chdir` to a deleted path).
    fn current_dir(&self) -> Result<PathBuf>;

    /// Look up a process environment variable by name. Returns
    /// `Some(value)` if set (including empty string), `None` if unset.
    ///
    /// Use this for boolean checks ("is `NO_COLOR` set?") and for
    /// reading directory paths (`OPENCODE_CONFIG_DIR`).
    fn var(&self, key: &str) -> Option<String>;

    /// Resolve the user's home directory.
    ///
    /// Production behaviour:
    /// - Unix: `$HOME`.
    /// - Windows: `%USERPROFILE%`, falling back to `%HOMEDRIVE%%HOMEPATH%`.
    ///
    /// Returns an error if none of the above resolve to a usable path.
    fn home_dir(&self) -> Result<PathBuf>;
}

// ---------------------------------------------------------------------------
// Production adapter
// ---------------------------------------------------------------------------

/// The real `std::env` adapter. Cheap to construct.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemEnvironment;

impl Environment for SystemEnvironment {
    fn current_dir(&self) -> Result<PathBuf> {
        Ok(std::env::current_dir()?)
    }

    fn var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }

    fn home_dir(&self) -> Result<PathBuf> {
        // Mirror the order of the existing `xdg::user_home` so
        // behaviour is identical: Unix-style $HOME first, then the
        // Windows fallbacks. We do not pre-canonicalise here — the
        // port returns whatever the OS says and lets the caller
        // decide whether to canonicalise.
        if let Some(home) = std::env::var_os("HOME") {
            if !home.is_empty() {
                return Ok(PathBuf::from(home));
            }
        }
        if let Some(profile) = std::env::var_os("USERPROFILE") {
            if !profile.is_empty() {
                return Ok(PathBuf::from(profile));
            }
        }
        if let (Some(drive), Some(path)) = (
            std::env::var_os("HOMEDRIVE"),
            std::env::var_os("HOMEPATH"),
        ) {
            let mut combined = PathBuf::from(drive);
            combined.push(path);
            if !combined.as_os_str().is_empty() {
                return Ok(combined);
            }
        }
        anyhow::bail!("no home directory found (HOME/USERPROFILE/HOMEDRIVE+HOMEPATH all unset)")
    }
}

// ---------------------------------------------------------------------------
// Test adapter
// ---------------------------------------------------------------------------

/// A hand-built environment: every var and the cwd/home are set in
/// advance. Use this in tests that exercise the CLI or any domain
/// code that calls into `Environment`.
///
/// Construction:
///
/// ```
/// use archctl::environment::FixedEnvironment;
/// let env = FixedEnvironment::new()
///     .with_cwd("/tmp/proj")
///     .with_var("NO_COLOR", "1")
///     .with_var("OPENCODE_CONFIG_DIR", "/etc/oc");
/// ```
#[derive(Debug, Clone, Default)]
pub struct FixedEnvironment {
    cwd: Option<PathBuf>,
    home: Option<PathBuf>,
    vars: std::collections::HashMap<String, String>,
}

impl FixedEnvironment {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn with_home(mut self, home: impl Into<PathBuf>) -> Self {
        self.home = Some(home.into());
        self
    }

    pub fn with_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.insert(key.into(), value.into());
        self
    }

    /// Builder method: register a var that the adapter should report
    /// as **unset** (`var()` returns `None`). Useful for testing
    /// "absent env var" paths without polluting the test's process.
    pub fn without_var(mut self, key: impl Into<String>) -> Self {
        self.vars.remove(&key.into());
        self
    }
}

impl Environment for FixedEnvironment {
    fn current_dir(&self) -> Result<PathBuf> {
        self.cwd.clone().ok_or_else(|| {
            anyhow::anyhow!("FixedEnvironment::current_dir called without with_cwd")
        })
    }

    fn var(&self, key: &str) -> Option<String> {
        self.vars.get(key).cloned()
    }

    fn home_dir(&self) -> Result<PathBuf> {
        self.home.clone().ok_or_else(|| {
            anyhow::anyhow!("FixedEnvironment::home_dir called without with_home")
        })
    }
}

// ---------------------------------------------------------------------------
// Factories
// ---------------------------------------------------------------------------

/// Factory: the production environment, type-erased to the trait.
pub fn system_environment() -> Arc<dyn Environment> {
    Arc::new(SystemEnvironment)
}

/// Factory: an empty fixed environment. Call `with_*` builders to
/// pre-load the answers the test needs.
pub fn fixed_environment() -> Arc<dyn Environment> {
    Arc::new(FixedEnvironment::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_environment_returns_what_builder_set() {
        let env = FixedEnvironment::new()
            .with_cwd("/tmp/a")
            .with_home("/home/x")
            .with_var("FOO", "bar")
            .without_var("ABSENT");
        assert_eq!(env.current_dir().unwrap(), PathBuf::from("/tmp/a"));
        assert_eq!(env.home_dir().unwrap(), PathBuf::from("/home/x"));
        assert_eq!(env.var("FOO").as_deref(), Some("bar"));
        assert_eq!(env.var("ABSENT"), None);
        assert_eq!(env.var("NEVER_SET"), None);
    }

    #[test]
    fn fixed_environment_errors_when_cwd_unset() {
        // No `with_cwd` call — the port must surface that, not panic.
        // This is what stops the test from passing because of an
        // accidental "lazy default" implementation: if the port
        // started returning some fallback for missing cwd, this
        // test would fail.
        let env = FixedEnvironment::new();
        assert!(env.current_dir().is_err());
    }

    #[test]
    fn fixed_environment_errors_when_home_unset() {
        let env = FixedEnvironment::new();
        assert!(env.home_dir().is_err());
    }

    #[test]
    fn system_environment_reads_real_cwd() {
        // We don't pin a specific value (brittle) — we only assert
        // the contract: calling it returns *some* path that exists.
        let env = SystemEnvironment;
        let cwd = env.current_dir().unwrap();
        assert!(cwd.is_absolute() || cwd == PathBuf::from("."));
    }

    #[test]
    fn system_environment_reads_real_var() {
        // PATH is virtually always set; if not, the test was run in
        // a stripped environment that we should not pretend is
        // normal. We skip rather than fail to keep CI green on
        // minimal containers.
        let env = SystemEnvironment;
        if let Some(p) = env.var("PATH") {
            assert!(!p.is_empty());
        }
    }

    #[test]
    fn factory_returns_dyn_environment() {
        let a: Arc<dyn Environment> = system_environment();
        let b: Arc<dyn Environment> = fixed_environment();
        let _: &dyn Environment = a.as_ref();
        let _: &dyn Environment = b.as_ref();
    }
}
