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

rm -rf "${DST}"
mkdir -p "${DST}"
# Copy everything except sourcemaps (dev-only, ~7MB of the 8MB dist).
(cd "${SRC}" && tar --exclude='*.map' -cf - .) | (cd "${DST}" && tar -xf -)
# Preserve the tracked placeholder README (it explains the folder).
if [[ -f "${REPO_ROOT}/archctl/assets-view/README.md" ]]; then :; fi
# Re-create README if the previous copy overwrote it (it never does: dist
# has no README.md, but guard against future dist layouts).
if [[ ! -f "${DST}/README.md" ]]; then
  cat > "${DST}/README.md" << 'EOF'
# assets-view

Carpeta de origen para el workbench `archview` embebido en el binario
`archctl` (ADR-033). Generada por `scripts/embed-view.sh` desde
`archview/dist` (sourcemaps excluidos). Gitignored; se reconstruye en CI.
EOF
fi

echo "embed-view: copied $(find "${DST}" -type f | wc -l) files to ${DST}"
