//! Plugin download + extract (M77 closes the deferred part of M76).

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tar::Archive;

use super::{PluginEntry, plugin_install_root};

/// Download + verify + extract a plugin to `~/.config/archctl/plugins/<author>/<name>/<version>/`.
/// Creates symlinks: <root>/current → <version>/, and per-file symlinks for skills/agents.
pub fn install_plugin(author: &str, name: &str, entry: &PluginEntry) -> Result<PathBuf> {
    // 1. Resolve URL.
    let url = entry
        .url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("plugin {} has no url", entry.name))?;

    // 2. Require SHA256 for remote downloads (SCN-PLG-02: fail closed).
    let is_remote = url.starts_with("http://") || url.starts_with("https://");
    if is_remote && entry.sha256.is_none() {
        anyhow::bail!(
            "remote plugin download requires sha256 checksum (url: {}). \
             Add a \"sha256\" field to the tap entry.",
            url
        );
    }

    // 3. Download.
    let bytes = download_plugin(url)?;

    // 4. Verify SHA256 if provided (mandatory for remote, checked above).
    if let Some(expected) = &entry.sha256 {
        verify_plugin_sha256(&bytes, expected)?;
    }

    // 3. Extract to staging.
    let root = plugin_install_root().join(author).join(name);
    std::fs::create_dir_all(&root)
        .with_context(|| format!("create plugin root {}", root.display()))?;
    let staging = tempfile::Builder::new()
        .prefix(&format!("{}-", entry.version))
        .tempdir_in(&root)?;
    let staging_path = staging.path().to_path_buf();
    extract_plugin(&bytes, &staging_path)?;

    // 4. Move to versioned dir.
    let version_dir = root.join(&entry.version);
    if version_dir.exists() {
        anyhow::bail!(
            "version {} of plugin {}@{} already installed",
            entry.version,
            author,
            name
        );
    }
    std::fs::create_dir_all(version_dir.parent().unwrap())?;
    // Transfer ownership without deleting: into_path() is deprecated but the
    // replacement (keep + path) requires &mut self during drop sequence which
    // is awkward in this context. The deprecated method is safe here.
    #[allow(deprecated)]
    let staging_persisted = staging.into_path();
    std::fs::rename(&staging_persisted, &version_dir)
        .with_context(|| format!("move staging to {}", version_dir.display()))?;

    // 5. Switch current symlink.
    let current = root.join("current");
    if current.is_symlink() || current.exists() {
        std::fs::remove_file(&current)?;
    }
    std::os::unix::fs::symlink(&entry.version, &current)?;

    Ok(version_dir)
}

fn download_plugin(url: &str) -> Result<Vec<u8>> {
    let body = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .user_agent("archctl-plugin/1.0")
        .build()
        .with_context(|| "build HTTP client")?
        .get(url)
        .send()
        .with_context(|| format!("GET {url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP GET {url}"))?
        .bytes()
        .with_context(|| format!("read response body from {url}"))?;
    Ok(body.to_vec())
}

fn verify_plugin_sha256(data: &[u8], expected_hex: &str) -> Result<()> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let actual = hasher.finalize();
    let actual_hex: String = actual.iter().map(|b| format!("{b:02x}")).collect();
    if actual_hex != expected_hex {
        anyhow::bail!(
            "SHA256 mismatch: expected {}, got {}",
            expected_hex,
            actual_hex
        );
    }
    Ok(())
}

/// Extract a plugin tarball, rejecting entries that attempt path traversal (SCN-PLG-03).
fn extract_plugin(bytes: &[u8], dest: &Path) -> Result<()> {
    let cursor = std::io::Cursor::new(bytes);
    let gz = GzDecoder::new(cursor);
    let mut archive = Archive::new(gz);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let path_str = path.to_string_lossy();

        // P0-08: reject path traversal and absolute paths before I/O.
        if path_str.contains("..") || path_str.starts_with('/') {
            anyhow::bail!("tar entry contains unsafe path: {path_str} (potential path traversal)");
        }

        // Reject non-regular files (devices, FIFOs, sockets) for safety.
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() && !entry_type.is_dir() && !entry_type.is_symlink() {
            anyhow::bail!(
                "tar entry is not a regular file, dir, or symlink: {path_str} (type: {entry_type:?})"
            );
        }

        entry.unpack_in(dest)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_plugin_tarball() -> Vec<u8> {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("SKILL.md"), "# test skill").unwrap();
        std::fs::write(tmp.path().join("agent.md"), "# test agent").unwrap();

        let tar_gz_path = tmp.path().join("plugin.tar.gz");
        let tar_file = std::fs::File::create(&tar_gz_path).unwrap();
        let enc = flate2::write::GzEncoder::new(tar_file, flate2::Compression::fast());
        let mut tar = tar::Builder::new(enc);
        tar.append_file(
            "SKILL.md",
            &mut std::fs::File::open(tmp.path().join("SKILL.md")).unwrap(),
        )
        .unwrap();
        tar.append_file(
            "agent.md",
            &mut std::fs::File::open(tmp.path().join("agent.md")).unwrap(),
        )
        .unwrap();
        tar.into_inner().unwrap().finish().unwrap();
        std::fs::read(&tar_gz_path).unwrap()
    }

    #[test]
    fn verify_plugin_sha256_accepts_correct_hash() {
        let data = b"hello world";
        let hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        verify_plugin_sha256(data, hash).unwrap();
    }

    #[test]
    fn verify_plugin_sha256_rejects_mismatch() {
        let result = verify_plugin_sha256(
            b"hello",
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
        assert!(result.is_err(), "expected SHA256 mismatch to fail");
    }

    #[test]
    fn extract_plugin_writes_files() {
        let bytes = build_test_plugin_tarball();
        let tmp = tempfile::tempdir().unwrap();
        extract_plugin(&bytes, tmp.path()).unwrap();
        assert!(
            tmp.path().join("SKILL.md").exists(),
            "SKILL.md should be extracted"
        );
        assert!(
            tmp.path().join("agent.md").exists(),
            "agent.md should be extracted"
        );
    }

    /// Build a malicious tarball with a path traversal entry (../../etc/evil).
    /// Uses raw tar header bytes because the `tar` crate refuses to create
    /// entries with `..` (which is exactly the behavior we're testing against).
    fn build_malicious_tarball() -> Vec<u8> {
        // Minimal tar: one safe entry + one path-traversal entry, then gzip.
        let safe_entry = build_raw_tar_entry("SKILL.md", b"safe");
        let evil_entry = build_raw_tar_entry("../../etc/evil", b"evil");
        let mut tar_bytes = Vec::new();
        tar_bytes.extend_from_slice(&safe_entry.0);
        tar_bytes.extend_from_slice(&safe_entry.1);
        tar_bytes.extend_from_slice(&evil_entry.0);
        tar_bytes.extend_from_slice(&evil_entry.1);
        // End-of-archive: two zero blocks.
        tar_bytes.extend_from_slice(&[0u8; 1024]);

        // Gzip the tar bytes.
        let gz_path = "/tmp/archctl-evil-test.tar.gz";
        let gz_file = std::fs::File::create(gz_path).unwrap();
        let mut encoder = flate2::write::GzEncoder::new(gz_file, flate2::Compression::fast());
        std::io::Write::write_all(&mut encoder, &tar_bytes).unwrap();
        encoder.finish().unwrap();
        std::fs::read(gz_path).unwrap()
    }

    /// Build a raw 512-byte tar header + padded data for a single file entry.
    fn build_raw_tar_entry(name: &str, data: &[u8]) -> ([u8; 512], Vec<u8>) {
        let mut header = [0u8; 512];
        // Name (offset 0, 100 bytes)
        let name_bytes = name.as_bytes();
        let copy_len = name_bytes.len().min(100);
        header[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
        // Mode (offset 100, 8 bytes, octal)
        let mode = format!("{:07o}\0", 0o644);
        header[100..108].copy_from_slice(mode.as_bytes());
        // UID (offset 108)
        header[108..116].copy_from_slice(b"0001000\0");
        // GID (offset 116)
        header[116..124].copy_from_slice(b"0001000\0");
        // Size (offset 124, 12 bytes, octal)
        let size_str = format!("{:011o}\0", data.len());
        header[124..136].copy_from_slice(size_str.as_bytes());
        // Mtime (offset 136, 12 bytes)
        header[136..148].copy_from_slice(b"00000000000\0");
        // Typeflag (offset 156) — '0' = regular file
        header[156] = b'0';
        // Magic (offset 257, 6 bytes)
        header[257..263].copy_from_slice(b"ustar\0");
        // Version (offset 263, 2 bytes)
        header[263..265].copy_from_slice(b"00");

        // Checksum (offset 148, 8 bytes) — sum of all bytes with checksum field as spaces
        header[148..156].copy_from_slice(b"        ");
        let cksum: u32 = header.iter().map(|&b| b as u32).sum();
        let cksum_str = format!("{:06o}\0 ", cksum);
        header[148..156].copy_from_slice(cksum_str.as_bytes());

        // Pad data to 512-byte boundary.
        let mut padded_data = data.to_vec();
        let remainder = padded_data.len() % 512;
        if remainder > 0 {
            padded_data.resize(padded_data.len() + (512 - remainder), 0);
        }
        (header, padded_data)
    }

    #[test]
    fn extract_plugin_rejects_path_traversal() {
        // SCN-PLG-03: tar with ../../outside must not escape staging.
        let bytes = build_malicious_tarball();
        let tmp = tempfile::tempdir().unwrap();
        let result = extract_plugin(&bytes, tmp.path());
        assert!(result.is_err(), "malicious tarball should be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("unsafe path"),
            "error should mention unsafe path, got: {err}"
        );
    }
}
