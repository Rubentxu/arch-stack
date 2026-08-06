#!/usr/bin/env bash
# bench/sandbox-e2e.sh — reproducible sandbox E2E (ADR-034 §4, M29.4).
#
# Runs the full C4 vertical INSIDE the Quadlet container with asserts:
#   build image → compile archctl in-container → discover → accept →
#   export → validate → JSON verdict.
#
# Usage:
#   bench/sandbox-e2e.sh [--dataset <name>] [--keep-container] [--no-build]
#
#   --dataset <name>   cache repo name (default: tokio-rs/axum)
#   --keep-container   do not remove the test container
#   --no-build         skip image build (use existing archctl-bench:latest)
#
# Exit: 0 = PASS, 1 = FAIL. Emits a JSON verdict on stdout.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATASET="${SANDBOX_DATASET:-tokio-rs/axum}"
IMAGE="archctl-bench:latest"
KEEP=0
NO_BUILD=0
CACHE_DIR="${CACHE_DIR:-$HOME/.cache/archctl-smoke}"
XDG_DATA="${XDG_DATA_HOME:-$HOME/.local/share/archctl}"

usage() {
  cat <<EOF
sandbox-e2e.sh — reproducible sandbox E2E

USAGE:
  bench/sandbox-e2e.sh [--dataset <name>] [--keep-container] [--no-build]

ENV:
  SANDBOX_DATASET   dataset name (default: tokio-rs/axum)
  CACHE_DIR         repo cache (default: ~/.cache/archctl-smoke)
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dataset) DATASET="$2"; shift 2 ;;
    --keep-container) KEEP=1; shift ;;
    --no-build) NO_BUILD=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown arg: $1" >&2; usage; exit 1 ;;
  esac
done

# JSON verdict accumulator
declare -a CHECKS=()
declare -a CHECK_NAMES=()
fail() {
  echo "sandbox-e2e: $1" >&2
  exit 1
}

record() {
  local name="$1" ok="$2" detail="$3"
  CHECKS+=("{\"name\":\"$name\",\"ok\":$ok,\"detail\":\"$detail\"}")
  CHECK_NAMES+=("$name")
  if [[ "$ok" == "true" ]]; then
    echo "[PASS] $name $detail"
  else
    echo "[FAIL] $name $detail"
  fi
}

# ── 0. Prerequisites ────────────────────────────────────────────────────────
command -v podman >/dev/null || fail "podman required"
mkdir -p "$XDG_DATA"

# Dataset must exist in cache (cloned by bench/datasets.sh or previous runs)
if [[ ! -d "$CACHE_DIR/$DATASET" ]]; then
  fail "dataset not cached: $CACHE_DIR/$DATASET — run bench/datasets.sh first"
fi

# ── 1. Build image (unless --no-build) ─────────────────────────────────────
if [[ $NO_BUILD -eq 0 ]]; then
  echo "== build image =="
  podman build -q -f "$REPO_ROOT/bench/Containerfile" -t "$IMAGE" "$REPO_ROOT/bench/" \
    || fail "image build failed"
  record "image-build" true "archctl-bench:latest"
fi

# ── 2. Compile archctl INSIDE the container ────────────────────────────────
echo "== compile archctl in-container =="
if podman run --rm --security-opt label=disable \
    -v "$REPO_ROOT:/src:rw" \
    -v "$HOME/.cargo:/root/.cargo:rw" \
    "$IMAGE" bash -c 'cd /src/archctl && cargo build --release --quiet' >/tmp/sandbox-build.log 2>&1; then
  record "in-container-build" true "cargo build --release OK"
else
  record "in-container-build" false "cargo build failed — see /tmp/sandbox-build.log"
  tail -5 /tmp/sandbox-build.log >&2
fi

# ── 3. Vertical C4 with asserts ────────────────────────────────────────────
echo "== vertical C4 in-container ($DATASET) =="
# Isolated XDG for reproducibility: the host's shared graph may already
# contain this dataset (from prior runs/bench), which would make --apply
# report "Applied: 0" (all skipped as existing). A fresh XDG per run makes
# the suite deterministic.
SANDBOX_XDG=$(mktemp -d)
mkdir -p "$SANDBOX_XDG/data" "$SANDBOX_XDG/config"
trap 'rm -rf "$SANDBOX_XDG"' EXIT

VERTICAL_SCRIPT=$(cat <<'VERTICAL'
set -e
DS="$1"
ARCHCTL=/usr/local/bin/archctl
export RUST_LOG=error

# 3a. Discover + apply: expect >= 1 container
OUT=$("$ARCHCTL" code c4-discover --cwd "/datasets/$DS" --apply 2>/dev/null | tail -1)
echo "DISCOVER_OUT=$OUT"
if echo "$OUT" | grep -qE "Applied: [1-9]"; then echo "DISCOVER=OK"; else echo "DISCOVER=FAIL"; exit 2; fi

# 3b. Drafted evidence >= 1
N=$("$ARCHCTL" evidence list --cwd "/datasets/$DS" --status drafted --json 2>/dev/null | jq 'length')
echo "EVIDENCE_COUNT=$N"
if [ "${N:-0}" -ge 1 ]; then echo "EVIDENCE=OK"; else echo "EVIDENCE=FAIL"; exit 3; fi

# 3c. Accept all drafted
for id in $("$ARCHCTL" evidence list --cwd "/datasets/$DS" --status drafted --json 2>/dev/null | jq -r '.[].id' | head -20); do
  "$ARCHCTL" evidence accept --id "$id" --cwd "/datasets/$DS" >/dev/null 2>&1 || true
done
echo "ACCEPT=OK"

# 3d. Export bundle
"$ARCHCTL" diagram export container:* --cwd "/datasets/$DS" --format viewer-bundle --output /tmp/bundle >/dev/null 2>&1
if [ -f /tmp/bundle/manifest.json ]; then echo "EXPORT=OK"; else echo "EXPORT=FAIL"; exit 4; fi

# 3e. Validate
if "$ARCHCTL" diagram validate /tmp/bundle --cwd "/datasets/$DS" >/dev/null 2>&1; then
  echo "VALIDATE=OK"
else
  echo "VALIDATE=FAIL"; exit 5
fi

echo "VERTICAL=PASS"
VERTICAL
)

VERTICAL_RESULT=$(podman run --rm --security-opt label=disable \
    -v "$REPO_ROOT/archctl/target/release/archctl:/usr/local/bin/archctl:ro" \
    -v "$CACHE_DIR:/datasets:ro" \
    -v "$SANDBOX_XDG/data:/xdg/data" \
    -v "$SANDBOX_XDG/config:/xdg/config" \
    -e XDG_DATA_HOME=/xdg/data \
    -e XDG_CONFIG_HOME=/xdg/config \
    "$IMAGE" bash -c "$VERTICAL_SCRIPT" bash "$DATASET" 2>&1) \
  || true

echo "$VERTICAL_RESULT" | sed 's/^/    /'

discover_ok=$(echo "$VERTICAL_RESULT" | grep -c "DISCOVER=OK" || true)
evidence_ok=$(echo "$VERTICAL_RESULT" | grep -c "EVIDENCE=OK" || true)
accept_ok=$(echo "$VERTICAL_RESULT" | grep -c "ACCEPT=OK" || true)
export_ok=$(echo "$VERTICAL_RESULT" | grep -c "EXPORT=OK" || true)
validate_ok=$(echo "$VERTICAL_RESULT" | grep -c "VALIDATE=OK" || true)

record "discover"  "$([[ $discover_ok -gt 0 ]] && echo true || echo false)" "$DATASET"
record "evidence"  "$([[ $evidence_ok -gt 0 ]] && echo true || echo false)" "drafted >= 1"
record "accept"    "$([[ $accept_ok -gt 0 ]] && echo true || echo false)" "all drafted"
record "export"    "$([[ $export_ok -gt 0 ]] && echo true || echo false)" "viewer-bundle"
record "validate"  "$([[ $validate_ok -gt 0 ]] && echo true || echo false)" "exit 0"

# ── 4. Verdict ─────────────────────────────────────────────────────────────
ALL_OK=true
for c in "${CHECKS[@]}"; do
  [[ "$c" == *'"ok":false'* ]] && ALL_OK=false
done

VERDICT="{\"verdict\":\"$([ "$ALL_OK" == true ] && echo PASS || echo FAIL)\",\"checks\":[$(IFS=,; echo "${CHECKS[*]}")]}"
echo "SANDBOX_VERDICT=$VERDICT"

[[ "$ALL_OK" == true ]] && exit 0 || exit 1
