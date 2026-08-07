//! Architecture diagram renderer.
//!
//! Implements `archctl render <source> [--format FMT] [--out DIR]`. Renders
//! an architecture diagram to local SVG without any network egress.
//!
//! Per ADR-011, renderers run **locally only**:
//! - **Structurizr** DSL → SVG via a minimal `petgraph + svg` renderer that
//!   parses a C4-shaped subset (`workspace { … } model { … } views { … }`).
//! - **Mermaid** → SVG via `merman` (Rust crate, pure Rust, no graphviz
//!   needed; covers sequence/flowchart/class/state/ER/etc).
//! - **PlantUML** → SVG (deferred: would require either vendor-graphviz
//!   strategy or `graphviz-anywhere` prebuilt binaries; tracked in M40).
//!
//! **No HTTP egress.** The previous remote-renderer POST path is removed.
//! The remote-URL CLI flag is removed (security fix per
//! `docs/audits/2026-08-01-archctl-adr-vs-impl.md` §F1).

use crate::Filesystem;
use crate::cli::RenderFormat;
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

mod mermaid;
mod structurizr;

/// Format identifier emitted by `detect_format` and consumed by `run`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderKind {
    Structurizr,
    Plantuml,
    Mermaid,
}

impl RenderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RenderKind::Structurizr => "structurizr",
            RenderKind::Plantuml => "plantuml",
            RenderKind::Mermaid => "mermaid",
        }
    }
}

pub fn run(
    source: PathBuf,
    format: RenderFormat,
    out: Option<PathBuf>,
    fs: &dyn Filesystem,
) -> Result<i32> {
    if !fs.exists(&source) {
        bail!("source not found: {}", source.display());
    }
    let kind = match format {
        RenderFormat::Auto => detect_format(&source),
        RenderFormat::Structurizr => RenderKind::Structurizr,
        RenderFormat::Plantuml => RenderKind::Plantuml,
        RenderFormat::Mermaid => RenderKind::Mermaid,
    };

    let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    let out_dir = out.unwrap_or_else(|| {
        source
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".archctl-rendered")
    });
    fs.create_dir_all(&out_dir)
        .with_context(|| format!("mkdir {}", out_dir.display()))?;
    let out_path = out_dir.join(format!("{stem}.svg"));

    let body = fs
        .read_to_string(&source)
        .with_context(|| format!("read {}", source.display()))?;
    info!(source = %source.display(), format = kind.as_str(), "rendering");

    let svg = match kind {
        RenderKind::Structurizr => structurizr::render(&body)
            .with_context(|| format!("structurizr render of {}", source.display()))?,
        RenderKind::Plantuml => {
            warn!("plantuml format is deferred to M40 (graphviz vendor strategy unresolved)");
            bail!(
                "plantuml rendering deferred to M40 — see ROADMAP M38 status note \
                 (graphviz-anywhere vendor OR pure-Rust subset renderer needed)"
            );
        }
        RenderKind::Mermaid => mermaid::render(&body)
            .with_context(|| format!("mermaid render of {}", source.display()))?,
    };

    fs.write(&out_path, svg.as_bytes())
        .with_context(|| format!("write {}", out_path.display()))?;

    let payload = serde_json::json!({
        "ok": true,
        "format": kind.as_str(),
        "source": source.display().to_string(),
        "output": out_path.display().to_string(),
        "bytes": svg.len(),
    });
    println!("{payload}");
    Ok(0)
}

pub fn detect_format(source: &Path) -> RenderKind {
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "puml" | "iuml" | "wsd" => RenderKind::Plantuml,
        "mmd" => RenderKind::Mermaid,
        // Default: anything else (including `.dsl` and absence of
        // extension) is treated as Structurizr DSL.
        _ => RenderKind::Structurizr,
    }
}
