//! Plugin tap model (ADR-057 §4) — minimal viable version.
//!
//! A "tap" is a JSON file served at a URL that lists available plugins.
//! Each plugin has a name, version, and (optional) archive URL.
//!
//! Format:
//! ```json
//! {
//!   "name": "archctl-official",
//!   "plugins": [
//!     {
//!       "name": "my-plugin",
//!       "version": "1.0.0",
//!       "url": "https://github.com/.../archive.tar.gz",
//!       "sha256": "..."
//!     }
//!   ]
//! }
//! ```

pub mod install;
use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

/// Plugin entry from a tap.
#[derive(Debug, Deserialize, Clone)]
pub struct PluginEntry {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
}

/// A tap is a named collection of plugin entries.
#[derive(Debug, Deserialize, Clone)]
pub struct Tap {
    pub name: String,
    pub plugins: Vec<PluginEntry>,
}

/// Fetch a tap JSON from a URL.
pub fn fetch_tap(url: &str) -> Result<Tap> {
    let body = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("archctl-plugin/1.0")
        .build()?
        .get(url)
        .send()?
        .error_for_status()?
        .text()?;
    Ok(serde_json::from_str(&body)?)
}

/// Returns the plugin install root path.
/// Uses ~/.local/share/archctl/plugins (via lifecycle::install_root).
pub fn plugin_install_root() -> PathBuf {
    crate::lifecycle::install_root::install_root().join("plugins")
}

/// Parse a plugin spec in the form "author/name@version" or "author/name" (latest).
pub fn parse_plugin_spec(spec: &str) -> Result<(String, String, String)> {
    // Format: "author/name@version" or "author/name" (latest).
    let (author_name, version) = match spec.split_once('@') {
        Some((an, v)) => (an.to_string(), v.to_string()),
        None => (spec.to_string(), "latest".to_string()),
    };
    let (author, name) = author_name
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("plugin spec must be 'author/name@version', got: {spec}"))?;
    Ok((author.to_string(), name.to_string(), version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_install_root_includes_archctl_segment() {
        // P0-04 regression: plugin root must be under archctl/, not a sibling.
        let root = plugin_install_root();
        let path_str = root.to_string_lossy();
        assert!(
            path_str.contains("archctl"),
            "expected path containing 'archctl', got: {}",
            root.display()
        );
        assert!(
            path_str.contains("plugins"),
            "expected path containing 'plugins', got: {}",
            root.display()
        );
        // Must NOT be under installs/ (that's for archctl binaries, not plugins)
        assert!(
            !path_str.contains("installs"),
            "plugin root must not be under installs/, got: {}",
            root.display()
        );
    }

    #[test]
    fn parse_plugin_spec_with_version() {
        let (author, name, version) = parse_plugin_spec("rubentxu/my-plugin@1.0.0").unwrap();
        assert_eq!(author, "rubentxu");
        assert_eq!(name, "my-plugin");
        assert_eq!(version, "1.0.0");
    }

    #[test]
    fn parse_plugin_spec_without_version_defaults_to_latest() {
        let (author, name, version) = parse_plugin_spec("rubentxu/my-plugin").unwrap();
        assert_eq!(author, "rubentxu");
        assert_eq!(name, "my-plugin");
        assert_eq!(version, "latest");
    }

    #[test]
    fn parse_tap_json() {
        let json = r#"{
            "name": "test-tap",
            "plugins": [
                {"name": "p1", "version": "1.0.0", "url": "https://x/y.tar.gz", "sha256": "abc123"}
            ]
        }"#;
        let tap: Tap = serde_json::from_str(json).unwrap();
        assert_eq!(tap.name, "test-tap");
        assert_eq!(tap.plugins.len(), 1);
        assert_eq!(tap.plugins[0].name, "p1");
        assert_eq!(tap.plugins[0].sha256.as_deref(), Some("abc123"));
    }

    #[test]
    fn parse_tap_json_without_optional_fields() {
        let json = r#"{
            "name": "minimal-tap",
            "plugins": [
                {"name": "p1", "version": "0.1.0"}
            ]
        }"#;
        let tap: Tap = serde_json::from_str(json).unwrap();
        assert_eq!(tap.plugins[0].url, None);
        assert_eq!(tap.plugins[0].sha256, None);
    }
}
