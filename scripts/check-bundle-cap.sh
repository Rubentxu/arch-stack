#!/usr/bin/env bash
# check-bundle-cap.sh — ADR-019 gzipped JS bundle cap (single source).
#
# Called by BOTH CI (.github/workflows/ci.yml web job) and local
# verification (scripts/verify-local.sh --full), so the 2MB rule lives in
# exactly one place.
#
# Usage:
#   scripts/check-bundle-cap.sh [glob]
#
#   glob   files to measure (default: archview/dist/assets/*.js)
#
# Exit codes:
#   0 = bundle within ADR-019 cap (gzipped <= 2MB)
#   1 = bundle exceeds the cap
#   2 = usage error or missing prerequisite
#
# The script never mutates source: it only reads gzipped byte counts.

set -euo pipefail

GLOB="${1:-archview/dist/assets/*.js}"

total=0
count=0
for f in $GLOB; do
    [ -f "$f" ] || continue
    size=$(gzip -c "$f" | wc -c)
    echo "$f: $size bytes gzipped"
    total=$((total + size))
    count=$((count + 1))
done

if [ "$count" -eq 0 ]; then
    echo "::error::No JS bundle files matched: $GLOB" >&2
    exit 2
fi

echo "total gzipped: $total bytes"
limit=$((2 * 1024 * 1024))
if [ "$total" -gt "$limit" ]; then
    echo "::error::Bundle exceeds ADR-019 cap (2MB gzipped): $total > $limit"
    exit 1
fi
echo "Bundle within ADR-019 cap."
exit 0
