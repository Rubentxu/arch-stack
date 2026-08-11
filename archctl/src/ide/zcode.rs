//! ZCode adapter — ZCode is a fork of OpenCode and shares its discovery paths.

use super::*;

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
    fn install_stack(&self, payload: &StackPayload) -> Result<InstallReport> {
        // ZCode shares OpenCode's paths — delegate to OpenCodeAdapter logic.
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
