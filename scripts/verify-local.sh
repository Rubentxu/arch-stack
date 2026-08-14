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
if [ "$MODE" = "full" ]; then
    if ! command -v pnpm >/dev/null 2>&1; then
        echo "verify-local: pnpm not found; required for --full web gates" >&2
        exit 2
    fi
fi

# ---- Rust gates (cheap + full) ---------------------------------------------
(
    cd "$REPO_ROOT/archctl"
    run_gate cargo test --quiet
    run_gate cargo clippy --quiet --all-targets -- -D warnings
    run_gate cargo fmt --check
    # Doctor mirrors the CI step: build release, then run from repo root.
    run_gate cargo build --quiet --release
    run_gate "$REPO_ROOT/archctl/target/release/archctl" doctor --scopes code --cwd "$REPO_ROOT"
)

# ---- dependency fitness ratchet (P1-09, report-only with baseline) ---------
run_gate "$REPO_ROOT/scripts/check-dep-fitness.sh"

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
    run_gate "$REPO_ROOT/e2e/install_e2e.sh" --bin "$REPO_ROOT/archctl/target/release/archctl"
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
