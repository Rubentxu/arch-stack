//! Integration tests for the `archctl architecture diff` command.

use std::fs;
use tempfile::TempDir;

use archctl::architecture::create;
use archctl::architecture::diff::ArchitectureDiffReport;
use archctl::filesystem::SystemFilesystem;
use archctl::store::{GraphStore, LbugStore};

/// Helper: open a LbugStore at the given project directory.
fn open_store(project_dir: &std::path::Path) -> LbugStore {
    let mut store = LbugStore::open(project_dir).expect("store must open");
    store.init().expect("store must init");
    store
}

/// Helper: create a minimal git repo in a temp directory using git CLI.
fn create_test_git_repo() -> TempDir {
    let tmp = TempDir::new().expect("temp dir");
    let repo_path = tmp.path();

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .expect("git init must succeed");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .expect("git config email must succeed");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(repo_path)
        .output()
        .expect("git config name must succeed");

    fs::write(repo_path.join("file.txt"), "test content\n").expect("write test file");

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("git add must succeed");

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("git commit must succeed");

    tmp
}

/// Return the path to the `archctl` binary under test.
fn archctl_bin() -> std::path::PathBuf {
    std::env::current_exe()
        .expect("current exe must be available")
        .parent()
        .expect("exe must have a parent dir")
        .parent()
        .expect("exe must have a grandparent dir")
        .join("archctl")
}

// ─── JSON round-trip ─────────────────────────────────────────────────────────

#[test]
fn json_roundtrip_emits_architecture_diff_report_1() {
    // Create two snapshots in two different git repos (each with one commit).
    // Using separate repos ensures each has a stable project_id (HEAD = first_commit).
    let _project_tmp = TempDir::new().expect("project temp dir");
    let git_tmp_a = create_test_git_repo();
    let git_tmp_b = create_test_git_repo();
    let fs = SystemFilesystem;

    let git_path_a = git_tmp_a.path().to_string_lossy();
    let git_path_b = git_tmp_b.path().to_string_lossy();

    // Compute XDG project directories (one per git repo).
    let xdg_dir_a = archctl::project::resolve_project(&git_path_a).project_dir;
    let xdg_dir_b = archctl::project::resolve_project(&git_path_b).project_dir;

    // Snapshot A in git repo A.
    let (snap_a_id, _seq_a) = create(
        &xdg_dir_a,
        &git_path_a,
        &fs,
        "architecture",
        1,
        Some("snap-a"),
        false,
        None,
    )
    .expect("first snapshot must be created");

    // Snapshot B in git repo B (unused in this test, but created to verify API works).
    let (_snap_b_id, _seq_b) = create(
        &xdg_dir_b,
        &git_path_b,
        &fs,
        "architecture",
        1,
        Some("snap-b"),
        false,
        None,
    )
    .expect("second snapshot must be created");

    // Run CLI with --cwd git_repo_A so it opens store at xdg_dir_a.
    // Diff snapshot A against itself to test --json output format.
    let output = std::process::Command::new(archctl_bin())
        .args(["architecture", "diff", &snap_a_id, &snap_a_id, "--json"])
        .arg("--cwd")
        .arg(git_path_a.as_ref())
        .current_dir(git_tmp_a.path())
        .output()
        .expect("diff --json must succeed");

    assert_eq!(
        output.status.code(),
        Some(0),
        "diff should exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json_str = String::from_utf8_lossy(&output.stdout);
    let report: ArchitectureDiffReport =
        serde_json::from_str(&json_str).expect("stdout must be valid JSON");

    assert_eq!(report.schema_version, "1.0");
    assert_eq!(report.capability, "architecture-diff-mvp");
    assert!(
        report.differences.is_empty(),
        "identical snapshot diff must have empty differences"
    );
}

// ─── Identical snapshots ─────────────────────────────────────────────────────

#[test]
fn identical_snapshots_yield_empty_differences() {
    let _project_tmp = TempDir::new().expect("project temp dir");
    let git_tmp = create_test_git_repo();
    let fs = SystemFilesystem;

    let git_path = git_tmp.path().to_string_lossy();
    let xdg_project_dir = archctl::project::resolve_project(&git_path).project_dir;

    let (snap_id, _seq) = create(
        &xdg_project_dir,
        &git_path,
        &fs,
        "architecture",
        1,
        Some("only"),
        false,
        None,
    )
    .expect("snapshot must be created");

    let output = std::process::Command::new(archctl_bin())
        .args(["architecture", "diff", &snap_id, &snap_id])
        .arg("--cwd")
        .arg(git_path.as_ref())
        .current_dir(git_tmp.path())
        .output()
        .expect("diff identical must succeed");

    assert_eq!(
        output.status.code(),
        Some(0),
        "diff identical should exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No differences found"),
        "identical snapshots must show no differences: {stdout}"
    );
}

// ─── Invalid id ─────────────────────────────────────────────────────────────

#[test]
fn invalid_snapshot_id_rejected() {
    let git_tmp = create_test_git_repo();

    // A snapshot id that fails validate_identifier (has spaces).
    let output = std::process::Command::new(archctl_bin())
        .args(["architecture", "diff", "bad id with spaces", "snap-abc123"])
        .arg("--cwd")
        .arg(git_tmp.path().to_string_lossy().as_ref())
        .current_dir(git_tmp.path())
        .output()
        .expect("diff with bad id must run");

    assert_eq!(
        output.status.code(),
        Some(1),
        "diff should exit 1 on invalid id"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unsafe") || stderr.contains("invalid"),
        "stderr should mention unsafe/invalid characters: {stderr}"
    );
}

// ─── Snapshot not found ─────────────────────────────────────────────────────

#[test]
fn snapshot_not_found_exits_with_error() {
    let git_tmp = create_test_git_repo();
    let git_path = git_tmp.path().to_string_lossy();

    // Compute the XDG project directory (CLI will look here).
    let xdg_project_dir = archctl::project::resolve_project(&git_path).project_dir;

    // Initialize the store so the project directory exists (but no snapshots).
    open_store(&xdg_project_dir);

    let output = std::process::Command::new(archctl_bin())
        .args([
            "architecture",
            "diff",
            "snap-nonexistent-a",
            "snap-nonexistent-b",
        ])
        .arg("--cwd")
        .arg(git_path.as_ref())
        .current_dir(git_tmp.path())
        .output()
        .expect("diff with missing snapshots must run");

    assert_eq!(
        output.status.code(),
        Some(1),
        "diff should exit 1 when snapshot not found"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("list"),
        "stderr should mention snapshot not found or list: {stderr}"
    );
}
