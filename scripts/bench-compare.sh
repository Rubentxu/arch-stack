#!/usr/bin/env bash
# bench-compare.sh — ADR-019 regression gate: compare current branch's
# criterion benchmarks against a `main` baseline. Exits 1 if any benchmark
# group degrades more than THRESHOLD_PCT (default 10%).
#
# Usage:
#   scripts/bench-compare.sh [--help]
#
# Env:
#   BENCH_NAME     benchmark binary (default: export_pipeline)
#   THRESHOLD_PCT  regression threshold (default: 10)
#   BENCH_DIR      crate dir relative to repo root (default: archctl)
#   TEST_FAKE_REGRESSION  if set to N>0, fake a synthetic N% regression
#                         (test mode — no real worktree/bench runs)
#
# The script:
#   1. Benchmarks `main` in a temporary worktree (--save-baseline main)
#   2. Benchmarks the current tree (--save-baseline pr)
#   3. Compares median.point_estimate (ns) per group; fails if pr > main*1.10
#
# Exit codes: 0 = no regression, 1 = regression detected, 2 = usage/error.

set -euo pipefail

if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    sed -n '1,25p' "$0" | sed 's/^# \{0,1\}//'
    exit 0
fi

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

BENCH_NAME="${BENCH_NAME:-export_pipeline}"
THRESHOLD_PCT="${THRESHOLD_PCT:-10}"
BENCH_DIR="${BENCH_DIR:-archctl}"
# Sampling: --quick is too noisy for a 10% threshold (measured ~20% run-to-run
# jitter). Moderate sampling (~3% jitter) keeps the gate meaningful while
# staying fast enough for CI.
SAMPLE_SIZE="${SAMPLE_SIZE:-30}"
MEASUREMENT_TIME="${MEASUREMENT_TIME:-5}"
WARM_UP_TIME="${WARM_UP_TIME:-1}"
WORKTREE_DIR="$(mktemp -d /tmp/bench-compare.XXXXXX)"
trap 'rm -rf "$WORKTREE_DIR"' EXIT

# ---- test mode -----------------------------------------------------------
if [ -n "${TEST_FAKE_REGRESSION:-}" ]; then
    echo "TEST MODE: fake regression ${TEST_FAKE_REGRESSION}%"
    groups=("export_base_revision_hash" "export_query_elements_small")
    for g in "${groups[@]}"; do
        base=$((RANDOM % 10000 + 1000))
        pr=$((base * (100 + TEST_FAKE_REGRESSION) / 100))
        echo "$g: main=${base}ns pr=${pr}ns delta=+${TEST_FAKE_REGRESSION}%"
    done
    # Deterministic rule: the requested fake delta IS the simulated delta.
    if [ "$TEST_FAKE_REGRESSION" -gt "$THRESHOLD_PCT" ]; then
        echo "REGRESSION DETECTED: +${TEST_FAKE_REGRESSION}% > ${THRESHOLD_PCT}%"
        exit 1
    fi
    echo "OK: within threshold"
    exit 0
fi

# ---- 1. benchmark main in a worktree -------------------------------------
echo "== Benchmarking main (baseline) =="
git worktree add --detach "$WORKTREE_DIR/main" origin/main >/dev/null
(
    cd "$WORKTREE_DIR/main/$BENCH_DIR"
    cargo bench --bench "$BENCH_NAME" -- --sample-size "$SAMPLE_SIZE" \
        --measurement-time "$MEASUREMENT_TIME" --warm-up-time "$WARM_UP_TIME" \
        --save-baseline main 2>/dev/null
)

# ---- 2. benchmark current tree -------------------------------------------
echo "== Benchmarking current branch (pr) =="
(
    cd "$REPO_ROOT/$BENCH_DIR"
    cargo bench --bench "$BENCH_NAME" -- --sample-size "$SAMPLE_SIZE" \
        --measurement-time "$MEASUREMENT_TIME" --warm-up-time "$WARM_UP_TIME" \
        --save-baseline pr 2>/dev/null
)

# ---- 3. copy main baselines from the worktree -----------------------------
# criterion writes each run's baseline into the tree that ran it; the worktree
# is ephemeral, so copy its `main/` estimates into the current tree's
# criterion dir before comparing.
CRITERION_DIR="$REPO_ROOT/$BENCH_DIR/target/criterion"
WORKTREE_CRITERION="$WORKTREE_DIR/main/$BENCH_DIR/target/criterion"
for group_dir in "$WORKTREE_CRITERION"/*/; do
    [ -d "$group_dir/main" ] || continue
    group="$(basename "$group_dir")"
    mkdir -p "$CRITERION_DIR/$group"
    cp -r "$group_dir/main" "$CRITERION_DIR/$group/"
done

# ---- 4. compare ------------------------------------------------------------
if [ ! -d "$CRITERION_DIR" ]; then
    echo "error: no criterion output at $CRITERION_DIR" >&2
    exit 2
fi

echo ""
printf "%-40s %14s %14s %+10s\n" "benchmark group" "main (ns)" "pr (ns)" "delta"
echo "--------------------------------------------------------------------------------"

failed=0
total_groups=0
for main_est in "$CRITERION_DIR"/*/main/estimates.json; do
    [ -f "$main_est" ] || continue
    group="$(basename "$(dirname "$(dirname "$main_est")")")"
    pr_est="$CRITERION_DIR/$group/pr/estimates.json"
    [ -f "$pr_est" ] || continue

    main_ns="$(python3 -c "import json;print(json.load(open('$main_est'))['median']['point_estimate'])")"
    pr_ns="$(python3 -c "import json;print(json.load(open('$pr_est'))['median']['point_estimate'])")"
    delta="$(python3 -c "
m=$main_ns; p=$pr_ns
if m == 0: print(0)
else: print(f'{(p-m)*100/m:.2f}')
")"

    total_groups=$((total_groups + 1))
    # LC_NUMERIC=C forces '.' decimal separator for printf %f.
    LC_NUMERIC=C printf "%-40s %14s %14s %+10s%%\n" "$group" "$main_ns" "$pr_ns" "$delta"

    # Regression = slower = pr > main by more than threshold.
    if python3 -c "
import sys
m=$main_ns; p=$pr_ns; t=$THRESHOLD_PCT
sys.exit(0 if (p > m * (1 + t/100)) else 1)
"; then
        echo "  ^^ REGRESSION: +${delta}% > ${THRESHOLD_PCT}% (ADR-019)"
        failed=1
    fi
done

if [ "$total_groups" -eq 0 ]; then
    echo "error: no comparable benchmark groups found" >&2
    exit 2
fi

echo ""
if [ "$failed" -eq 1 ]; then
    echo "FAIL: performance regression detected (threshold ${THRESHOLD_PCT}%)."
    exit 1
fi
echo "PASS: no regression > ${THRESHOLD_PCT}% vs main."
exit 0
