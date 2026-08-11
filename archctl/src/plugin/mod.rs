//! Plugin tap model (ADR-040 §4) — minimal viable version.
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
/// Uses XDG_DATA_HOME/plugins or ~/.local/share/archctl/plugins.
pub fn plugin_install_root() -> PathBuf {
    // Use lifecycle::install_root to get the archctl data dir,
    // then navigate to plugins subdirectory.
    let archctl_data = crate::lifecycle::install_root::install_root();
    archctl_data
        .parent()
        .map(|p| p.join("plugins"))
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
                format!(
                    "{}/.local/share",
                    std::env::var("HOME").unwrap_or_else(|_| "~".into())
                )
            }))
            .join("archctl")
            .join("plugins")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_install_root_ends_with_plugins() {
        let p = plugin_install_root();
        assert!(
            p.ends_with("plugins"),
            "expected path ending with 'plugins', got: {}",
            p.display()
        );
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
