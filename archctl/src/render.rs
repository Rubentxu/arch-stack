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
use tracing::info;

pub mod mermaid;
pub mod plantuml;
pub mod structurizr;

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
        RenderKind::Plantuml => plantuml::render(&body)
            .with_context(|| format!("plantuml render of {}", source.display()))?,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `RenderKind::as_str` returns the canonical lowercase id for each
    /// variant. Locks the wire format consumed by `detect_format` and
    /// the JSON `"format"` payload in `run`.
    #[test]
    fn render_kind_as_str_returns_canonical_ids() {
        assert_eq!(RenderKind::Structurizr.as_str(), "structurizr");
        assert_eq!(RenderKind::Plantuml.as_str(), "plantuml");
        assert_eq!(RenderKind::Mermaid.as_str(), "mermaid");
    }

    /// `RenderKind` derives `PartialEq`, `Eq`, `Copy`, `Clone`, `Debug`.
    /// Locks the basic trait contracts for use in match expressions.
    #[test]
    fn render_kind_basic_derive_traits() {
        let a = RenderKind::Structurizr;
        let b = a; // Copy
        assert_eq!(a, b);
        let cloned = a; // Copy (Clone is automatic)
        assert_eq!(a, cloned);

        // Debug includes variant name
        let dbg = format!("{a:?}");
        assert!(dbg.contains("Structurizr"));
    }

    /// `detect_format` maps `.puml`, `.iuml`, `.wsd` → PlantUML.
    /// Locks the extension detection contract.
    #[test]
    fn detect_format_maps_plantuml_extensions() {
        for ext in ["puml", "iuml", "wsd"] {
            let path = PathBuf::from(format!("diagram.{ext}"));
            assert_eq!(
                detect_format(&path),
                RenderKind::Plantuml,
                "extension .{ext} must map to Plantuml"
            );
        }
    }

    /// `detect_format` maps `.mmd` → Mermaid.
    #[test]
    fn detect_format_maps_mermaid_extension() {
        let path = PathBuf::from("diagram.mmd");
        assert_eq!(detect_format(&path), RenderKind::Mermaid);
    }

    /// `detect_format` is case-insensitive on the extension (via
    /// `to_ascii_lowercase`). `.PUML`, `.Puml`, `.pUmL` all → Plantuml.
    #[test]
    fn detect_format_is_case_insensitive_on_extension() {
        for ext in ["PUML", "Puml", "pUmL", "IUMl", "WSd", "MMD", "MmD"] {
            let path = PathBuf::from(format!("diagram.{ext}"));
            let expected = match ext.to_ascii_lowercase().as_str() {
                "puml" | "iuml" | "wsd" => RenderKind::Plantuml,
                "mmd" => RenderKind::Mermaid,
                _ => RenderKind::Structurizr,
            };
            assert_eq!(
                detect_format(&path),
                expected,
                "extension .{ext} (case test) must match case-insensitively"
            );
        }
    }

    /// `detect_format` defaults to Structurizr for unknown extensions
    /// (`.dsl`, no extension, `.txt`, etc.). Locks the fallthrough.
    #[test]
    fn detect_format_defaults_to_structurizr_for_unknown_extensions() {
        for path_str in ["diagram.dsl", "diagram.txt", "diagram", "diagram.json"] {
            let path = PathBuf::from(path_str);
            assert_eq!(
                detect_format(&path),
                RenderKind::Structurizr,
                "unknown extension '{path_str}' must default to Structurizr"
            );
        }
    }
}
