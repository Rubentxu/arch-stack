//! `archctl self` — CLI lifecycle management (asdf-inspired).
//!
//! Implements ADR-040 (versioned distribution): multi-version installs in
//! `~/.local/share/archctl/installs/<version>/`, per-project pin via
//! `.arch-version`, shim binary, self-update via GitHub Releases.

use std::path::Path;

pub mod install_root;
pub mod shim;

// T2 sub-modules.
pub mod install;
pub mod list;
pub mod uninstall;
pub mod use_version;
pub mod version_file;

// T3 sub-modules (stubs until T3).
#[allow(dead_code)]
pub mod channels;
#[allow(dead_code)]
pub mod migration;
#[allow(dead_code)]
pub mod release;
#[allow(dead_code)]
pub mod update;

/// Version type alias — semver::Version is used throughout the module.
pub type Version = semver::Version;

// ---------------------------------------------------------------------------
// Re-exports from T2 sub-modules (these replace the stubs below).
// ---------------------------------------------------------------------------

pub use install::install;
pub use list::{list, InstalledVersion};
pub use uninstall::uninstall;
pub use use_version::use_version;
pub use version_file::{find_arch_version, resolve_active_version};

// ---------------------------------------------------------------------------
// T3 stub implementations — replaced in their respective tasks.
// ---------------------------------------------------------------------------

/// Stub: channel resolution (stable/rc/nightly). Replaced in T3.
#[allow(dead_code)]
pub fn resolve_channel(_chan: Channel) -> anyhow::Result<String> {
    unimplemented!("T3: resolve_channel")
}

/// Stub: fetches release info from GitHub. Replaced in T3.
#[allow(dead_code)]
pub fn fetch_release_info(_tag: &str) -> anyhow::Result<ReleaseInfo> {
    unimplemented!("T3: fetch_release_info")
}

/// Stub: downloads and verifies a release asset. Replaced in T3.
#[allow(dead_code)]
pub fn download_and_verify(_asset_url: &str, _sha256_expected: &[u8]) -> anyhow::Result<Vec<u8>> {
    unimplemented!("T3: download_and_verify")
}

/// Stub: runs migration manifest between versions. Replaced in T3.
#[allow(dead_code)]
pub fn execute_manifest(
    _manifest: &MigrationManifest,
    _from_dir: &Path,
    _to_dir: &Path,
) -> anyhow::Result<()> {
    unimplemented!("T3: migration")
}

/// Stub: self-update orchestration. Replaced in T3.
#[allow(dead_code)]
pub fn update(_target: Option<&Version>, _channel: Channel, _install_root: &Path) -> anyhow::Result<()> {
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
