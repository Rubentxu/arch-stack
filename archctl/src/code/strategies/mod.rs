//! Strategy trait + registry for code::c4_discover inference strategies.

use std::path::Path;

use anyhow::Result;

use crate::code::c4_discover::ContainerCandidate;
use crate::filesystem::Filesystem;

pub mod cargo;
pub mod components;
pub mod dockerfile;
pub mod helm;
pub mod npm;
pub mod npm_single;

/// A C4 Container inference strategy. Each strategy detects Containers
/// in the project tree using a single, focused signal (Cargo workspaces,
/// npm workspaces, Dockerfile, Helm, etc.). Strategies are stateless and
/// pure: same project_root + same Filesystem → same ContainerCandidate[].
pub trait Strategy: Send + Sync {
    /// Stable id used for filtering (`--strategy <id>`) and reporting.
    fn id(&self) -> &'static str;

    /// Confidence score (0.0-1.0) emitted by this strategy for every
    /// Container it detects. Per-strategy hard-coded (D2).
    fn confidence(&self) -> f64;

    /// MetaType id produced by this strategy (e.g. "mt.container", "mt.component").
    /// Used to route the apply path and link Element→MetaType edges.
    fn metatype(&self) -> &'static str;

    /// Walk the project tree and emit Container candidates.
    /// Errors are captured by the caller into DiscoverReport.errors[],
    /// not propagated (graceful degradation per SCN-103).
    fn detect(&self, project_root: &Path, fs: &dyn Filesystem) -> Result<Vec<ContainerCandidate>>;
}

/// Build the default set of MVP strategies: Cargo workspace, npm
/// workspace, npm single-package, Dockerfile per service, Helm charts.
/// The order is the display order in the human table output.
pub fn register_strategies() -> Vec<Box<dyn Strategy>> {
    vec![
        Box::new(crate::code::strategies::cargo::CargoWorkspace),
        Box::new(crate::code::strategies::npm::NpmWorkspace),
        Box::new(crate::code::strategies::npm_single::NpmSinglePackage),
        Box::new(crate::code::strategies::dockerfile::DockerfilePerService),
        Box::new(crate::code::strategies::helm::HelmCharts),
        Box::new(crate::code::strategies::components::ComponentsStrategy),
    ]
}
