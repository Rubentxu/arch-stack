//! `archctl stack` — install/update/status of the arch-stack product.
//!
//! The stack is ONE product: archctl binary (self) + archview workbench
//! (embedded, ADR-033) + agent skills/agents/plugin (embedded here).
//! `stack install` copies the embedded skills/agents/plugin into the
//! OpenCode/ZCode discovery paths; `stack update` re-runs idempotently;
//! `stack status` reports drift between installed and embedded.

use anyhow::{Context, Result};
use rust_embed::RustEmbed;
use std::path::{Path, PathBuf};

#[derive(RustEmbed)]
#[folder = "assets-stack/"]
struct StackAssets;

/// Where stack components are installed (defaults to OpenCode config root).
pub fn default_install_root() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(".config"))
                .unwrap_or_else(|| PathBuf::from("."))
        })
        .join("opencode")
}

fn write_asset(rel_path: &str, content: &[u8], root: &Path) -> Result<bool> {
    let target = root.join(rel_path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create dir {}", parent.display()))?;
    }
    // Idempotent: skip write if content is identical.
    if target.exists() {
        let existing = std::fs::read(&target).unwrap_or_default();
        if existing == content {
            return Ok(false);
        }
    }
    std::fs::write(&target, content).with_context(|| format!("write {}", target.display()))?;
    Ok(true)
}

/// Copy all embedded stack assets under `prefix/` into `root/prefix/`.
/// Returns the list of files (re)written.
pub fn install(root: &Path) -> Result<Vec<String>> {
    let mut written = Vec::new();
    let mut embedded: Vec<String> = StackAssets::iter()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();
    embedded.sort();

    if embedded.is_empty() {
        anyhow::bail!("stack assets not embedded — run: scripts/embed-stack.sh && cargo build");
    }

    for path in &embedded {
        if path.ends_with('/') {
            continue;
        }
        let file = StackAssets::get(path).expect("embedded asset exists");
        if write_asset(path, &file.data, root)? {
            written.push(path.clone());
        }
    }
    Ok(written)
}

/// Report of what is installed vs what the binary embeds.
#[derive(Debug)]
pub struct StackStatus {
    pub binary_version: &'static str,
    pub embedded_skills: usize,
    pub embedded_agents: usize,
    pub installed_skills: usize,
    pub installed_agents: usize,
    pub drift: Vec<String>,
}

/// Count top-level component dirs under `root/skills` that contain SKILL.md.
fn count_skills(root: &Path) -> usize {
    let skills_dir = root.join("skills");
    let Ok(entries) = std::fs::read_dir(&skills_dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|e| e.path().join("SKILL.md").is_file())
        .count()
}

fn count_files(root: &Path, sub: &str) -> usize {
    let dir = root.join(sub);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .count()
}

/// Compute alignment between embedded assets and the install target.
pub fn status(root: &Path) -> Result<StackStatus> {
    let embedded: Vec<String> = StackAssets::iter()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty() && !s.ends_with('/'))
        .collect();

    let embedded_skills = embedded
        .iter()
        .filter(|s| s.starts_with("skills/") && s.ends_with("SKILL.md"))
        .count();
    let embedded_agents = embedded
        .iter()
        .filter(|s| s.starts_with("agents/") && s.ends_with(".md"))
        .count();

    let installed_skills = count_skills(root);
    let installed_agents = count_files(root, "agents");

    // Drift: embedded file present but different at target.
    let mut drift = Vec::new();
    for path in &embedded {
        let target = root.join(path);
        if !target.exists() {
            drift.push(format!("missing: {path}"));
            continue;
        }
        let Some(file) = StackAssets::get(path) else {
            continue;
        };
        let existing = std::fs::read(&target).unwrap_or_default();
        if existing != file.data.into_owned() {
            drift.push(format!("stale: {path}"));
        }
    }

    Ok(StackStatus {
        binary_version: env!("CARGO_PKG_VERSION"),
        embedded_skills,
        embedded_agents,
        installed_skills,
        installed_agents,
        drift,
    })
}

/// Print a human-readable status report.
pub fn print_status(s: &StackStatus) {
    println!("arch-stack status (binary v{})", s.binary_version);
    println!(
        "  embedded: {} skills, {} agents",
        s.embedded_skills, s.embedded_agents
    );
    println!(
        "  installed: {} skills, {} agents",
        s.installed_skills, s.installed_agents
    );
    if s.drift.is_empty() {
        println!("  drift: none — stack aligned");
    } else {
        println!("  drift ({}):", s.drift.len());
        for d in &s.drift {
            println!("    - {d}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root() -> PathBuf {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path().to_path_buf()
    }

    #[test]
    fn install_copies_embedded_assets() {
        // Only meaningful when assets were embedded (embed-stack.sh).
        if StackAssets::get("skills/stack-management/SKILL.md").is_none() {
            return;
        }
        let root = test_root();
        let written = install(&root).unwrap();
        assert!(!written.is_empty());
        // Idempotent: second run writes nothing.
        let written2 = install(&root).unwrap();
        assert!(written2.is_empty());
    }

    #[test]
    fn status_reports_drift_on_missing() {
        if StackAssets::get("skills/stack-management/SKILL.md").is_none() {
            return;
        }
        let root = test_root();
        let s = status(&root).unwrap();
        assert!(!s.drift.is_empty(), "empty target must show drift");
        assert!(s.installed_skills == 0);
        assert!(s.embedded_skills >= 8);
    }

    #[test]
    fn status_aligned_after_install() {
        if StackAssets::get("skills/stack-management/SKILL.md").is_none() {
            return;
        }
        let root = test_root();
        install(&root).unwrap();
        let s = status(&root).unwrap();
        assert!(
            s.drift.is_empty(),
            "installed must be aligned: {:?}",
            s.drift
        );
        assert_eq!(s.embedded_skills, s.installed_skills);
    }

    #[test]
    fn default_root_uses_xdg_or_home() {
        let root = default_install_root();
        assert!(root.ends_with("opencode"), "root: {}", root.display());
    }
}
