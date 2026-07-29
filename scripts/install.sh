#!/usr/bin/env bash
# scripts/install.sh — install the OpenCode profile to $XDG_CONFIG_HOME/opencode-architecture.
# Idempotent. Does not overwrite unrelated files.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
XDG_CONFIG_HOME="${XDG_CONFIG_HOME:-$HOME/.config}"
TARGET="${XDG_CONFIG_HOME}/opencode-architecture"
SOURCE="${REPO_ROOT}/profile"

if [[ ! -d "${SOURCE}" ]]; then
  echo "archctl install: profile/ not found at ${SOURCE}" >&2
  exit 1
fi

mkdir -p "${TARGET}"
cp -R "${SOURCE}/." "${TARGET}/"

cat <<EOF
Installed OpenCode profile to ${TARGET}.

To launch OpenCode with this profile:

  export OPENCODE_CONFIG_DIR="${TARGET}"
  opencode

Or use the wrapper (future):

  archcode

Verify with:

  archctl doctor
EOF
