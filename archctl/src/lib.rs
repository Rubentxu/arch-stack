pub mod astgrep;
pub mod cli;
pub mod clock;
pub mod code;
pub mod cognitive;
pub mod diagram;
pub mod doctor;
pub mod environment;
pub mod evaluation;
pub mod evidence;
pub mod filesystem;
pub mod graph;
pub mod ide;
pub mod identity;
pub mod inventory;
pub mod lifecycle;
pub mod migrations;
pub mod plugin;
pub mod project;
pub mod render;
pub mod row;
pub mod scope; // M73: CLI lifecycle (asdf-inspired versioned installs).
pub mod self_lifecycle {
    // Re-export under the user-facing `archctl self` command name while
    // keeping the Rust module `lifecycle` (since `self` is a reserved keyword).
    pub use crate::lifecycle::*;
}
pub mod skills;
pub mod source;
pub mod store;
pub mod telemetry;
/// Test-only helpers shared across integration test files.
///
/// M56: extracted `backend_available()` (skip-on-missing-backend) from
/// 5 e2e tests into `test_helpers::plantuml::backend_available`. Kept in
/// the main lib (not gated behind `#[cfg(test)]`) because integration
/// tests in `tests/` are separate crates and need to import via
/// `archctl::test_helpers`.
pub mod test_helpers;
pub mod view;
pub mod xdg;

pub use cli::{
    Cli, Command, DiagramAction, EvidenceAction, GraphAction, InventoryAction, ProjectAction,
    RenderFormat, SkillsAction, run,
};
pub use clock::{Clock, FixedClock, SystemClock, fixed_clock, system_clock};
pub use doctor::{
    DoctorScope, StorageFinding, StorageReport, StorageSeverity, StorageStatus,
    manifest::validate_manifests,
    render_json, render_text, run_scope, run_storage_probe,
    runner::{SmokeResult, run_all_smoke_gates, run_smoke_gate},
    storage::{LbugStorageProbe, StorageProbe},
};
pub use environment::{
    Environment, FixedEnvironment, SystemEnvironment, fixed_environment, system_environment,
};
pub use evaluation::Evaluation;
pub use filesystem::{
    DirEntry, EntryKind, Filesystem, MemoryFilesystem, SystemFilesystem, memory_filesystem,
    system_filesystem,
};
pub use graph::{
    GraphStat, database_path, init as graph_init, neighbours, open_session, query as graph_query,
    stat as graph_stat, validate_identifier,
};
pub use identity::{
    SourceIdentity, blake_like, identity_summary, normalize_remote, portable_project_id,
    resolve_source_identity,
};
pub use project::{ProjectInfo, resolve_project};
pub use row::{Cell, Row};
pub use scope::{
    ScopeCheckReport, ScopeFinding, ScopeGate, ScopeManifest, ScopeSeverity, check_all_scopes,
    check_scope,
};
pub use skills::{
    SkillLockEntry, SkillMode, SkillsLock, SyncReport, VerifyReport, activate_skill, load_lock,
    sync_skill, sync_skills, verify_skills,
};
pub use source::SourceArtifact;
pub use xdg::{XdgLayout, ensure_xdg, resolve_xdg, user_home};
