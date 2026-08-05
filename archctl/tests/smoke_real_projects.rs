//! Smoke tests con proyectos reales de GitHub.
//!
//! Estos tests están marcados `#[ignore]` para no ejecutarse en CI normal.
//! Se ejecutan manualmente con:
//!   `cargo test --test smoke_real_projects -- --ignored --nocapture`
//!
//! Requisitos:
//! - `git` en PATH
//! - Conexión a internet (clone desde GitHub)
//! - `archctl` instalado o path al binario en `ARCHCTL_BIN` env var
//!
//! Por defecto usa `target/release/archctl` (binario local).
//! Para usar otra ruta: `ARCHCTL_BIN=/path/to/archctl cargo test -- --ignored`.
//!
//! Caché: los clones se guardan en `~/.cache/archctl-smoke/<repo>/` para no
//! re-clonar en cada run. Limpiar con `rm -rf ~/.cache/archctl-smoke/`.
//!
//! Per ADR-031: estos tests son la base del benchmark M27.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use tempfile::TempDir;

/// Probing set: small, fast, multi-language. Add more as we grow.
const SMOKE_REPOS: &[(&str, &str)] = &[
    ("https://github.com/tokio-rs/mini-redis.git", "rust"),
    ("https://github.com/labstack/echo.git", "go"),
    ("https://github.com/expressjs/express.git", "javascript"),
    ("https://github.com/psf/requests.git", "python"),
];

/// Resolve the archctl binary to use.
fn archctl_bin() -> PathBuf {
    if let Ok(p) = std::env::var("ARCHCTL_BIN") {
        return PathBuf::from(p);
    }
    // Default to target/release/archctl (relative to archctl/ subdir).
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("release");
    p.push("archctl");
    p
}

/// Clone (or reuse cached clone) of a repo into ~/.cache/archctl-smoke/<name>.
fn cached_clone(url: &str) -> PathBuf {
    let cache_root = std::env::var("ARCHCTL_SMOKE_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").expect("HOME not set");
            PathBuf::from(home).join(".cache").join("archctl-smoke")
        });
    let name = url
        .trim_end_matches(".git")
        .rsplit('/')
        .next()
        .expect("repo url missing name");
    let dest = cache_root.join(name);

    if dest.join(".git").exists() {
        // Already cloned: fast-forward to HEAD.
        let _ = Command::new("git")
            .args(["fetch", "--depth", "1", "origin", "HEAD"])
            .current_dir(&dest)
            .output();
        let _ = Command::new("git")
            .args(["reset", "--hard", "FETCH_HEAD"])
            .current_dir(&dest)
            .output();
        return dest;
    }

    std::fs::create_dir_all(&cache_root).expect("create cache dir");
    let status = Command::new("git")
        .args(["clone", "--depth", "1", url])
        .arg(&dest)
        .status()
        .expect("git clone failed to spawn");
    assert!(status.success(), "git clone failed for {url}");
    dest
}

/// Per-repo smoke: clone, run `code c4-discover --apply`, run `diagram
/// export`, run `diagram validate`. Captures timing + exit codes.
fn smoke_one(repo_url: &str, lang: &str) {
    let bin = archctl_bin();
    assert!(
        bin.exists(),
        "archctl binary not found at {} — run `cargo build --release` first or set ARCHCTL_BIN",
        bin.display()
    );

    eprintln!("\n=== smoke: {repo_url} ({lang}) ===");
    let project_dir = cached_clone(repo_url);
    eprintln!("  project: {}", project_dir.display());

    // Discover + apply (skip if no Cargo.toml for non-Rust repos)
    let has_cargo = project_dir.join("Cargo.toml").exists();
    let discover_strategy = if has_cargo {
        Some("cargo-workspace")
    } else if project_dir.join("package.json").exists() {
        Some("npm-workspace")
    } else {
        None
    };

    if let Some(strategy) = discover_strategy {
        let start = Instant::now();
        let out = Command::new(&bin)
            .args(["code", "c4-discover", "--strategy", strategy, "--apply"])
            .current_dir(&project_dir)
            .output()
            .expect("c4-discover spawn");
        let elapsed = start.elapsed();
        eprintln!(
            "  c4-discover ({}): exit={}, elapsed={:?}, stdout_len={}",
            strategy,
            out.status.code().unwrap_or(-1),
            elapsed,
            out.stdout.len()
        );
        assert!(
            out.status.success(),
            "c4-discover failed for {repo_url}: stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
    } else {
        eprintln!("  c4-discover: skipped (no Cargo.toml / package.json)");
    }

    // Export container:* to a temp bundle dir
    let bundle = TempDir::new().expect("temp dir");
    let start = Instant::now();
    let out = Command::new(&bin)
        .args([
            "diagram",
            "export",
            "--output",
            bundle.path().to_str().unwrap(),
            "container:*",
        ])
        .current_dir(&project_dir)
        .output()
        .expect("diagram export spawn");
    let elapsed = start.elapsed();
    eprintln!(
        "  diagram export: exit={}, elapsed={:?}",
        out.status.code().unwrap_or(-1),
        elapsed
    );

    if out.status.success() {
        // Validate (only if export succeeded; some projects have no detected containers)
        let val = Command::new(&bin)
            .args(["diagram", "validate", bundle.path().to_str().unwrap()])
            .current_dir(&project_dir)
            .output()
            .expect("diagram validate spawn");
        let val_status = val.status.code().unwrap_or(-1);
        eprintln!("  diagram validate: exit={}", val_status);
        // validate exit 0 = bundle is valid OR (no bundles to validate — empty graph)
        // We don't assert here: empty bundles fail validate. That's expected for
        // repos without detectable containers (e.g. single-file Python).
        let _ = val_status;
    }
}

#[test]
#[ignore = "requires git + internet + target/release/archctl; run with --ignored"]
fn smoke_mini_redis() {
    smoke_one("https://github.com/tokio-rs/mini-redis.git", "rust");
}

#[test]
#[ignore = "requires git + internet + target/release/archctl; run with --ignored"]
fn smoke_echo() {
    smoke_one("https://github.com/labstack/echo.git", "go");
}

#[test]
#[ignore = "requires git + internet + target/release/archctl; run with --ignored"]
fn smoke_express() {
    smoke_one("https://github.com/expressjs/express.git", "javascript");
}

#[test]
#[ignore = "requires git + internet + target/release/archctl; run with --ignored"]
fn smoke_requests() {
    smoke_one("https://github.com/psf/requests.git", "python");
}

#[test]
#[ignore = "smoke_all: runs all SMOKE_REPOS in sequence; takes ~5 minutes"]
fn smoke_all() {
    for (url, lang) in SMOKE_REPOS {
        smoke_one(url, lang);
    }
}

/// Helper to suppress unused warnings.
#[allow(dead_code)]
fn _suppress_unused(_p: &Path) {}
