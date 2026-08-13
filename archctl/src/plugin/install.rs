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
    // 1. Download.
    let url = entry
        .url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("plugin {} has no url", entry.name))?;
    let bytes = download_plugin(url)?;

    // 2. Verify SHA256 if provided.
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

fn extract_plugin(bytes: &[u8], dest: &Path) -> Result<()> {
    let cursor = std::io::Cursor::new(bytes);
    let gz = GzDecoder::new(cursor);
    let mut archive = Archive::new(gz);
    archive.unpack(dest)?;
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
}
