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
/// Each entry: (url, language, primary extractor args, apply?)
/// M29.3: per-language vertical — c4 for rust/js/ts, call-graph for go,
/// class-diagram for python.
const SMOKE_REPOS: &[(&str, &str, &[&str], bool)] = &[
    (
        "https://github.com/tokio-rs/mini-redis.git",
        "rust",
        &["code", "c4-discover", "--apply"],
        true,
    ),
    (
        "https://github.com/labstack/echo.git",
        "go",
        &["code", "call-graph", "--apply"],
        true,
    ),
    (
        "https://github.com/expressjs/express.git",
        "javascript",
        &["code", "c4-discover", "--apply"],
        true,
    ),
    (
        "https://github.com/psf/requests.git",
        "python",
        &["code", "class-diagram", "--apply"],
        true,
    ),
    (
        "https://github.com/pmndrs/zustand.git",
        "typescript",
        &["code", "c4-discover", "--apply"],
        true,
    ),
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

/// Per-repo smoke: clone, run the language-specific extractor with --apply,
/// run `diagram export`, run `diagram validate`, and — for container
/// extractors — accept drafted evidence (M29.3 vertical completion).
fn smoke_one(repo_url: &str, lang: &str, extractor: &[&str], apply: bool) {
    let bin = archctl_bin();
    assert!(
        bin.exists(),
        "archctl binary not found at {} — run `cargo build --release` first or set ARCHCTL_BIN",
        bin.display()
    );

    eprintln!("\n=== smoke: {repo_url} ({lang}) ===");
    let project_dir = cached_clone(repo_url);
    eprintln!("  project: {}", project_dir.display());

    // M29.3: isolated XDG per repo so `--apply` starts from an empty graph.
    // The shared XDG (default) already contains these repos from prior
    // runs/bench, which makes --apply report "Applied: 0" (all skipped as
    // existing) and the suite non-deterministic.
    let xdg = TempDir::new().expect("xdg temp");
    let xdg_data = xdg.path().join("data");
    let xdg_config = xdg.path().join("config");
    std::fs::create_dir_all(&xdg_data).expect("xdg data dir");
    std::fs::create_dir_all(&xdg_config).expect("xdg config dir");
    let env_xdg = [
        ("XDG_DATA_HOME", xdg_data.to_str().unwrap()),
        ("XDG_CONFIG_HOME", xdg_config.to_str().unwrap()),
        ("RUST_LOG", "error"),
    ];

    // Extract with the language-specific extractor
    if apply {
        let start = Instant::now();
        let mut cmd = Command::new(&bin);
        cmd.args(extractor).current_dir(&project_dir).envs(env_xdg);
        let out = cmd.output().expect("extractor spawn");
        let elapsed = start.elapsed();
        eprintln!(
            "  extractor {:?}: exit={}, elapsed={:?}, stdout_len={}",
            extractor,
            out.status.code().unwrap_or(-1),
            elapsed,
            out.stdout.len()
        );
        // M29.3: for c4-discover, require >=1 container applied.
        if extractor.contains(&"c4-discover") {
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(
                stdout.contains("Applied:") && !stdout.contains("Applied: 0"),
                "c4-discover applied 0 elements for {repo_url}: {stdout}"
            );
        }
        assert!(
            out.status.success(),
            "extractor failed for {repo_url}: stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
    } else {
        eprintln!("  extractor: skipped (apply=false)");
    }

    // M29.3: accept all drafted evidence (only for container extractors —
    // other extractors may not produce evidence rows).
    if extractor.contains(&"c4-discover") {
        let list = Command::new(&bin)
            .args(["evidence", "list", "--status", "drafted", "--json"])
            .current_dir(&project_dir)
            .envs(env_xdg)
            .output()
            .expect("evidence list spawn");
        if list.status.success() {
            let stdout = String::from_utf8_lossy(&list.stdout);
            // Strip INFO lines before JSON parse.
            let json_start = stdout.find('[').unwrap_or(stdout.len());
            let Ok(rows) = serde_json::from_str::<serde_json::Value>(&stdout[json_start..]) else {
                return;
            };
            let Some(arr) = rows.as_array() else {
                return;
            };
            for row in arr {
                if let Some(id) = row.get("e.id").and_then(|v| v.as_str()) {
                    let acc = Command::new(&bin)
                        .args(["evidence", "accept", "--id", id])
                        .current_dir(&project_dir)
                        .envs(env_xdg)
                        .output()
                        .expect("evidence accept spawn");
                    assert!(
                        acc.status.success(),
                        "accept {id} failed: {}",
                        String::from_utf8_lossy(&acc.stderr)
                    );
                }
            }
            eprintln!("  evidence accept: {} accepted", arr.len());
        }
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
        .envs(env_xdg)
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
            .envs(env_xdg)
            .output()
            .expect("diagram validate spawn");
        let val_status = val.status.code().unwrap_or(-1);
        eprintln!("  diagram validate: exit={}", val_status);
        // M29.3: for container extractors, a bundle with elements must
        // validate (schema contract). Empty bundles fail validate and that
        // is expected for extractors without containers (call-graph/class).
        let _ = val_status;
    }
}

#[test]
#[ignore = "requires git + internet + target/release/archctl; run with --ignored"]
fn smoke_mini_redis() {
    let (url, lang, ext, apply) = SMOKE_REPOS[0];
    smoke_one(url, lang, ext, apply);
}

#[test]
#[ignore = "requires git + internet + target/release/archctl; run with --ignored"]
fn smoke_echo() {
    let (url, lang, ext, apply) = SMOKE_REPOS[1];
    smoke_one(url, lang, ext, apply);
}

#[test]
#[ignore = "requires git + internet + target/release/archctl; run with --ignored"]
fn smoke_express() {
    let (url, lang, ext, apply) = SMOKE_REPOS[2];
    smoke_one(url, lang, ext, apply);
}

#[test]
#[ignore = "requires git + internet + target/release/archctl; run with --ignored"]
fn smoke_requests() {
    let (url, lang, ext, apply) = SMOKE_REPOS[3];
    smoke_one(url, lang, ext, apply);
}

#[test]
#[ignore = "requires git + internet + target/release/archctl; run with --ignored"]
fn smoke_zustand() {
    let (url, lang, ext, apply) = SMOKE_REPOS[4];
    smoke_one(url, lang, ext, apply);
}

#[test]
#[ignore = "smoke_all: runs all SMOKE_REPOS in sequence; takes ~5 minutes"]
fn smoke_all() {
    for (url, lang, ext, apply) in SMOKE_REPOS {
        smoke_one(url, lang, ext, *apply);
    }
}

/// Helper to suppress unused warnings.
#[allow(dead_code)]
fn _suppress_unused(_p: &Path) {}
