use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::project::resolve_project;
use crate::skills;
use crate::{doctor, graph, render};

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
#[command(name = "archctl", version, about = "OpenCode Architecture Diagrammer sidecar CLI (M2)")]
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
    let path = graph::init(&info.project_dir).context("graph init")?;
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
    let stat = graph::stat(&info.project_dir).context("graph stat")?;
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
    let rows = graph::query(&info.project_dir, cypher).context("graph query")?;
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
    let cwd = cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let info = resolve_project(&cwd.to_string_lossy());
    let rows = graph::neighbours(&info.project_dir, id, depth).context("graph neighbours")?;
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
