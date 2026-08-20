#!/usr/bin/env bash
# bench-compare-archview.sh — ADR-019 regression gate for archview: compare
# current branch's TTFP and FPS metrics against a `main` baseline. Exits 1 if
# either metric regresses more than 10%.
#
# Usage:
#   scripts/bench-compare-archview.sh [--help]
#                                      [--fake-ttfp-regression N]
#                                      [--fake-fps-regression N]
#                                      [baseline-ref]
#
#   --fake-ttfp-regression N  test mode: simulate N% TTFP regression
#                              (pr = main * (1 + N/100)). Deterministic.
#   --fake-fps-regression N   test mode: simulate N% FPS regression
#                              (pr = main * (1 - N/100)). Deterministic.
#   baseline-ref              git ref/SHA to compare the current tree against
#                             (default: origin/main). CI passes
#                             github.event.before; local --full verification
#                             passes origin/main.
#
# Env:
#   ARCHVIEW_DIR    archview dir relative to repo root (default: archview)
#   THRESHOLD_PCT   regression threshold (default: 10)
#
# The script:
#   1. Benchmarks the baseline ref in a temporary worktree
#   2. Benchmarks the current tree
#   3. Compares TTFP and FPS per metric; fails if TTFP pr > main*1.10
#      or if FPS pr < main*0.90
#
# Exit codes: 0 = no regression, 1 = regression detected, 2 = setup/usage error.

set -uo pipefail

# ---- prerequisites ---------------------------------------------------------
# python3 is required to compare JSON metric files. Detect it before creating
# any worktree so the failure is a clear prerequisite error.
if ! command -v python3 >/dev/null 2>&1; then
    echo "error: python3 required for metric comparison; install python3" >&2
    exit 2
fi

if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    sed -n '1,41p' "$0" | sed 's/^# \{0,1\}//'
    exit 0
fi

# ---- parse flags ----------------------------------------------------------
# --fake-ttfp-regression N / --fake-fps-regression N simulates a synthetic
# regression deterministically, exercising the ADR-019 threshold logic without
# real benchmark runs. It still validates the baseline first (zero-SHA /
# unreachable refs exit 2).

FAKE_TTFP=""
FAKE_FPS=""
while [[ "${1:-}" == "--fake-ttfp-regression" ]] || [[ "${1:-}" == "--fake-fps-regression" ]]; do
    case "${1}" in
        --fake-ttfp-regression)
            FAKE_TTFP="${2:-}"
            shift 2 || true
            ;;
        --fake-fps-regression)
            FAKE_FPS="${2:-}"
            shift 2 || true
            ;;
    esac
done

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

BASELINE_REF="${1:-origin/main}"

# ---- baseline validation ---------------------------------------------------
# A baseline must be a real, reachable commit. The all-zero SHA (first push
# to an empty repository or history rewrite) is not a valid baseline and MUST
# NOT pass; an absent/invalid/unreachable ref fails the same way. This is the
# ADR-019 regression contract under the post-merge flow.
ZERO_SHA="0000000000000000000000000000000000000000"
if [[ "$BASELINE_REF" == "$ZERO_SHA" ]]; then
    echo "error: baseline ref is the all-zero SHA (first push or history rewrite); no previous main to compare against" >&2
    exit 2
fi

if ! git rev-parse --verify --quiet "${BASELINE_REF}^{commit}" >/dev/null; then
    echo "error: baseline ref not resolvable to a commit: ${BASELINE_REF}" >&2
    exit 2
fi

ARCHVIEW_DIR="${ARCHVIEW_DIR:-archview}"
THRESHOLD_PCT="${THRESHOLD_PCT:-10}"
WORKTREE_DIR="$(mktemp -d /tmp/bench-compare-archview.XXXXXX)"

# Remove the worktree from disk AND git metadata on any exit.
cleanup_worktrees() {
    if [[ -d "$WORKTREE_DIR/baseline" ]]; then
        git worktree remove --force "$WORKTREE_DIR/baseline" >/dev/null 2>&1 || true
    fi
    git worktree prune >/dev/null 2>&1 || true
    rm -rf "$WORKTREE_DIR"
}
trap cleanup_worktrees EXIT

# ---- test mode: generate synthetic JSON files ---------------------------
if [[ -n "$FAKE_TTFP" ]] || [[ -n "$FAKE_FPS" ]]; then
    echo "TEST MODE: fake TTFP regression ${FAKE_TTFP:-0}%, fake FPS regression ${FAKE_FPS:-0}%"

    # Synthetic baseline values (realistic numbers for c4-stress-1k.json)
    base_ttfp=1800
    base_fps=60

    # Simulate PR values with fake regression applied
    pr_ttfp=$(( base_ttfp * (100 + ${FAKE_TTFP:-0}) / 100 ))
    pr_fps=$(( base_fps * (100 - ${FAKE_FPS:-0}) / 100 ))

    echo "Synthetic baseline: ttfp=${base_ttfp}ms fps=${base_fps}"
    echo "Synthetic PR:      ttfp=${pr_ttfp}ms fps=${pr_fps}"

    # Write synthetic JSON files
    mkdir -p "$WORKTREE_DIR/baseline/$ARCHVIEW_DIR"
    python3 -c "
import json
with open('$WORKTREE_DIR/baseline/$ARCHVIEW_DIR/perf-baseline.json', 'w') as f:
    json.dump({'ttfp_ms': $base_ttfp, 'fps': $base_fps, 'sample': 'c4-stress-1k.json', 'runner': 'test', 'timestamp': '2026-08-20T00:00:00Z', 'duration_ms': 5000}, f)
with open('$WORKTREE_DIR/baseline/$ARCHVIEW_DIR/perf-head.json', 'w') as f:
    json.dump({'ttfp_ms': $pr_ttfp, 'fps': $pr_fps, 'sample': 'c4-stress-1k.json', 'runner': 'test', 'timestamp': '2026-08-20T00:00:01Z', 'duration_ms': 5000}, f)
"
    BASELINE_JSON="$WORKTREE_DIR/baseline/$ARCHVIEW_DIR/perf-baseline.json"
    HEAD_JSON="$WORKTREE_DIR/baseline/$ARCHVIEW_DIR/perf-head.json"
else
    # ---- 1. benchmark baseline in a worktree ----------------------------------
    echo "== Benchmarking baseline (${BASELINE_REF}) =="
    git worktree add --detach "$WORKTREE_DIR/baseline" "$BASELINE_REF" >/dev/null
    (
        set -e
        cd "$WORKTREE_DIR/baseline/$ARCHVIEW_DIR"
        pnpm install --frozen-lockfile >/dev/null 2>&1
        pnpm build >/dev/null 2>&1
        pnpm preview --port 18080 &
        SERVER_PID=$!
        # Wait for server to be ready (up to 30s)
        for i in $(seq 1 30); do
            if curl -s "http://localhost:18080" >/dev/null 2>&1; then
                break
            fi
            sleep 1
        done
        node bench/perf-cull.mjs --output perf-baseline.json --warmup 1 || true
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    )
    BASELINE_JSON="$WORKTREE_DIR/baseline/$ARCHVIEW_DIR/perf-baseline.json"

    # ---- 2. benchmark current tree -------------------------------------------
    echo "== Benchmarking current branch (pr) =="
    (
        set -e
        cd "$REPO_ROOT/$ARCHVIEW_DIR"
        pnpm install --frozen-lockfile >/dev/null 2>&1
        pnpm build >/dev/null 2>&1
        pnpm preview --port 18080 &
        SERVER_PID=$!
        for i in $(seq 1 30); do
            if curl -s "http://localhost:18080" >/dev/null 2>&1; then
                break
            fi
            sleep 1
        done
        node bench/perf-cull.mjs --output perf-head.json --warmup 1 || true
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    )
    HEAD_JSON="$REPO_ROOT/$ARCHVIEW_DIR/perf-head.json"
fi

# ---- 3. compare -----------------------------------------------------------
if [[ ! -f "$BASELINE_JSON" ]]; then
    echo "error: no baseline JSON at $BASELINE_JSON" >&2
    exit 2
fi
if [[ ! -f "$HEAD_JSON" ]]; then
    echo "error: no head JSON at $HEAD_JSON" >&2
    exit 2
fi

# Read metrics from JSON using python3
read_metric() {
    python3 -c "import json,sys; d=json.load(open('$1')); print(d.get('$2',''))"
}

baseline_ttfp=$(read_metric "$BASELINE_JSON" "ttfp_ms")
baseline_fps=$(read_metric "$BASELINE_JSON" "fps")
head_ttfp=$(read_metric "$HEAD_JSON" "ttfp_ms")
head_fps=$(read_metric "$HEAD_JSON" "fps")

echo ""
printf "%-20s %14s %14s %+10s\n" "metric" "baseline" "pr" "delta"
echo "------------------------------------------------------------"

# TTFP: higher = worse. Regression if pr > baseline * (1 + threshold/100)
ttfp_delta=$(python3 -c "
m=$baseline_ttfp; p=$head_ttfp
if m == 0: print(0)
else: print(f'{(p-m)*100/m:.2f}')
")
LC_NUMERIC=C printf "%-20s %14s %14s %+10s%%\n" "TTFP (ms)" "$baseline_ttfp" "$head_ttfp" "$ttfp_delta"

fps_delta=$(python3 -c "
m=$baseline_fps; p=$head_fps
if m == 0: print(0)
else: print(f'{(p-m)*100/m:.2f}')
")
LC_NUMERIC=C printf "%-20s %14s %14s %+10s%%\n" "FPS" "$baseline_fps" "$head_fps" "$fps_delta"

echo ""

failed=0
regressions="[]"

# TTFP check: pr > baseline * (1 + threshold/100) is a regression
# We use || true to prevent set -e from exiting on non-zero python exit.
python3 -c "
import sys
m=$baseline_ttfp; p=$head_ttfp; t=$THRESHOLD_PCT
sys.exit(0 if (p > m * (1 + t/100)) else 1)
" && {
    echo "  ^^ TTFP REGRESSION: +${ttfp_delta}% > ${THRESHOLD_PCT}% (ADR-019)"
    failed=1
    regressions=$(python3 -c "import json; r=json.loads('$regressions'); r.append({'metric':'ttfp','delta':${ttfp_delta},'threshold':${THRESHOLD_PCT}}); print(json.dumps(r))")
} || true

# FPS check: pr < baseline * (1 - threshold/100) is a regression (lower = worse)
python3 -c "
import sys
m=$baseline_fps; p=$head_fps; t=$THRESHOLD_PCT
sys.exit(0 if (p < m * (1 - t/100)) else 1)
" && {
    echo "  ^^ FPS REGRESSION: ${fps_delta}% < -${THRESHOLD_PCT}% (ADR-019)"
    failed=1
    regressions=$(python3 -c "import json; r=json.loads('$regressions'); r.append({'metric':'fps','delta':${fps_delta},'threshold':${THRESHOLD_PCT}}); print(json.dumps(r))")
} || true

# Emit structured JSON to stdout
python3 -c "
import json
print(json.dumps({
    'baseline': '$BASELINE_REF',
    'head': '$(git rev-parse HEAD)',
    'metrics': {
        'ttfp_ms': {'baseline': $baseline_ttfp, 'head': $head_ttfp, 'delta_pct': $ttfp_delta},
        'fps': {'baseline': $baseline_fps, 'head': $head_fps, 'delta_pct': $fps_delta}
    },
    'regressions': json.loads('$regressions'),
    'threshold_pct': $THRESHOLD_PCT
}, indent=2))
"

echo ""
if [[ "$failed" -eq 1 ]]; then
    echo "FAIL: performance regression detected (threshold ${THRESHOLD_PCT}%)."
    exit 1
fi
echo "PASS: no regression > ${THRESHOLD_PCT}% vs ${BASELINE_REF}."
exit 0
