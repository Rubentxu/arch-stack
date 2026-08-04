use crate::Filesystem;
use std::path::PathBuf;

pub struct XdgLayout {
    pub data: PathBuf,
    pub config: PathBuf,
    pub state: PathBuf,
    pub cache: PathBuf,
}

impl XdgLayout {
    pub fn skills_root(&self) -> PathBuf {
        self.data.join("skills")
    }
    pub fn projects_root(&self) -> PathBuf {
        self.data.join("projects")
    }
    pub fn lib_root(&self) -> PathBuf {
        self.data.join("lib")
    }
    pub fn sources_root(&self) -> PathBuf {
        self.skills_root().join("sources")
    }
    /// Path to the policies directory (TOML rule files).
    /// Falls back to `$XDG_CONFIG_HOME/archctl/policies/`.
    pub fn policies_root(&self) -> PathBuf {
        self.config.join("policies")
    }
}

/// Resolve the user's home directory via the `Environment` port, with
/// `/tmp` as a defensive fallback when the OS reports no home at all
/// (very rare; can happen in minimal containers).
///
/// This is the production call: uses [`crate::environment::SystemEnvironment`]
/// under the hood. Callers that need a different home — e.g. tests
/// — should use [`crate::environment::FixedEnvironment::with_home`]
/// and the [`crate::cli::run_inner`] entry point, not this function.
pub fn user_home() -> PathBuf {
    use crate::environment::Environment;
    crate::environment::SystemEnvironment
        .home_dir()
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

pub fn resolve_xdg() -> XdgLayout {
    let env: std::collections::HashMap<String, String> = std::env::vars().collect();
    resolve_xdg_from_env(&env)
}

pub fn resolve_xdg_from_env(env: &std::collections::HashMap<String, String>) -> XdgLayout {
    let home = env
        .get("HOME")
        .or_else(|| env.get("USERPROFILE"))
        .cloned()
        .map(PathBuf::from)
        .unwrap_or_else(user_home);
    let home_str = home.to_string_lossy().to_string();

    let data = env
        .get("XDG_DATA_HOME")
        .cloned()
        .unwrap_or_else(|| xdg_default("data", &home_str));
    let config = env
        .get("XDG_CONFIG_HOME")
        .cloned()
        .unwrap_or_else(|| xdg_default("config", &home_str));
    let state = env
        .get("XDG_STATE_HOME")
        .cloned()
        .unwrap_or_else(|| xdg_default("state", &home_str));
    let cache = env
        .get("XDG_CACHE_HOME")
        .cloned()
        .unwrap_or_else(|| xdg_default("cache", &home_str));

    XdgLayout {
        data: append_archctl(data),
        config: append_archctl(config),
        state: append_archctl(state),
        cache: append_archctl(cache),
    }
}

fn append_archctl(path: String) -> PathBuf {
    let p = PathBuf::from(path);
    if p.file_name().and_then(|n| n.to_str()) == Some("archctl") {
        p
    } else {
        p.join("archctl")
    }
}

fn xdg_default(kind: &str, home: &str) -> String {
    let sep = std::path::MAIN_SEPARATOR;
    match kind {
        "data" => format!("{home}{sep}.local{sep}share"),
        "config" => format!("{home}{sep}.config"),
        "state" => format!("{home}{sep}.local{sep}state"),
        "cache" => format!("{home}{sep}.cache"),
        _ => home.to_string(),
    }
}

pub fn ensure_xdg(layout: &XdgLayout, fs: &dyn Filesystem) -> anyhow::Result<()> {
    for dir in [
        layout.data.clone(),
        layout.state.clone(),
        layout.cache.clone(),
        layout.config.clone(),
        layout.skills_root(),
        layout.lib_root(),
        layout.projects_root(),
        layout.sources_root(),
        layout.policies_root(),
    ] {
        fs.create_dir_all(&dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn policies_root_joins_config() {
        let layout = XdgLayout {
            data: PathBuf::from("/home/user/.local/share/archctl"),
            config: PathBuf::from("/home/user/.config/archctl"),
            state: PathBuf::from("/home/user/.local/state/archctl"),
            cache: PathBuf::from("/home/user/.cache/archctl"),
        };
        assert_eq!(
            layout.policies_root(),
            PathBuf::from("/home/user/.config/archctl/policies")
        );
    }

    #[test]
    fn policies_root_default_under_archctl() {
        let layout = XdgLayout {
            data: PathBuf::from("/home/user/.local/share/archctl"),
            config: PathBuf::from("/home/user/.config/archctl"),
            state: PathBuf::from("/home/user/.local/state/archctl"),
            cache: PathBuf::from("/home/user/.cache/archctl"),
        };
        let policies = layout.policies_root();
        assert!(policies.to_str().unwrap().ends_with("archctl/policies"));
    }
}
