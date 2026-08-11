//! Codex adapter — converts SKILL.md to TOML prompts.

use super::*;

pub struct CodexAdapter;

impl IdeAdapter for CodexAdapter {
    fn id(&self) -> &'static str {
        "codex"
    }
    fn name(&self) -> &'static str {
        "Codex CLI"
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
        // Codex CLI does NOT respect XDG_CONFIG_HOME — it looks at
        // ~/.codex/ directly. We must match that path exactly.
        crate::xdg::home_dir().join(".codex")
    }
    fn install_stack(&self, payload: &StackPayload) -> Result<InstallReport> {
        let mut report = InstallReport::default();
        let root = self.config_root();
        for skill in &payload.skills {
            let dir = root.join("prompts");
            std::fs::create_dir_all(&dir)?;
            let path = dir.join(format!("{}.toml", skill.name));
            let toml_content = format!(
                r#"prompt = """
{}
"""
"#,
                skill.markdown
            );
            std::fs::write(&path, &toml_content)?;
            report.written.push(path);
        }
        Ok(report)
    }
    fn remove_stack(&self, payload_id: &str) -> Result<InstallReport> {
        let mut report = InstallReport::default();
        let root = self.config_root();
        let prompts_dir = root.join("prompts");
        if prompts_dir.exists() {
            for entry in std::fs::read_dir(&prompts_dir)?.filter_map(Result::ok) {
                let path = entry.path();
                if path.extension().map(|e| e == "toml").unwrap_or(false) {
                    std::fs::remove_file(&path)?;
                    report.written.push(path);
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
            let path = root.join("prompts").join(format!("{}.toml", skill.name));
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
