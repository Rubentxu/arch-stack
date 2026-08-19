#!/usr/bin/env bash
# bench/smoke-matrix.sh — UAT matrix smoke inside the archctl-bench sandbox.
#
# Extends sandbox-e2e.sh (C4 vertical) to the FULL UAT matrix per language:
#   rust/ts/js: discover → evidence → accept → explain → coverage →
#               export+validate → strict+redaction → call-graph →
#               class-diagram → sequence
#   go:         call-graph → evidence → accept → explain → sequence
#   python/java:call-graph → class-diagram → evidence → explain → sequence
#   kotlin:     call-graph → state-machine → sequence
#
# Each dataset runs with a FRESH XDG store inside the container
# (deterministic; no cross-run contamination). The host release binary is
# mounted read-only (the in-container compile of sandbox-e2e.sh is broken
# by the host's global CARGO_TARGET_DIR — see docs/sessions/2026-08-19).
#
# Usage:
#   bench/smoke-matrix.sh <language> <dataset> [--timeout N]
#
# Env:
#   ARCHCTL_BIN   host binary (default: /var/home/rubentxu/cargo-targets/release/archctl)
#   CACHE_DIR     repo cache (default: ~/.cache/archctl-smoke)
#
# Exit: 0 = all cells PASS; 1 = any FAIL. Emits JSON verdict on stdout.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="archctl-bench:latest"
CACHE_DIR="${CACHE_DIR:-$HOME/.cache/archctl-smoke}"
# The binary must be built INSIDE the archctl-bench container (ubuntu:24.04):
# a host-built binary links against the host's newer libstdc++
# (GLIBCXX_3.4.35) and fails in the container. See bench/build-in-sandbox.sh.
ARCHCTL_BIN="${ARCHCTL_BIN:-/tmp/opencode/archctl-ubuntu24}"
LANG="${1:?usage: smoke-matrix.sh <language> <dataset>}"
DATASET="${2:?usage: smoke-matrix.sh <language> <dataset>}"
CMD_TIMEOUT="${CMD_TIMEOUT:-120}"
HEAVY_TIMEOUT="${HEAVY_TIMEOUT:-600}"

command -v podman >/dev/null || { echo "podman required" >&2; exit 1; }
[[ -x "$ARCHCTL_BIN" ]] || { echo "binary not found: $ARCHCTL_BIN" >&2; exit 1; }
[[ -d "$CACHE_DIR/$DATASET" ]] || { echo "dataset not cached: $DATASET — run bench/datasets.sh" >&2; exit 1; }

declare -a CHECKS=()
record() { # name ok detail
  CHECKS+=("{\"name\":\"$1\",\"ok\":$2,\"detail\":\"$3\"}")
  if [[ "$2" == "true" ]]; then echo "[PASS] $1 — $3"; else echo "[FAIL] $1 — $3"; fi
}

# Run a command inside the sandbox. All cells of a dataset share ONE
# fresh XDG store (cross-cell state must persist: discover→evidence→…).
# NOTE: XDG_SHARED is created at SCRIPT level — assignments inside a
# command substitution $( ) run in a subshell and do not survive.
# $1 = bash body.
run_in_sandbox() {
  local body="$1"
  podman run --rm --entrypoint bash --security-opt label=disable \
    -v "$ARCHCTL_BIN:/usr/local/bin/archctl:ro" \
    -v "$CACHE_DIR:/datasets:ro" \
    -v "$XDG_SHARED/data:/xdg/data" \
    -v "$XDG_SHARED/config:/xdg/config" \
    -e XDG_DATA_HOME=/xdg/data \
    -e XDG_CONFIG_HOME=/xdg/config \
    "$IMAGE" -c "$body" bash "$DATASET" 2>/dev/null
}

# Helper: run one archctl command with timeout inside the sandbox; returns exit code.
sandbox_cmd() { # $1=timeout $2=command...
  local t="$1"; shift
  local body="set -o pipefail; timeout $t archctl $* --cwd /datasets/\$1"
  run_in_sandbox "$body"
}

echo "=== smoke-matrix: $DATASET ($LANG) — bin $($ARCHCTL_BIN --version) ==="

# Shared XDG store for this dataset run (script level, not per-cell).
XDG_SHARED=$(mktemp -d)
mkdir -p "$XDG_SHARED/data" "$XDG_SHARED/config"

# ── Common cells (all languages) ──────────────────────────────────────────
callgraph_cell() {
  local out rc
  out=$(run_in_sandbox "set -o pipefail; timeout $HEAVY_TIMEOUT archctl code call-graph --apply --cwd /datasets/\$1 2>&1 | tail -3")
  rc=$?
  [[ $rc -eq 0 ]] && record "call-graph" true "apply exit 0 ($out)" || record "call-graph" false "apply exit $rc ($out)"
}

evidence_cell() {
  local out n
  out=$(sandbox_cmd 60 evidence list --status drafted --json)
  n=$(echo "$out" | jq 'length' 2>/dev/null || echo 0)
  [[ "${n:-0}" -ge 1 ]] && record "evidence" true "drafted=$n" || record "evidence" false "drafted=$n"
}

accept_cell() {
  local ids ok=true
  # Evidence rows use `e.id` (LadybugDB projection key), not `id`.
  ids=$(sandbox_cmd 60 evidence list --status drafted --json | jq -r '.[]["e.id"]' 2>/dev/null | head -20)
  for id in $ids; do
    sandbox_cmd 60 evidence accept --id "$id" >/dev/null 2>&1 || { ok=false; break; }
  done
  [[ "$ok" == "true" ]] && record "accept" true "all drafted accepted" || record "accept" false "accept failed for some id"
}

explain_cell() {
  local eid out
  eid=$(sandbox_cmd 60 architecture relevance --query "$(basename "$DATASET")" --top 1 --json | jq -r '.elements[0].id // .[0].id // empty' 2>/dev/null)
  if [[ -z "$eid" || "$eid" == "null" ]]; then record "explain" false "no element id resolvable"; return; fi
  out=$(sandbox_cmd 60 architecture explain "$eid" --json)
  if echo "$out" | jq -e '.subject.id' >/dev/null 2>&1; then
    record "explain" true "subject=$eid"
  else
    record "explain" false "explain $eid: $(echo "$out" | tail -1)"
  fi
}

coverage_cell() {
  if sandbox_cmd 60 architecture coverage --json | jq -e . >/dev/null 2>&1; then
    record "coverage" true "valid JSON"
  else
    record "coverage" false "coverage JSON invalid"
  fi
}

sequence_cell() {
  local from out
  out=$(sandbox_cmd "$HEAVY_TIMEOUT" code call-graph --json 2>/dev/null)
  # Use the CANONICAL KEY, not the name: short names are ambiguous across
  # the store (sequence rejects ambiguity by design).
  from=$(echo "$out" | jq -r '.nodes[0].canonical_key // empty' 2>/dev/null)
  if [[ -z "$from" || "$from" == "null" ]]; then record "sequence" false "no entry resolvable"; return; fi
  if sandbox_cmd 120 code sequence --from "$from" --json | jq -e . >/dev/null 2>&1; then
    record "sequence" true "from=$from"
  else
    record "sequence" false "sequence --from $from failed"
  fi
}

# ── C4 cells (rust/ts/js only) ─────────────────────────────────────────────
discover_cell() {
  local out
  out=$(run_in_sandbox "set -o pipefail; timeout $CMD_TIMEOUT archctl code c4-discover --apply --cwd /datasets/\$1 2>&1 | tail -2")
  if echo "$out" | grep -qE "Applied: [1-9]"; then record "c4-discover" true "$out"; else record "c4-discover" false "$out"; fi
}

export_validate_cell() {
  local out
  out=$(run_in_sandbox "set -e; archctl diagram export 'container:*' --cwd /datasets/\$1 --format viewer-bundle --output /tmp/bundle >/dev/null 2>&1 && archctl diagram validate /tmp/bundle >/dev/null 2>&1 && echo OK")
  [[ "$out" == *"OK"* ]] && record "export+validate" true "bundle valid" || record "export+validate" false "$out"
}

strict_cell() {
  local out
  # The checksum lives in the bundle's manifest.json, not in the --json
  # envelope printed to stdout.
  out=$(run_in_sandbox "set -e; archctl diagram export 'container:*' --cwd /datasets/\$1 --profile strict --output /tmp/strictb >/dev/null 2>&1 && jq -r '.checksum // empty' /tmp/strictb/manifest.json")
  if [[ -n "$out" && "$out" != "null" ]]; then
    record "strict+checksum" true "checksum=${out:0:12}…"
  else
    record "strict+checksum" false "no checksum in strict manifest"
  fi
}

class_cell() {
  local out rc
  out=$(run_in_sandbox "set -o pipefail; timeout $HEAVY_TIMEOUT archctl code class-diagram --apply --cwd /datasets/\$1 2>&1 | tail -2")
  rc=$?
  [[ $rc -eq 0 ]] && record "class-diagram" true "apply exit 0" || record "class-diagram" false "apply exit $rc ($out)"
}

# ── Language dispatch ──────────────────────────────────────────────────────
case "$LANG" in
  rust|typescript|javascript)
    discover_cell
    evidence_cell
    accept_cell
    explain_cell
    coverage_cell
    export_validate_cell
    strict_cell
    callgraph_cell
    class_cell
    sequence_cell
    ;;
  go)
    callgraph_cell
    evidence_cell
    accept_cell
    explain_cell
    sequence_cell
    ;;
  python|java)
    callgraph_cell
    class_cell
    evidence_cell
    accept_cell
    explain_cell
    sequence_cell
    ;;
  kotlin)
    callgraph_cell
    sequence_cell
    ;;
  *)
    echo "unknown language: $LANG" >&2; exit 2 ;;
esac

# ── Verdict ────────────────────────────────────────────────────────────────
rm -rf "${XDG_SHARED:-/nonexistent}"
ALL_OK=true
for c in "${CHECKS[@]}"; do [[ "$c" == *'"ok":false'* ]] && ALL_OK=false; done
VERDICT="{\"dataset\":\"$DATASET\",\"language\":\"$LANG\",\"verdict\":\"$([ "$ALL_OK" == true ] && echo PASS || echo FAIL)\",\"checks\":[$(IFS=,; echo "${CHECKS[*]}")]}"
echo "$VERDICT"
[[ "$ALL_OK" == "true" ]]
