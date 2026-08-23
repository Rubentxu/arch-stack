//! Codex adapter — converts SKILL.md to TOML prompts.

use super::*;
use std::path::Path;

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
            agents: vec![],
            plugins: vec![],
        }
    }

    #[test]
    fn codex_install_with_install_root_uses_override_path() {
        // M84: --install-root must override the hardcoded ~/.codex/ path.
        // Codex converts SKILL.md → TOML prompts; verify the override
        // path is used for both the file content and location.
        let a = CodexAdapter;
        let tmp = tempfile::tempdir().unwrap();
        let custom = tmp.path().join("my-custom-codex");
        let payload = sample_payload();
        let report = a.install_stack(&payload, Some(&custom)).unwrap();
        assert!(report.errors.is_empty());
        let prompt_path = custom.join("prompts").join("test-skill.toml");
        assert!(prompt_path.exists());
        let body = std::fs::read_to_string(&prompt_path).unwrap();
        assert!(
            body.contains("prompt ="),
            "TOML must wrap the markdown as a prompt key"
        );
        assert!(
            body.contains("# test"),
            "TOML must preserve the original markdown"
        );
    }

    #[test]
    fn codex_install_with_none_uses_config_root() {
        // None → ~/.codex/ (hardcoded, NOT XDG-respecting per
        // archctl/src/ide/codex.rs:23-25 comment).
        let a = CodexAdapter;
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
    }
}
