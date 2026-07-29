use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::project::resolve_project;
use crate::skills;
use crate::{doctor, render};

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

#[derive(Debug, Parser)]
#[command(name = "archctl", version, about = "OpenCode Architecture Diagrammer sidecar CLI (M1)")]
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
