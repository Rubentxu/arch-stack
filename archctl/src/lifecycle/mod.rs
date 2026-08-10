//! `archctl self` — CLI lifecycle management (asdf-inspired).
//!
//! Implements ADR-040 (versioned distribution): multi-version installs in
//! `~/.local/share/archctl/installs/<version>/`, per-project pin via
//! `.arch-version`, shim binary, self-update via GitHub Releases.

use anyhow::Result;
use std::path::Path;

pub mod install_root;
pub mod shim;

// Sub-modules added in T2/T3:
pub mod install;
pub mod list;
pub mod uninstall;
pub mod use_version;
pub mod version_file;

#[allow(dead_code)] // Used in T3.
pub mod channels;
#[allow(dead_code)] // Used in T3.
pub mod migration;
#[allow(dead_code)] // Used in T3.
pub mod release;
#[allow(dead_code)] // Used in T3.
pub mod update;

/// Version type alias — semver::Version is used throughout the module.
pub type Version = semver::Version;

// ---------------------------------------------------------------------------
// T2/T3 stub implementations — replaced in their respective tasks.
// ---------------------------------------------------------------------------

/// Stub: installs a versioned archctl binary. Replaced in T2.
pub fn install(_version: Option<&Version>, _install_root: &Path) -> Result<()> {
    unimplemented!("T2: archctl self install")
}

/// Stub: lists installed versions. Replaced in T2.
pub fn list(_install_root: &Path) -> Result<Vec<InstalledVersion>> {
    unimplemented!("T2: archctl self list")
}

/// Stub: changes the active symlink. Replaced in T2.
pub fn use_version(_version: &Version, _install_root: &Path) -> Result<()> {
    unimplemented!("T2: archctl self use")
}

/// Stub: removes a version or purges all. Replaced in T2.
pub fn uninstall(_version: Option<&Version>, _install_root: &Path, _purge: bool) -> Result<()> {
    unimplemented!("T2: archctl self uninstall")
}

/// Stub: walks up directories looking for .arch-version. Replaced in T2.
pub fn find_arch_version(_cwd: &Path) -> Option<Version> {
    unimplemented!("T2: .arch-version walking")
}

/// Stub: resolves active version with precedence. Replaced in T2.
pub fn resolve_active_version(
    _override_flag: Option<&Version>,
    _env_var: Option<&str>,
    _cwd: &Path,
    _install_root: &Path,
) -> Result<std::path::PathBuf> {
    unimplemented!("T2: resolve_active_version")
}

/// Stub: channel resolution (stable/rc/nightly). Replaced in T3.
pub fn resolve_channel(_chan: Channel) -> Result<String> {
    unimplemented!("T3: resolve_channel")
}

/// Stub: fetches release info from GitHub. Replaced in T3.
pub fn fetch_release_info(_tag: &str) -> Result<ReleaseInfo> {
    unimplemented!("T3: fetch_release_info")
}

/// Stub: downloads and verifies a release asset. Replaced in T3.
pub fn download_and_verify(_asset_url: &str, _sha256_expected: &[u8]) -> Result<Vec<u8>> {
    unimplemented!("T3: download_and_verify")
}

/// Stub: runs migration manifest between versions. Replaced in T3.
pub fn execute_manifest(
    _manifest: &MigrationManifest,
    _from_dir: &Path,
    _to_dir: &Path,
) -> Result<()> {
    unimplemented!("T3: migration")
}

/// Stub: self-update orchestration. Replaced in T3.
pub fn update(_target: Option<&Version>, _channel: Channel, _install_root: &Path) -> Result<()> {
    unimplemented!("T3: archctl self update")
}

// ---------------------------------------------------------------------------
// Data types used across the module.
// ---------------------------------------------------------------------------

/// Channel for self-update (stable / rc / nightly).
#[derive(Debug, Clone, Copy)]
pub enum Channel {
    Stable,
    Rc,
    Nightly,
}

/// Information about an installed version.
#[derive(Debug, Clone)]
pub struct InstalledVersion {
    /// The semver version string.
    pub version: Version,
    /// Full path to the install directory.
    pub path: std::path::PathBuf,
}

/// Metadata for a single migration step.
#[derive(Debug, Clone)]
pub struct Migration {
    pub id: String,
    pub description: String,
    pub script: String,
    pub rollback_supported: bool,
}

/// A manifest of migrations to run between two versions.
#[derive(Debug, Clone)]
pub struct MigrationManifest {
    pub from_version: Version,
    pub to_version: Version,
    pub migrations: Vec<Migration>,
}

/// Information fetched from a GitHub release.
#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub tag: String,
    pub version: Version,
    pub assets: Vec<Asset>,
}

/// A single asset in a GitHub release.
#[derive(Debug, Clone)]
pub struct Asset {
    pub name: String,
    pub download_url: String,
    pub sha256: String,
}
