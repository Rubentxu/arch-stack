//! Mermaid → SVG renderer (M38).
//!
//! Uses the `merman` Rust crate (pure Rust, no graphviz, no network) to
//! parse + layout + render Mermaid source to SVG. Supports the full
//! Mermaid subset: sequenceDiagram, flowchart, classDiagram, stateDiagram,
//! ER diagram, etc.
//!
//! Pipeline:
//!   1. `Engine::new()` + `parse_diagram_sync(text, ParseOptions::default())`
//!      → ParsedDiagram
//!   2. `merman_render::layout_parsed(parsed, options)` → LayoutedDiagram
//!   3. `merman_render::svg::parity::render_layouted_svg(layouted, measurer, options)`
//!      → String (SVG)
//!
//! Text measurement uses merman's built-in `DeterministicTextMeasurer`
//! which doesn't require system font libraries.

use anyhow::{Context, Result};

pub fn render(source: &str) -> Result<String> {
    let engine = merman_core::Engine::new();
    let parsed = engine
        .parse_diagram_sync(source, merman_core::ParseOptions::default())
        .context("mermaid parse failed")?
        .context("mermaid parse returned no diagram")?;

    let layout_options = merman_render::LayoutOptions::headless_svg_defaults();
    let layouted =
        merman_render::layout_parsed(&parsed, &layout_options).context("mermaid layout failed")?;

    let svg_options = merman_render::svg::SvgRenderOptions::default();
    let measurer = merman_render::text::DeterministicTextMeasurer::default();
    let svg = merman_render::svg::render_layouted_svg(&layouted, &measurer, &svg_options)
        .context("mermaid SVG render failed")?;
    Ok(svg)
}
