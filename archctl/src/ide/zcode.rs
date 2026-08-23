//! ZCode adapter — ZCode is a fork of OpenCode and shares its discovery paths.

use super::*;
use std::path::Path;

pub struct ZCodeAdapter;

impl IdeAdapter for ZCodeAdapter {
    fn id(&self) -> &'static str {
        "zcode"
    }
    fn name(&self) -> &'static str {
        "ZCode"
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
        crate::xdg::xdg_config_home().join("opencode")
    }
    fn install_stack(
        &self,
        payload: &StackPayload,
        install_root: Option<&Path>,
    ) -> Result<InstallReport> {
        // ZCode shares OpenCode's paths — delegate to OpenCodeAdapter logic.
        let mut report = InstallReport::default();
        let root = install_root
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.config_root());
        for skill in &payload.skills {
            let dir = root.join("skills").join(&skill.name);
            std::fs::create_dir_all(&dir)?;
            let path = dir.join("SKILL.md");
            std::fs::write(&path, &skill.markdown)?;
            report.written.push(path);
        }
        for agent in &payload.agents {
            let dir = root.join("agents");
            std::fs::create_dir_all(&dir)?;
            let path = dir.join(format!("{}.md", agent.name));
            std::fs::write(&path, &agent.markdown)?;
            report.written.push(path);
        }
        for plugin in &payload.plugins {
            let dir = root.join("plugins");
            std::fs::create_dir_all(&dir)?;
            let path = dir.join(format!("{}.ts", plugin.name));
            std::fs::write(&path, &plugin.source)?;
            report.written.push(path);
        }
        Ok(report)
    }
    fn remove_stack(&self, payload_id: &str) -> Result<InstallReport> {
        let mut report = InstallReport::default();
        let root = self.config_root();
        if let Ok(entries) = std::fs::read_dir(root.join("skills")) {
            for entry in entries.filter_map(Result::ok) {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    std::fs::remove_dir_all(entry.path())?;
                    report.written.push(entry.path());
                }
            }
        }
        let _ = payload_id;
        Ok(report)
    }
    fn diff_stack(&self, payload: &StackPayload) -> Result<Vec<DriftEntry>> {
        let mut drift = vec![];
        let root = self.config_root();
        for skill in &payload.skills {
            let path = root.join("skills").join(&skill.name).join("SKILL.md");
            if !path.exists() {
                drift.push(DriftEntry {
                    path,
                    kind: DriftKind::Missing,
                });
            }
        }
        Ok(drift)
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
    fn zcode_install_with_install_root_uses_override_path() {
        // M84: --install-root must override ZCode's XDG-respecting path.
        let a = ZCodeAdapter;
        let tmp = tempfile::tempdir().unwrap();
        let custom = tmp.path().join("my-custom-zcode");
        let payload = sample_payload();
        let report = a.install_stack(&payload, Some(&custom)).unwrap();
        assert!(report.errors.is_empty());
        assert!(custom.join("skills/test-skill/SKILL.md").exists());
        assert!(custom.join("agents/test-agent.md").exists());
    }

    #[test]
    fn zcode_install_with_none_uses_config_root() {
        // None → XDG-respecting ~/.config/opencode/ (ZCode shares OpenCode's
        // discovery path).
        let a = ZCodeAdapter;
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
        let _ = std::fs::remove_dir_all(default_root.join("skills").join("test-skill"));
        let _ = std::fs::remove_file(default_root.join("agents").join("test-agent.md"));
    }
}
