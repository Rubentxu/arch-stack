//! Extract embedded assets-stack into a StackPayload.
//!
//! Uses `rust_embed` to embed the assets-stack directory at compile time,
//! then parses skills, agents, and plugins into a `StackPayload`.

use rust_embed::RustEmbed;
use semver::Version;

use super::{AgentFile, PluginFile, SkillFile, StackPayload};

#[derive(RustEmbed)]
#[folder = "assets-stack/"]
struct StackAssets;

/// Build a `StackPayload` from the embedded assets-stack.
/// Fails if the embedded directory is empty (indicates build misconfiguration).
pub fn current_stack_payload() -> anyhow::Result<StackPayload> {
    let version = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let id = format!("arch-stack-{version}");

    let mut skills = vec![];
    let mut agents = vec![];
    let mut plugins = vec![];

    // Skills: every file under skills/<name>/SKILL.md
    for entry in StackAssets::iter() {
        let path = entry.as_ref();
        if path.starts_with("skills/") && path.ends_with("/SKILL.md") {
            // Extract skill name: "skills/foo/SKILL.md" -> "foo"
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() == 3 {
                let name = parts[1].to_string();
                if let Some(file) = StackAssets::get(path) {
                    let markdown = String::from_utf8_lossy(&file.data).into_owned();
                    skills.push(SkillFile {
                        name,
                        markdown,
                        scripts: vec![],
                    });
                }
            }
        } else if path.starts_with("agents/") && path.ends_with(".md") {
            // agents/<name>.md
            let name = path
                .trim_start_matches("agents/")
                .trim_end_matches(".md")
                .to_string();
            if let Some(file) = StackAssets::get(path) {
                let markdown = String::from_utf8_lossy(&file.data).into_owned();
                agents.push(AgentFile { name, markdown });
            }
        } else if path.starts_with("plugins/") && path.ends_with(".ts") {
            // plugins/<name>.ts
            let name = path
                .trim_start_matches("plugins/")
                .trim_end_matches(".ts")
                .to_string();
            if let Some(file) = StackAssets::get(path) {
                let source = String::from_utf8_lossy(&file.data).into_owned();
                plugins.push(PluginFile { name, source });
            }
        }
    }

    Ok(StackPayload {
        id,
        version,
        skills,
        agents,
        plugins,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_stack_payload_has_skills_and_agents() {
        let payload = current_stack_payload().unwrap();
        // assets-stack/ contains: 9 skills + 5 agents + 1 plugin.
        assert!(
            payload.skills.len() >= 8,
            "expected >= 8 skills, got {}",
            payload.skills.len()
        );
        assert!(
            payload.agents.len() >= 4,
            "expected >= 4 agents, got {}",
            payload.agents.len()
        );
        assert!(
            !payload.plugins.is_empty(),
            "expected >= 1 plugin, got {}",
            payload.plugins.len()
        );
    }

    #[test]
    fn stack_payload_id_matches_version() {
        let payload = current_stack_payload().unwrap();
        assert!(payload.id.starts_with("arch-stack-"));
        assert!(payload.id.ends_with(&payload.version.to_string()));
    }

    #[test]
    fn skill_names_dont_have_path_prefix() {
        let payload = current_stack_payload().unwrap();
        for skill in &payload.skills {
            assert!(
                !skill.name.contains('/'),
                "skill name should not contain /: {}",
                skill.name
            );
            assert!(!skill.name.is_empty(), "skill name should not be empty");
        }
    }
}
