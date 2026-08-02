#!/usr/bin/env bash
# fmt-staged.sh — Format only Rust files that are staged for commit.
#
# Gotcha: `cargo fmt` (and `cargo fmt -- <file>`) formats the ENTIRE
# workspace, not just the file passed. To scope formatting to staged
# files, we use `rustfmt` directly on each .rs file in the git index.
#
# Usage:
#   scripts/fmt-staged.sh           # check (exit 1 if drift)
#   scripts/fmt-staged.sh --apply   # format and re-stage
#
# Exit codes:
#   0 = no drift (or successfully applied)
#   1 = drift detected (in check mode) or rustfmt failed

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

STAGED_RS=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.rs$' || true)

if [ -z "$STAGED_RS" ]; then
    echo "No staged .rs files to format."
    exit 0
fi

if [ "${1:-}" = "--apply" ]; then
    echo "Formatting $(echo "$STAGED_RS" | wc -l) staged .rs file(s)..."
    for f in $STAGED_RS; do
        if [ -f "$f" ]; then
            rustfmt "$f"
            git add "$f"
        fi
    done
    echo "Done. Files re-staged."
    exit 0
fi

# Check mode: report files that would change under rustfmt
DRIFT=0
for f in $STAGED_RS; do
    if [ -f "$f" ]; then
        # rustfmt --check exits 0 if file is formatted, 1 if not
        if ! rustfmt --check --edition 2021 "$f" >/dev/null 2>&1; then
            echo "drift: $f"
            DRIFT=1
        fi
    fi
done

if [ "$DRIFT" -eq 1 ]; then
    echo ""
    echo "Run: scripts/fmt-staged.sh --apply"
    exit 1
fi

echo "All staged .rs files are formatted."
exit 0
