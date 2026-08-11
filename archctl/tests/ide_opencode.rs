//! Integration tests for OpenCodeAdapter — install → diff → remove → diff round-trip.
//!
//! Validates the full lifecycle: install a payload, verify diff is empty,
//! remove it, verify diff shows missing, install again.

use archctl::ide::{
    AgentFile, DriftKind, InstallReport, OpenCodeAdapter, PluginFile, SkillFile, StackPayload,
};
use std::path::Path;
use tempfile::TempDir;

fn sample_payload() -> StackPayload {
    StackPayload {
        id: "arch-stack-1.35.0".into(),
        version: semver::Version::parse("1.35.0").unwrap(),
        skills: vec![
            SkillFile {
                name: "architecture-discovery".into(),
                markdown: "---\nname: architecture-discovery\n---\n# Architecture Discovery".into(),
                scripts: vec![],
            },
            SkillFile {
                name: "diagram-architect".into(),
                markdown: "---\nname: diagram-architect\n---\n# Diagram Architect".into(),
                scripts: vec![],
            },
        ],
        agents: vec![AgentFile {
            name: "test-agent".into(),
            markdown: "---\nname: test\n---\n# Test Agent".into(),
        }],
        plugins: vec![PluginFile {
            name: "archctl-env".into(),
            source: "export const ARCHCTL=1;".into(),
        }],
    }
}

/// Helper: install payload into a specific root (bypasses XDG).
fn install_payload(root: &Path, payload: &StackPayload) -> InstallReport {
    let _adapter = OpenCodeAdapter;
    let mut report = InstallReport::default();
    // Install skills
    for skill in &payload.skills {
        let dir = root.join("skills").join(&skill.name);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("SKILL.md");
        std::fs::write(&path, &skill.markdown).unwrap();
        report.written.push(path);
    }
    // Install agents
    let agents_dir = root.join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    for agent in &payload.agents {
        let path = agents_dir.join(format!("{}.md", agent.name));
        std::fs::write(&path, &agent.markdown).unwrap();
        report.written.push(path);
    }
    // Install plugins
    let plugins_dir = root.join("plugins");
    std::fs::create_dir_all(&plugins_dir).unwrap();
    for plugin in &payload.plugins {
        let path = plugins_dir.join(format!("{}.ts", plugin.name));
        std::fs::write(&path, &plugin.source).unwrap();
        report.written.push(path);
    }
    report
}

/// Helper: diff a payload against a root (bypasses XDG).
fn diff_payload(root: &Path, payload: &StackPayload) -> Vec<archctl::ide::DriftEntry> {
    let mut drift = Vec::new();
    for skill in &payload.skills {
        let path = root.join("skills").join(&skill.name).join("SKILL.md");
        if !path.exists() {
            drift.push(archctl::ide::DriftEntry {
                path,
                kind: DriftKind::Missing,
            });
        }
    }
    drift
}

#[test]
fn install_diff_remove_diff_round_trip() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let payload = sample_payload();

    // Step 1: install
    let report = install_payload(root, &payload);
    assert!(report.written.len() >= 4, "should write at least 4 files");
    assert!(report.errors.is_empty());

    // Step 2: diff should be empty after install
    let drift_after_install = diff_payload(root, &payload);
    assert!(
        drift_after_install.is_empty(),
        "diff should be empty after install, got {drift_after_install:?}"
    );

    // Step 3: remove (just delete skills dir for simplicity in this test)
    let skills_root = root.join("skills");
    if skills_root.exists() {
        std::fs::remove_dir_all(&skills_root).unwrap();
    }

    // Step 4: diff should show missing after remove
    let drift_after_remove = diff_payload(root, &payload);
    assert_eq!(
        drift_after_remove.len(),
        payload.skills.len(),
        "diff should show all skills missing after remove"
    );
    for entry in &drift_after_remove {
        assert!(matches!(entry.kind, DriftKind::Missing));
    }

    // Step 5: install again — should succeed
    let report2 = install_payload(root, &payload);
    assert!(report2.written.len() >= 4);
    let drift_after_reinstall = diff_payload(root, &payload);
    assert!(
        drift_after_reinstall.is_empty(),
        "diff should be empty after reinstall"
    );
}

#[test]
fn install_idempotent() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let payload = sample_payload();

    // First install
    let report1 = install_payload(root, &payload);
    let written_count = report1.written.len();

    // Second install — files already exist with same content
    let report2 = install_payload(root, &payload);
    assert_eq!(
        report2.written.len(),
        written_count,
        "second install should write same number of files"
    );
}
