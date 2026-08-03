#!/usr/bin/env bash
# check-branch-protection.sh — live-verify GitHub branch protection (ADR-025).
#
# Read-only gate: queries the live `gh api` protection settings for the
# default branch and asserts the ADR-025 contract:
#   - pull requests are required
#   - admins cannot bypass (enforce_admins)
#   - force pushes and deletions are disabled
#   - zero required status checks (CI is post-merge evidence, not a gate)
#
# Runs ONLY from `scripts/verify-local.sh --full --check-branch-protection`.
# Cheap pre-push verification must never invoke this (network + external
# mutable state). The script is read-only: it never modifies protection.
#
# Usage:
#   scripts/check-branch-protection.sh [owner/repo] [branch]
#
# Exit codes:
#   0 = live protection matches ADR-025
#   1 = protection does not match (readable status printed)
#   2 = gh CLI missing, not authenticated, or API/network error
#
# Security: never prints raw API output or tokens — only derived booleans.

set -euo pipefail

REPO="${1:-}"
BRANCH="${2:-main}"

if [ -z "$REPO" ]; then
    REMOTE="$(git remote get-url origin 2>/dev/null || true)"
    REMOTE="${REMOTE%.git}"
    REPO="$(printf '%s' "$REMOTE" | sed -E 's#^.*github\.com[:/]([^/]+/[^/]+)$#\1#')"
fi
if [ -z "$REPO" ]; then
    echo "check-branch-protection: cannot determine repo; pass owner/repo" >&2
    exit 2
fi

if ! command -v gh >/dev/null 2>&1; then
    echo "check-branch-protection: gh CLI not found; install GitHub CLI (see ADR-025)" >&2
    exit 2
fi

if ! gh auth status >/dev/null 2>&1; then
    echo "check-branch-protection: not authenticated to GitHub; run gh auth login" >&2
    exit 2
fi

# Fetch protection JSON. On API/network failure gh exits non-zero; we fail
# clearly rather than comparing against stale/absent state. stdout is the
# only channel we read — gh never writes tokens to stdout for api calls.
PROT="$(gh api "repos/${REPO}/branches/${BRANCH}/protection" 2>/dev/null || true)"
if [ -z "$PROT" ]; then
    echo "check-branch-protection: cannot fetch branch protection for ${REPO} (auth/network/API error)" >&2
    exit 2
fi

# Parse the JSON with python3 (already a declared prerequisite elsewhere) and
# print ONLY derived booleans — never the raw payload, which may contain
# tokens in other fields.
STATUS="$(printf '%s' "$PROT" | python3 -c '
import json, sys
try:
    d = json.load(sys.stdin)
except Exception as e:
    print("ERROR: invalid protection JSON: %s" % e)
    sys.exit(2)
pr_req = d.get("required_pull_request_reviews") is not None
admins = bool(d.get("enforce_admins", {}).get("enabled"))
force_blocked = bool(d.get("allow_force_pushes", {}).get("enabled")) is False
deletion_blocked = bool(d.get("allow_deletions", {}).get("enabled")) is False
zero_checks = d.get("required_status_checks") is None
ok = pr_req and admins and force_blocked and deletion_blocked and zero_checks
status = "PASS" if ok else "FAIL"
print("%s pr_required=%s admins_enforced=%s force_push_blocked=%s deletion_blocked=%s zero_status_checks=%s" % (
    status, pr_req, admins, force_blocked, deletion_blocked, zero_checks))
sys.exit(0 if ok else 1)
' 2>/dev/null || true)"

if [ -z "$STATUS" ]; then
    echo "check-branch-protection: cannot parse protection payload for ${REPO}" >&2
    exit 2
fi

echo "check-branch-protection: ${STATUS}"
case "$STATUS" in
    PASS*) exit 0 ;;
    *) exit 1 ;;
esac
