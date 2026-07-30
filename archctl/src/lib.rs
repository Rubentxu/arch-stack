pub mod astgrep;
pub mod cli;
pub mod clock;
pub mod doctor;
pub mod environment;
pub mod evidence;
pub mod graph;
pub mod identity;
pub mod inventory;
pub mod project;
pub mod render;
pub mod row;
pub mod skills;
pub mod store;
pub mod telemetry;
pub mod tsg;
pub mod xdg;

pub use cli::{run, Cli, Command, EvidenceAction, GraphAction, InventoryAction, ProjectAction, RenderFormat, SkillsAction};
pub use clock::{fixed_clock, system_clock, Clock, FixedClock, SystemClock};
pub use environment::{fixed_environment, system_environment, Environment, FixedEnvironment, SystemEnvironment};
pub use graph::{database_path, init as graph_init, neighbours, open_session, query as graph_query, stat as graph_stat, validate_identifier, GraphStat};
pub use row::{Cell, Row};
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
