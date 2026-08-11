//! M73 T3: GitHub Releases API client — fetch release info, pick asset, download, verify SHA256.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

/// Fetch release info from GitHub Releases API. Uses blocking reqwest
/// client (timeout 30s) — synchronous calls are simpler for a CLI.
pub fn fetch_release_info(tag: Option<&str>) -> Result<ReleaseInfo> {
    let url = match tag {
        Some(t) => format!(
            "https://api.github.com/repos/Rubentxu/arch-stack/releases/tags/{}",
            t
        ),
        None => "https://api.github.com/repos/Rubentxu/arch-stack/releases/latest".to_string(),
    };
    let body = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("archctl-self-update/1.0")
        .build()?
        .get(&url)
        .send()
        .with_context(|| format!("GET {url}"))?
        .error_for_status()?
        .text()?;
    serde_json::from_str(&body).with_context(|| "parse GitHub release JSON")
}

/// Pick the asset matching the current target triple (e.g.
/// `archctl-x86_64-unknown-linux-gnu.tar.gz`).
pub fn pick_asset(release: &ReleaseInfo) -> Option<&ReleaseAsset> {
    let target = current_target_triple();
    release
        .assets
        .iter()
        .find(|a| a.name.starts_with(&format!("archctl-{target}")))
}

pub fn current_target_triple() -> String {
    // Build triple: x86_64-unknown-linux-gnu (Linux), etc.
    // Simplest: just use a stable identifier; we don't compile platform-
    // specific binaries today, so all our releases have the same
    // filename. Use a generic identifier.
    "x86_64-unknown-linux-gnu".to_string()
}

/// Download asset bytes from GitHub Releases.
pub fn download_asset(asset: &ReleaseAsset) -> Result<Vec<u8>> {
    let body = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .user_agent("archctl-self-update/1.0")
        .build()?
        .get(&asset.browser_download_url)
        .send()
        .with_context(|| format!("GET {}", asset.browser_download_url))?
        .error_for_status()?
        .bytes()?;
    Ok(body.to_vec())
}

/// Verify that data matches the expected SHA256 (hex-encoded).
pub fn verify_sha256(data: &[u8], expected_hex: &str) -> Result<()> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let actual = hasher.finalize();
    let actual_hex = actual
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    if actual_hex != expected_hex {
        anyhow::bail!("SHA256 mismatch: expected {expected_hex}, got {actual_hex}");
    }
    Ok(())
}

/// Fetch SHA256SUMS file from the release and find the entry for the
/// given asset name.
pub fn fetch_sha256_for(release: &ReleaseInfo, asset_name: &str) -> Result<String> {
    let sums_url = release
        .assets
        .iter()
        .find(|a| a.name == "SHA256SUMS")
        .map(|a| a.browser_download_url.clone())
        .ok_or_else(|| anyhow!("release has no SHA256SUMS asset"))?;
    let body = download_asset(&ReleaseAsset {
        name: "SHA256SUMS".into(),
        browser_download_url: sums_url,
        size: 0,
    })?;
    let text = std::str::from_utf8(&body)?;
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        if let Some(hash) = parts.next()
            && let Some(name) = parts.next()
            && name == asset_name
        {
            return Ok(hash.to_string());
        }
    }
    anyhow::bail!("SHA256SUMS has no entry for {asset_name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_sha256_matches_correct_hash() {
        let data = b"hello world";
        // sha256("hello world") = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
        verify_sha256(
            data,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
        )
        .unwrap();
    }

    #[test]
    fn verify_sha256_rejects_mismatch() {
        let data = b"hello world";
        assert!(
            verify_sha256(
                data,
                "0000000000000000000000000000000000000000000000000000000000000000"
            )
            .is_err()
        );
    }

    #[test]
    fn pick_asset_finds_matching_target() {
        let release = ReleaseInfo {
            tag_name: "v1.34.0".into(),
            assets: vec![ReleaseAsset {
                name: "archctl-x86_64-unknown-linux-gnu.tar.gz".into(),
                browser_download_url: "x".into(),
                size: 100,
            }],
        };
        let asset = pick_asset(&release).unwrap();
        assert_eq!(asset.name, "archctl-x86_64-unknown-linux-gnu.tar.gz");
    }

    #[test]
    fn pick_asset_returns_none_when_no_match() {
        let release = ReleaseInfo {
            tag_name: "v1.34.0".into(),
            assets: vec![ReleaseAsset {
                name: "archctl-other-platform.tar.gz".into(),
                browser_download_url: "x".into(),
                size: 100,
            }],
        };
        assert!(pick_asset(&release).is_none());
    }
}
