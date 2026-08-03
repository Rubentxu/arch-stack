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
#   scripts/verify-local.sh [--full] [--help]
#
# Exit codes:
#   0 = all gates passed
#   1 = a gate failed
#   2 = usage error or missing prerequisite
#
# Env:
#   VERIFY_LOCAL_DRY_RUN=1  print the gates that would run without executing
#                           (used by scripts/test-ci-gates.sh; never mutates)
#
# The script never mutates tracked source: cargo/pnpm write only to
# gitignored build dirs (target/, dist/).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

MODE="cheap"
if [ "${1:-}" = "--full" ]; then
    MODE="full"
elif [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    sed -n '1,30p' "$0" | sed 's/^# \{0,1\}//'
    exit 0
elif [ $# -gt 0 ]; then
    echo "verify-local: unknown argument '${1}' (expected --full or --help)" >&2
    exit 2
fi

run_gate() {
    if [ "${VERIFY_LOCAL_DRY_RUN:-0}" = "1" ]; then
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
    # ADR-019 bundle cap: gzipped JS <= 2MB.
    if [ "${VERIFY_LOCAL_DRY_RUN:-0}" = "1" ]; then
        echo "[dry-run] bundle cap check (gzipped dist/assets/*.js <= 2MB)"
    else
        total=0
        for f in "$REPO_ROOT"/archview/dist/assets/*.js; do
            [ -f "$f" ] || continue
            size=$(gzip -c "$f" | wc -c)
            echo "$f: $size bytes gzipped"
            total=$((total + size))
        done
        echo "total gzipped: $total bytes"
        limit=$((2 * 1024 * 1024))
        if [ "$total" -gt "$limit" ]; then
            echo "::error::Bundle exceeds ADR-019 cap (2MB gzipped): $total > $limit"
            exit 1
        fi
        echo "Bundle within ADR-019 cap."
    fi

    # ---- benchmark gates (ADR-019) -------------------------------------------
    (
        cd "$REPO_ROOT/archctl"
        run_gate cargo bench --bench export_pipeline -- --quick
    )
    run_gate "$REPO_ROOT/scripts/bench-compare.sh" origin/main
fi

echo "verify-local: ${MODE} mode PASS"
exit 0
