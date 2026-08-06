#!/usr/bin/env bash
# embed-stack.sh — copy profile/{skills,agents,plugins} into
# archctl/assets-stack for rust-embed (stack distribution, ADR-033).
#
# The embedded set is the single source of truth for `archctl stack
# install|update|status`. Idempotent.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${REPO_ROOT}/profile"
DST="${REPO_ROOT}/archctl/assets-stack"

if [[ ! -d "${SRC}/skills" ]]; then
  echo "embed-stack: profile/skills not found at ${SRC}" >&2
  exit 1
fi

rm -rf "${DST}"
mkdir -p "${DST}"

for component in skills agents plugins; do
  if [[ -d "${SRC}/${component}" ]]; then
    cp -R "${SRC}/${component}" "${DST}/${component}"
  fi
done

# Guard: every skill dir must have a SKILL.md (frontmatter contract).
missing=0
for d in "${DST}"/skills/*/; do
  [[ -f "${d}/SKILL.md" ]] || { echo "embed-stack: missing SKILL.md in ${d}" >&2; missing=1; }
done
[[ "$missing" -eq 0 ]] || exit 1

echo "embed-stack: copied $(find "${DST}" -type f | wc -l) files to ${DST}"
