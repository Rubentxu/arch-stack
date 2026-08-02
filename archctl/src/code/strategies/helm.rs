//! S6: Helm chart detection.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::code::c4_discover::{ContainerCandidate, Evidence, EvidenceKind};
use crate::code::strategies::Strategy;
use crate::filesystem::Filesystem;

const CHART_ROOTS: &[&str] = &["charts", "helm", "k8s/charts"];
const EXCLUDED_PATH_PREFIXES: &[&str] = &["examples/", "docs/", "tools/"];

#[derive(Debug, Deserialize)]
struct ChartYaml {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
}

pub struct HelmCharts;

impl Strategy for HelmCharts {
    fn id(&self) -> &'static str {
        "helm"
    }
    fn confidence(&self) -> f64 {
        0.70
    }

    fn detect(&self, project_root: &Path, fs: &dyn Filesystem) -> Result<Vec<ContainerCandidate>> {
        let mut candidates = Vec::new();
        for root_str in CHART_ROOTS {
            let root = project_root.join(root_str);
            if !root.is_dir() {
                continue;
            }

            for entry in
                std::fs::read_dir(&root).with_context(|| format!("read_dir {}", root.display()))?
            {
                let entry = entry?;
                if !entry.file_type()?.is_dir() {
                    continue;
                }
                let chart_yaml = entry.path().join("Chart.yaml");
                if !chart_yaml.is_file() {
                    continue;
                }
                let rel_chart = chart_yaml
                    .strip_prefix(project_root)
                    .unwrap_or(&chart_yaml)
                    .to_string_lossy()
                    .replace('\\', "/");
                if EXCLUDED_PATH_PREFIXES
                    .iter()
                    .any(|p| rel_chart.starts_with(p))
                {
                    continue;
                }

                let text = fs.read_to_string(&chart_yaml)?;
                let chart: ChartYaml = serde_yaml::from_str(&text)
                    .with_context(|| format!("parse {}", chart_yaml.display()))?;
                let name = chart
                    .name
                    .clone()
                    .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());
                let confidence = if chart.name.is_some() {
                    self.confidence()
                } else {
                    0.50
                };

                candidates.push(ContainerCandidate {
                    canonical_key: name.clone(),
                    name,
                    strategy: self.id().to_string(),
                    confidence,
                    evidences: vec![Evidence {
                        file: rel_chart,
                        line: 1,
                        kind: EvidenceKind::Config,
                        text: format!("Helm chart: {}", chart.version.unwrap_or_default()),
                    }],
                });
            }
        }
        Ok(candidates)
    }
}
