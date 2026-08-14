#!/usr/bin/env bash
# scripts/install-hooks.sh — wire `.githooks/` as the active
# `core.hooksPath` for this repository.
#
# Idempotent. Safe to re-run. Required once per clone because
# `.git/config` is per-machine and not transferred via push.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
HOOKS_DIR="${REPO_ROOT}/.githooks"
HOOK_FILE="${HOOKS_DIR}/commit-msg"

if [[ ! -f "${HOOK_FILE}" ]]; then
  echo "install-hooks: missing ${HOOK_FILE}" >&2
  echo "  Have you forgotten to switch branches? .githooks/ lives on main." >&2
  exit 1
fi

chmod +x "${HOOK_FILE}"

# `git config --local` writes to .git/config, not the user's
# global gitconfig. This is the per-clone configuration we want.
git config --local core.hooksPath .githooks

echo "install-hooks: core.hooksPath = .githooks"
echo "  hook: ${HOOK_FILE} (executable)"
echo
echo "Validating against the current HEAD commit history:"

# Sanity check: the .githooks scripts must accept every commit
# already in the repo. If any historical commit fails, surface it.
HISTORY_FAIL=0
while read -r sha rest; do
  msg=$(git log -1 --format='%B' "${sha}")
  if ! printf '%s' "${msg}" | "${HOOK_FILE}" /dev/stdin > /dev/null 2>&1; then
    echo "  WARN: ${sha} ${rest} (predates the hook — git does not re-validate history)"
    HISTORY_FAIL=1
  fi
done < <(git log --format='%h %s' -50)

if [[ "${HISTORY_FAIL}" -eq 0 ]]; then
  echo "  last 50 commits all conform."
else
  echo "  historical commits above predate the hook. The hook only blocks NEW commits."
fi

echo
echo "Done. Try a non-conventional commit to confirm the hook is active:"
echo "  git commit --allow-empty -m 'this should be blocked' -m 'and so should this.'"
echo
echo "Bypass with --no-verify only after human review of the rule in"
echo "docs/git-trunk-base.md."
