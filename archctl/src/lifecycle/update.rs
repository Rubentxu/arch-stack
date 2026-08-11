//! M73 T3: Self-update orchestration — download, verify, migrate, install.

use anyhow::{Context, Result, anyhow};
use flate2::read::GzDecoder;
use semver::Version;
use std::path::Path;
use tar::Archive;

use super::channels::Channel;
use super::install_root::{current_symlink, install_dir};
use super::migration::{MigrationManifest, execute_manifest};
use super::release::{
    download_asset, fetch_release_info, fetch_sha256_for, pick_asset, verify_sha256,
};

/// Self-update: download new version from GitHub, verify, run migrations,
/// install, switch symlink. If anything fails after the symlink switch
/// was attempted, the caller is expected to rollback (we don't rollback
/// automatically here; we exit early before touching the symlink).
#[allow(unused_variables)] // `channel` reserved for future channel-specific tag logic (M76).
pub fn update(
    target: Option<&Version>,
    channel: Channel,
    install_root: &Path,
    current_version: &Version,
) -> Result<Version> {
    // 1. Resolve target version.
    let release_info = if let Some(v) = target {
        fetch_release_info(Some(&format!("v{v}")))?
    } else {
        fetch_release_info(None)?
    };
    let new_version = Version::parse(release_info.tag_name.trim_start_matches('v'))
        .with_context(|| format!("parse tag '{}' as semver", release_info.tag_name))?;

    if new_version == *current_version {
        eprintln!("archctl {} is already the latest", current_version);
        return Ok(new_version);
    }

    // 2. Pick + download asset + SHA256SUMS.
    let asset = pick_asset(&release_info).ok_or_else(|| {
        anyhow!(
            "no matching binary for this platform in {}",
            release_info.tag_name
        )
    })?;
    eprintln!("downloading {}", asset.name);
    let asset_bytes = download_asset(asset)?;
    let expected_sha = fetch_sha256_for(&release_info, &asset.name)?;
    verify_sha256(&asset_bytes, &expected_sha)?;

    // 3. Extract tarball to a staging dir. The staging dir is wrapped in a
    //    guard struct so it's cleaned up automatically if any later step
    //    fails before we move it into installs/.
    // Staging dir under <root>/cache/. Auto-cleaned if any step below fails.
    let cache_root = install_root.join("cache");
    std::fs::create_dir_all(&cache_root)
        .with_context(|| format!("create dir {}", cache_root.display()))?;
    let staging_guard = tempfile::Builder::new()
        .prefix(&format!("staging-{new_version}-"))
        .tempdir_in(&cache_root)?;
    let staging = staging_guard.path().to_path_buf();
    extract_tarball(&asset_bytes, &staging)?;

    // 4. Run migrations if present.
    let manifest_path = staging.join("migration-manifest.json");
    if manifest_path.exists() {
        let manifest_bytes = std::fs::read(&manifest_path)?;
        let manifest = MigrationManifest::from_bytes(&manifest_bytes)?;
        let from_dir = install_dir(install_root, current_version);
        execute_manifest(&manifest, &from_dir, &staging)?;
    }

    // 5. Move staging into installs/v<new>/. Disable the guard so the
    //    staging dir persists under its new location.
    let target_dir = install_dir(install_root, &new_version);
    if target_dir.exists() {
        std::fs::remove_dir_all(&target_dir)?;
    }
    std::fs::create_dir_all(target_dir.parent().unwrap())?;
    let staging_path = staging_guard.keep(); // consumes guard, persists
    std::fs::rename(&staging_path, &target_dir)?;

    // 6. Sanity check + switch symlink.
    let binary = target_dir.join("archctl");
    let output = std::process::Command::new(&binary)
        .arg("--version")
        .output()
        .with_context(|| format!("sanity check: run '{}'", binary.display()))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.contains(&new_version.to_string()) {
        anyhow::bail!(
            "post-update sanity check failed: '{}' doesn't mention {}",
            stdout.trim(),
            new_version
        );
    }
    let current = current_symlink(install_root);
    if current.is_symlink() || current.exists() {
        std::fs::remove_file(&current)?;
    }
    std::os::unix::fs::symlink(
        install_dir(install_root, &new_version).file_name().unwrap(),
        &current,
    )?;

    Ok(new_version)
}

fn extract_tarball(bytes: &[u8], dest: &Path) -> Result<()> {
    let cursor = std::io::Cursor::new(bytes);
    let gz = GzDecoder::new(cursor);
    let mut archive = Archive::new(gz);
    archive.unpack(dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_tarball_simple() {
        // Build a minimal tarball with one file.
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tempfile::tempdir().unwrap();
        std::fs::write(src_dir.path().join("hello.txt"), b"hi").unwrap();

        // Use the `tar` crate to create a tar.gz
        let tar_gz_path = tmp.path().join("test.tar.gz");
        let tar_file = std::fs::File::create(&tar_gz_path).unwrap();
        let enc = flate2::write::GzEncoder::new(tar_file, flate2::Compression::fast());
        let mut tar = tar::Builder::new(enc);
        tar.append_file(
            "hello.txt",
            &mut std::fs::File::open(src_dir.path().join("hello.txt")).unwrap(),
        )
        .unwrap();
        tar.into_inner().unwrap().finish().unwrap();

        let extract_dir = tempfile::tempdir().unwrap();
        let bytes = std::fs::read(&tar_gz_path).unwrap();
        extract_tarball(&bytes, extract_dir.path()).unwrap();
        assert!(extract_dir.path().join("hello.txt").exists());
    }
}
