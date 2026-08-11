//! `archctl ide` — multi-IDE adapter abstraction (ADR-042).
//!
//! Replaces the hardcoded `~/.config/opencode/` from `stack.rs` with a
//! trait-based dispatcher. Each adapter handles its own discovery paths
//! and skill format conversions.

use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SkillFile {
    pub name: String,
    pub markdown: String,
    pub scripts: Vec<(String, Vec<u8>)>, // (relative_path, content)
}

#[derive(Debug, Clone)]
pub struct AgentFile {
    pub name: String,
    pub markdown: String,
}

#[derive(Debug, Clone)]
pub struct PluginFile {
    pub name: String,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct StackPayload {
    pub id: String, // e.g. "arch-stack-1.35.0"
    pub version: semver::Version,
    pub skills: Vec<SkillFile>,
    pub agents: Vec<AgentFile>,
    pub plugins: Vec<PluginFile>,
}

#[derive(Debug, Default)]
pub struct InstallReport {
    pub written: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DriftEntry {
    pub path: PathBuf,
    pub kind: DriftKind,
}

#[derive(Debug, Clone)]
pub enum DriftKind {
    Missing,
    Stale,
    Extra,
}

#[derive(Debug, Clone)]
pub struct IdePresence {
    pub installed: bool,
    pub hint: Option<String>,
}

/// The trait every IDE adapter must implement.
pub trait IdeAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn detect(&self) -> Result<IdePresence>;
    fn config_root(&self) -> PathBuf;
    fn install_stack(&self, payload: &StackPayload) -> Result<InstallReport>;
    fn remove_stack(&self, payload_id: &str) -> Result<InstallReport>;
    fn diff_stack(&self, payload: &StackPayload) -> Result<Vec<DriftEntry>>;
    /// Format converter for skills. Default impl returns unchanged.
    fn convert_skill(&self, skill_md: &str, _skill_name: &str) -> Result<String> {
        Ok(skill_md.to_string())
    }
}

/// All built-in adapters, in stable order.
pub fn builtin_adapters() -> Vec<Box<dyn IdeAdapter>> {
    vec![
        Box::new(opencode::OpenCodeAdapter),
        Box::new(zcode::ZCodeAdapter),
        Box::new(claude_code::ClaudeCodeAdapter),
        Box::new(codex::CodexAdapter),
    ]
}

pub mod claude_code;
pub mod codex;
pub mod opencode;
pub mod zcode;

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_payload() -> StackPayload {
        StackPayload {
            id: "arch-stack-0.0.0".into(),
            version: semver::Version::new(0, 0, 0),
            skills: vec![],
            agents: vec![],
            plugins: vec![],
        }
    }

    #[test]
    fn empty_payload_serializes_round_trip() {
        let p = empty_payload();
        assert_eq!(p.skills.len(), 0);
        assert_eq!(p.agents.len(), 0);
        assert_eq!(p.plugins.len(), 0);
    }

    #[test]
    fn builtin_adapters_returns_4_adapters() {
        let adapters = builtin_adapters();
        assert_eq!(adapters.len(), 4);
        let ids: Vec<&str> = adapters.iter().map(|a| a.id()).collect();
        assert_eq!(ids, vec!["opencode", "zcode", "claude-code", "codex"]);
    }
}
