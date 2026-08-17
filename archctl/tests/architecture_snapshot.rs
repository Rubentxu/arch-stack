//! Integration tests for the architecture snapshot bounded context.
//!
//! Tests cover: concurrent creates, GC retention, NotGitRepository error,
//! and schema-version compatibility props.

use std::fs;
use std::sync::{Arc, Barrier};
use tempfile::TempDir;

use archctl::architecture::errors::SnapshotError;
use archctl::architecture::{create, gc, list};
use archctl::filesystem::SystemFilesystem;
use archctl::store::{GraphStore, LbugStore};

/// Helper: open a LbugStore at the given project directory.
fn open_store(project_dir: &std::path::Path) -> LbugStore {
    let mut store = LbugStore::open(project_dir).expect("store must open");
    store.init().expect("store must init");
    store
}

/// Helper: create a minimal git repo in a temp directory using git CLI.
/// The repo has one commit so resolve_repository_identity can work.
fn create_test_git_repo() -> TempDir {
    let tmp = TempDir::new().expect("temp dir");
    let repo_path = tmp.path();

    // Use git CLI to create a proper git repo
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

    // Create a file and commit
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

// ─── T2.2: Concurrent create with UnitOfWork boundary ────────────────────────

#[test]
fn concurrent_creates_produce_distinct_ids_and_monotonic_sequences() {
    // Verifies that two simultaneous create() calls on distinct repos both succeed,
    // produce distinct snapshot ids, and assign monotonic sequence numbers.
    // Uses Barrier::wait(2) to ensure both threads reach the create() call before
    // either proceeds, forcing true concurrent execution and testing the flock boundary.

    let project_tmp = TempDir::new().expect("project temp dir");
    let git_tmp1 = create_test_git_repo();
    let git_tmp2 = create_test_git_repo();
    let fs = SystemFilesystem;
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let project_dir = project_tmp.path().to_path_buf();

    let git_path1: String = git_tmp1.path().to_string_lossy().into_owned();
    let git_path2: String = git_tmp2.path().to_string_lossy().into_owned();
    let barrier1 = Arc::clone(&barrier);
    let barrier2 = Arc::clone(&barrier);

    let fs1 = Arc::new(SystemFilesystem);
    let fs2 = Arc::new(SystemFilesystem);

    let handle1 = std::thread::spawn(move || {
        let _ = barrier1.wait(); // wait for thread 2 to be ready
        create(&project_dir, &git_path1, fs1.as_ref(), "architecture", 1, None, false)
    });

    let handle2 = std::thread::spawn(move || {
        let _ = barrier2.wait(); // wait for thread 1 to be ready
        create(&project_dir, &git_path2, fs2.as_ref(), "architecture", 2, None, false)
    });

    let result1 = handle1.join().expect("thread 1 must not panic");
    let result2 = handle2.join().expect("thread 2 must not panic");

    let (id1, seq1) = result1.expect("first create must succeed");
    let (id2, seq2) = result2.expect("second create must succeed");

    assert!(!id1.is_empty(), "snapshot id1 must be non-empty");
    assert!(!id2.is_empty(), "snapshot id2 must be non-empty");
    assert!(seq1 >= 0, "sequence1 must be non-negative");
    assert!(seq2 >= 0, "sequence2 must be non-negative");

    // Distinct repos → distinct identity tuples → distinct snapshot ids
    assert_ne!(
        id1, id2,
        "concurrent creates on distinct repos must produce distinct ids"
    );

    // Sequence numbers must be monotonic (seq2 > seq1 if id2 was committed after id1)
    assert!(
        seq2 > seq1 || (seq2 == seq1 && id1 != id2),
        "sequence numbers must be monotonic or tie-broken by id"
    );
}

// ─── T4.2: NotGitRepository on non-Git path ──────────────────────────────────

#[test]
fn create_on_non_git_directory_returns_not_git_repository_error() {
    let project_tmp = TempDir::new().expect("project temp dir");
    let non_git_tmp = TempDir::new().expect("non-git temp dir"); // NOT a git repo
    let fs = SystemFilesystem;

    let result = create(
        project_tmp.path(),
        &non_git_tmp.path().to_string_lossy(),
        &fs,
        "architecture",
        1,
        None,
        false,
    );

    let err = result.expect_err("create on non-git dir must fail");
    assert!(
        matches!(err, SnapshotError::NotGitRepository(_)),
        "expected NotGitRepository error, got: {err}"
    );
}

// ─── T4.1: GC retention logic ─────────────────────────────────────────────────

#[test]
fn gc_preserves_pinned_snapshots() {
    let project_tmp = TempDir::new().expect("project temp dir");
    let git_tmp = create_test_git_repo();
    let fs = SystemFilesystem;
    let project_dir = project_tmp.path().to_path_buf();
    let git_path = git_tmp.path().to_string_lossy();

    // Create 3 snapshots with distinct schema versions, pin the middle one.
    // Distinct schema_versions ensure distinct identity keys (idempotency prevents duplicates).
    let (id1, _seq1) = create(
        &project_dir,
        &git_path,
        &fs,
        "architecture",
        1,
        Some("first"),
        false,
    )
    .expect("create first");

    let (id2, _seq2) = create(
        &project_dir,
        &git_path,
        &fs,
        "architecture",
        2,
        Some("pinned"),
        true,
    )
    .expect("create pinned");

    let (id3, _seq3) = create(
        &project_dir,
        &git_path,
        &fs,
        "architecture",
        3,
        Some("third"),
        false,
    )
    .expect("create third");

    // GC with keep_last=1 should delete id1 and id3, preserve id2 (pinned)
    let report = gc(&project_dir, 1, true, false).expect("gc dry-run must succeed");

    assert!(
        report.deleted.contains(&id1) || report.deleted.contains(&id3),
        "at least one unpinned should be in deleted list"
    );
    assert!(
        report.preserved.contains(&id2),
        "pinned snapshot must be preserved"
    );
}

#[test]
fn gc_dry_run_does_not_delete() {
    let project_tmp = TempDir::new().expect("project temp dir");
    let git_tmp = create_test_git_repo();
    let fs = SystemFilesystem;
    let project_dir = project_tmp.path().to_path_buf();
    let git_path = git_tmp.path().to_string_lossy();

    // Create 2 snapshots with distinct schema versions.
    // Distinct schema_versions ensure distinct identity keys (idempotency prevents duplicates).
    let (id1, _seq1) =
        create(&project_dir, &git_path, &fs, "architecture", 1, None, false).expect("create 1");
    let (id2, _seq2) =
        create(&project_dir, &git_path, &fs, "architecture", 2, None, false).expect("create 2");

    // GC with dry_run=true should not actually delete, but should report what would be deleted.
    // With 2 snapshots and keep_last=1, 1 snapshot (the older one) would be marked for deletion.
    let report = gc(&project_dir, 1, true, false).expect("gc dry-run must succeed");

    // dry_run=true means nothing is actually deleted, but report.deleted shows what WOULD be deleted
    assert_eq!(
        report.deleted.len(),
        1,
        "dry_run=true should report 1 snapshot for deletion with keep_last=1 and 2 snapshots"
    );
    // The older snapshot (id1) should be in the deleted list (id2 is newer and kept)
    assert!(
        report.deleted.contains(&id1),
        "older snapshot id1 should be in deleted list"
    );
    assert!(report.dry_run, "report.dry_run must be true");

    // Verify snapshots still exist
    let store = open_store(&project_dir);
    let snapshots = list(&store).expect("list must succeed");
    let ids: Vec<_> = snapshots.iter().map(|s| s.id.clone()).collect();
    assert!(ids.contains(&id1), "id1 must still exist");
    assert!(ids.contains(&id2), "id2 must still exist");
}

#[test]
fn gc_yes_flag_enables_deletion() {
    let project_tmp = TempDir::new().expect("project temp dir");
    let git_tmp = create_test_git_repo();
    let fs = SystemFilesystem;
    let project_dir = project_tmp.path().to_path_buf();
    let git_path = git_tmp.path().to_string_lossy();

    // Create 3 snapshots with distinct schema versions.
    // Distinct schema_versions ensure distinct identity keys (idempotency prevents duplicates).
    let (_id1, _seq1) =
        create(&project_dir, &git_path, &fs, "architecture", 1, None, false).expect("create 1");
    create(&project_dir, &git_path, &fs, "architecture", 2, None, false).expect("create 2");
    create(&project_dir, &git_path, &fs, "architecture", 3, None, false).expect("create 3");

    // GC with keep_last=2, dry_run=false, confirmed=true should delete excess
    let report = gc(&project_dir, 2, false, true).expect("gc with --yes must succeed");

    assert!(
        !report.deleted.is_empty(),
        "confirmed gc must mark some snapshots for deletion"
    );
    assert!(
        !report.dry_run,
        "report.dry_run must be false when confirmed"
    );
}

#[test]
fn gc_requires_confirmation_when_not_dry_run() {
    let project_tmp = TempDir::new().expect("project temp dir");
    let git_tmp = create_test_git_repo();
    let fs = SystemFilesystem;
    let project_dir = project_tmp.path().to_path_buf();
    let git_path = git_tmp.path().to_string_lossy();

    // Create 2 snapshots with distinct schema versions.
    // Distinct schema_versions ensure distinct identity keys (idempotency prevents duplicates).
    create(&project_dir, &git_path, &fs, "architecture", 1, None, false).expect("create 1");
    create(&project_dir, &git_path, &fs, "architecture", 2, None, false).expect("create 2");

    // GC with dry_run=false, confirmed=false must fail with GcRequiresConfirmation
    let result = gc(&project_dir, 1, false, false);

    let err = result.expect_err("gc without confirmation must fail");
    assert!(
        matches!(err, SnapshotError::GcRequiresConfirmation { .. }),
        "expected GcRequiresConfirmation error, got: {err}"
    );
}

#[test]
fn gc_keeps_last_n_by_created_at() {
    let project_tmp = TempDir::new().expect("project temp dir");
    let git_tmp = create_test_git_repo();
    let fs = SystemFilesystem;
    let project_dir = project_tmp.path().to_path_buf();
    let git_path = git_tmp.path().to_string_lossy();

    // Create 5 snapshots with distinct schema versions.
    // Using different schema_versions to ensure distinct identity keys
    // (idempotency is based on commit_hash + schema_version + repo_identity + extractor_digest).
    let ids: Vec<_> = (0..5)
        .map(|i| {
            create(
                &project_dir,
                &git_path,
                &fs,
                "architecture",
                i + 1, // distinct schema_version for each snapshot
                Some(&format!("snap_{}", i)),
                false,
            )
            .expect("create snapshot")
            .0
        })
        .collect();

    // GC with keep_last=3 should keep the 3 most recent
    let report = gc(&project_dir, 3, true, false).expect("gc dry-run must succeed");

    // The 3 most recent (ids[-3:]) should be preserved, older ones deleted
    let preserved: usize = report
        .preserved
        .iter()
        .filter(|id| ids.contains(id))
        .count();
    let deleted: usize = report.deleted.iter().filter(|id| ids.contains(id)).count();

    assert_eq!(preserved, 3, "should preserve 3 snapshots");
    assert_eq!(deleted, 2, "should mark 2 snapshots for deletion");
}

// ─── T5.1: Schema-version compatibility props ─────────────────────────────────

#[test]
fn create_sets_schema_version_and_compatibility_props() {
    let project_tmp = TempDir::new().expect("project temp dir");
    let git_tmp = create_test_git_repo();
    let fs = SystemFilesystem;
    let project_dir = project_tmp.path().to_path_buf();
    let git_path = git_tmp.path().to_string_lossy();

    let schema_version: i64 = 2;
    let (id, _seq) = create(
        &project_dir,
        &git_path,
        &fs,
        "architecture",
        schema_version,
        Some("test"),
        false,
    )
    .expect("create must succeed");

    // Read back the snapshot and verify props
    let store = open_store(&project_dir);
    let snapshots = list(&store).expect("list must succeed");

    let snap = snapshots
        .iter()
        .find(|s| s.id == id)
        .expect("snapshot must exist");

    // Column stores major version
    assert_eq!(
        snap.schema_version, schema_version,
        "column schema_version (major) must match"
    );

    // Props store full semver
    let props_version = snap
        .props
        .get("schema_version")
        .expect("props.schema_version must exist")
        .as_str()
        .expect("props.schema_version must be string");
    assert!(
        props_version.starts_with(&format!("{}.", schema_version)),
        "props.schema_version must be full semver starting with major, got: {}",
        props_version
    );

    // Props store compatibility string
    let props_compat = snap
        .props
        .get("schema_compatibility")
        .expect("props.schema_compatibility must exist")
        .as_str()
        .expect("props.schema_compatibility must be string");
    assert_eq!(
        props_compat, "1.0",
        "props.schema_compatibility must be '1.0', got: {}",
        props_compat
    );
}

// ─── List tests ────────────────────────────────────────────────────────────────

#[test]
fn list_returns_all_snapshots_ordered_by_created_at_desc() {
    let project_tmp = TempDir::new().expect("project temp dir");
    let git_tmp = create_test_git_repo();
    let fs = SystemFilesystem;
    let project_dir = project_tmp.path().to_path_buf();
    let git_path = git_tmp.path().to_string_lossy();

    // Create 3 snapshots with distinct schema versions.
    // Distinct schema_versions ensure distinct identity keys (idempotency prevents duplicates).
    create(
        &project_dir,
        &git_path,
        &fs,
        "architecture",
        1,
        Some("first"),
        false,
    )
    .expect("create 1");
    create(
        &project_dir,
        &git_path,
        &fs,
        "architecture",
        2,
        Some("second"),
        false,
    )
    .expect("create 2");
    create(
        &project_dir,
        &git_path,
        &fs,
        "architecture",
        3,
        Some("third"),
        false,
    )
    .expect("create 3");

    let store = open_store(&project_dir);
    let snapshots = list(&store).expect("list must succeed");

    assert_eq!(snapshots.len(), 3, "should have 3 snapshots");

    // Verify ordering: most recent first (descending by created_at)
    for window in snapshots.windows(2) {
        assert!(
            window[0].created_at >= window[1].created_at,
            "snapshots must be ordered by created_at descending"
        );
    }
}
