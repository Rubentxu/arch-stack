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
}

pub fn user_home() -> PathBuf {
    if let Some(p) = std::env::var_os("HOME") {
        return PathBuf::from(p);
    }
    if let Some(p) = std::env::var_os("USERPROFILE") {
        return PathBuf::from(p);
    }
    if let (Some(drive), Some(path)) = (std::env::var_os("HOMEDRIVE"), std::env::var_os("HOMEPATH")) {
        let mut s = std::ffi::OsString::from(drive);
        s.push(path);
        return PathBuf::from(s);
    }
    PathBuf::from("/tmp")
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

pub fn ensure_xdg(layout: &XdgLayout) -> std::io::Result<()> {
    for dir in [
        layout.data.clone(),
        layout.state.clone(),
        layout.cache.clone(),
        layout.config.clone(),
        layout.skills_root(),
        layout.lib_root(),
        layout.projects_root(),
        layout.sources_root(),
    ] {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(())
}
