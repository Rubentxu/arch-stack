pub mod cli;
pub mod doctor;
pub mod identity;
pub mod project;
pub mod render;
pub mod skills;
pub mod telemetry;
pub mod xdg;

pub use cli::{run, Cli, Command, ProjectAction, RenderFormat, SkillsAction};
pub use identity::{
    blake_like, identity_summary, normalize_remote, portable_project_id, resolve_source_identity,
    SourceIdentity,
};
pub use project::{resolve_project, ProjectInfo};
pub use skills::{
    activate_skill, load_lock, sync_skill, sync_skills, verify_skills, SkillLockEntry, SkillMode,
    SkillsLock, SyncReport, VerifyReport,
};
pub use xdg::{ensure_xdg, resolve_xdg, user_home, XdgLayout};
