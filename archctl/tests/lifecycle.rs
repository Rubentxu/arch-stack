//! Integration tests for the archctl self lifecycle commands:
//! install → use → list → uninstall (full cycle).
//!
//! These tests run the public API of the lifecycle module directly,
//! simulating the real install_root layout without touching the filesystem.

use archctl::lifecycle::install::install as lifecycle_install;
use archctl::lifecycle::list::list as lifecycle_list;
use archctl::lifecycle::uninstall::uninstall as lifecycle_uninstall;
use archctl::lifecycle::use_version::use_version as lifecycle_use_version;
use archctl::lifecycle::Version;

fn parse_version(s: &str) -> Version {
    // Use the module's Version type alias (semver::Version).
    s.parse().expect("valid semver")
}

#[test]
fn full_install_use_uninstall_cycle() {
    let tmp = tempfile::tempdir().unwrap();

    // 1. Install a mock binary.
    let src = tmp.path().join("src");
    std::fs::write(&src, b"#!/bin/sh\necho v1.34").unwrap();

    let v = parse_version("1.34.0");
    lifecycle_install(&v, tmp.path(), &src).unwrap();

    // Verify the binary was installed.
    let installed_path = tmp.path().join("installs/v1.34.0/archctl");
    assert!(installed_path.exists(), "binary not installed");

    // 2. Use the installed version.
    lifecycle_use_version(&v, tmp.path()).unwrap();
    let target = std::fs::read_link(tmp.path().join("current")).unwrap();
    assert!(
        target.ends_with("v1.34.0"),
        "current symlink should point to v1.34.0, got: {}",
        target.display()
    );

    // 3. List marks the active version.
    let listed = lifecycle_list(tmp.path()).unwrap();
    assert_eq!(listed.len(), 1, "expected 1 installed version");
    assert!(
        listed[0].is_active,
        "installed version should be marked active"
    );

    // 4. Uninstall the version.
    lifecycle_uninstall(Some(&v), tmp.path(), false).unwrap();
    assert!(
        !tmp.path().join("installs/v1.34.0").exists(),
        "version directory should be removed"
    );

    // After uninstall, list should be empty (current symlink is now dangling).
    let listed = lifecycle_list(tmp.path()).unwrap();
    assert!(listed.is_empty(), "no versions should remain after uninstall");
}

#[test]
fn install_multiple_versions_and_switch() {
    let tmp = tempfile::tempdir().unwrap();

    // Install two versions.
    for (v_str, content) in [("1.32.0", b"v1.32"), ("1.34.0", b"v1.34")] {
        let src = tmp.path().join(format!("src_{}", v_str));
        std::fs::write(&src, content).unwrap();
        let v = parse_version(v_str);
        lifecycle_install(&v, tmp.path(), &src).unwrap();
    }

    // List should show both.
    let listed = lifecycle_list(tmp.path()).unwrap();
    assert_eq!(listed.len(), 2);

    // Switch to v1.32.0.
    let v132 = parse_version("1.32.0");
    lifecycle_use_version(&v132, tmp.path()).unwrap();

    // List should mark v1.32.0 as active.
    let listed = lifecycle_list(tmp.path()).unwrap();
    let active: Vec<_> = listed.iter().filter(|v| v.is_active).collect();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].version.to_string(), "1.32.0");

    // Uninstall v1.34.0, v1.32.0 should remain.
    let v134 = parse_version("1.34.0");
    lifecycle_uninstall(Some(&v134), tmp.path(), false).unwrap();
    let listed = lifecycle_list(tmp.path()).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].version.to_string(), "1.32.0");
}

#[test]
fn purge_removes_all_versions_and_symlink() {
    let tmp = tempfile::tempdir().unwrap();

    // Install two versions.
    for v_str in ["1.32.0", "1.34.0"] {
        let src = tmp.path().join(format!("src_{}", v_str));
        std::fs::write(&src, b"mock").unwrap();
        let v = parse_version(v_str);
        lifecycle_install(&v, tmp.path(), &src).unwrap();
    }

    // Purge all.
    lifecycle_uninstall(None, tmp.path(), true).unwrap();

    // Both versions + current symlink gone.
    assert!(!tmp.path().join("installs").exists(), "installs dir should be gone");
    assert!(!tmp.path().join("current").exists(), "current symlink should be gone");
}
