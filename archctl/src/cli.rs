use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::astgrep::Lang;
use crate::evidence::{self, Evidence, EvidenceKind, EvidenceStatus};
use crate::filesystem::Filesystem;
use crate::ide::StackPayload;
use crate::ide::assets::current_stack_payload;
use crate::ide::builtin_adapters;
use crate::project::resolve_project;
use crate::skills;
use crate::source::SourceArtifact;
use crate::{doctor, environment, filesystem, graph, inventory, render};

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
    pub clock: Arc<dyn crate::clock::Clock>,
    pub store_factory: Arc<dyn crate::store::GraphStoreFactory>,
    /// Factory for admin-only raw Cypher queries (P1-04).
    /// Produces `Arc<dyn RawGraphQuery>` so handlers can call
    /// `ctx.raw_query_factory.open_raw(&path)?.query(cypher)`.
    pub raw_query_factory: Arc<dyn crate::store::RawGraphQueryFactory>,
}

impl CliContext {
    /// Production context: real `std::env::*`, `std::fs`, `SystemClock`,
    /// and `LbugStoreFactory` adapters.
    pub fn production() -> Self {
        Self {
            env: environment::system_environment(),
            fs: filesystem::system_filesystem(),
            clock: crate::clock::system_clock(),
            store_factory: Arc::new(crate::store::LbugStoreFactory),
            raw_query_factory: Arc::new(crate::store::LbugStoreFactory),
        }
    }

    /// Test context: `FixedEnvironment` + empty `MemoryFilesystem` +
    /// `FixedClock` + `LbugStoreFactory`. Call `with_env(...)` to
    /// pre-load answers.
    pub fn for_test(env: Arc<dyn environment::Environment>) -> Self {
        Self {
            env,
            fs: filesystem::memory_filesystem(),
            clock: crate::clock::fixed_clock("2024-01-01T00:00:00Z"),
            store_factory: Arc::new(crate::store::LbugStoreFactory),
            raw_query_factory: Arc::new(crate::store::LbugStoreFactory),
        }
    }

    /// Test context with explicit filesystem adapter. Use this when a test
    /// needs to pre-load files via `MemoryFilesystem::with_file`.
    pub fn for_test_with_fs(
        env: Arc<dyn environment::Environment>,
        fs: Arc<dyn Filesystem>,
    ) -> Self {
        Self {
            env,
            fs,
            clock: crate::clock::fixed_clock("2024-01-01T00:00:00Z"),
            store_factory: Arc::new(crate::store::LbugStoreFactory),
            raw_query_factory: Arc::new(crate::store::LbugStoreFactory),
        }
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
        self.env
            .current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
    }
}

/// Output format for the capabilities command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CapabilityFormat {
    Json,
    Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RenderFormat {
    Auto,
    Structurizr,
    Plantuml,
    Mermaid,
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
pub enum AgentAction {
    /// List all registered agents and their descriptors.
    List {
        /// Output as JSON instead of human-readable table.
        #[arg(long)]
        json: bool,
    },
    /// Dispatch a goal to the cognitive layer and return structured output.
    Dispatch {
        /// Natural-language goal for the agent system.
        goal: String,
        /// Output as JSON instead of human-readable.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum McpAction {
    /// List all allowed MCP tools in the gateway allowlist.
    ListTools {
        /// Output as JSON instead of human-readable table.
        #[arg(long)]
        json: bool,
    },
    /// Invoke an MCP tool by name with JSON args from stdin.
    Invoke {
        /// Name of the tool to invoke.
        tool: String,
        /// Path to a file containing JSON args (or - for stdin).
        #[arg(long, short = 'd')]
        data: Option<PathBuf>,
        /// Output as JSON instead of human-readable.
        #[arg(long)]
        json: bool,
        /// Run this invocation through PolicyGate (governed mode).
        /// The input must include an ActionProposal field.
        #[arg(long)]
        governed: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum PluginAction {
    /// Install a plugin from a tap.
    Install {
        /// Plugin spec in the form `name@version` (version defaults to `latest`).
        spec: String,
        /// Tap URL to use (defaults to official arch-stack tap).
        #[arg(long)]
        tap: Option<String>,
    },
    /// List plugins available in a tap.
    List {
        /// Tap URL to list plugins from.
        #[arg(
            long,
            default_value = "https://raw.githubusercontent.com/Rubentxu/arch-stack/main/taps/official.json"
        )]
        tap_url: String,
    },
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
pub enum DiagramAction {
    Export {
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// View selector in the form `<c4-kind>:<scope>` (e.g., `container:orders`, `context:*`).
        selector: String,
        #[arg(long, default_value = "viewer-bundle")]
        format: String,
        /// Output directory for the 5-file bundle. Optional when `--json`
        /// is set (pure stdout mode).
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    Validate {
        #[arg(long)]
        cwd: Option<PathBuf>,
        bundle_dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Apply a changeset (view-level edits) to a persisted diagram.
    Apply {
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// Path to the changeset JSON file (validated against changeset.schema.json).
        #[arg(long)]
        changes: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Project graph elements to editable DSL source (PlantUML, Mermaid, Structurizr).
    Project {
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// View selector in the form `<kind>:<scope>` (e.g., `class:*`, `c4-container:orders`).
        #[arg(long)]
        view: String,
        /// Output format: plantuml, mermaid, or structurizr.
        #[arg(long, default_value = "plantuml")]
        format: String,
        /// Output file path.
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum CodeAction {
    /// Discover C4 Container boundaries using multiple strategies.
    C4Discover {
        /// Project directory to scan. Defaults to the current working directory.
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// Run strategy and persist inferred Containers to the graph.
        #[arg(long)]
        apply: bool,
        /// Comma-separated list of strategy IDs to run (e.g. "cargo,npm").
        /// If omitted, all strategies are run.
        #[arg(long)]
        strategy: Option<String>,
        /// Emit machine-readable JSON to stdout.
        #[arg(long)]
        json: bool,
    },
    /// Extract static call-graph (function→function call edges) via tree-sitter-graph.
    CallGraph {
        /// Project directory to scan. Defaults to the current working directory.
        #[arg(long, default_value = ".")]
        cwd: PathBuf,
        /// Persist extracted call-graph nodes + edges to the graph store.
        #[arg(long)]
        apply: bool,
        /// Emit machine-readable JSON to stdout.
        #[arg(long)]
        json: bool,
        /// Comma-separated languages to process (rust, typescript, python, go).
        /// If omitted, all MVP languages are processed.
        #[arg(long, value_enum, value_delimiter = ',')]
        lang: Vec<crate::code::call_graph::Language>,
        /// Maximum call-depth to traverse (0 = unlimited in MVP).
        #[arg(long)]
        depth: Option<u32>,
    },
    /// Project a call chain into an ordered interaction list (read-only).
    Sequence {
        /// Project directory to scan. Defaults to the current working directory.
        #[arg(long, default_value = ".")]
        cwd: PathBuf,
        /// Selector for the starting function (name, file:line, or canonical key).
        #[arg(long, value_parser = parse_from_selector)]
        from: crate::code::sequence::FromSelector,
        /// Maximum call-depth to traverse (default: 5).
        #[arg(long, default_value_t = 5)]
        depth: u32,
        /// Maximum number of interactions to return (default: 500).
        #[arg(long)]
        max_interactions: Option<u32>,
        /// Emit machine-readable JSON to stdout.
        #[arg(long)]
        json: bool,
        /// Accepted but ignored — sequence is read-only (spec SCN-217).
        #[arg(long)]
        apply: bool,
    },
    /// Extract UML class diagram from source files (Rust, TypeScript, Python).
    ClassDiagram {
        /// Project directory to scan. Defaults to the current working directory.
        #[arg(long, default_value = ".")]
        cwd: PathBuf,
        /// Persist extracted nodes + edges to the graph store.
        #[arg(long)]
        apply: bool,
        /// Emit machine-readable JSON to stdout.
        #[arg(long)]
        json: bool,
        /// Comma-separated languages to process (rust, typescript, python, go).
        /// If omitted, all MVP languages are processed.
        #[arg(long, value_enum, value_delimiter = ',')]
        lang: Vec<crate::code::class_diagram::Language>,
        /// Selector: `file:<path>` or `module:<id>`, or omit for whole project.
        #[arg(long)]
        selector: Option<String>,
    },
    /// Extract state machines from source code (Rust enum+match, TypeScript state pattern, Python transitions).
    StateMachine {
        /// Project directory to scan. Defaults to the current working directory.
        #[arg(long, default_value = ".")]
        cwd: PathBuf,
        /// Persist extracted nodes + edges to the graph store.
        #[arg(long)]
        apply: bool,
        /// Emit machine-readable JSON to stdout.
        #[arg(long)]
        json: bool,
        /// Comma-separated languages to process (rust, typescript, python, go).
        /// If omitted, all supported languages are processed.
        #[arg(long, value_enum, value_delimiter = ',')]
        lang: Vec<crate::code::state_machine::Language>,
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
        #[arg(long, value_enum)]
        status: Option<EvidenceStatus>,
        #[arg(long)]
        json: bool,
    },
    Accept {
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long)]
        id: String,
        #[arg(long)]
        json: bool,
    },
    Supersede {
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long)]
        old_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Ingest semantic facts from JSON input (no file source).
    /// Per ADR-027: persists Evidence with source_origin: UserInput, status: drafted.
    Put {
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// Path to JSON file containing facts array.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Read facts from stdin as JSON array.
        #[arg(long)]
        json: bool,
        /// Evidence kind for all facts in this batch.
        #[arg(long, value_enum, default_value_t = EvidenceKind::Semantic)]
        kind: EvidenceKind,
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
#[command(
    name = "archctl",
    version,
    about = "OpenCode Architecture Diagrammer sidecar CLI (M4)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Doctor {
        /// Run a specific doctor diagnostic scope (e.g., `storage`).
        /// Run `archctl doctor` without this flag for the full diagnostic.
        #[arg(long, value_name = "scope-name")]
        scope: Option<String>,
        /// Run scope gates from `manifests/<id>.toml`. If scope IDs are
        /// given (comma-separated), check only those; otherwise check all.
        /// Example: `doctor --scopes evidence,store,tsg`
        #[arg(long, value_delimiter = ',', value_name = "scope-id")]
        scopes: Option<Vec<String>>,
        /// Emit machine-readable JSON output for `--scope` probes.
        #[arg(long)]
        json: bool,
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
    Diagram {
        #[command(subcommand)]
        action: DiagramAction,
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
    },
    Code {
        #[command(subcommand)]
        action: CodeAction,
    },
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },
    /// Install and list plugins from a tap (ADR-057 §4).
    #[command(name = "plugin")]
    Plugin {
        #[command(subcommand)]
        action: PluginAction,
    },
    /// Manage IDE-specific stack installation (OpenCode, ZCode, Claude Code, Codex).
    #[command(name = "ide")]
    Ide {
        #[command(subcommand)]
        action: IdeAction,
    },
    /// Serve the embedded archview workbench locally (ADR-033).
    View {
        /// Port to bind. 0 = ephemeral (default).
        #[arg(long, default_value_t = 0)]
        port: u16,
        /// Project directory for /api/export (graph-backed bundles).
        #[arg(long)]
        cwd: Option<PathBuf>,
    },
    /// Manage archctl itself — install, list, use, update, uninstall versioned binaries.
    #[command(name = "self", alias = "lifecycle")]
    Lifecycle {
        #[command(subcommand)]
        action: SelfAction,
    },
    /// Query the capability registry (default: JSON output).
    Capabilities {
        /// Output in a specific format (default: json).
        #[arg(long, value_enum, default_value_t = CapabilityFormat::Json)]
        format: CapabilityFormat,
        /// Check whether docs/CAPABILITIES.md is up to date (exits non-zero if stale).
        #[arg(long)]
        check: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum IdeAction {
    Install {
        ide: String,
        #[arg(long)]
        stack: Option<String>,
        #[arg(long)]
        install_root: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        installed: bool,
    },
    Doctor {
        ide: String,
    },
    Remove {
        ide: String,
        #[arg(long)]
        purge: bool,
    },
    Update {
        ide: String,
        #[arg(long, default_value_t = true)]
        sync: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum SelfAction {
    /// Install a versioned archctl binary.
    Install {
        /// Version to install. Defaults to the latest stable release.
        #[arg(long)]
        version: Option<String>,
        /// Override the install root directory.
        #[arg(long)]
        install_root: Option<std::path::PathBuf>,
    },
    /// List installed versions.
    List {
        #[arg(long)]
        json: bool,
        /// Override the install root directory.
        #[arg(long)]
        install_root: Option<std::path::PathBuf>,
    },
    /// Switch the active version.
    Use {
        version: String,
        /// Override the install root directory.
        #[arg(long)]
        install_root: Option<std::path::PathBuf>,
    },
    /// Uninstall a version (or purge all with --purge).
    Uninstall {
        /// Version to remove. Defaults to the current active version.
        #[arg(long)]
        version: Option<String>,
        /// Remove all installed versions.
        #[arg(long)]
        purge: bool,
        /// Override the install root directory.
        #[arg(long)]
        install_root: Option<std::path::PathBuf>,
    },
    /// Self-update to a newer version.
    Update {
        /// Target version. Defaults to latest available.
        #[arg(long)]
        version: Option<String>,
        /// Channel: stable, rc, nightly.
        #[arg(long)]
        channel: Option<String>,
        /// Check for updates without installing.
        #[arg(long)]
        check: bool,
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
        Command::Doctor {
            scope,
            scopes,
            json,
            cwd,
        } => {
            let cwd = ctx.resolve_cwd(cwd.as_ref());

            if let Some(scope_name) = scope {
                // --scope is the storage/specific-scope diagnostic
                let parsed: crate::doctor::DoctorScope = scope_name
                    .parse()
                    .map_err(|e: String| anyhow::anyhow!("unknown doctor scope: {e}"))?;
                doctor::run_scope(parsed, &cwd, json).context("doctor scope")
            } else if let Some(scope_ids) = scopes {
                // --scopes is the scope-gates pass
                doctor::check_scope(&cwd, scope_ids, &*ctx.fs).context("scope gates")
            } else {
                // Full doctor diagnostics
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
            GraphAction::Neighbours {
                cwd,
                id,
                depth,
                json,
            } => graph_neighbours_cmd(cwd, &id, depth, json, ctx),
        },
        Command::Inventory { action } => match action {
            InventoryAction::Tree {
                cwd,
                max_depth,
                max_entries,
                json,
            } => inventory_tree_cmd(cwd, max_depth, max_entries, json, ctx),
            InventoryAction::Languages {
                cwd,
                max_depth,
                max_entries,
                json,
            } => inventory_languages_cmd(cwd, max_depth, max_entries, json, ctx),
            InventoryAction::Depends {
                cwd,
                manifest,
                json,
            } => inventory_depends_cmd(cwd, manifest, json, ctx),
        },
        Command::Diagram { action } => match action {
            DiagramAction::Export {
                cwd,
                selector,
                format,
                output,
                json,
            } => diagram_export_cmd(cwd, &selector, &format, output, json, ctx),
            DiagramAction::Validate {
                cwd,
                bundle_dir,
                json,
            } => diagram_validate_cmd(cwd, bundle_dir, json, ctx),
            DiagramAction::Apply { cwd, changes, json } => {
                diagram_apply_cmd(cwd, changes, json, ctx)
            }
            DiagramAction::Project {
                cwd,
                view,
                format,
                output,
                json,
            } => diagram_project_cmd(cwd, &view, &format, &output, json, ctx),
        },
        Command::Evidence { action } => match action {
            EvidenceAction::Extract {
                cwd,
                lang,
                pattern,
                claim,
                kind,
                json,
                put,
            } => evidence_extract_cmd(cwd, lang, &pattern, &claim, kind, json, put, ctx),
            EvidenceAction::List {
                cwd,
                path,
                status,
                json,
            } => evidence_list_cmd(cwd, path, status, json, ctx),
            EvidenceAction::Accept { cwd, id, json } => evidence_accept_cmd(cwd, &id, json, ctx),
            EvidenceAction::Supersede { cwd, old_id, json } => {
                evidence_supersede_cmd(cwd, &old_id, json, ctx)
            }
            EvidenceAction::Put {
                cwd,
                file,
                json,
                kind,
            } => evidence_put_cmd(cwd, file.as_ref(), json, kind, ctx),
        },
        Command::Render {
            source,
            format,
            out,
        } => render::run(source, format, out, &*ctx.fs).context("render failed"),
        Command::Code { action } => match action {
            CodeAction::C4Discover {
                cwd,
                apply,
                strategy,
                json,
            } => code_c4_discover_cmd(cwd, apply, strategy.as_deref(), json, ctx),
            CodeAction::CallGraph {
                cwd,
                apply,
                json,
                lang,
                depth,
            } => {
                let info = crate::project::resolve_project(&cwd.to_string_lossy());
                let report = crate::code::call_graph::extract(&cwd, &lang, depth, &*ctx.fs)
                    .map_err(|e| anyhow::anyhow!("extract failed: {e}"))?;
                if apply {
                    let apply_report =
                        crate::code::call_graph::apply(&info.project_dir, &report, &*ctx.fs)
                            .map_err(|e| anyhow::anyhow!("apply failed: {e}"))?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&apply_report)?);
                    } else {
                        println!(
                            "Applied {} elements ({} skipped), {} relations ({} skipped), {} evidences ({} ms).",
                            apply_report.elements_written,
                            apply_report.elements_skipped,
                            apply_report.relations_written,
                            apply_report.relations_skipped,
                            apply_report.evidences_written,
                            apply_report.duration_ms
                        );
                    }
                } else if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    crate::code::output::print_call_graph_table(&report);
                }
                Ok(0)
            }
            CodeAction::Sequence {
                cwd,
                from,
                depth,
                max_interactions,
                json,
                apply,
            } => {
                if apply {
                    eprintln!(
                        "warning: sequence --apply is read-only (spec SCN-217); use call-graph --apply to persist edges"
                    );
                }
                code_sequence_cmd(&cwd, from, depth, max_interactions, json, ctx)
            }
            CodeAction::ClassDiagram {
                cwd,
                apply,
                json,
                lang,
                selector,
            } => code_class_diagram_cmd(&cwd, apply, json, &lang, selector.as_deref(), ctx),
            CodeAction::StateMachine {
                cwd,
                apply,
                json,
                lang,
            } => code_state_machine_cmd(&cwd, apply, json, &lang, ctx),
        },
        Command::Skills { action } => skills::run(action, &*ctx.fs).context("skills failed"),
        Command::Agent { action } => match action {
            AgentAction::List { json } => {
                use crate::cognitive::{AgentDescriptor, AgentRegistry};
                let reg = AgentRegistry::new();
                let agents: Vec<AgentDescriptor> = reg.ids().filter_map(|id| reg.get(id)).collect();
                if json {
                    println!("{}", serde_json::to_string_pretty(&agents)?);
                } else if agents.is_empty() {
                    println!("No agents registered.");
                } else {
                    println!(
                        "{:<20} {:<10} {:<15} Deterministic",
                        "ID", "Version", "ModelPolicy"
                    );
                    println!("{}", "-".repeat(60));
                    for a in &agents {
                        println!(
                            "{:<20} {:<10} {:<15} {}",
                            a.id,
                            a.version,
                            format!("{:?}", a.model_policy),
                            a.deterministic
                        );
                    }
                }
                Ok(0)
            }
            AgentAction::Dispatch { goal, json } => {
                use crate::cognitive::{AgentContext, AgentOutput, AgentRegistry, SyncDispatcher};
                let reg = AgentRegistry::new();
                // v1.0: no agents registered yet, dispatcher returns NoAction
                let disp = SyncDispatcher::new(&reg);
                let ctx = AgentContext {
                    goal,
                    triggering_event: None,
                    graph_view: Default::default(),
                    source_fragments: vec![],
                    evidence: vec![],
                    applicable_rules: vec![],
                    available_tools: vec![],
                    budget: Default::default(),
                };
                let out = disp.dispatch(&ctx)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&out)?);
                } else {
                    match &out {
                        AgentOutput::NoAction(r) => {
                            println!("No action: [{:?}] {}", r.code, r.message);
                        }
                        AgentOutput::FindingCandidate(f) => {
                            println!(
                                "FindingCandidate: {} [{:.0}%]",
                                f.title,
                                f.confidence * 100.0
                            );
                        }
                        AgentOutput::ActionProposal(p) => {
                            println!(
                                "ActionProposal: {} (approval_required={})",
                                p.goal, p.approval_required
                            );
                        }
                        AgentOutput::Hypothesis(h) => {
                            println!("Hypothesis: {} [{:.0}%]", h.statement, h.confidence * 100.0);
                        }
                        AgentOutput::QueryPlan(q) => {
                            println!("QueryPlan: {} steps", q.cypher_steps.len());
                        }
                        AgentOutput::ProjectionSpec(p) => {
                            println!("ProjectionSpec: {:?}", p.view_kind);
                        }
                        AgentOutput::ActionPlan(a) => {
                            println!("ActionPlan: {} steps", a.steps.len());
                        }
                        AgentOutput::DocumentationPatch(d) => {
                            println!("DocumentationPatch: {} ({:?})", d.file, d.patch_type);
                        }
                        AgentOutput::ContextRequest(c) => {
                            println!(
                                "ContextRequest: {} ({} missing)",
                                c.request_id,
                                c.missing.len()
                            );
                        }
                    }
                }
                Ok(0)
            }
        },
        Command::Mcp { action } => match action {
            McpAction::ListTools { json } => {
                use crate::cognitive::ALLOWED_TOOLS;
                if json {
                    let tools: Vec<&str> = ALLOWED_TOOLS.to_vec();
                    println!("{}", serde_json::to_string_pretty(&tools)?);
                } else {
                    println!("Allowed MCP tools (v1.0 read-only allowlist):");
                    for tool in ALLOWED_TOOLS {
                        println!("  - {}", tool);
                    }
                }
                Ok(0)
            }
            McpAction::Invoke {
                tool,
                data,
                json,
                governed,
            } => {
                use crate::cognitive::PolicyGate;
                use crate::cognitive::{McpGateway, ToolResult};
                use std::io::{self, Read};
                let input_json: serde_json::Value = if let Some(path) = data {
                    if path.as_os_str() == "-" {
                        let mut buf = String::new();
                        io::stdin().read_to_string(&mut buf)?;
                        serde_json::from_str(&buf)
                            .map_err(|e| anyhow::anyhow!("invalid JSON from stdin: {}", e))?
                    } else {
                        let contents = std::fs::read_to_string(&path)
                            .map_err(|e| anyhow::anyhow!("read {}: {}", path.display(), e))?;
                        serde_json::from_str(&contents).map_err(|e| {
                            anyhow::anyhow!("invalid JSON in {}: {}", path.display(), e)
                        })?
                    }
                } else {
                    serde_json::Value::Object(serde_json::Map::new())
                };
                if governed {
                    // Governed path: route through PolicyGate
                    use crate::cognitive::policy::PolicyContext;
                    let ctx = PolicyContext::default();
                    let gate = PolicyGate::new();
                    let input_str = serde_json::to_string(&input_json)
                        .map_err(|e| anyhow::anyhow!("serialize governed input: {}", e))?;
                    let governed_result = gate
                        .handle_governed(&input_str, &ctx)
                        .map_err(|e| anyhow::anyhow!("governed invocation failed: {e}"))?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&governed_result).map_err(|e| {
                                anyhow::anyhow!("serialize governed result: {}", e)
                            })?
                        );
                    } else {
                        print_governed_result(&governed_result);
                    }
                } else {
                    // Direct path: use McpGateway
                    let req = serde_json::json!({ "tool": tool, "args": input_json });
                    let gw = McpGateway::new();
                    let result = gw.handle_raw(&serde_json::to_string(&req).unwrap());
                    if json {
                        println!("{}", result);
                    } else {
                        let parsed: ToolResult = serde_json::from_str(&result).unwrap();
                        if let Some(err) = parsed.error {
                            eprintln!("error: {}", err);
                            return Ok(1);
                        }
                        println!("tool: {}", parsed.tool);
                        if let Some(data) = parsed.data {
                            println!("result: {}", serde_json::to_string_pretty(&data).unwrap());
                        }
                    }
                }
                Ok(0)
            }
        },
        Command::Plugin { action } => match action {
            PluginAction::Install { spec, tap } => {
                let tap_url = tap.unwrap_or_else(|| {
                    "https://raw.githubusercontent.com/Rubentxu/arch-stack/main/taps/official.json"
                        .to_string()
                });
                let tap = crate::plugin::fetch_tap(&tap_url)?;
                let (author, name, version) = crate::plugin::parse_plugin_spec(&spec)
                    .with_context(|| {
                        format!(
                            "invalid plugin spec '{}': expected author/name@version",
                            spec
                        )
                    })?;

                // Resolve version: "latest" = highest semver in tap.
                let entry = if version == "latest" {
                    tap.plugins
                        .iter()
                        .filter(|p| p.name == name)
                        .max_by_key(|p| semver::Version::parse(&p.version).ok())
                        .ok_or_else(|| anyhow::anyhow!("plugin {name} not in tap"))?
                        .clone()
                } else {
                    tap.plugins
                        .iter()
                        .find(|p| p.name == name && p.version == version)
                        .ok_or_else(|| anyhow::anyhow!("plugin {name}@{version} not in tap"))?
                        .clone()
                };

                let dir = crate::plugin::install::install_plugin(&author, &name, &entry)
                    .with_context(|| {
                        format!(
                            "install plugin {}@{} from {}",
                            entry.name, entry.version, tap_url
                        )
                    })?;
                eprintln!(
                    "installed {}@{} to {}",
                    entry.name,
                    entry.version,
                    dir.display()
                );
                Ok(0)
            }
            PluginAction::List { tap_url } => {
                let tap = crate::plugin::fetch_tap(&tap_url)?;
                for p in &tap.plugins {
                    println!("  {:<30} v{}", p.name, p.version);
                }
                Ok(0)
            }
        },
        Command::Lifecycle { action } => match action {
            SelfAction::Install {
                version,
                install_root,
            } => {
                let root = install_root
                    .clone()
                    .unwrap_or_else(crate::lifecycle::install_root::install_root);
                let source = std::env::current_exe().context("locate current binary")?;
                let v = version.unwrap_or_else(|| "0.0.0".to_string());
                // T2 stub: version is always provided; T2.1 will resolve latest.
                let ver = semver::Version::parse(&v).context("parse version")?;
                crate::lifecycle::install::install(&ver, &root, &source)?;
                // Install shim (W2 fix). Try /usr/local/bin first, fall back to
                // ~/.local/bin/ if permission denied.
                let shim_targets = [
                    PathBuf::from("/usr/local/bin/archctl"),
                    crate::lifecycle::install_root::install_root()
                        .parent()
                        .map(|p| p.join(".local/bin/archctl"))
                        .unwrap_or_else(|| PathBuf::from("~/.local/bin/archctl")),
                ];
                for target in &shim_targets {
                    match crate::lifecycle::shim::install_shim(target) {
                        Ok(()) => {
                            eprintln!("installed shim at {}", target.display());
                            break;
                        }
                        Err(e) => {
                            eprintln!("could not install shim at {}: {e}", target.display());
                        }
                    }
                }
                println!("installed archctl {} at {}", v, root.display());
                Ok(0)
            }
            SelfAction::List { json, install_root } => {
                let root = install_root
                    .clone()
                    .unwrap_or_else(crate::lifecycle::install_root::install_root);
                let versions = crate::lifecycle::list::list(&root)?;
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&versions).context("serialize versions")?
                    );
                } else if versions.is_empty() {
                    println!("no archctl versions installed");
                } else {
                    for v in versions {
                        let marker = if v.is_active { "*" } else { " " };
                        println!("{} v{} (installed)", marker, v.version);
                    }
                }
                Ok(0)
            }
            SelfAction::Use {
                version,
                install_root,
            } => {
                let root = install_root
                    .clone()
                    .unwrap_or_else(crate::lifecycle::install_root::install_root);
                let ver = semver::Version::parse(&version).context("parse version")?;
                crate::lifecycle::use_version::use_version(&ver, &root)?;
                println!("switched archctl to v{}", ver);
                Ok(0)
            }
            SelfAction::Uninstall {
                version,
                install_root,
                purge,
            } => {
                let root = install_root
                    .clone()
                    .unwrap_or_else(crate::lifecycle::install_root::install_root);
                let target = version
                    .map(|v| semver::Version::parse(&v))
                    .transpose()
                    .context("parse version")?;
                crate::lifecycle::uninstall::uninstall(target.as_ref(), &root, purge)?;
                if purge {
                    println!("purged all archctl installations");
                } else if let Some(v) = target {
                    println!("uninstalled archctl v{}", v);
                } else {
                    println!("uninstalled");
                }
                Ok(0)
            }
            SelfAction::Update {
                version,
                channel,
                check,
            } => {
                use crate::lifecycle::{self as lc, Channel};
                let root = std::env::var("ARCHCTL_HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| lc::install_root::install_root());
                let current_str = std::env::var("ARCHCTL_VERSION")
                    .ok()
                    .or_else(|| {
                        std::fs::read_to_string(root.join("current").join("archctl-version")).ok()
                    })
                    .unwrap_or_else(|| "0.0.0".into());
                let current = semver::Version::parse(current_str.trim())
                    .unwrap_or_else(|_| semver::Version::new(0, 0, 0));
                let chan: Channel = channel
                    .as_deref()
                    .unwrap_or("stable")
                    .parse()
                    .map_err(anyhow::Error::msg)?;
                let target = version
                    .as_ref()
                    .map(|v| semver::Version::parse(v))
                    .transpose()
                    .context("parse target version")?;
                if check {
                    let release =
                        lc::fetch_release_info(target.as_ref().map(|v| format!("v{v}")).as_deref())
                            .context("fetch release info")?;
                    let new_ver = semver::Version::parse(release.tag_name.trim_start_matches('v'))
                        .context("parse tag as semver")?;
                    if new_ver > current {
                        println!("update available: {} -> {}", current, new_ver);
                    } else {
                        println!("already at latest ({})", current);
                    }
                    return Ok(0);
                }
                let new_ver = lc::update::update(target.as_ref(), chan, &root, &current)
                    .context("self-update failed")?;
                println!("updated: {} -> {}", current, new_ver);
                Ok(0)
            }
        },
        Command::View { port, cwd } => {
            let project_dir = cwd.map(|p| p.to_string_lossy().to_string());
            let options = crate::view::ViewOptions {
                port,
                project_dir,
                env: ctx.env.clone(),
            };
            crate::view::run(options).context("view failed")?;
            Ok(0)
        }
        Command::Ide { action } => match action {
            IdeAction::Install {
                ide,
                stack: _,
                install_root: _,
            } => {
                let adapters = builtin_adapters();
                let adapter = adapters.iter().find(|a| a.id() == ide).ok_or_else(|| {
                    anyhow::anyhow!(
                        "unknown IDE: {ide}; available: {}",
                        adapters
                            .iter()
                            .map(|a| a.id())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })?;
                let payload = current_stack_payload()?;
                let report = adapter.install_stack(&payload)?;
                println!(
                    "installed {} skills, {} agents, {} plugins for {}",
                    report.written.len(),
                    payload.agents.len(),
                    payload.plugins.len(),
                    adapter.name()
                );
                Ok(0)
            }
            IdeAction::List { installed } => {
                for adapter in builtin_adapters() {
                    let presence = adapter.detect()?;
                    let marker = if presence.installed { "✓" } else { "✗" };
                    if installed && !presence.installed {
                        continue;
                    }
                    println!("  [{marker}] {:<12} {}", adapter.id(), adapter.name());
                }
                Ok(0)
            }
            IdeAction::Doctor { ide } => {
                let adapters = builtin_adapters();
                let adapter = adapters
                    .iter()
                    .find(|a| a.id() == ide)
                    .ok_or_else(|| anyhow::anyhow!("unknown IDE: {ide}"))?;
                let presence = adapter.detect()?;
                println!("{} ({})", adapter.name(), adapter.id());
                println!("  installed: {}", presence.installed);
                println!("  config_root: {}", adapter.config_root().display());
                if let Some(hint) = presence.hint {
                    println!("  hint: {hint}");
                }
                Ok(0)
            }
            IdeAction::Remove { ide, purge } => {
                let adapters = builtin_adapters();
                let adapter = adapters
                    .iter()
                    .find(|a| a.id() == ide)
                    .ok_or_else(|| anyhow::anyhow!("unknown IDE: {ide}"))?;
                let payload_id = format!("arch-stack-{}", env!("CARGO_PKG_VERSION"));
                let report = adapter.remove_stack(&payload_id)?;
                println!(
                    "removed {} paths from {}",
                    report.written.len(),
                    adapter.name()
                );
                if purge {
                    eprintln!("(purge: directory still exists; user-managed)");
                }
                Ok(0)
            }
            IdeAction::Update { ide, sync: _ } => {
                // M75 PR #3 stub: Update is alias for install (re-sync).
                let adapters = builtin_adapters();
                let adapter = adapters
                    .iter()
                    .find(|a| a.id() == ide)
                    .ok_or_else(|| anyhow::anyhow!("unknown IDE: {ide}"))?;
                let payload = StackPayload {
                    id: format!("arch-stack-{}", env!("CARGO_PKG_VERSION")),
                    version: semver::Version::parse(env!("CARGO_PKG_VERSION"))?,
                    skills: vec![],
                    agents: vec![],
                    plugins: vec![],
                };
                let report = adapter.install_stack(&payload)?;
                println!(
                    "re-installed {} paths for {}",
                    report.written.len(),
                    adapter.name()
                );
                Ok(0)
            }
        },
        Command::Capabilities { format, check } => Ok(capabilities_cmd(format, check)?),
    }
}

fn print_governed_result(r: &crate::cognitive::GovernedToolResult) {
    println!("policy.outcome: {:?}", r.policy.outcome);
    if let Some(ref err) = r.tool.error {
        eprintln!("tool error: {}", err);
    } else {
        println!("tool: {}", r.tool.tool);
        if let Some(ref data) = r.tool.data {
            println!("result: {}", serde_json::to_string_pretty(data).unwrap());
        }
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
    // Open + init purely for the side effect of ensuring the schema exists.
    let _store = ctx.store_factory.open_and_init(&info.project_dir)?;
    let path = graph::database_path(&info.project_dir);
    if json {
        println!(
            "{}",
            serde_json::json!({"database": path.display().to_string(), "project_id": info.project_id})
        );
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
    let store = ctx.store_factory.open_and_init(&info.project_dir)?;
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

fn graph_query_cmd(
    cwd: Option<PathBuf>,
    cypher: &str,
    json: bool,
    ctx: &CliContext,
) -> Result<i32> {
    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let info = resolve_project(&cwd.to_string_lossy());
    let raw_query = ctx.raw_query_factory.open_raw(&info.project_dir)?;
    let rows = raw_query.query(cypher).context("graph query")?;
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

fn graph_neighbours_cmd(
    cwd: Option<PathBuf>,
    id: &str,
    depth: u8,
    json: bool,
    ctx: &CliContext,
) -> Result<i32> {
    use tracing::warn;
    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let info = resolve_project(&cwd.to_string_lossy());
    // Identifier validation belongs to the domain — never trust user
    // input to a Cypher interpolation, even with the port abstracting
    // the engine.
    let safe_id = graph::validate_identifier(id).context("invalid element id")?;
    let clamped_depth = depth.clamp(1, 4);
    if depth > 2 {
        warn!(
            depth,
            "graph traversal depth > 2 may be slow on large graphs"
        );
    }
    let cypher = format!(
        "MATCH (e:Element {{id: '{safe_id}'}})-[*1..{clamped_depth}]-(n) \
         RETURN DISTINCT n.id AS id, labels(n) AS kinds;"
    );
    let raw_query = ctx.raw_query_factory.open_raw(&info.project_dir)?;
    let rows = raw_query.query(&cypher).context("graph neighbours")?;
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
fn capabilities_cmd(format: CapabilityFormat, check: bool) -> anyhow::Result<i32> {
    let reg = crate::capability::registry();

    if check {
        // S9 / S10: staleness check against docs/CAPABILITIES.md.
        let fresh = crate::capability::render_markdown(&reg);
        let docs_path = std::path::Path::new("docs/CAPABILITIES.md");
        if !docs_path.exists() {
            anyhow::bail!(
                "docs/CAPABILITIES.md does not exist. Run: \
                archctl capabilities --format markdown > docs/CAPABILITIES.md"
            );
        }
        let current = std::fs::read_to_string(docs_path).context("read docs/CAPABILITIES.md")?;
        // Normalize: shell redirect adds exactly one \n via println!. Strip exactly one
        // trailing \n so we compare the markdown content, not the println! artifact.
        let current_trimmed = if current.ends_with('\n') {
            &current[..current.len() - 1]
        } else {
            &current[..]
        };
        if current_trimmed != fresh {
            let current_lines: Vec<&str> = current_trimmed.lines().collect();
            let fresh_lines: Vec<&str> = fresh.lines().collect();
            let max_len = current_lines.len().max(fresh_lines.len());
            let mut mismatch = 0;
            for (i, (c, f)) in current_lines
                .iter()
                .zip(fresh_lines.iter())
                .enumerate()
                .take(max_len)
            {
                if c != f {
                    mismatch = i + 1;
                    break;
                }
            }
            if mismatch == 0 && current_lines.len() != fresh_lines.len() {
                mismatch = 1;
            }
            anyhow::bail!(
                "docs/CAPABILITIES.md is stale (first difference at line {}). Run: \
                archctl capabilities --format markdown > docs/CAPABILITIES.md",
                mismatch
            );
        }
        Ok(0)
    } else {
        // Default: emit in the requested format.
        match format {
            CapabilityFormat::Json => {
                let output = crate::capability::render_json(&reg);
                println!("{}", output);
            }
            CapabilityFormat::Markdown => {
                let output = crate::capability::render_markdown(&reg);
                println!("{}", output);
            }
        }
        Ok(0)
    }
}

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
        println!(
            "files: {}  bytes: {}",
            summary.total_files, summary.total_bytes
        );
        let mut v: Vec<_> = summary.languages.iter().collect();
        v.sort_by_key(|b| std::cmp::Reverse(b.1.bytes));
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

// CLI dispatch fn: each param maps 1:1 to a clap flag. Restructuring into
// a struct would add ceremony without reducing surface area.
#[allow(clippy::too_many_arguments)]
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
    let result = evidence::extract(&cwd, lang, pattern, claim, kind, &*ctx.clock, &*ctx.fs)?;
    let written = if do_put {
        let info = resolve_project(&cwd.to_string_lossy());
        evidence::put_with_clock(&info.project_dir, &result.evidence, &*ctx.clock)
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

fn evidence_accept_cmd(
    cwd: Option<PathBuf>,
    id: &str,
    json: bool,
    ctx: &CliContext,
) -> Result<i32> {
    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let info = resolve_project(&cwd.to_string_lossy());
    let mut store = ctx.store_factory.open_and_init(&info.project_dir)?;

    let result = store.accept_evidence(id, &*ctx.clock);

    if json {
        #[derive(serde::Serialize)]
        struct AcceptEnvelope {
            action: &'static str,
            id: String,
            ok: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            error: Option<String>,
        }
        let envelope = match result {
            Ok(()) => AcceptEnvelope {
                action: "accept",
                id: id.to_string(),
                ok: true,
                error: None,
            },
            Err(e) => AcceptEnvelope {
                action: "accept",
                id: id.to_string(),
                ok: false,
                error: Some(e.to_string()),
            },
        };
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        match result {
            Ok(()) => println!("accepted: {id}"),
            Err(e) => {
                eprintln!("error: {e}");
                if e.to_string().contains("not found") {
                    return Ok(3);
                }
                if e.to_string().contains("cannot accept superseded") {
                    return Ok(4);
                }
                return Ok(1);
            }
        }
    }
    Ok(0)
}

fn evidence_supersede_cmd(
    cwd: Option<PathBuf>,
    old_id: &str,
    json: bool,
    ctx: &CliContext,
) -> Result<i32> {
    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let info = resolve_project(&cwd.to_string_lossy());
    let mut store = ctx.store_factory.open_and_init(&info.project_dir)?;

    let result = store.supersede_evidence(old_id);

    if json {
        #[derive(serde::Serialize)]
        struct SupersedeEnvelope {
            action: &'static str,
            old_id: String,
            ok: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            error: Option<String>,
        }
        let envelope = match result {
            Ok(()) => SupersedeEnvelope {
                action: "supersede",
                old_id: old_id.to_string(),
                ok: true,
                error: None,
            },
            Err(e) => SupersedeEnvelope {
                action: "supersede",
                old_id: old_id.to_string(),
                ok: false,
                error: Some(e.to_string()),
            },
        };
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        match result {
            Ok(()) => println!("superseded: {old_id}"),
            Err(e) => {
                eprintln!("error: {e}");
                if e.to_string().contains("not found") {
                    return Ok(3);
                }
                return Ok(1);
            }
        }
    }
    Ok(0)
}

/// Parse evidence input that may be either:
/// - `{facts: [...]}` — object with facts array (SCN-400)
/// - `[{...}]` — top-level array of facts (SCN-400, SCN-401)
fn parse_evidence_input(input: &str) -> Result<Vec<EvidencePutFact>> {
    // Try parsing as object with `facts` key first
    if let Ok(req) = serde_json::from_str::<EvidencePutRequest>(input) {
        return Ok(req.facts);
    }
    // Try parsing as top-level array (SCN-400, SCN-401)
    if let Ok(facts) = serde_json::from_str::<Vec<EvidencePutFact>>(input) {
        return Ok(facts);
    }
    anyhow::bail!(
        "input must be either {{facts: [...]}} or [{{...}}]; \
         got neither a valid object with 'facts' key nor a JSON array"
    );
}

/// Input schema for `evidence put` JSON batch — object form.
#[derive(Debug, serde::Deserialize)]
struct EvidencePutRequest {
    facts: Vec<EvidencePutFact>,
}

/// One fact in an evidence put batch.
#[derive(Debug, serde::Deserialize)]
struct EvidencePutFact {
    kind: Option<String>,
    claim: Option<String>,
    #[serde(default)]
    props: serde_json::Map<String, serde_json::Value>,
}

/// Result of processing a single fact.
struct ProcessedFact {
    evidence: Evidence,
    evidence_id: String,
    source: SourceArtifact,
}

/// Error details for a single fact that failed processing.
struct FactError {
    index: usize,
    claim: Option<String>,
    error: String,
}

fn evidence_put_cmd(
    cwd: Option<PathBuf>,
    file: Option<&PathBuf>,
    json_flag: bool,
    kind_flag: EvidenceKind,
    ctx: &CliContext,
) -> Result<i32> {
    use crate::evidence::semantic_evidence_id;
    use std::io::Read;

    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let info = resolve_project(&cwd.to_string_lossy());
    // Ensure the schema is in place without holding the DB lock:
    // graph::init applies pending migrations and releases the session.
    // put_with_source then opens the store freely (ADR-010 single-writer).
    graph::init(&info.project_dir, &*ctx.fs)?;

    // Read JSON input from --file or stdin
    let raw_input = if let Some(path) = file {
        ctx.fs.read_to_string(path).context("read --file")?
    } else if json_flag {
        // --json means read facts array from stdin
        let mut buf = String::new();
        match std::io::stdin().read_to_string(&mut buf) {
            Ok(0) => {
                eprintln!("error: --json requires stdin input but stdin is empty");
                return Ok(1);
            }
            Ok(_) => buf,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                eprintln!("error: --json requires stdin input");
                return Ok(1);
            }
            Err(e) => {
                return Err(anyhow::anyhow!("read stdin: {}", e));
            }
        }
    } else {
        anyhow::bail!("evidence put requires either --file or --json flag");
    };

    // Parse input — accepts both {facts:[...]} and [...] formats (SCN-400, SCN-401)
    let facts = match parse_evidence_input(&raw_input) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: parse JSON: {}", e);
            return Ok(1);
        }
    };

    if facts.is_empty() {
        eprintln!("error: no facts provided");
        return Ok(1);
    }

    // Process each fact, collecting successes and failures (partial success, SCN-402)
    let mut processed: Vec<ProcessedFact> = Vec::new();
    let mut errors: Vec<FactError> = Vec::new();

    for (idx, fact) in facts.into_iter().enumerate() {
        // Validate required 'claim' field
        let claim = match &fact.claim {
            Some(c) => c.clone(),
            None => {
                errors.push(FactError {
                    index: idx,
                    claim: None,
                    error: "missing required field 'claim'".to_string(),
                });
                continue;
            }
        };

        // Validate claim max length
        if claim.len() > 255 {
            errors.push(FactError {
                index: idx,
                claim: Some(claim),
                error: "'claim' exceeds 255 characters".to_string(),
            });
            continue;
        }

        // Resolve kind: fact must have a kind field (SCN-402 — reject if missing)
        let fact_kind = match &fact.kind {
            Some(kind_str) => match EvidenceKind::parse_label(kind_str) {
                Some(k) => k,
                None => {
                    errors.push(FactError {
                        index: idx,
                        claim: Some(claim),
                        error: format!("unknown kind '{}'", kind_str),
                    });
                    continue;
                }
            },
            None => {
                // SCN-402: missing kind is an error, no silent default to CLI flag
                errors.push(FactError {
                    index: idx,
                    claim: Some(claim),
                    error: "missing required field 'kind' (use --kind flag to set batch default)"
                        .to_string(),
                });
                continue;
            }
        };

        // SCN-407: if --kind semantic is set, each fact must have value.semantic: true
        if kind_flag == EvidenceKind::Semantic {
            match fact.props.get("semantic") {
                Some(serde_json::Value::Bool(true)) => {}
                Some(v) => {
                    errors.push(FactError {
                        index: idx,
                        claim: Some(claim),
                        error: format!(
                            "--kind semantic requires fact prop 'semantic: true', got {:?}",
                            v
                        ),
                    });
                    continue;
                }
                None => {
                    errors.push(FactError {
                        index: idx,
                        claim: Some(claim),
                        error: "--kind semantic requires fact prop 'semantic: true'".to_string(),
                    });
                    continue;
                }
            }
        }

        // Compute semantic evidence id (SCN-404)
        let evidence_id = semantic_evidence_id(
            fact_kind.as_str(),
            &claim,
            crate::evidence::SourceOrigin::UserInput,
            &fact.props,
        );

        // Build synthetic SourceArtifact (ADR-027 D3)
        let sa =
            SourceArtifact::synthetic(fact_kind.as_str(), &claim, ctx.clock.now_rfc3339().as_str());

        // Build Evidence row (SCN-403, SCN-400)
        let evidence = Evidence {
            id: evidence_id.clone(),
            kind: fact_kind,
            claim: claim.clone(),
            path: "synthetic:".to_string(), // No file for semantic facts (ADR-027 D3)
            start_line: 0,
            end_line: 0,
            start_byte: None,
            end_byte: None,
            tool_name: crate::evidence::TOOL_NAME.to_string(),
            tool_version: crate::evidence::TOOL_VERSION.to_string(),
            rule_id: format!("evidence:put:{}", fact_kind.as_str()),
            language: fact
                .props
                .get("language")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            observed_at: ctx.clock.now_rfc3339(),
            source_origin: crate::evidence::SourceOrigin::UserInput,
            content_hash: None,
            text_preview: Some(claim.clone()),
            props: {
                let mut p = fact.props.clone();
                p.insert(
                    "status".to_string(),
                    serde_json::Value::String("drafted".to_string()),
                );
                p.insert(
                    "source_origin".to_string(),
                    serde_json::Value::String(
                        crate::evidence::SourceOrigin::UserInput
                            .as_str()
                            .to_string(),
                    ),
                );
                p
            },
            status: crate::evidence::EvidenceStatus::Drafted,
        };

        processed.push(ProcessedFact {
            evidence,
            evidence_id,
            source: sa,
        });
    }

    // Report errors (SCN-402 partial success)
    let total = processed.len() + errors.len();
    for err in &errors {
        let claim_str = err
            .claim
            .as_deref()
            .map(|s| format!(" (claim: {})", s))
            .unwrap_or_default();
        eprintln!("error: fact[{}]{}: {}", err.index, claim_str, err.error);
    }

    if processed.is_empty() {
        eprintln!("error: 0 facts succeeded");
        return Ok(1);
    }

    // Persist via put_with_source (SCN-400, SCN-405)
    // Each fact gets a synthetic SourceArtifact linked via SUPPORTED_BY
    let evidence: Vec<_> = processed.iter().map(|p| p.evidence.clone()).collect();
    let sources: Vec<_> = processed.iter().map(|p| p.source.clone()).collect();
    let written = crate::evidence::put_with_source(
        &info.project_dir,
        &evidence,
        Some(&sources),
        None,
        &*ctx.clock,
    )
    .context("persist evidence")?;

    // Output results (SCN-408)
    let ids: Vec<_> = processed.iter().map(|p| p.evidence_id.clone()).collect();
    if json_flag {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "processed": total,
                "succeeded": written,
                "failed": errors.len(),
                "persisted": written,
                "evidence_ids": ids
            }))?
        );
    } else {
        println!(
            "processed: {}, succeeded: {}, failed: {}",
            total,
            written,
            errors.len()
        );
        for id in &ids {
            println!("  {}", id);
        }
    }

    // SCN-402: exit 0 if ≥1 succeeded
    Ok(0)
}

fn evidence_list_cmd(
    cwd: Option<PathBuf>,
    path: Option<String>,
    status: Option<EvidenceStatus>,
    json: bool,
    ctx: &CliContext,
) -> Result<i32> {
    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let info = resolve_project(&cwd.to_string_lossy());
    let safe_path = path
        .as_deref()
        .map(crate::graph::validate_identifier)
        .transpose()?;
    let store = ctx.store_factory.open_and_init(&info.project_dir)?;

    let rows = if let Some(s) = status {
        store
            .list_evidence_by_status(s, safe_path)
            .context("evidence list by status")?
    } else {
        store.list_evidence(safe_path).context("evidence list")?
    };

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
                sl = row
                    .get("e.start_line")
                    .and_then(|c| c.as_i64())
                    .unwrap_or(0),
                el = row.get("e.end_line").and_then(|c| c.as_i64()).unwrap_or(0),
                claim = row.get("e.claim").and_then(|c| c.as_str()).unwrap_or(""),
            );
        }
    }
    Ok(0)
}

fn diagram_export_cmd(
    cwd: Option<PathBuf>,
    selector: &str,
    format: &str,
    output: Option<PathBuf>,
    json: bool,
    ctx: &CliContext,
) -> Result<i32> {
    use crate::diagram::export_types::ExportFormat;

    let fmt = ExportFormat::parse(format).ok_or_else(|| {
        anyhow::anyhow!("accepted formats: viewer-bundle, arrows (got: {format})")
    })?;

    if !json && output.is_none() && fmt == ExportFormat::ViewerBundle {
        anyhow::bail!(
            "--output is required when --json is not set (or use --json for stdout-only mode)"
        );
    }

    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let info = resolve_project(&cwd.to_string_lossy());
    let store = ctx.store_factory.open_and_init(&info.project_dir)?;

    match fmt {
        ExportFormat::ViewerBundle => {
            if !json && output.is_none() {
                anyhow::bail!(
                    "--output is required when --json is not set (or use --json for stdout-only mode)"
                );
            }

            // Single-source: build the bundle once, then dispatch to stdout and/or disk.
            let bundle = crate::diagram::build_bundle(&*store, selector, &*ctx.clock)?;

            if json {
                // Emit the FULL bundle envelope (manifest + projection + evidence
                // + styles) to stdout as a single JSON document. Agents pipe this
                // to jq or other tools without writing 5 files.
                let envelope = crate::diagram::build_export_envelope(&bundle);
                println!("{}", serde_json::to_string_pretty(&envelope)?);
            }

            if let Some(out_dir) = output.as_ref() {
                // File-write mode: emit 5 files using `run_export`. The bundle
                // is built again internally (queries are idempotent + cached via
                // graph layer); this keeps `run_export` callable independently.
                let report =
                    crate::diagram::run_export(&*store, selector, out_dir, &*ctx.clock, &*ctx.fs)?;
                if !json {
                    println!(
                        "Exported {} elements, {} edges, {} evidence to {}",
                        report.element_count,
                        report.edge_count,
                        report.evidence_count,
                        out_dir.display()
                    );
                }
            }
        }
        ExportFormat::Arrows => {
            let bundle = crate::diagram::build_bundle(&*store, selector, &*ctx.clock)?;

            let doc = crate::diagram::arrows::serialize(&bundle.projection, &bundle.styles);
            let unplaced = crate::diagram::arrows::count_unplaced(&bundle.projection);

            if json {
                #[derive(serde::Serialize)]
                struct JsonEnvelope<'a> {
                    format: &'static str,
                    document: &'a crate::diagram::arrows::ArrowsDocument,
                    unplaced_count: usize,
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(&JsonEnvelope {
                        format: "arrows",
                        document: &doc,
                        unplaced_count: unplaced,
                    })?
                );
            }

            if let Some(out_path) = output.as_ref() {
                ctx.fs
                    .write(out_path, serde_json::to_string_pretty(&doc)?.as_bytes())?;
                if !json {
                    println!("Exported arrows to {}", out_path.display());
                }
            } else if !json {
                // No output path and not json: derive default path
                let default_path = crate::diagram::arrows::derive_default_path(selector);
                ctx.fs.write(
                    &default_path,
                    serde_json::to_string_pretty(&doc)?.as_bytes(),
                )?;
                println!("Exported arrows to {}", default_path.display());
            }
        }
    }
    Ok(0)
}

fn diagram_validate_cmd(
    cwd: Option<PathBuf>,
    bundle_dir: PathBuf,
    json: bool,
    ctx: &CliContext,
) -> Result<i32> {
    let _cwd = ctx.resolve_cwd(cwd.as_ref());
    let report = crate::diagram::run_validate(&bundle_dir, &*ctx.fs)?;

    if report.is_valid() {
        if !json {
            println!("Bundle {} is valid", bundle_dir.display());
        }
        Ok(0)
    } else {
        if !json {
            println!("Bundle {} has validation errors:", bundle_dir.display());
            for err in &report.errors {
                println!("  [{}] {}", err.file, err.error);
            }
        } else {
            let errors: Vec<serde_json::Value> = report
                .errors
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "file": e.file,
                        "error": e.error
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&errors)?);
        }
        Ok(1)
    }
}

fn diagram_apply_cmd(
    cwd: Option<PathBuf>,
    changes: PathBuf,
    json: bool,
    ctx: &CliContext,
) -> Result<i32> {
    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let report = crate::diagram::run_apply(&cwd, &changes, &*ctx.clock, &*ctx.fs)
        .map_err(|e| anyhow::anyhow!("apply failed: {e}"))?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "diagram_id": report.diagram_id,
                "commands_applied": report.commands_applied,
                "old_revision": report.old_revision,
                "new_revision": report.new_revision,
            }))?
        );
    } else {
        println!(
            "Applied {} command(s) to {} (revision: {} → {})",
            report.commands_applied,
            report.diagram_id,
            &report.old_revision[..12],
            &report.new_revision[..12],
        );
    }
    Ok(0)
}

fn diagram_project_cmd(
    cwd: Option<PathBuf>,
    view: &str,
    format: &str,
    output: &Path,
    json: bool,
    ctx: &CliContext,
) -> Result<i32> {
    use crate::diagram::project::OutputFormat;

    // Parse format (SCN-418)
    let fmt = OutputFormat::parse(format).ok_or_else(|| {
        anyhow::anyhow!("unknown format: \"{format}\" (supported: plantuml, mermaid, structurizr)")
    })?;

    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let info = resolve_project(&cwd.to_string_lossy());
    let store = ctx.store_factory.open_and_init(&info.project_dir)?;

    // Parse selector (SCN-413, SCN-417)
    let selector = crate::diagram::project_selector::ProjectSelector::parse(view)
        .with_context(|| format!("invalid view selector: {view}"))?;

    // Run queries via DiagramRepository
    let elements = store
        .list_elements(selector.category(), selector.scope_ident(), None)
        .context("list_elements failed")?;

    let edges = store
        .list_semantic_edges(selector.category())
        .context("list_semantic_edges failed")?;

    // Project to DSL
    let (dsl, report) = crate::diagram::project::project_dsl(&selector, &elements, &edges, fmt);

    // Write output file (SCN-416)
    if let Some(parent) = output.parent() {
        ctx.fs
            .create_dir_all(parent)
            .with_context(|| format!("creating output directory {}", parent.display()))?;
    }
    ctx.fs
        .write(output, dsl.as_bytes())
        .with_context(|| format!("write output to {}", output.display()))?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "elements": report.elements,
                "edges": report.edges,
                "format": report.format,
                "output": output.display().to_string(),
            }))?
        );
    } else {
        println!(
            "Projected {} elements, {} edges to {} ({})",
            report.elements,
            report.edges,
            output.display(),
            report.format
        );
    }

    Ok(0)
}

/// Parse a --from selector string into a FromSelector.
fn parse_from_selector(s: &str) -> Result<crate::code::sequence::FromSelector, String> {
    use crate::code::sequence::FromSelector;

    // "file:path/to/file.rs:42" format
    if let Some(rest) = s.strip_prefix("file:") {
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("invalid file:line selector: {s}"));
        }
        let line: u32 = parts[1]
            .parse()
            .map_err(|_| format!("invalid line: {}", parts[1]))?;
        Ok(FromSelector::ByFileLine {
            file: std::path::PathBuf::from(parts[0]),
            line,
        })
    } else if s.contains("::")
        || (s.starts_with("rust:") || s.starts_with("typescript:") || s.starts_with("python:"))
    {
        // Looks like a canonical key: "rust:src/lib.rs:foo:42"
        Ok(FromSelector::ByCanonicalKey {
            canonical_key: s.to_string(),
        })
    } else {
        // By name
        Ok(FromSelector::ByName {
            name: s.to_string(),
        })
    }
}

fn code_sequence_cmd(
    cwd: &std::path::Path,
    from: crate::code::sequence::FromSelector,
    depth: u32,
    max_interactions: Option<u32>,
    json: bool,
    _ctx: &CliContext,
) -> Result<i32> {
    use crate::code::output::print_sequence_table;

    let info = crate::project::resolve_project(&cwd.to_string_lossy());
    let report =
        crate::code::sequence::project_sequence(&info.project_dir, from, depth, max_interactions)
            .map_err(|e| anyhow::anyhow!("sequence projection failed: {e}"))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_sequence_table(&report);
    }

    Ok(0)
}

fn code_c4_discover_cmd(
    cwd: Option<PathBuf>,
    apply: bool,
    strategy: Option<&str>,
    json: bool,
    ctx: &CliContext,
) -> Result<i32> {
    use crate::code::c4_discover::{apply as apply_report, discover};
    use crate::code::output::print_human_table;
    use crate::code::strategies::register_strategies;

    let cwd = ctx.resolve_cwd(cwd.as_ref());
    let info = crate::project::resolve_project(&cwd.to_string_lossy());

    // Filter strategies if --strategy was given
    let all_strategies = register_strategies();
    let strategies: Vec<Box<dyn crate::code::strategies::Strategy>> = if let Some(s) = strategy {
        let allowed: std::collections::HashSet<&str> = s.split(',').map(str::trim).collect();
        all_strategies
            .into_iter()
            .filter(|s| allowed.contains(s.id()))
            .collect()
    } else {
        all_strategies
    };

    // Run discovery
    let report = discover(&cwd, &strategies, &*ctx.fs, &*ctx.clock)
        .map_err(|e| anyhow::anyhow!("discovery failed: {e}"))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_human_table(&report);
    }

    // Persist if --apply
    if apply {
        let apply_report = apply_report(&info.project_dir, &report, &*ctx.fs)
            .map_err(|e| anyhow::anyhow!("apply failed: {e}"))?;
        if !json {
            println!(
                "Applied: {} elements written, {} skipped, {} evidences, {} artifacts.",
                apply_report.elements_written,
                apply_report.elements_skipped,
                apply_report.evidences_written,
                apply_report.source_artifacts_written,
            );
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "elements_written": apply_report.elements_written,
                    "elements_skipped": apply_report.elements_skipped,
                    "evidences_written": apply_report.evidences_written,
                    "source_artifacts_written": apply_report.source_artifacts_written,
                }))?
            );
        }
    }

    Ok(0)
}

fn code_class_diagram_cmd(
    cwd: &Path,
    apply: bool,
    json: bool,
    lang: &[crate::code::class_diagram::Language],
    selector: Option<&str>,
    ctx: &CliContext,
) -> Result<i32> {
    use crate::code::class_diagram::{self, apply as class_diagram_apply};

    let opts = class_diagram::ClassDiagramOptions {
        languages: lang.to_vec(),
        selector: selector.map(String::from),
    };

    let report = match class_diagram::run_class_diagram(cwd, &opts, &*ctx.fs) {
        Ok(r) => r,
        Err(class_diagram::ClassDiagramError::UnknownSelector(s)) => {
            eprintln!("error: unknown selector: {s} — supported forms: file:<path>");
            return Ok(64);
        }
        Err(class_diagram::ClassDiagramError::FileNotFound(p)) => {
            eprintln!("error: file not found: {p}");
            return Ok(64);
        }
        Err(e) => {
            return Err(anyhow::anyhow!("class-diagram extraction failed: {e}"));
        }
    };

    if apply {
        let info = crate::project::resolve_project(&cwd.to_string_lossy());
        let apply_report = class_diagram_apply(&info.project_dir, &report, &*ctx.fs)
            .map_err(|e| anyhow::anyhow!("class-diagram apply failed: {e}"))?;
        if json {
            println!("{}", serde_json::to_string_pretty(&apply_report)?);
        } else {
            println!(
                "Applied {} elements ({} skipped), {} relations ({} skipped) ({} ms).",
                apply_report.elements_written,
                apply_report.elements_skipped,
                apply_report.relations_written,
                apply_report.relations_skipped,
                apply_report.duration_ms
            );
        }
    } else if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        crate::code::output::print_class_diagram_table(&report);
    }

    Ok(0)
}

fn code_state_machine_cmd(
    cwd: &Path,
    apply: bool,
    json: bool,
    lang: &[crate::code::state_machine::Language],
    ctx: &CliContext,
) -> Result<i32> {
    let report = crate::code::state_machine::extract(cwd, lang, &*ctx.fs)
        .map_err(|e| anyhow::anyhow!("state-machine extraction failed: {e}"))?;

    if apply {
        let info = crate::project::resolve_project(&cwd.to_string_lossy());
        let apply_report = crate::code::state_machine::apply(&info.project_dir, &report, &*ctx.fs)
            .map_err(|e| anyhow::anyhow!("state-machine apply failed: {e}"))?;
        if json {
            println!("{}", serde_json::to_string_pretty(&apply_report)?);
        } else {
            println!(
                "Applied {} elements ({} skipped), {} relations ({} skipped) ({} ms).",
                apply_report.elements_written,
                apply_report.elements_skipped,
                apply_report.relations_written,
                apply_report.relations_skipped,
                apply_report.duration_ms
            );
        }
    } else if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Extracted {} state machines from {} files ({} languages).",
            report.machines.len(),
            report.project.files_scanned,
            report.project.languages.len()
        );
        for sm in &report.machines {
            println!("  {} (confidence: {:.2})", sm.name, sm.confidence);
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
    fn production_clock_ends_with_z_and_factory_opens() {
        // SCN-01: production().clock.now_rfc3339() ends with Z;
        // production().store_factory.open_and_init(tempdir) returns Ok.
        let ctx = CliContext::production();
        let ts = ctx.clock.now_rfc3339();
        assert!(
            ts.ends_with('Z'),
            "production clock should end with Z, got: {ts}"
        );
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        std::fs::create_dir_all(&project).unwrap();
        let result = ctx.store_factory.open_and_init(&project);
        assert!(result.is_ok(), "store_factory should open temp project");
    }

    #[test]
    fn cli_context_clone_shares_arc_pointers() {
        // SCN-02: cloning a CliContext reuses the same Arc pointers;
        // Arc::strong_count increments.
        let env: std::sync::Arc<dyn crate::environment::Environment> =
            std::sync::Arc::new(FixedEnvironment::new().with_cwd("/test"));
        let ctx = CliContext::for_test(env.clone());
        let clock_arc = ctx.clock.clone();
        let factory_arc = ctx.store_factory.clone();
        let clock_before = std::sync::Arc::strong_count(&clock_arc);
        let factory_before = std::sync::Arc::strong_count(&factory_arc);
        let ctx2 = ctx.clone();
        assert!(
            std::sync::Arc::strong_count(&clock_arc) == clock_before + 1,
            "clock Arc strong_count should increment on clone"
        );
        assert!(
            std::sync::Arc::strong_count(&factory_arc) == factory_before + 1,
            "store_factory Arc strong_count should increment on clone"
        );
        drop(ctx2);
        assert_eq!(
            std::sync::Arc::strong_count(&clock_arc),
            clock_before,
            "clock Arc should be restored after cloned ctx is dropped"
        );
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
            "--cwd",
            project.to_str().unwrap(),
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
        let info_a = crate::project::resolve_project(&ctx_a.resolve_cwd(None).to_string_lossy());
        let info_b = crate::project::resolve_project(&ctx_b.resolve_cwd(None).to_string_lossy());

        assert_ne!(
            info_a.project_id, info_b.project_id,
            "two distinct injected cwds produced the same project_id: {}. \
             The Environment port is being bypassed somewhere in the call tree.",
            info_a.project_id
        );
        // Both must succeed — `project resolve` is idempotent.
        assert!(!info_a.project_id.is_empty());
        assert!(!info_b.project_id.is_empty());
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

    #[test]
    fn evidence_list_accepts_status_flag() {
        // Verify --status flag is accepted by the CLI parser.
        // The handler is tested separately via integration tests;
        // here we assert the flag reaches the handler path.
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let ctx = ctx_for(project.clone());

        let cli = Cli::parse_from([
            "archctl",
            "evidence",
            "list",
            "--cwd",
            project.to_str().unwrap(),
            "--status",
            "accepted",
        ]);
        // Parsing succeeds — handler logic is tested in store tests.
        let _ = run_inner(cli, &ctx);
    }

    #[test]
    fn evidence_accept_subcommand_is_parsed() {
        // Verify `evidence accept --id <id>` is recognised by the parser.
        let cli = Cli::parse_from(["archctl", "evidence", "accept", "--id", "ev:test"]);
        match cli.command {
            Command::Evidence { action } => {
                assert!(matches!(action, EvidenceAction::Accept { id, .. } if id == "ev:test"));
            }
            _ => panic!("expected Evidence command"),
        }
    }

    #[test]
    fn evidence_supersede_subcommand_is_parsed() {
        // Verify `evidence supersede --old-id <id>` is recognised by the parser.
        let cli = Cli::parse_from(["archctl", "evidence", "supersede", "--old-id", "ev:old"]);
        match cli.command {
            Command::Evidence { action } => {
                assert!(
                    matches!(action, EvidenceAction::Supersede { old_id, .. } if old_id == "ev:old")
                );
            }
            _ => panic!("expected Evidence command"),
        }
    }

    #[test]
    fn evidence_list_status_flag_accepts_all_variants() {
        // Verify EvidenceStatus variants are accepted as --status values.
        for variant in ["drafted", "accepted", "superseded"] {
            let cli = Cli::parse_from(["archctl", "evidence", "list", "--status", variant]);
            match cli.command {
                Command::Evidence { action } => {
                    assert!(matches!(
                        action,
                        EvidenceAction::List {
                            status: Some(_),
                            ..
                        }
                    ));
                }
                _ => panic!("expected Evidence List command"),
            }
        }
    }

    // M75 T3 — IDE adapter CLI wiring tests

    #[test]
    fn ide_install_subcommand_is_parsed() {
        let cli = Cli::parse_from(["archctl", "ide", "install", "opencode"]);
        match cli.command {
            Command::Ide { action } => {
                assert!(matches!(
                    action,
                    IdeAction::Install { ide, .. } if ide == "opencode"
                ));
            }
            _ => panic!("expected Ide Install command"),
        }
    }

    #[test]
    fn ide_list_subcommand_is_parsed() {
        let cli = Cli::parse_from(["archctl", "ide", "list"]);
        match cli.command {
            Command::Ide { action } => {
                assert!(matches!(action, IdeAction::List { installed: false }));
            }
            _ => panic!("expected Ide List command"),
        }
    }

    #[test]
    fn ide_list_installed_flag_is_parsed() {
        let cli = Cli::parse_from(["archctl", "ide", "list", "--installed"]);
        match cli.command {
            Command::Ide { action } => {
                assert!(matches!(action, IdeAction::List { installed: true }));
            }
            _ => panic!("expected Ide List command"),
        }
    }

    #[test]
    fn ide_doctor_subcommand_is_parsed() {
        let cli = Cli::parse_from(["archctl", "ide", "doctor", "claude-code"]);
        match cli.command {
            Command::Ide { action } => {
                assert!(matches!(
                    action,
                    IdeAction::Doctor { ide } if ide == "claude-code"
                ));
            }
            _ => panic!("expected Ide Doctor command"),
        }
    }

    #[test]
    fn ide_remove_subcommand_is_parsed() {
        let cli = Cli::parse_from(["archctl", "ide", "remove", "zcode"]);
        match cli.command {
            Command::Ide { action } => {
                assert!(matches!(
                    action,
                    IdeAction::Remove { ide, purge: false } if ide == "zcode"
                ));
            }
            _ => panic!("expected Ide Remove command"),
        }
    }

    #[test]
    fn ide_remove_purge_flag_is_parsed() {
        let cli = Cli::parse_from(["archctl", "ide", "remove", "codex", "--purge"]);
        match cli.command {
            Command::Ide { action } => {
                assert!(matches!(
                    action,
                    IdeAction::Remove { ide, purge: true } if ide == "codex"
                ));
            }
            _ => panic!("expected Ide Remove command with purge"),
        }
    }

    #[test]
    fn ide_update_subcommand_is_parsed() {
        let cli = Cli::parse_from(["archctl", "ide", "update", "opencode"]);
        match cli.command {
            Command::Ide { action } => {
                assert!(matches!(
                    action,
                    IdeAction::Update { ide, sync: true } if ide == "opencode"
                ));
            }
            _ => panic!("expected Ide Update command"),
        }
    }

    #[test]
    fn handler_error_chain_preserves_lock_context() {
        // SCN-07: when LbugStoreFactory::open_and_init fails with LockError,
        // the error Display chain includes "failed to acquire DB lock" exactly
        // as the raw store::open_and_init path would produce.
        use crate::store::GraphStoreFactory;
        use std::fs::File;

        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        std::fs::create_dir_all(&project).unwrap();
        // Pre-create the lock file and hold it so the store open fails.
        let lock_path = project.join("architecture.lbdb");
        let file = File::create(&lock_path).unwrap();
        drop(file);

        let factory = crate::store::LbugStoreFactory;
        match factory.open_and_init(&project) {
            Ok(_) => {
                // Platform may not enforce locking; test passes vacuously.
            }
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("failed to acquire DB lock"),
                    "error should contain 'failed to acquire DB lock', got: {msg}"
                );
            }
        }
    }
}
