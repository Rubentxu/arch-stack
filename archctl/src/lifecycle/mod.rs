//! `archctl self` — CLI lifecycle management (asdf-inspired).
//!
//! Implements ADR-057 (versioned distribution): multi-version installs in
//! `~/.local/share/archctl/installs/<version>/`, per-project pin via
//! `.arch-version`, shim binary, self-update via GitHub Releases.

pub mod install_root;
pub mod shim;

// T2 sub-modules.
pub mod install;
pub mod list;
pub mod uninstall;
pub mod use_version;
pub mod version_file;

// T3 sub-modules.
pub mod channels;
pub mod migration;
pub mod release;
pub mod update;

/// Version type alias — semver::Version is used throughout the module.
pub type Version = semver::Version;

// ---------------------------------------------------------------------------
// Re-exports from T2 sub-modules.
// ---------------------------------------------------------------------------

pub use install::install;
pub use list::{InstalledVersion, list};
pub use uninstall::uninstall;
pub use use_version::use_version;
pub use version_file::{find_arch_version, resolve_active_version};

// ---------------------------------------------------------------------------
// Re-exports from T3 sub-modules.
// ---------------------------------------------------------------------------

pub use channels::{Channel, channel_label};
pub use migration::{Migration, MigrationManifest, execute_manifest};
pub use release::{
    ReleaseAsset, ReleaseInfo, current_target_triple, download_asset, fetch_release_info,
    fetch_sha256_for, pick_asset, verify_sha256,
};
pub use update::update;
