use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::astgrep::Lang;
use crate::evidence::{self, EvidenceKind};
use crate::project::resolve_project;
use crate::skills;
use crate::{doctor, graph, inventory, render, store};

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
    Doctor,
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

pub fn run(cli: Cli) -> Result<i32> {
    match cli.command {
        Command::Doctor => doctor::run(),
        Command::Project { action } => match action {
            ProjectAction::Resolve { cwd, json } => resolve_project_cmd(cwd, json),
        },
        Command::Graph { action } => match action {
            GraphAction::Init { cwd, json } => graph_init_cmd(cwd, json),
            GraphAction::Stat { cwd, json } => graph_stat_cmd(cwd, json),
            GraphAction::Query { cwd, cypher, json } => graph_query_cmd(cwd, &cypher, json),
            GraphAction::Neighbours { cwd, id, depth, json } => {
                graph_neighbours_cmd(cwd, &id, depth, json)
            }
        },
        Command::Inventory { action } => match action {
            InventoryAction::Tree { cwd, max_depth, max_entries, json } => {
                inventory_tree_cmd(cwd, max_depth, max_entries, json)
            }
            InventoryAction::Languages { cwd, max_depth, max_entries, json } => {
                inventory_languages_cmd(cwd, max_depth, max_entries, json)
            }
            InventoryAction::Depends { cwd, manifest, json } => {
                inventory_depends_cmd(cwd, manifest, json)
            }
        },
        Command::Evidence { action } => match action {
            EvidenceAction::Extract { cwd, lang, pattern, claim, kind, json, put } => {
                evidence_extract_cmd(cwd, lang, &pattern, &claim, kind, json, put)
            }
            EvidenceAction::List { cwd, path, json } => evidence_list_cmd(cwd, path, json),
        },
        Command::Render { source, format, out, kroki_url } => {
            render::run(source, format, out, &kroki_url).context("render failed")
        }
        Command::Skills { action } => skills::run(action).context("skills failed"),
    }
}

fn resolve_project_cmd(cwd: Option<PathBuf>, json: bool) -> Result<i32> {
    let cwd = cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
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

fn graph_init_cmd(cwd: Option<PathBuf>, json: bool) -> Result<i32> {
    let cwd = cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
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

fn graph_stat_cmd(cwd: Option<PathBuf>, json: bool) -> Result<i32> {
    let cwd = cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
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

fn graph_query_cmd(cwd: Option<PathBuf>, cypher: &str, json: bool) -> Result<i32> {
    let cwd = cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let info = resolve_project(&cwd.to_string_lossy());
    let mut store = store::open_default(&info.project_dir).context("open graph store")?;
    store.init().context("graph init (query prerequisite)")?;
    let rows = store.query(cypher).context("graph query")?;
    if json || rows.is_empty() {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        for row in &rows {
            println!("{}", serde_json::to_string(row)?);
        }
    }
    Ok(0)
}

fn graph_neighbours_cmd(cwd: Option<PathBuf>, id: &str, depth: u8, json: bool) -> Result<i32> {
    use tracing::warn;
    let cwd = cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
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
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        for row in &rows {
            let id = row.get("id").and_then(json_string).unwrap_or("?");
            let kinds = row
                .get("kinds")
                .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "<>".into()))
                .unwrap_or_else(|| "<>".into());
            println!("{id}\t{kinds}");
        }
    }
    Ok(0)
}

fn json_string(v: &serde_json::Value) -> Option<&str> {
    v.as_str()
}

fn inventory_tree_cmd(
    cwd: Option<PathBuf>,
    max_depth: Option<usize>,
    max_entries: usize,
    json: bool,
) -> Result<i32> {
    let cwd = cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
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
) -> Result<i32> {
    let cwd = cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
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
) -> Result<i32> {
    let manifest_path = manifest.map(|p| {
        if p.is_relative() {
            let base = cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
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
) -> Result<i32> {
    let cwd = cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    // CLI is the production entry point — always uses SystemClock.
    // The Clock port lets tests inject deterministic timestamps via
    // FixedClock; the CLI does not need that.
    let clock: &dyn crate::clock::Clock = &crate::clock::SystemClock;
    let result = evidence::extract(&cwd, lang, pattern, claim, kind, clock)?;
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

fn evidence_list_cmd(cwd: Option<PathBuf>, path: Option<String>, json: bool) -> Result<i32> {
    let cwd = cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
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
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        for row in &rows {
            println!(
                "{id}\t{kind}\t{path}:{sl}-{el}\t{claim}",
                id = row.get("e.id").and_then(json_string).unwrap_or("?"),
                kind = row.get("e.kind").and_then(json_string).unwrap_or("?"),
                path = row.get("e.path").and_then(json_string).unwrap_or("?"),
                sl = row.get("e.start_line").map(|v| v.to_string()).unwrap_or_default(),
                el = row.get("e.end_line").map(|v| v.to_string()).unwrap_or_default(),
                claim = row.get("e.claim").and_then(json_string).unwrap_or(""),
            );
        }
    }
    Ok(0)
}
