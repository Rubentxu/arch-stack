//! Integration tests for `archctl::lifecycle::release`.
//!
//! These exercise the public API of the release module: `pick_asset`
//! (selects the matching tarball among release assets) and `verify_sha256`
//! (hash check after download). Pure functions with no I/O, so they run
//! under the normal `cargo test` invocation.

use archctl::lifecycle::release::{ReleaseAsset, ReleaseInfo, pick_asset, verify_sha256};

#[test]
fn pick_asset_prefers_targz_over_shasums() {
    // pick_asset should select the tarball (matching the current target
    // triple), not the SHA256SUMS companion file. Verifies the preference
    // logic on a 2-asset release where the order matters.
    let release = ReleaseInfo {
        tag_name: "v1.36.0".into(),
        assets: vec![
            ReleaseAsset {
                name: "archctl-x86_64-unknown-linux-gnu.tar.gz".into(),
                browser_download_url: "http://example.com/asset.tar.gz".into(),
                size: 1024,
            },
            ReleaseAsset {
                name: "SHA256SUMS".into(),
                browser_download_url: "http://example.com/SHA256SUMS".into(),
                size: 100,
            },
        ],
    };
    let asset = pick_asset(&release);
    assert!(asset.is_some());
    assert_eq!(
        asset.unwrap().name,
        "archctl-x86_64-unknown-linux-gnu.tar.gz"
    );
}

#[test]
fn verify_sha256_accepts_correct_hash() {
    let data = b"hello world";
    // sha256("hello world") = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
    verify_sha256(
        data,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
    )
    .unwrap();
}

#[test]
fn verify_sha256_rejects_incorrect_hash() {
    let data = b"hello world";
    let hash = "0000000000000000000000000000000000000000000000000000000000000000";
    assert!(verify_sha256(data, hash).is_err());
}
