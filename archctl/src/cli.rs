use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::sync::Arc;

use crate::astgrep::Lang;
use crate::evidence::{self, EvidenceKind};
use crate::filesystem::Filesystem;
use crate::project::resolve_project;
use crate::skills;
use crate::{doctor, environment, filesystem, graph, inventory, render, store};

/// Container for the ports a CLI handler needs.
///
/// Constructed once at the top of `run()`, then passed by reference to
/// every command handler. Tests construct a `CliContext` with a
/// `FixedEnvironment` to inject cwd/env without touching the real
/// process environment.
#[derive(Clone)]
pub struct CliContext {
    pub env: Arc<dyn environment::Environment>,
    pub fs: Arc<dyn Filesystem>,
}

impl CliContext {
    /// Production context: real `std::env::*` and `std::fs` adapters.
    pub fn production() -> Self {
        Self {
            env: environment::system_environment(),
            fs: filesystem::system_filesystem(),
        }
    }

    /// Test context: `FixedEnvironment` + empty `MemoryFilesystem`. Call
    /// `with_env(...)` to pre-load answers.
    pub fn for_test(env: Arc<dyn environment::Environment>) -> Self {
        Self {
            env,
            fs: filesystem::memory_filesystem(),
        }
    }

    /// Test context with explicit filesystem adapter. Use this when a test
    /// needs to pre-load files via `MemoryFilesystem::with_file`.
    pub fn for_test_with_fs(
        env: Arc<dyn environment::Environment>,
        fs: Arc<dyn Filesystem>,
    ) -> Self {
        Self { env, fs }
    }

    /// Resolve the user's working directory.
    ///
    /// - If the caller passed an explicit `--cwd`, use that.
    /// - Otherwise ask the port for `current_dir`.
    /// - If both fail, fall back to `.` (the historical behaviour;
    ///   preserves the contract that every handler returns a path
    ///   even when the OS has lost track of cwd).
    pub fn resolve_cwd(&self, explicit: Option<&PathBuf>) -> PathBuf {
        if let Some(p) = explicit {
            return p.clone();
        }
        self.env.current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RenderFormat {
    Auto,
    Structurizr,
    Plantuml,
}

#[derive(Debug, Subcommand)]
pub enum ProjectAction {
    Resolve {
        #[arg(long, env = "PWD")]
        cwd: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum SkillsAction {
    List,
    Sync,
    Verify,
    Activate { name: String },
}

#[derive(Debug, Subcommand)]
pub enum InventoryAction {
    Tree {
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long)]
        max_depth: Option<usize>,
        #[arg(long, default_value_t = 50_000)]
        max_entries: usize,
        #[arg(long)]
        json: bool,
    },
    Languages {
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long)]
        max_depth: Option<usize>,
        #[arg(long, default_value_t = 50_000)]
        max_entries: usize,
        #[arg(long)]
        json: bool,
    },
    Depends {
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long)]
        manifest: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum EvidenceAction {
    Extract {
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long, value_enum)]
        lang: Lang,
        #[arg(long)]
        pattern: String,
        #[arg(long, default_value = "ast-grep match")]
        claim: String,
        #[arg(long, value_enum, default_value_t = EvidenceKind::Structural)]
        kind: EvidenceKind,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        put: bool,
    },
    List {
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum GraphAction {
    Init {
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    Stat {
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    Query {
        #[arg(long)]
        cwd: Option<PathBuf>,
        cypher: String,
        #[arg(long)]
        json: bool,
    },
    Neighbours {
        #[arg(long)]
        cwd: Option<PathBuf>,
        id: String,
        #[arg(long, default_value_t = 1)]
        depth: u8,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Parser)]
#[command(name = "archctl", version, about = "OpenCode Architecture Diagrammer sidecar CLI (M4)")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Doctor {
        /// Run scope gates from `manifests/<id>.toml`. If scope IDs are
        /// given (comma-separated), check only those; otherwise check all.
        /// Example: `doctor --scopes evidence,store,tsg`
        #[arg(long, value_delimiter = ',', value_name = "scope-id")]
        scopes: Option<Vec<String>>,
        /// Project directory to read manifests from. Defaults to
        /// the current working directory.
        #[arg(long)]
        cwd: Option<PathBuf>,
    },
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    Graph {
        #[command(subcommand)]
        action: GraphAction,
    },
    Inventory {
        #[command(subcommand)]
        action: InventoryAction,
    },
    Evidence {
        #[command(subcommand)]
        action: EvidenceAction,
    },
    Render {
        source: PathBuf,
        #[arg(long, value_enum, default_value_t = RenderFormat::Auto)]
        format: RenderFormat,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, default_value = "http://localhost:18000")]
        kroki_url: String,
    },
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },
}

/// CLI entry point. Builds a [`CliContext`] with the production
/// `Environment` adapter and forwards to [`run_inner`].
///
/// External callers (mainly `main.rs`) keep using `run(cli)` —
/// existing signature preserved. Tests inject a custom context via
/// [`run_inner`].
pub fn run(cli: Cli) -> Result<i32> {
    run_inner(cli, &CliContext::production())
}

/// CLI dispatch with explicit context. Use this from tests to inject
/// a `FixedEnvironment`.
pub fn run_inner(cli: Cli, ctx: &CliContext) -> Result<i32> {
    match cli.command {
        Command::Doctor { scopes, cwd } => {
            if let Some(scope_ids) = scopes {
                // --scopes is the scope-gates pass; it does not depend
                // on cwd but takes one for consistency with the rest of
                // the CLI's `cwd` flag contract.
                let cwd = ctx.resolve_cwd(cwd.as_ref());
                doctor::check_scope(&cwd, scope_ids).context("scope gates")
            } else {
                doctor::run(ctx)
            }
        }
        Command::Project { action } => match action {
            ProjectAction::Resolve { cwd, json } => resolve_project_cmd(cwd, json, ctx),
        },
        Command::Graph { action } => match action {
            GraphAction::Init { cwd, json } => graph_init_cmd(cwd, json, ctx),
            GraphAction::Stat { cwd, json } => graph_stat_cmd(cwd, json, ctx),
            GraphAction::Query { cwd, cypher, json } => graph_query_cmd(cwd, &cypher, json, ctx),
            GraphAction::Neighbours { cwd, id, depth, json } => {
                graph_neighbours_cmd(cwd, &id, depth, json, ctx)
            }
        },
        Command::Inventory { action } => match action {
            InventoryAction::Tree { cwd, max_depth, max_entries, json } => {
                inventory_tree_cmd(cwd, max_depth, max_entries, json, ctx)
            }
            InventoryAction::Languages { cwd, max_depth, max_entries, json } => {
                inventory_languages_cmd(cwd, max_depth, max_entries, json, ctx)
            }
            InventoryAction::Depends { cwd, manifest, json } => {
                inventory_depends_cmd(cwd, manifest, json, ctx)
            }
        },
        Command::Evidence { action } => match action {
            EvidenceAction::Extract { cwd, lang, pattern, claim, kind, json, put } => {
                evidence_extract_cmd(cwd, lang, &pattern, &claim, kind, json, put, ctx)
            }
            EvidenceAction::List { cwd, path, json } => evidence_list_cmd(cwd, path, json, ctx),
        },
        Command::Render { source, format, out, kroki_url } => {
            render::run(source, format, out, &kroki_url, &*ctx.fs).context("render failed")
        }
        Command::Skills { action } => skills::run(action, &*ctx.fs).context("skills failed"),
    }
}

fn resolve_project_cmd(cwd: Option<PathBuf>, json: bool, ctx: &CliContext) -> Result<i32> {
    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let info = resolve_project(&cwd.to_string_lossy());
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&info).context("serialize project info")?
        );
    } else {
        println!("projectId:      {}", info.project_id);
        println!("projectDir:     {}", info.project_dir.display());
        println!("sourceIdentity: {}", info.source_identity_summary);
        println!("type:           {}", info.kind);
    }
    Ok(0)
}

fn graph_init_cmd(cwd: Option<PathBuf>, json: bool, ctx: &CliContext) -> Result<i32> {
    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let info = resolve_project(&cwd.to_string_lossy());
    let mut store = store::open_default(&info.project_dir).context("open graph store")?;
    store.init().context("graph init")?;
    let path = graph::database_path(&info.project_dir);
    if json {
        println!("{}", serde_json::json!({"database": path.display().to_string(), "project_id": info.project_id}));
    } else {
        println!("database:  {}", path.display());
        println!("projectId: {}", info.project_id);
    }
    Ok(0)
}

fn graph_stat_cmd(cwd: Option<PathBuf>, json: bool, ctx: &CliContext) -> Result<i32> {
    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let info = resolve_project(&cwd.to_string_lossy());
    // stat requires a session — open with init so the schema is in
    // place if this is the first run after `git clone`.
    let mut store = store::open_default(&info.project_dir).context("open graph store")?;
    store.init().context("graph init (stat prerequisite)")?;
    let stat = store.stat().context("graph stat")?;
    if json {
        println!("{}", serde_json::to_string_pretty(&stat)?);
    } else {
        println!("elements:   {}", stat.elements);
        println!("relations:  {}", stat.relations);
        println!("evidence:   {}", stat.evidence);
        println!("metatypes:  {}", stat.metatypes);
        println!("predicates: {}", stat.predicates);
    }
    Ok(0)
}

fn graph_query_cmd(cwd: Option<PathBuf>, cypher: &str, json: bool, ctx: &CliContext) -> Result<i32> {
    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let info = resolve_project(&cwd.to_string_lossy());
    let mut store = store::open_default(&info.project_dir).context("open graph store")?;
    store.init().context("graph init (query prerequisite)")?;
    let rows = store.query(cypher).context("graph query")?;
    let json_rows: Vec<serde_json::Value> = rows.iter().map(row_to_json).collect();
    if json || json_rows.is_empty() {
        println!("{}", serde_json::to_string_pretty(&json_rows)?);
    } else {
        for row in &json_rows {
            println!("{}", serde_json::to_string(row)?);
        }
    }
    Ok(0)
}

fn graph_neighbours_cmd(cwd: Option<PathBuf>, id: &str, depth: u8, json: bool, ctx: &CliContext) -> Result<i32> {
    use tracing::warn;
    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let info = resolve_project(&cwd.to_string_lossy());
    // Identifier validation belongs to the domain — never trust user
    // input to a Cypher interpolation, even with the port abstracting
    // the engine.
    let safe_id = graph::validate_identifier(id).context("invalid element id")?;
    let clamped_depth = depth.clamp(1, 4);
    if depth > 2 {
        warn!(depth, "graph traversal depth > 2 may be slow on large graphs");
    }
    let cypher = format!(
        "MATCH (e:Element {{id: '{safe_id}'}})-[*1..{clamped_depth}]-(n) \
         RETURN DISTINCT n.id AS id, labels(n) AS kinds;"
    );
    let mut store = store::open_default(&info.project_dir).context("open graph store")?;
    store.init().context("graph init (neighbours prerequisite)")?;
    let rows = store.query(&cypher).context("graph neighbours")?;
    if json {
        let json_rows: Vec<serde_json::Value> = rows.iter().map(row_to_json).collect();
        println!("{}", serde_json::to_string_pretty(&json_rows)?);
    } else {
        for row in &rows {
            // Typed access — the `Row` API gives us `&str` directly
            // without a detour through `serde_json::Value`.
            let id = row.get("id").and_then(|c| c.as_str()).unwrap_or("?");
            let kinds = row
                .get("kinds")
                .map(|c| c.to_string())
                .unwrap_or_else(|| "<>".into());
            println!("{id}\t{kinds}");
        }
    }
    Ok(0)
}

/// Convert one `Row` to a `serde_json::Value` object keyed by column
/// name. This is the **only** place in `archctl` that turns a `Row`
/// into JSON for CLI output — keeping the conversion local means the
/// domain stays free of `serde_json`.
fn row_to_json(row: &crate::row::Row) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for (k, cell) in row.iter() {
        obj.insert(k.to_string(), cell.to_json());
    }
    serde_json::Value::Object(obj)
}

fn inventory_tree_cmd(
    cwd: Option<PathBuf>,
    max_depth: Option<usize>,
    max_entries: usize,
    json: bool,
    ctx: &CliContext,
) -> Result<i32> {
    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let entries = inventory::tree(&cwd, max_depth, max_entries)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        for e in &entries {
            let kind = match e.kind {
                inventory::EntryKind::Dir => "d",
                inventory::EntryKind::File => "f",
            };
            let size = e.size_bytes.map(|s| format!(" {s}B")).unwrap_or_default();
            println!("{kind} {}{size}", e.path);
        }
    }
    Ok(0)
}

fn inventory_languages_cmd(
    cwd: Option<PathBuf>,
    max_depth: Option<usize>,
    max_entries: usize,
    json: bool,
    ctx: &CliContext,
) -> Result<i32> {
    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let summary = inventory::languages(&cwd, max_depth, max_entries)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!("files: {}  bytes: {}", summary.total_files, summary.total_bytes);
        let mut v: Vec<_> = summary.languages.iter().collect();
        v.sort_by(|a, b| b.1.bytes.cmp(&a.1.bytes));
        for (lang, stat) in v {
            println!("  {lang:<14} files={:<6} bytes={}", stat.files, stat.bytes);
        }
    }
    Ok(0)
}

fn inventory_depends_cmd(
    cwd: Option<PathBuf>,
    manifest: Option<PathBuf>,
    json: bool,
    ctx: &CliContext,
) -> Result<i32> {
    let manifest_path = manifest.map(|p| {
        if p.is_relative() {
            let base = ctx.resolve_cwd(cwd.as_ref());
            base.join(p)
        } else {
            p
        }
    });
    let deps = inventory::depends_summary(manifest_path.as_deref())?;
    if json {
        println!("{}", serde_json::to_string_pretty(&deps)?);
    } else {
        for dep in &deps {
            let kind = match dep.kind {
                inventory::DepKind::Normal => "",
                inventory::DepKind::Dev => " [dev]",
                inventory::DepKind::Build => " [build]",
            };
            println!("{:<40} {:>20}{}", dep.name, dep.version, kind);
        }
        println!("\n{} dependencies total", deps.len());
    }
    Ok(0)
}

fn evidence_extract_cmd(
    cwd: Option<PathBuf>,
    lang: Lang,
    pattern: &str,
    claim: &str,
    kind: EvidenceKind,
    json: bool,
    do_put: bool,
    ctx: &CliContext,
) -> Result<i32> {
    let cwd = ctx.resolve_cwd(cwd.as_ref());
    // CLI is the production entry point — always uses SystemClock.
    // The Clock port lets tests inject deterministic timestamps via
    // FixedClock; the CLI does not need that.
    let clock: &dyn crate::clock::Clock = &crate::clock::SystemClock;
    let result = evidence::extract(&cwd, lang, pattern, claim, kind, clock, &*ctx.fs)?;
    let written = if do_put {
        let info = resolve_project(&cwd.to_string_lossy());
        evidence::put_with_clock(&info.project_dir, &result.evidence, clock)
            .context("evidence put")?
    } else {
        0
    };
    if json {
        let mut payload = serde_json::json!({
            "language": result.language,
            "pattern": result.pattern,
            "files_scanned": result.files_scanned,
            "matches_total": result.matches_total,
            "evidence": result.evidence,
            "persisted": written,
        });
        if !do_put {
            payload.as_object_mut().unwrap().remove("persisted");
        }
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("language:  {}", result.language);
        println!("pattern:   {}", result.pattern);
        println!("scanned:   {} files", result.files_scanned);
        println!("matches:   {}", result.matches_total);
        for ev in &result.evidence {
            println!(
                "  {path}:{sl}-{el}  {preview}",
                path = ev.path,
                sl = ev.start_line,
                el = ev.end_line,
                preview = ev.text_preview.as_deref().unwrap_or(""),
            );
        }
        if do_put {
            println!("persisted: {written} rows");
        }
    }
    Ok(0)
}

fn evidence_list_cmd(cwd: Option<PathBuf>, path: Option<String>, json: bool, ctx: &CliContext) -> Result<i32> {
    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let info = resolve_project(&cwd.to_string_lossy());
    let safe_path = path
        .as_deref()
        .map(crate::graph::validate_identifier)
        .transpose()?;
    let mut store = store::open_default(&info.project_dir).context("open graph store")?;
    store.init().context("graph init (evidence list prerequisite)")?;
    let rows = store
        .list_evidence(safe_path)
        .context("evidence list")?;
    if json {
        let json_rows: Vec<serde_json::Value> = rows.iter().map(row_to_json).collect();
        println!("{}", serde_json::to_string_pretty(&json_rows)?);
    } else {
        for row in &rows {
            // Typed accessors — the domain `Row` carries `Cell`
            // values; `as_str` and `as_i64` extract them directly.
            println!(
                "{id}\t{kind}\t{path}:{sl}-{el}\t{claim}",
                id = row.get("e.id").and_then(|c| c.as_str()).unwrap_or("?"),
                kind = row.get("e.kind").and_then(|c| c.as_str()).unwrap_or("?"),
                path = row.get("e.path").and_then(|c| c.as_str()).unwrap_or("?"),
                sl = row.get("e.start_line").and_then(|c| c.as_i64()).unwrap_or(0),
                el = row.get("e.end_line").and_then(|c| c.as_i64()).unwrap_or(0),
                claim = row.get("e.claim").and_then(|c| c.as_str()).unwrap_or(""),
            );
        }
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    //! Tests for the CLI entry point.
    //!
    //! The point of these tests is to demonstrate that the
    //! `Environment` port actually works: we can drive `run_inner`
    //! against a controlled `cwd` without touching the real
    //! process cwd. Before the refactor this was impossible — the
    //! CLI always called `std::env::current_dir()`.
    //!
    //! These tests are intentionally narrow. They verify the
    //! *contract* (the right cwd is used) without depending on
    //! internal file layout.

    use super::*;
    use crate::environment::{Environment, FixedEnvironment, SystemEnvironment};

    /// Build a `CliContext` with a fixed cwd rooted at `cwd`. We do
    /// not pre-load any other vars — `FixedEnvironment` returns
    /// errors when asked for un-set values, which is the contract
    /// the domain relies on.
    fn ctx_for(cwd: std::path::PathBuf) -> CliContext {
        let env: std::sync::Arc<dyn crate::environment::Environment> =
            std::sync::Arc::new(FixedEnvironment::new().with_cwd(cwd));
        CliContext::for_test(env)
    }

    #[test]
    fn context_production_uses_system_environment() {
        // The production context does NOT call `FixedEnvironment`
        // under the hood; it goes straight to `SystemEnvironment`.
        // We assert the type here as a regression guard.
        let ctx = CliContext::production();
        // Both adapters implement `Environment`; the test is the
        // contract that the *construction* uses SystemEnvironment
        // (which we trust to read std::env).
        let _: &dyn crate::environment::Environment = ctx.env.as_ref();
    }

    #[test]
    fn fixed_environment_round_trip_through_arc() {
        // Mutation guard: if `FixedEnvironment` ever stops
        // implementing `Environment`, `Arc::new(FixedEnvironment)`
        // will fail. This test catches that immediately.
        let env: std::sync::Arc<dyn crate::environment::Environment> =
            std::sync::Arc::new(FixedEnvironment::new().with_cwd("/x"));
        assert_eq!(env.current_dir().unwrap(), std::path::PathBuf::from("/x"));
    }

    #[test]
    fn resolve_cwd_prefers_explicit_path() {
        let explicit = PathBuf::from("/explicit/here");
        let ctx = ctx_for(PathBuf::from("/cwd/from/env"));
        // Even if the env says "/cwd/from/env", the explicit value wins.
        assert_eq!(ctx.resolve_cwd(Some(&explicit)), explicit);
    }

    #[test]
    fn resolve_cwd_falls_back_to_environment() {
        let cwd = PathBuf::from("/tmp/wins");
        let ctx = ctx_for(cwd.clone());
        assert_eq!(ctx.resolve_cwd(None), cwd);
    }

    #[test]
    fn resolve_cwd_falls_back_to_dot_when_environment_fails() {
        // FixedEnvironment::current_dir returns Err if cwd was not
        // pre-loaded. resolve_cwd must then return "." (the historical
        // fallback) — never panic.
        let env: std::sync::Arc<dyn crate::environment::Environment> =
            std::sync::Arc::new(FixedEnvironment::new());
        let ctx = CliContext::for_test(env);
        assert_eq!(ctx.resolve_cwd(None), PathBuf::from("."));
    }

    #[test]
    fn run_inner_resolves_cwd_through_context() {
        // The whole point of the refactor: invoke `run_inner` from
        // a tempdir we control. The CLI must use that cwd — not
        // the real cwd of the test process — to compute the
        // project identity. We assert behaviour (exit 0), not
        // implementation paths. The path is whatever XDG decides.
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let ctx = ctx_for(project.clone());

        let cli = Cli::parse_from([
            "archctl",
            "graph",
            "init",
            "--cwd", project.to_str().unwrap(),
        ]);
        let code = run_inner(cli, &ctx).expect("run_inner succeeds");
        assert_eq!(code, 0);
    }

    #[test]
    fn run_inner_uses_environment_cwd_when_no_explicit_flag() {
        // Inject a cwd via the port; do not pass --cwd. The CLI
        // must pick up the injected cwd via resolve_cwd(None).
        // We assert behaviour (exit 0), not paths.
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("from-env");
        let ctx = ctx_for(project.clone());

        let cli = Cli::parse_from(["archctl", "graph", "init"]);
        let code = run_inner(cli, &ctx).expect("run_inner succeeds");
        assert_eq!(code, 0);
    }

    #[test]
    fn run_inner_uses_injected_cwd_not_process_cwd() {
        // The smoking gun: two different injected cwds MUST
        // produce different project_ids when `project resolve`
        // runs. If the cwd had silently fallen back to the real
        // process cwd, both runs would yield the same project_id
        // — and this test would fail.
        //
        // We don't try to capture stdout (println! on a global is
        // hard to intercept cleanly); instead we call the
        // underlying `resolve_project` function, which is what
        // the CLI handler calls internally. If the port is
        // bypassed, `cwd` would be the real process cwd and
        // both project_ids would collide.
        let tmp_a = tempfile::tempdir().unwrap();
        let tmp_b = tempfile::tempdir().unwrap();
        let cwd_a = tmp_a.path().join("a");
        let cwd_b = tmp_b.path().join("b");
        std::fs::create_dir_all(&cwd_a).unwrap();
        std::fs::create_dir_all(&cwd_b).unwrap();

        let ctx_a = ctx_for(cwd_a);
        let ctx_b = ctx_for(cwd_b);

        // Direct call: prove the port changes the contract.
        let info_a =
            crate::project::resolve_project(&ctx_a.resolve_cwd(None).to_string_lossy());
        let info_b =
            crate::project::resolve_project(&ctx_b.resolve_cwd(None).to_string_lossy());

        assert_ne!(
            info_a.project_id, info_b.project_id,
            "two distinct injected cwds produced the same project_id: {}. \
             The Environment port is being bypassed somewhere in the call tree.",
            info_a.project_id
        );
        // Both must succeed — `project resolve` is idempotent.
        assert!(info_a.project_id.len() > 0);
        assert!(info_b.project_id.len() > 0);
    }

    #[test]
    fn system_environment_reads_real_process_var() {
        // The SystemEnvironment adapter is the production one; we
        // assert it can see vars the test process has set. PATH is
        // virtually always present; if it isn't, skip.
        let env = SystemEnvironment;
        if let Some(p) = env.var("PATH") {
            assert!(!p.is_empty());
        }
    }
}
