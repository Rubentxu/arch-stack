use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::cli::SkillsAction;
use crate::environment::Environment;
use crate::xdg::{ensure_xdg, resolve_xdg};
use crate::Filesystem;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillMode {
    Direct,
    Wrapped,
    Patched,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillLockEntry {
    pub source: String,
    pub commit: String,
    pub mode: SkillMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrapper: Option<String>,
    pub license: String,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsLock {
    #[serde(rename = "schema-version")]
    pub schema_version: u32,
    pub skills: HashMap<String, SkillLockEntry>,
}

pub fn load_lock(path: &Path, fs: &dyn Filesystem) -> Result<SkillsLock> {
    let text = fs.read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_yaml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

pub fn sync_skill(name: &str, entry: &SkillLockEntry, dest: &Path, fs: &dyn Filesystem) -> Result<()> {
    fs.create_dir_all(dest).with_context(|| format!("mkdir {}", dest.display()))?;
    info!(skill = name, source = %entry.source, dest = %dest.display(), "cloning");
    let status = std::process::Command::new("git")
        .args(["clone", "--depth", "1", &entry.source, "."])
        .current_dir(dest)
        .status()
        .with_context(|| format!("git clone {} into {}", entry.source, dest.display()))?;
    if !status.success() {
        anyhow::bail!("git clone {} exited non-zero", entry.source);
    }
    if entry.commit != "<pin at first sync>" && !entry.commit.is_empty() {
        let status = std::process::Command::new("git")
            .current_dir(dest)
            .args(["checkout", &entry.commit])
            .status()
            .with_context(|| format!("git checkout {}", entry.commit))?;
        if !status.success() {
            anyhow::bail!("git checkout {} failed", entry.commit);
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct SyncReport {
    pub synced: Vec<String>,
    pub skipped: Vec<String>,
    pub failures: Vec<(String, String)>,
}

pub fn sync_skills(lock: &SkillsLock, into: &Path, fs: &dyn Filesystem) -> SyncReport {
    let mut report = SyncReport { synced: Vec::new(), skipped: Vec::new(), failures: Vec::new() };
    fs.create_dir_all(into).ok();
    for (name, entry) in &lock.skills {
        let dest = into.join(name.replace(['/', '\\'], "_"));
        if fs.exists(&dest.join("SKILL.md")) {
            debug!(skill = %name, "already synced, skipping");
            report.skipped.push(name.clone());
            continue;
        }
        match sync_skill(name, entry, &dest, fs) {
            Ok(()) => report.synced.push(name.clone()),
            Err(err) => {
                warn!(skill = %name, error = %err, "sync failed");
                report.failures.push((name.clone(), err.to_string()));
            }
        }
    }
    report
}

#[derive(Debug, Clone)]
pub struct VerifyReport {
    pub ok: bool,
    pub present: Vec<String>,
    pub missing: Vec<String>,
}

pub fn verify_skills(lock: &SkillsLock, source_root: &Path, fs: &dyn Filesystem) -> VerifyReport {
    let mut present = Vec::new();
    let mut missing = Vec::new();
    for name in lock.skills.keys() {
        let path = source_root
            .join(name.replace(['/', '\\'], "_"))
            .join("SKILL.md");
        if fs.exists(&path) {
            present.push(name.clone());
        } else {
            missing.push(name.clone());
        }
    }
    VerifyReport { ok: missing.is_empty(), present, missing }
}

pub fn activate_skill(name: &str, source: &Path, profile_skills_dir: &Path, fs: &dyn Filesystem) -> Result<PathBuf> {
    fs.create_dir_all(profile_skills_dir)
        .with_context(|| format!("mkdir {}", profile_skills_dir.display()))?;
    let target = profile_skills_dir.join(name);
    let _ = fs.remove_file(&target);
    create_symlink(source, &target)
        .with_context(|| format!("symlink {} -> {}", source.display(), target.display()))?;
    Ok(target)
}

#[cfg(unix)]
fn create_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn create_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(src, dst)
}

pub fn run(action: SkillsAction, fs: &dyn Filesystem) -> Result<i32> {
    let layout = resolve_xdg();
    let lock_path = layout.config.join("skills.lock.yaml");
    debug!(lock = %lock_path.display(), "loading skills lockfile");
    let lock = match load_lock(&lock_path, fs) {
        Ok(l) => l,
        Err(err) if matches!(action, SkillsAction::List) => {
            anyhow::bail!("could not load {}: {err}", lock_path.display());
        }
        Err(_) => SkillsLock { schema_version: 1, skills: HashMap::new() },
    };

    match action {
        SkillsAction::List => {
            println!("{}", serde_json::to_string_pretty(&lock)?);
            Ok(0)
        }
        SkillsAction::Verify => {
            let r = verify_skills(&lock, &layout.sources_root(), fs);
            for name in &r.present {
                println!("[OK]   {name}");
            }
            for name in &r.missing {
                println!("[MISS] {name}");
            }
            Ok(if r.ok { 0 } else { 1 })
        }
        SkillsAction::Sync => {
            ensure_xdg(&layout, fs)?;
            let r = sync_skills(&lock, &layout.sources_root(), fs);
            for name in &r.synced {
                println!("[SYNC] {name}");
            }
            for name in &r.skipped {
                println!("[SKIP] {name}");
            }
            for (name, err) in &r.failures {
                eprintln!("[FAIL] {name}: {err}");
            }
            Ok(if r.failures.is_empty() { 0 } else { 1 })
        }
        SkillsAction::Activate { name } => {
            if !lock.skills.contains_key(&name) {
                anyhow::bail!("unknown skill in lockfile: {name}");
            }
            let src = layout
                .sources_root()
                .join(name.replace(['/', '\\'], "_"));
            if !fs.exists(&src.join("SKILL.md")) {
                anyhow::bail!("source not synced: {}", src.display());
            }
            let profile_dir = crate::environment::SystemEnvironment
                .var("OPENCODE_CONFIG_DIR")
                .context("OPENCODE_CONFIG_DIR not set")?;
            let target_dir = PathBuf::from(profile_dir).join("skills/upstream");
            let target = activate_skill(&name, &src, &target_dir, fs)?;
            println!("[OK] activated {} -> {}", name, target.display());
            Ok(0)
        }
    }
}
