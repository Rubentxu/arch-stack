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

/// Validate a plugin identifier (author or name) for path traversal safety.
/// Rejects empty strings, `..`, path separators, null bytes, and flag-like inputs.
fn validate_plugin_identifier(id: &str, kind: &str) -> Result<()> {
    if id.is_empty() {
        anyhow::bail!("plugin {kind} cannot be empty");
    }
    if id.contains("..") {
        anyhow::bail!("plugin {kind} contains path traversal ('..'): {id}");
    }
    if id.contains('/') || id.contains('\\') {
        anyhow::bail!("plugin {kind} contains path separator: {id}");
    }
    if id.contains('\0') {
        anyhow::bail!("plugin {kind} contains null byte");
    }
    if id.starts_with('-') {
        anyhow::bail!("plugin {kind} starts with '-' (flag-like): {id}");
    }
    Ok(())
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
    // P0-06: validate identifiers before any I/O (SCN-PLG-01).
    validate_plugin_identifier(author, "author")?;
    validate_plugin_identifier(name, "name")?;
    Ok((author.to_string(), name.to_string(), version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_install_root_is_install_root_plus_plugins() {
        // P0-04 regression: plugin root must be install_root().join("plugins"),
        // not install_root().parent().join("plugins").
        let root = plugin_install_root();
        let expected = crate::lifecycle::install_root::install_root().join("plugins");
        assert_eq!(
            root, expected,
            "plugin_install_root must be install_root/plugins"
        );
    }

    #[test]
    fn parse_plugin_spec_rejects_path_traversal_in_author() {
        // SCN-PLG-01: ../../evil rejected before I/O.
        let result = parse_plugin_spec("../evil/my-plugin@1.0.0");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("traversal"));
    }

    #[test]
    fn parse_plugin_spec_rejects_path_traversal_in_name() {
        let result = parse_plugin_spec("author/../../etc/passwd@1.0.0");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("traversal"));
    }

    #[test]
    fn parse_plugin_spec_rejects_flag_like_identifier() {
        let result = parse_plugin_spec("-flag/my-plugin");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("flag"));
    }

    #[test]
    fn parse_plugin_spec_accepts_valid_identifiers() {
        let (author, name, version) = parse_plugin_spec("rubentxu/my-cool-plugin@2.3.4").unwrap();
        assert_eq!(author, "rubentxu");
        assert_eq!(name, "my-cool-plugin");
        assert_eq!(version, "2.3.4");
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
