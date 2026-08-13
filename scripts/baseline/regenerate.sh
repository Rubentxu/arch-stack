#!/usr/bin/env bash
#
# regenerate.sh — Regenerate all frontier-freeze baselines
#
# Usage: scripts/baseline/regenerate.sh
#
set -uo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

BIN="archctl/target/debug/archctl"
DIR="scripts/baseline"

echo "=== Rebuilding binary ==="
(cd archctl && cargo build --quiet) || exit 1

echo "=== Golden outputs ==="
mkdir -p "$DIR/golden-outputs"
"$BIN" --version > "$DIR/golden-outputs/version.txt" 2>&1
"$BIN" --help > "$DIR/golden-outputs/help.txt" 2>&1
for cmd in doctor project graph inventory diagram evidence render code skills agent mcp plugin ide view self; do
    "$BIN" "$cmd" --help > "$DIR/golden-outputs/help-${cmd}.txt" 2>&1
done
echo "  $(ls "$DIR/golden-outputs/" | wc -l) files"

echo "=== File sizes ==="
find archctl/src -name '*.rs' -exec wc -c {} \; | sort -rn > "$DIR/file-sizes.txt"
echo "  $(wc -l < "$DIR/file-sizes.txt") files"

echo "=== Import map ==="
rg '^use (crate|super)::' archctl/src/ --type rust -n \
    | sed 's|archctl/src/||' | sort > "$DIR/import-map.txt"
echo "  $(wc -l < "$DIR/import-map.txt") imports"

echo "=== Done ==="
echo "Review changes with: git diff scripts/baseline/"
