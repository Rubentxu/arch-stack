#!/usr/bin/env bash
# verify-local.sh — tiered local verification before push/merge (ADR-025).
#
# Cheap mode (default) runs the Rust gates that pre-push needs:
#   cargo test, clippy -D warnings, fmt --check, doctor (code scope)
# --full additionally runs the web gates, the CI-gate contract tests, and
#   the ADR-019 benchmark regression comparison against a FRESH origin/main:
#   pnpm test, pnpm build, bundle cap <= 2MB gzipped,
#   scripts/test-ci-gates.sh, bench smoke,
#   scripts/bench-compare.sh <baseline>
#   The default baseline is origin/main, refreshed with `git fetch origin
#   main` before comparison so a stale baseline cannot silently pass. Pass
#   --baseline <ref> to compare against an explicit ref instead (no fetch).
# --full --check-branch-protection additionally queries the LIVE GitHub
#   branch protection via scripts/check-branch-protection.sh. It is a
#   network-dependent read-only check and is NEVER part of cheap pre-push.
#
# Usage:
#   scripts/verify-local.sh [--full] [--baseline <ref>] [--check-branch-protection] [--dry-run] [--help]
#
#   --baseline <ref>  benchmark baseline ref; default origin/main (fetched)
#   --dry-run         print the gates that would run without executing them
#                     (deterministic; used by scripts/test-ci-gates.sh)
#
# Exit codes:
#   0 = all gates passed
#   1 = a gate failed
#   2 = usage error or missing prerequisite
#
# The script never mutates tracked source: cargo/pnpm write only to
# gitignored build dirs (target/, dist/).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# Bootstrap archctl/assets-stack/ if absent (M33).
#
# The pre-push hook (`.githooks/pre-push`, ADR-025) checks out each pushed
# commit into a fresh worktree and runs `cargo test` against it. The worktree
# does NOT have archctl/assets-stack/ populated (it's gitignored; generated
# by `scripts/embed-stack.sh` for rust-embed distribution, ADR-033). Without
# this bootstrap, #[derive(RustEmbed)] fails to compile in the fresh worktree
# and `git push` is blocked for EVERY commit. The fix is to call the embed
# script early in verify-local.sh — it's idempotent (copies a fixed set of
# files; safe to re-run on every invocation).
ASSETS_STACK="${REPO_ROOT}/archctl/assets-stack"
if [ ! -d "${ASSETS_STACK}" ]; then
    if [ -x "${REPO_ROOT}/scripts/embed-stack.sh" ]; then
        bash "${REPO_ROOT}/scripts/embed-stack.sh"
    else
        echo "verify-local: archctl/assets-stack missing and scripts/embed-stack.sh not executable; push will fail in fresh worktree" >&2
    fi
fi

MODE="cheap"
DRY_RUN=0
CHECK_BP=0
BASELINE=""
while [ $# -gt 0 ]; do
    case "$1" in
        --full) MODE="full" ;;
        --check-branch-protection) CHECK_BP=1 ;;
        --baseline) BASELINE="${2:-}"; shift || true ;;
        --dry-run) DRY_RUN=1 ;;
        --help|-h) sed -n '1,40p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "verify-local: unknown argument '${1}' (expected --full, --baseline, --check-branch-protection, --dry-run or --help)" >&2; exit 2 ;;
    esac
    shift || true
done

if [ "$CHECK_BP" = "1" ] && [ "$MODE" != "full" ]; then
    echo "verify-local: --check-branch-protection requires --full" >&2
    exit 2
fi

run_gate() {
    if [ "$DRY_RUN" = "1" ]; then
        echo "[dry-run] $*"
        return 0
    fi
    echo "== $* =="
    "$@"
}

# ---- prerequisites ---------------------------------------------------------
if ! command -v cargo >/dev/null 2>&1; then
    echo "verify-local: cargo not found; install Rust via rustup" >&2
    exit 2
fi
# Resolve the archctl binary path: prefer $ARCHCTL_BIN env, then the
# cargo target-dir declared in ~/.cargo/config.toml (global override),
# then the legacy in-tree target. The in-tree path is stale on machines
# that set `target-dir` globally (the host's `archctl/target/release/`
# is an outdated v1.45.0 build that lies about capabilities). See
# docs/STATE.md "Known issues".
CARGO_TARGET_DIR_FROM_CONFIG=""
if [ -f "$HOME/.cargo/config.toml" ] && command -v python3 >/dev/null 2>&1; then
    CARGO_TARGET_DIR_FROM_CONFIG=$(python3 -c "
import sys, re
try:
    text = open('$HOME/.cargo/config.toml').read()
except OSError:
    sys.exit(0)
m = re.search(r'^target-dir\s*=\s*\"([^\"]+)\"', text, re.M)
if m: print(m.group(1))
")
fi
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${CARGO_TARGET_DIR_FROM_CONFIG:-${REPO_ROOT}/archctl/target}}"
ARCHCTL_REAL="${ARCHCTL_BIN:-${CARGO_TARGET_DIR}/release/archctl}"
if [ ! -x "$ARCHCTL_REAL" ]; then
    echo "verify-local: archctl binary not found at $ARCHCTL_REAL" >&2
    echo "  hint: cargo build --release, or set ARCHCTL_BIN to the right path" >&2
    exit 2
fi
if [ "$MODE" = "full" ]; then
    if ! command -v pnpm >/dev/null 2>&1; then
        echo "verify-local: pnpm not found; required for --full web gates" >&2
        exit 2
    fi
fi

# ---- Rust gates (cheap + full) ---------------------------------------------
(
    cd "$REPO_ROOT/archctl"
    run_gate cargo test --features test-fixtures --quiet
    run_gate cargo clippy --quiet --all-targets --features test-fixtures -- -D warnings
    run_gate cargo fmt --check
    # Doctor mirrors the CI step: build release, then run from repo root.
    run_gate cargo build --quiet --release
    run_gate "$ARCHCTL_REAL" doctor --scopes code --cwd "$REPO_ROOT"
)

# ---- dependency fitness ratchet (P1-09, report-only with baseline) ---------
run_gate "$REPO_ROOT/scripts/check-dep-fitness.sh"

# ---- capability registry staleness gate (P1-08) ------------------------------
# Regenerate markdown to a temp file and diff against the committed docs/CAPABILITIES.md.
# The binary uses the process cwd; run from repo root so docs/CAPABILITIES.md resolves.
run_gate bash -c '
    '"$ARCHCTL_REAL"' capabilities --format markdown > /tmp/capabilities_fresh.md || exit 1
    diff -q docs/CAPABILITIES.md /tmp/capabilities_fresh.md || { echo "verify-local: docs/CAPABILITIES.md is stale. Run: archctl capabilities --format markdown > docs/CAPABILITIES.md"; exit 1; }
'

if [ "$MODE" = "full" ]; then
    # ---- fresh baseline: fetch origin/main unless --baseline given ----
    if [ -z "$BASELINE" ]; then
        if [ "$DRY_RUN" = "1" ]; then
            echo "[dry-run] git fetch origin main"
        elif ! git fetch origin main >/dev/null 2>&1; then
            echo "verify-local: cannot fetch origin/main; refusing stale baseline comparison" >&2
            exit 2
        fi
        BASELINE="origin/main"
    fi

    # ---- web gates (archview) ------------------------------------------------
    (
        cd "$REPO_ROOT/archview"
        run_gate pnpm test
        run_gate pnpm build
    )
    # ADR-019 bundle cap: single-source script, shared with CI.
    run_gate "$REPO_ROOT/scripts/check-bundle-cap.sh" "$REPO_ROOT/archview/dist/assets/*.js"

    # ---- archview perf gate (ADR-019, M23 perf-ci-gate) -----------------------
    # Run real perf measurement when playwright is available (pnpm exec playwright --version succeeds).
    # Uses --fake-ttfp-regression 0 --fake-fps-regression 0: no synthetic regression,
    # real measurement against the current tree vs the baseline SHA.
    if pnpm exec playwright --version >/dev/null 2>&1; then
        run_gate "$REPO_ROOT/scripts/bench-compare-archview.sh" "$BASELINE"
    else
        echo "verify-local: archview perf gate skipped: playwright not installed (install: pnpm add -D playwright && pnpm exec playwright install chromium)"
    fi

    # ---- CI-gate contract tests (deterministic; enforces this suite) ---------
    run_gate "$REPO_ROOT/scripts/test-ci-gates.sh"

    # ---- benchmark gates (ADR-019) -------------------------------------------
    (
        cd "$REPO_ROOT/archctl"
        run_gate cargo bench --bench export_pipeline -- --quick
    )
    run_gate "$REPO_ROOT/scripts/bench-compare.sh" "$BASELINE"

    # ---- E2E suites (M29, ADR-034) -------------------------------------------
    # Install E2E: always (no external deps beyond the release binary).
    run_gate "$REPO_ROOT/e2e/install_e2e.sh" --bin "$ARCHCTL_REAL"
    # Render E2E: only when playwright is available.
    if python3 -c "import playwright" 2>/dev/null; then
        run_gate python3 "$REPO_ROOT/e2e/render_e2e.py" --samples-only
    else
        echo "verify-local: playwright not found; skipping render E2E (install: pip install playwright && playwright install chromium)"
    fi
    # Sandbox E2E: only when podman is available (needs network for image).
    if command -v podman >/dev/null 2>&1; then
        run_gate "$REPO_ROOT/bench/sandbox-e2e.sh" --no-build
    else
        echo "verify-local: podman not found; skipping sandbox E2E"
    fi

    # ---- live branch protection (explicit opt-in, never cheap pre-push) ------
    if [ "$CHECK_BP" = "1" ]; then
        run_gate "$REPO_ROOT/scripts/check-branch-protection.sh"
    fi
fi

echo "verify-local: ${MODE} mode PASS"
exit 0
