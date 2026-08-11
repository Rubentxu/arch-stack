//! Claude Code adapter — installs skills as Claude plugin format.

use super::*;

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
        crate::xdg::xdg_config_home().join("claude")
    }
    fn install_stack(&self, payload: &StackPayload) -> Result<InstallReport> {
        let mut report = InstallReport::default();
        let root = self.config_root();
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
