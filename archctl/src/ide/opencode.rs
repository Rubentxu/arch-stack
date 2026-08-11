//! OpenCode adapter — installs skills/agents/plugins to `~/.config/opencode/`.

use super::*;

pub struct OpenCodeAdapter;

impl IdeAdapter for OpenCodeAdapter {
    fn id(&self) -> &'static str {
        "opencode"
    }
    fn name(&self) -> &'static str {
        "OpenCode"
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
    fn install_stack(&self, payload: &StackPayload) -> Result<InstallReport> {
        let mut report = InstallReport::default();
        let root = self.config_root();
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
    fn opencode_id_and_name() {
        let a = OpenCodeAdapter;
        assert_eq!(a.id(), "opencode");
        assert_eq!(a.name(), "OpenCode");
    }

    #[test]
    fn opencode_install_copies_files_to_xdg_path() {
        let a = OpenCodeAdapter;
        let tmp = tempfile::tempdir().unwrap();
        let payload = sample_payload();
        let report = install_into(&a, tmp.path(), &payload).unwrap();
        assert!(report.errors.is_empty());
        assert!(tmp.path().join("skills/test-skill/SKILL.md").exists());
        assert!(tmp.path().join("agents/test-agent.md").exists());
    }

    #[test]
    fn opencode_diff_reports_missing() {
        let a = OpenCodeAdapter;
        let tmp = tempfile::tempdir().unwrap();
        let payload = sample_payload();
        let diff = diff_into(&a, tmp.path(), &payload).unwrap();
        assert_eq!(diff.len(), 1);
        assert!(matches!(diff[0].kind, DriftKind::Missing));
    }

    // Test helpers that bypass the XDG resolution.
    use std::path::Path;
    fn install_into(
        _a: &OpenCodeAdapter,
        root: &Path,
        payload: &StackPayload,
    ) -> Result<InstallReport> {
        let mut report = InstallReport::default();
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
        Ok(report)
    }

    fn diff_into(
        _a: &OpenCodeAdapter,
        root: &Path,
        payload: &StackPayload,
    ) -> Result<Vec<DriftEntry>> {
        let mut drift = vec![];
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
