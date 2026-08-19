//! Diagram project — graph-to-DSL projection.
//!
//! `archctl diagram project` reads the graph via `query_elements`/`query_semantic_edges`,
//! then projects to PlantUML, Mermaid, or Structurizr DSL. Deterministic output
//! (sorted by canonical_key).

pub mod mermaid;
pub mod plantuml;
pub mod structurizr;

use crate::diagram::project_selector::ProjectSelector;
use crate::graph::{ElementRow, SemanticEdgeRow};

/// Output format for diagram project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Plantuml,
    Mermaid,
    Structurizr,
}

impl OutputFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "plantuml" => Some(OutputFormat::Plantuml),
            "mermaid" => Some(OutputFormat::Mermaid),
            "structurizr" => Some(OutputFormat::Structurizr),
            _ => None,
        }
    }
}

/// Result of a project run.
#[derive(Debug)]
pub struct ProjectReport {
    pub elements: usize,
    pub edges: usize,
    pub format: String,
}

impl ProjectReport {
    pub fn new(elements: usize, edges: usize, format: OutputFormat) -> Self {
        Self {
            elements,
            edges,
            format: format!("{:?}", format).to_lowercase(),
        }
    }
}

/// Project elements + edges to the given DSL format.
/// Returns the DSL string and a report.
pub fn project_dsl(
    selector: &ProjectSelector,
    elements: &[ElementRow],
    edges: &[SemanticEdgeRow],
    format: OutputFormat,
) -> (String, ProjectReport) {
    let elements_sorted = {
        let mut e = elements.to_vec();
        e.sort_by(|a, b| a.canonical_key.cmp(&b.canonical_key));
        e
    };

    let edges_sorted = {
        let mut e = edges.to_vec();
        e.sort_by(|a, b| a.relation_id.cmp(&b.relation_id));
        e
    };

    let dsl = match format {
        OutputFormat::Plantuml => plantuml::project(&elements_sorted, &edges_sorted, selector),
        OutputFormat::Mermaid => mermaid::project(&elements_sorted, &edges_sorted, selector),
        OutputFormat::Structurizr => {
            structurizr::project(&elements_sorted, &edges_sorted, selector)
        }
    };

    (
        dsl,
        ProjectReport::new(elements_sorted.len(), edges_sorted.len(), format),
    )
}
