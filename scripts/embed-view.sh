#!/usr/bin/env bash
# embed-view.sh — copy archview/dist into archctl/assets-view for rust-embed.
#
# Excludes sourcemaps (*.map) — dev-only, ~7MB of the 8MB dist.
# Idempotent. Called by scripts/verify-local.sh pre-push and CI release job.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${REPO_ROOT}/archview/dist"
DST="${REPO_ROOT}/archctl/assets-view"

if [[ ! -d "${SRC}" ]]; then
  echo "embed-view: archview/dist not found — run 'pnpm build' in archview first" >&2
  exit 1
fi

# Preserve the tracked placeholder README across the rm -rf (it explains
# the folder; the dist never contains one, but guard against future layouts).
README_SRC="${REPO_ROOT}/archctl/assets-view/README.md"
if [[ -f "${README_SRC}" ]]; then
  README_CONTENT="$(cat "${README_SRC}")"
fi

rm -rf "${DST}"
mkdir -p "${DST}"
# Copy everything except sourcemaps (dev-only, ~7MB of the 8MB dist).
(cd "${SRC}" && tar --exclude='*.map' -cf - .) | (cd "${DST}" && tar -xf -)
# Restore the tracked README.
if [[ -n "${README_CONTENT:-}" ]]; then
  printf '%s\n' "${README_CONTENT}" > "${DST}/README.md"
else
  cat > "${DST}/README.md" << 'EOF'
# assets-view

Carpeta de origen para el workbench `archview` embebido en el binario
`archctl` (ADR-033). Generada por `scripts/embed-view.sh` desde
`archview/dist` (sourcemaps excluidos). Gitignored; se reconstruye en CI.
EOF
fi

echo "embed-view: copied $(find "${DST}" -type f | wc -l) files to ${DST}"
