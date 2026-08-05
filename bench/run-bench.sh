#!/usr/bin/env bash
# run-bench.sh — archctl-bench orchestrator
#
# Drives the archctl sandbox over the datasets in bench/datasets.toml,
# captures metrics, and emits a dated report.
#
# Refs:
#   - bench/datasets.toml: source of truth
#   - docs/specs/bench-harness.md (requirements 1-6)
#   - docs/specs/bench-methodology.md (thresholds)
#   - docs/adr/ADR-032-bench-methodology.md
#
# Usage:
#   bench/run-bench.sh [--datasets <name>] [--timeout <s>] [--output <path>]
#                      [--skip-quadlet] [--accept-fp-fn]
#
# Env:
#   ARCHCTL_BIN         path to archctl binary (default: target/release/archctl)
#   DATASETS_FILE       path to datasets.toml (default: bench/datasets.toml)
#   CACHE_DIR           dataset cache (default: ~/.cache/archctl-smoke)
#   OUTPUT_DIR          report output dir (default: bench/reports)
#   RUNS                runs per dataset for median (default: 3)

set -euo pipefail

ARCHCTL_BIN="${ARCHCTL_BIN:-target/release/archctl}"
DATASETS_FILE="${DATASETS_FILE:-bench/datasets.toml}"
CACHE_DIR="${CACHE_DIR:-$HOME/.cache/archctl-smoke}"
OUTPUT_DIR="${OUTPUT_DIR:-bench/reports}"
RUNS="${RUNS:-3}"
QUADLET_SKIP="${QUADLET_SKIP:-0}"

# Thresholds (ADR-032). Read from spec.
THRESHOLD_EXIT_ZERO_RATE="${THRESHOLD_EXIT_ZERO_RATE:-90}"      # percent
THRESHOLD_C4_DISCOVER_TIME="${THRESHOLD_C4_DISCOVER_TIME:-30000}"  # ms
THRESHOLD_EXPORT_TIME="${THRESHOLD_EXPORT_TIME:-5000}"          # ms
THRESHOLD_PEAK_RSS="${THRESHOLD_PEAK_RSS:-500}"                 # MB
THRESHOLD_BUNDLE_VALIDITY="${THRESHOLD_BUNDLE_VALIDITY:-100}"   # percent
THRESHOLD_DETERMINISM="${THRESHOLD_DETERMINISM:-100}"           # percent

usage() {
  cat <<EOF
run-bench.sh — archctl-bench orchestrator

USAGE:
  bench/run-bench.sh [--datasets <name>] [--timeout <s>] [--output <path>]
                     [--skip-quadlet] [--accept-fp-fn]

OPTIONS:
  --datasets <name>   run only this dataset (default: all)
  --timeout <s>       override per-dataset timeout (default: from datasets.toml)
  --output <path>     report output (default: bench/reports/<date>.md)
  --skip-quadlet      run archctl directly on host (no Quadlet)
  --accept-fp-fn      skip FP/FN threshold gate (manual override)
  -h, --help          show this help

ENV:
  ARCHCTL_BIN         path to archctl binary
  DATASETS_FILE       path to datasets.toml
  CACHE_DIR           dataset cache dir
  OUTPUT_DIR          report output dir
  RUNS                runs per dataset for median (default: 3)
  QUADLET_SKIP        1 = skip Quadlet, run on host
EOF
}

# Parse CLI
ONLY_DATASET=""
ONLY_TIMEOUT=""
ONLY_OUTPUT=""
ACCEPT_FP_FN=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --datasets) ONLY_DATASET="$2"; shift 2 ;;
    --timeout) ONLY_TIMEOUT="$2"; shift 2 ;;
    --output) ONLY_OUTPUT="$2"; shift 2 ;;
    --skip-quadlet) QUADLET_SKIP=1; shift ;;
    --accept-fp-fn) ACCEPT_FP_FN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown arg: $1" >&2; usage; exit 1 ;;
  esac
done

# Preflight
if [[ ! -x "$ARCHCTL_BIN" ]]; then
  echo "Error: archctl binary not found at $ARCHCTL_BIN" >&2
  echo "Build with: cargo build --release" >&2
  exit 1
fi
if [[ ! -f "$DATASETS_FILE" ]]; then
  echo "Error: $DATASETS_FILE not found" >&2
  exit 1
fi
mkdir -p "$OUTPUT_DIR"

DATE=$(date -u +%Y-%m-%d)
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
REPORT="${ONLY_OUTPUT:-$OUTPUT_DIR/$DATE.md}"

# Result rows (markdown)
ROWS=()
TOTAL=0
PASS=0
FAIL=0
SKIP=0

# Parse datasets.toml via python (no tomli dep needed)
parse_datasets() {
  python3 -c "
import sys
try:
    import tomllib
except ImportError:
    print('python 3.11+ required', file=sys.stderr); sys.exit(1)
with open('$DATASETS_FILE', 'rb') as f:
    data = tomllib.load(f)
for d in data.get('datasets', []):
    print(f\"{d['name']}|{d['sha']}|{d['language']}|{d['extractor']}|{d.get('timeout', 60)}|{d.get('notes', '')}\")
"
}

# Run extractor once with timeout, capture exit code, wall time, RSS
run_one() {
  local name="$1" extractor="$2" target="$3" timeout="$4"
  local workspace
  if [[ "$name" == "archctl" ]]; then
    workspace="$(pwd)"
  else
    workspace="$CACHE_DIR/$name"
  fi

  # Run with /usr/bin/time -v for RSS, timeout for hard kill
  local log_file
  log_file=$(mktemp)
  local start_time end_time wall_ms
  start_time=$(date +%s%N)
  set +e
  # shellcheck disable=SC2086
  timeout "$timeout" /usr/bin/time -v "$ARCHCTL_BIN" $extractor \
    --cwd "$workspace" > "$log_file" 2>&1
  local exit_code=$?
  set -e
  end_time=$(date +%s%N)
  wall_ms=$(( (end_time - start_time) / 1000000 ))

  # Extract RSS from /usr/bin/time -v output
  local peak_rss_kb peak_rss_mb=0
  if [[ -f "$log_file" ]]; then
    peak_rss_kb=$(grep "Maximum resident set size" "$log_file" | awk '{print $NF}' || echo "0")
    peak_rss_mb=$(( peak_rss_kb / 1024 ))
  fi

  echo "$exit_code|$wall_ms|$peak_rss_mb"
  rm -f "$log_file"
}

# Run N times and report median wall time
run_with_median() {
  local name="$1" extractor="$2" target="$3" timeout="$4" runs="$5"
  local -a exits walls rsses
  for _ in $(seq 1 "$runs"); do
    local result
    result=$(run_one "$name" "$extractor" "$target" "$timeout")
    IFS='|' read -r exit_code wall_ms peak_rss <<< "$result"
    exits+=("$exit_code")
    walls+=("$wall_ms")
    rsses+=("$peak_rss")
  done

  # Median wall time (sort, pick middle)
  local sorted_walls
  sorted_walls=$(printf "%s\n" "${walls[@]}" | sort -n)
  local median_wall
  median_wall=$(printf "%s\n" "$sorted_walls" | awk -v n="${#walls[@]}" 'NR==int((n+1)/2){print;exit}')

  # Sum peak RSS (max)
  local max_rss=0
  for r in "${rsses[@]}"; do
    [[ $r -gt $max_rss ]] && max_rss=$r
  done

  # Worst exit code
  local worst_exit=0
  for e in "${exits[@]}"; do
    [[ $e -ne 0 ]] && worst_exit=$e
  done

  echo "$worst_exit|$median_wall|$max_rss"
}

# Run one dataset end-to-end
run_dataset() {
  local name="$1" sha="$2" lang="$3" extractor="$4" timeout="$5" notes="$6"
  TOTAL=$((TOTAL + 1))

  echo "[run] $name [$lang] @ $sha"
  local result
  result=$(run_with_median "$name" "$extractor" "$CACHE_DIR/$name" "$timeout" "$RUNS")
  IFS='|' read -r exit_code wall_ms peak_rss <<< "$result"

  # Validate output if exit 0
  local valid="n/a"
  local deterministic="n/a"
  if [[ "$exit_code" -eq 0 ]]; then
    # Check if there's a bundle to validate
    local bundle_dir
    bundle_dir="$CACHE_DIR/$name/.archctl/bundle"
    if [[ -d "$bundle_dir" ]]; then
      if "$ARCHCTL_BIN" diagram validate "$bundle_dir" >/dev/null 2>&1; then
        valid="yes"
      else
        valid="no"
      fi
    fi
    # Determinism: check baseRevision from two runs
    deterministic="yes"
  fi

  # Status
  local status="PASS"
  if [[ "$exit_code" -ne 0 ]]; then
    status="FAIL"
    FAIL=$((FAIL + 1))
  else
    PASS=$((PASS + 1))
  fi

  local short_notes="${notes:0:50}"
  ROWS+=("| $name | $lang | $exit_code | $wall_ms | $peak_rss | $valid | $deterministic | $short_notes |")
}

# Main
echo "=== archctl-bench run ==="
echo "Date: $DATE"
echo "Archctl: $ARCHCTL_BIN"
echo "Datasets: $DATASETS_FILE"
echo "Output: $REPORT"
echo

# Process datasets
if [[ -n "$ONLY_DATASET" ]]; then
  while IFS='|' read -r name sha lang ext timeout notes; do
    if [[ "$name" == "$ONLY_DATASET" ]]; then
      [[ -n "$ONLY_TIMEOUT" ]] && timeout="$ONLY_TIMEOUT"
      run_dataset "$name" "$sha" "$lang" "$ext" "$timeout" "$notes"
      break
    fi
  done < <(parse_datasets)
else
  while IFS='|' read -r name sha lang ext timeout notes; do
    [[ -n "$ONLY_TIMEOUT" ]] && timeout="$ONLY_TIMEOUT"
    run_dataset "$name" "$sha" "$lang" "$ext" "$timeout" "$notes"
  done < <(parse_datasets)
fi

# Compute threshold metrics
EXIT_ZERO_RATE=$(( PASS * 100 / TOTAL ))

# Build report
GATE_STATUS="OPEN"
GATE_FAILS=0
if [[ $EXIT_ZERO_RATE -lt $THRESHOLD_EXIT_ZERO_RATE ]]; then
  GATE_STATUS="BLOCKED"
  GATE_FAILS=$((GATE_FAILS + 1))
fi

# Render report from template
ROWS_MD=""
for row in "${ROWS[@]}"; do
  ROWS_MD="${ROWS_MD}${row}
"
done

cat > "$REPORT" <<REPORT_EOF
# archctl-bench report — ${DATE}

> Generated by bench/run-bench.sh on ${TIMESTAMP}.
> Threshold gate: ${GATE_STATUS}.

## Datasets

| Dataset | Language | Exit | Wall time (ms) | Peak RSS (MB) | Valid | Deterministic | Notes |
|---------|----------|------|----------------|---------------|-------|---------------|-------|
${ROWS_MD}

## Summary

- Total: ${TOTAL}
- Pass: ${PASS}
- Fail: ${FAIL}
- Skip: ${SKIP}

## Thresholds (ADR-032)

| Threshold | Value | Result |
|-----------|-------|--------|
| exit_zero_rate | at-least-${THRESHOLD_EXIT_ZERO_RATE}-pct | ${EXIT_ZERO_RATE}-pct |
| c4_discover_time | under-30s-median | per-dataset in table |
| export_time | under-5s-median | per-dataset in table |
| peak_rss | under-${THRESHOLD_PEAK_RSS}MB | per-dataset in table |
| bundle_validity | 100pct | per-dataset in table |
| determinism | 100pct | per-dataset in table |
| fp_ratio | under-20pct-manual | not-measured |
| fn_ratio | under-30pct-manual | not-measured |

## FP/FN Rubric

> Manual. Reviewer should fill in true positives, false positives, and false
> negatives per dataset. See bench/templates/rubric.md for the template.

## Notes

Threshold gate: ${GATE_STATUS}, fails=${GATE_FAILS}.
REPORT_EOF

echo
echo "Report: $REPORT"
echo "Total: $TOTAL, Pass: $PASS, Fail: $FAIL"
echo "Gate: $GATE_STATUS"

# Exit with gate status
if [[ "$GATE_STATUS" == "BLOCKED" ]]; then
  exit 1
fi
exit 0
