//! Claude Code adapter — installs skills as Claude plugin format.

use super::*;
use std::path::Path;

pub struct ClaudeCodeAdapter;

impl IdeAdapter for ClaudeCodeAdapter {
    fn id(&self) -> &'static str {
        "claude-code"
    }
    fn name(&self) -> &'static str {
        "Claude Code"
    }
    fn detect(&self) -> Result<IdePresence> {
        let config = self.config_root();
        let installed = config.exists();
        Ok(IdePresence {
            installed,
            hint: None,
        })
    }
    fn config_root(&self) -> PathBuf {
        // Claude Code does NOT respect XDG_CONFIG_HOME — it looks at
        // ~/.claude/ directly. We must match that path exactly, or the
        // installed skills/agents are invisible to the IDE.
        crate::xdg::home_dir().join(".claude")
    }
    fn install_stack(
        &self,
        payload: &StackPayload,
        install_root: Option<&Path>,
    ) -> Result<InstallReport> {
        let mut report = InstallReport::default();
        let root = install_root
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.config_root());
        for skill in &payload.skills {
            let skill_md = self.convert_skill(&skill.markdown, &skill.name)?;
            let dir = root
                .join("plugins")
                .join("arch-stack")
                .join("skills")
                .join(&skill.name);
            std::fs::create_dir_all(&dir)?;
            let path = dir.join("SKILL.md");
            std::fs::write(&path, &skill_md)?;
            report.written.push(path);
        }
        for agent in &payload.agents {
            let dir = root.join("agents");
            std::fs::create_dir_all(&dir)?;
            let path = dir.join(format!("{}.md", agent.name));
            std::fs::write(&path, &agent.markdown)?;
            report.written.push(path);
        }
        Ok(report)
    }
    fn remove_stack(&self, payload_id: &str) -> Result<InstallReport> {
        let mut report = InstallReport::default();
        let root = self.config_root();
        let plugin_dir = root.join("plugins").join("arch-stack");
        if plugin_dir.exists() {
            std::fs::remove_dir_all(&plugin_dir)?;
            report.written.push(plugin_dir);
        }
        let _ = payload_id;
        Ok(report)
    }
    fn diff_stack(&self, payload: &StackPayload) -> Result<Vec<DriftEntry>> {
        let mut drift = vec![];
        let root = self.config_root();
        for skill in &payload.skills {
            let path = root
                .join("plugins")
                .join("arch-stack")
                .join("skills")
                .join(&skill.name)
                .join("SKILL.md");
            if !path.exists() {
                drift.push(DriftEntry {
                    path,
                    kind: DriftKind::Missing,
                });
            }
        }
        Ok(drift)
    }
    fn convert_skill(&self, skill_md: &str, _skill_name: &str) -> Result<String> {
        // M75 PR #3 will do actual frontmatter translation.
        // For now, return unchanged.
        Ok(skill_md.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> StackPayload {
        StackPayload {
            id: "arch-stack-1.35.0".into(),
            version: semver::Version::parse("1.35.0").unwrap(),
            skills: vec![SkillFile {
                name: "test-skill".into(),
                markdown: "---\nname: test\n---\n# test".into(),
                scripts: vec![],
            }],
            agents: vec![AgentFile {
                name: "test-agent".into(),
                markdown: "---\nname: test\n---\n# agent".into(),
            }],
            plugins: vec![],
        }
    }

    #[test]
    fn claude_code_install_with_install_root_uses_override_path() {
        // M84: --install-root must override the hardcoded ~/.claude/ path.
        // Particularly useful for Claude Code because config_root does NOT
        // respect XDG_CONFIG_HOME — the override is the only way to install
        // to a custom location.
        let a = ClaudeCodeAdapter;
        let tmp = tempfile::tempdir().unwrap();
        let custom = tmp.path().join("my-custom-claude");
        let payload = sample_payload();
        let report = a.install_stack(&payload, Some(&custom)).unwrap();
        assert!(report.errors.is_empty());
        assert!(
            custom
                .join("plugins")
                .join("arch-stack")
                .join("skills")
                .join("test-skill")
                .join("SKILL.md")
                .exists()
        );
        assert!(custom.join("agents").join("test-agent.md").exists());
    }

    #[test]
    fn claude_code_install_with_none_uses_config_root() {
        // None → ~/.claude/ (hardcoded path, NOT XDG-respecting per
        // archctl/src/ide/claude_code.rs:23-26 comment).
        let a = ClaudeCodeAdapter;
        let payload = sample_payload();
        let report = a.install_stack(&payload, None).unwrap();
        let default_root = a.config_root();
        for path in &report.written {
            assert!(
                path.starts_with(&default_root),
                "None fallback must write under config_root ({default_root:?}); got {path:?}"
            );
        }
        // Cleanup
        for path in &report.written {
            let _ = std::fs::remove_file(path);
        }
        let _ = std::fs::remove_dir_all(default_root.join("plugins").join("arch-stack"));
    }
}
