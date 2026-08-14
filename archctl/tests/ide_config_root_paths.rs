//! E2E regression test for M77a bug fix.
//!
//! Background: v1.37.0 used `xdg::xdg_config_home()` for Claude Code and
//! Codex adapters, which writes to `~/.config/claude/` and `~/.config/codex/`.
//! Neither IDE looks there — Claude Code reads `~/.claude/`, Codex reads
//! `~/.codex/`. The install reported success but the files were invisible
//! to the IDEs.
//!
//! This test pins the `config_root()` for each adapter to the path the
//! corresponding IDE actually reads. If anyone changes the adapter logic
//! in a way that re-introduces XDG paths for Claude/Codex, this test
//! fails with a clear error message pointing at the M77a hotfix.
//!
//! The test uses a synthetic `$HOME` so it doesn't depend on the host
//! environment, and verifies the path shape (HOME-relative vs XDG) per
//! the per-IDE expectations documented in the install path decisions
//! (OpenCode + ZCode respect XDG; Claude Code + Codex do not).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use archctl::ide::IdeAdapter;
use archctl::ide::claude_code::ClaudeCodeAdapter as CC;
use archctl::ide::codex::CodexAdapter as Codex;
use archctl::ide::opencode::OpenCodeAdapter as OC;
use archctl::ide::zcode::ZCodeAdapter as ZC;

// Tests in this file mutate process-global env vars (HOME,
// XDG_CONFIG_HOME). `cargo test` runs tests within a binary in PARALLEL
// by default, so they must be serialized against each other.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Run `body` with `$HOME` set to `home` and `$XDG_CONFIG_HOME` set to
/// `<home>/.config`, restoring the previous values afterwards. Pinning
/// both vars makes the test deterministic: `xdg::xdg_config_home()`
/// resolves `XDG_CONFIG_HOME` first, so a runner-provided value (GitHub
/// Actions sets it) would otherwise break the "under $HOME" assertion.
/// Uses `catch_unwind` so a panic in one test doesn't leave the env
/// corrupted for subsequent tests.
fn with_home<F: FnOnce()>(home: &Path, body: F) {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prev_home = std::env::var_os("HOME");
    let prev_xdg = std::env::var_os("XDG_CONFIG_HOME");
    let xdg = home.join(".config");
    // SAFETY: all tests in this file serialize on ENV_LOCK, so no
    // concurrent env mutation can race with this one.
    unsafe {
        std::env::set_var("HOME", home);
        std::env::set_var("XDG_CONFIG_HOME", &xdg);
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
    unsafe {
        match prev_home {
            Some(p) => std::env::set_var("HOME", p),
            None => std::env::remove_var("HOME"),
        }
        match prev_xdg {
            Some(p) => std::env::set_var("XDG_CONFIG_HOME", p),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

/// Strip the leading `$HOME` and any XDG prefix to produce the path
/// shape (e.g. `/.config/opencode` or `/.claude`). Used for assertions.
fn relative_to_home(p: &Path, home: &Path) -> Option<PathBuf> {
    p.strip_prefix(home).ok().map(|x| x.to_path_buf())
}

#[test]
fn opencode_config_root_is_xdg_compliant() {
    with_home(Path::new("/tmp/m77a-test"), || {
        let r = OC.config_root();
        let rel = relative_to_home(&r, Path::new("/tmp/m77a-test"))
            .expect("config_root should be under $HOME");
        assert_eq!(
            rel,
            PathBuf::from(".config/opencode"),
            "OpenCodeAdapter config_root drifted from XDG path. \
             OpenCode DOES respect XDG_CONFIG_HOME per its docs. \
             If you changed this to ~/.opencode, the IDE will not find the files. \
             See M77a (commit 2f5f573)."
        );
    });
}

#[test]
fn zcode_config_root_is_xdg_compliant_alias_of_opencode() {
    with_home(Path::new("/tmp/m77a-test"), || {
        let r = ZC.config_root();
        let rel = relative_to_home(&r, Path::new("/tmp/m77a-test"))
            .expect("config_root should be under $HOME");
        assert_eq!(
            rel,
            PathBuf::from(".config/opencode"),
            "ZCodeAdapter config_root drifted. ZCode is a fork of \
             OpenCode and shares the same XDG discovery path."
        );
    });
}

#[test]
fn claude_code_config_root_is_home_relative_not_xdg() {
    with_home(Path::new("/tmp/m77a-test"), || {
        let r = CC.config_root();
        let rel = relative_to_home(&r, Path::new("/tmp/m77a-test"))
            .expect("config_root should be under $HOME");
        assert_eq!(
            rel,
            PathBuf::from(".claude"),
            "REGRESSION: ClaudeCodeAdapter is now writing to {:?}. \
             Claude Code does NOT respect XDG_CONFIG_HOME — it looks at \
             ~/.claude/ directly. If this test fails, you have re-introduced \
             the v1.37.0 bug where 'install 14 skills for Claude Code' \
             wrote to ~/.config/claude/ where the IDE never reads. \
             See M77a (commit 2f5f573).",
            rel
        );
        // Negative assertion: explicitly fail if someone re-introduces XDG.
        assert!(
            !rel.to_string_lossy().contains(".config/"),
            "ClaudeCodeAdapter config_root contains '.config/': {:?} \
             — this is the v1.37.0 bug!",
            rel
        );
    });
}

#[test]
fn codex_config_root_is_home_relative_not_xdg() {
    with_home(Path::new("/tmp/m77a-test"), || {
        let r = Codex.config_root();
        let rel = relative_to_home(&r, Path::new("/tmp/m77a-test"))
            .expect("config_root should be under $HOME");
        assert_eq!(
            rel,
            PathBuf::from(".codex"),
            "REGRESSION: CodexAdapter is now writing to {:?}. \
             Codex CLI does NOT respect XDG_CONFIG_HOME — it looks at \
             ~/.codex/ directly. If this test fails, you have re-introduced \
             the v1.37.0 bug. See M77a (commit 2f5f573).",
            rel
        );
        // Negative assertion: explicitly fail if someone re-introduces XDG.
        assert!(
            !rel.to_string_lossy().contains(".config/"),
            "CodexAdapter config_root contains '.config/': {:?} \
             — this is the v1.37.0 bug!",
            rel
        );
    });
}

/// Smoke: confirm the synthetic `$HOME` propagates (catches a class of
/// regressions where `dirs`/`home` caches the value at startup).
#[test]
fn home_env_propagates_through_helper() {
    with_home(Path::new("/tmp/synthetic-home"), || {
        assert_eq!(
            std::env::var("HOME").as_deref(),
            Ok("/tmp/synthetic-home"),
            "with_home helper did not set $HOME correctly"
        );
    });
    // And the previous value is restored after the helper exits.
    let prev = std::env::var("HOME").unwrap_or_default();
    assert_ne!(prev, "/tmp/synthetic-home", "HOME was not restored");
}
