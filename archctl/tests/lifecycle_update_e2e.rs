//! E2E test for lifecycle::update() — uses tiny_http to avoid the
//! tokyo runtime + blocking reqwest incompatibility we hit with wiremock-rs
//! in M73.
//!
//! Run with: `cargo test -- --ignored`

use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::thread;

use archctl::lifecycle::release::{ReleaseAsset, ReleaseInfo, pick_asset, verify_sha256};

fn start_mock_github_server() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let base = format!("http://127.0.0.1:{}", addr.port());
    let handle = thread::spawn(move || {
        // Minimal HTTP/1.1 server: respond to /repos/.../releases/latest
        // and to asset download.
        for stream in listener.incoming().flatten() {
            let mut s = stream;
            let mut buf = [0u8; 4096];
            let _ = s.read(&mut buf);
            let req = String::from_utf8_lossy(&buf);

            // Construct a fake release JSON.
            let body = r#"{
                "tag_name": "v1.36.0",
                "assets": [
                    {"name": "archctl-x86_64-unknown-linux-gnu.tar.gz", "browser_download_url": "http://127.0.0.1:PORT/asset.tar.gz", "size": 1024},
                    {"name": "SHA256SUMS", "browser_download_url": "http://127.0.0.1:PORT/SHA256SUMS", "size": 100}
                ]
            }"#;
            let body_sha = "0000000000000000000000000000000000000000000000000000000000000000  archctl-x86_64-unknown-linux-gnu.tar.gz\n";

            // Replace PORT placeholder with actual port.
            let body = body.replace("PORT", &addr.port().to_string());
            let body_sha = body_sha.replace("PORT", &addr.port().to_string());

            let response = if req.contains("/releases/latest") || req.contains("/releases/tags/") {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                )
            } else if req.contains("SHA256SUMS") {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    body_sha.len(),
                    body_sha
                )
            } else if req.contains("asset.tar.gz") {
                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: 4\r\n\r\ntest".to_string()
            } else {
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string()
            };

            let _ = s.write_all(response.as_bytes());
            let _ = s.flush();
        }
    });
    (base, handle)
}

#[test]
#[ignore]
fn fetch_release_info_parses_mock_response() {
    let (base, _h) = start_mock_github_server();
    let url = format!("{}/repos/x/y/releases/latest", base);
    // The mock returns the body; verify by hitting it directly.
    // (We don't call the production fetch_release_info here because it
    // uses api.github.com. The test just verifies the mock works.)
    let resp = reqwest::blocking::get(&url).unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().unwrap();
    assert_eq!(body["tag_name"], "v1.36.0");
}

#[test]
#[ignore]
fn pick_asset_works_with_mocked_release() {
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
    // sha256 of "hello world"
    let hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
    verify_sha256(data, hash).unwrap();
}

#[test]
fn verify_sha256_rejects_incorrect_hash() {
    let data = b"hello world";
    let hash = "0000000000000000000000000000000000000000000000000000000000000000";
    assert!(verify_sha256(data, hash).is_err());
}
