#!/usr/bin/env bash
# datasets.sh — clone helper for archctl-bench datasets
#
# Refs:
#   - bench/datasets.toml: source of truth
#   - docs/specs/bench-harness.md (Requirement: Datasets)
#
# Usage:
#   bench/datasets.sh --clone <name>     # clone one dataset
#   bench/datasets.sh --clone-all        # clone all datasets
#   bench/datasets.sh --validate         # verify SHA matches for all
#   bench/datasets.sh --list             # list all datasets
#   bench/datasets.sh --purge            # remove all cached clones

set -euo pipefail

DATASETS_FILE="${DATASETS_FILE:-bench/datasets.toml}"
CACHE_DIR="${CACHE_DIR:-$HOME/.cache/archctl-smoke}"

usage() {
  cat <<EOF
datasets.sh — Manage archctl-bench dataset caches

USAGE:
  bench/datasets.sh [--clone <name>] [--clone-all] [--validate] [--list] [--purge]
                    [--populate-self-dogfood]

ENV:
  DATASETS_FILE  path to datasets.toml (default: bench/datasets.toml)
  CACHE_DIR      cache directory (default: ~/.cache/archctl-smoke)
EOF
}

if [[ ! -f "$DATASETS_FILE" ]]; then
  echo "Error: $DATASETS_FILE not found" >&2
  exit 1
fi

ensure_tomlls() {
  if ! command -v python3 >/dev/null; then
    echo "Error: python3 required (for TOML parsing)" >&2
    exit 1
  fi
}

# Parse datasets.toml via python (no tomli dep needed; python 3.11+ has tomllib)
parse_datasets() {
  ensure_tomlls
  python3 -c "
import sys, os
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

list_datasets() {
  parse_datasets | while IFS='|' read -r name sha lang ext timeout notes; do
    printf "%-30s %-12s %-12s %s\n" "$name" "$lang" "$ext" "$sha"
  done
}

clone_one() {
  local name="$1" sha="$2"
  local target="$CACHE_DIR/$name"
  local repo_url="https://github.com/$name.git"

  if [[ "$name" == "archctl" ]]; then
    # Self-dogfood uses local checkout
    echo "[skip clone] archctl uses local checkout (HEAD)"
    return 0
  fi

  if [[ -d "$target/.git" ]]; then
    echo "[cached] $name at $target"
    # Verify SHA matches
    local current_sha
    current_sha=$(git -C "$target" rev-parse HEAD)
    if [[ "$current_sha" == "$sha" ]]; then
      echo "  SHA matches: $sha"
      return 0
    else
      echo "  SHA mismatch (current=$current_sha, expected=$sha); re-cloning"
      rm -rf "$target"
    fi
  fi
  mkdir -p "$(dirname "$target")"
  echo "[clone] $name @ $sha -> $target"
  git clone --depth 1 --quiet "$repo_url" "$target"
  git -C "$target" fetch --depth 1 --quiet origin "$sha"
  git -C "$target" checkout --quiet "$sha"
  echo "  SHA: $(git -C "$target" rev-parse HEAD)"
}

clone_all() {
  parse_datasets | while IFS='|' read -r name sha lang ext timeout notes; do
    clone_one "$name" "$sha"
  done
}

validate() {
  echo "Validating $DATASETS_FILE..."
  parse_datasets | while IFS='|' read -r name sha lang ext timeout notes; do
    if [[ "$name" == "archctl" ]]; then
      echo "[OK] $name (self-dogfood)"
      continue
    fi
    local target="$CACHE_DIR/$name"
    if [[ ! -d "$target/.git" ]]; then
      echo "[MISSING] $name (not cloned)"
      continue
    fi
    local current_sha
    current_sha=$(git -C "$target" rev-parse HEAD)
    if [[ "$current_sha" == "$sha" ]]; then
      echo "[OK] $name SHA=$sha"
    else
      echo "[MISMATCH] $name current=$current_sha expected=$sha"
    fi
  done
}

purge() {
  echo "Purging $CACHE_DIR/*"
  rm -rf "$CACHE_DIR"/*
  echo "Done."
}

# Populate the self-dogfood dataset (archctl) by rsyncing the local
# checkout into $CACHE_DIR/archctl, skipping git/target/node_modules so
# the call-graph extractor can walk a clean source tree. Required before
# smoke-matrix.sh rust archctl.
#
# Usage:
#   bench/datasets.sh --populate-self-dogfood
populate_self_dogfood() {
  local target="$CACHE_DIR/archctl"
  local src="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
  if [[ ! -d "$src/archctl" ]]; then
    echo "Error: archctl source not found at $src/archctl" >&2
    exit 1
  fi
  if ! command -v rsync >/dev/null; then
    echo "Error: rsync required for --populate-self-dogfood" >&2
    exit 1
  fi
  mkdir -p "$target"
  rsync -a --delete \
    --exclude='.git/' \
    --exclude='target/' \
    --exclude='node_modules/' \
    --exclude='dist/' \
    --exclude='sddk/' \
    --exclude='docs/reports/' \
    --exclude='docs/sessions/' \
    "$src/" "$target/"
  echo "[populated] $target (HEAD source, no .git/target)"
}

# Parse CLI
case "${1:-}" in
  --clone) clone_one "$2" "$(parse_datasets | grep -F "$2|" | head -1 | cut -d'|' -f2)" ;;
  --clone-all) clone_all ;;
  --validate) validate ;;
  --list) list_datasets ;;
  --purge) purge ;;
  --populate-self-dogfood) populate_self_dogfood ;;
  -h|--help|'') usage ;;
  *) echo "Unknown arg: $1" >&2; usage; exit 1 ;;
esac
