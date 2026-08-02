use crate::filesystem::system_filesystem;
use crate::identity::{identity_summary, portable_project_id, resolve_source_identity};
use crate::xdg::resolve_xdg;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectInfo {
    pub project_id: String,
    pub project_dir: PathBuf,
    pub source_identity: String,
    pub source_identity_summary: String,
    #[serde(rename = "type")]
    pub kind: String,
}

pub fn resolve_project(cwd: &str) -> ProjectInfo {
    let identity = resolve_source_identity(cwd, &*system_filesystem())
        .expect("resolve_source_identity failed");
    let project_id = portable_project_id(&identity);
    let layout = resolve_xdg();
    let project_dir = layout.projects_root().join(&project_id);
    let kind = match &identity {
        crate::identity::SourceIdentity::Git { .. } => "git",
        crate::identity::SourceIdentity::Directory { .. } => "directory",
    };
    ProjectInfo {
        project_id,
        project_dir,
        source_identity: serde_json::to_string(&identity)
            .unwrap_or_else(|_| "<unserializable>".to_string()),
        source_identity_summary: identity_summary(&identity),
        kind: kind.to_string(),
    }
}
