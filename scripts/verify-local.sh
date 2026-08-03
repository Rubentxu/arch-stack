#!/usr/bin/env bash
# verify-local.sh — tiered local verification before push/merge (ADR-025).
#
# Cheap mode (default) runs the Rust gates that pre-push needs:
#   cargo test, clippy -D warnings, fmt --check, doctor (code scope)
# --full additionally runs the web gates and the ADR-019 benchmark
#   regression comparison against origin/main:
#   pnpm test, pnpm build, bundle cap <= 2MB gzipped, bench smoke,
#   scripts/bench-compare.sh origin/main
#
# Usage:
#   scripts/verify-local.sh [--full] [--dry-run] [--help]
#
#   --dry-run  print the gates that would run without executing them
#              (deterministic; used by scripts/test-ci-gates.sh; never mutates)
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

MODE="cheap"
DRY_RUN=0
for arg in "$@"; do
    case "$arg" in
        --full) MODE="full" ;;
        --dry-run) DRY_RUN=1 ;;
        --help|-h) sed -n '1,30p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "verify-local: unknown argument '${arg}' (expected --full, --dry-run or --help)" >&2; exit 2 ;;
    esac
done

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

if [ "$MODE" = "full" ]; then
    # ---- web gates (archview) ------------------------------------------------
    (
        cd "$REPO_ROOT/archview"
        run_gate pnpm test
        run_gate pnpm build
    )
    # ADR-019 bundle cap: single-source script, shared with CI.
    run_gate "$REPO_ROOT/scripts/check-bundle-cap.sh" "$REPO_ROOT/archview/dist/assets/*.js"

    # ---- benchmark gates (ADR-019) -------------------------------------------
    (
        cd "$REPO_ROOT/archctl"
        run_gate cargo bench --bench export_pipeline -- --quick
    )
    run_gate "$REPO_ROOT/scripts/bench-compare.sh" origin/main
fi

echo "verify-local: ${MODE} mode PASS"
exit 0
