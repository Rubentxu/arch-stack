#!/usr/bin/env bash
# e2e/install_e2e.sh — E2E installation suite (ADR-034 §1, M29.1).
#
# Tests the PRODUCT install flow against an isolated HOME: stack install,
# drift check, idempotency, doctor, view health, skill frontmatter.
#
# Usage:
#   e2e/install_e2e.sh [--bin <path>] [--keep]
#
#   --bin <path>   archctl binary to test (default: download latest release)
#   --keep         keep the temp HOME (debug); default removes it
#
# Exit code: 0 = all checks pass; non-zero = first failure (set -e).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARCHCTL_BIN="${ARCHCTL_BIN:-}"
KEEP=0
FAILURES=0

usage() {
  cat <<EOF
install_e2e.sh — E2E installation suite

USAGE:
  e2e/install_e2e.sh [--bin <path>] [--keep]

ENV:
  ARCHCTL_BIN   path to archctl binary (default: download latest release)
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bin) ARCHCTL_BIN="$2"; shift 2 ;;
    --keep) KEEP=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown arg: $1" >&2; usage; exit 1 ;;
  esac
done

check() {
  local name="$1"; shift
  if "$@"; then
    echo "[PASS] $name"
  else
    echo "[FAIL] $name"
    FAILURES=$((FAILURES + 1))
  fi
}

# ── 1. Resolve binary ──────────────────────────────────────────────────────
if [[ -z "$ARCHCTL_BIN" ]]; then
  echo "== resolving latest release binary =="
  TMP_BIN_DIR=$(mktemp -d)
  gh release download --pattern 'archctl' --dir "$TMP_BIN_DIR" 2>/dev/null \
    || { echo "[FAIL] gh release download (is gh authenticated?)"; exit 1; }
  chmod +x "$TMP_BIN_DIR/archctl"
  ARCHCTL_BIN="$TMP_BIN_DIR/archctl"
fi

echo "== binary: $ARCHCTL_BIN =="
"$ARCHCTL_BIN" --version 2>/dev/null | grep -q "archctl" \
  || { echo "[FAIL] binary does not run"; exit 1; }

# ── 2. Isolated HOME ───────────────────────────────────────────────────────
E2E_ROOT=$(mktemp -d)
export HOME="$E2E_ROOT/home"
export XDG_CONFIG_HOME="$E2E_ROOT/home/.config"
mkdir -p "$HOME" "$XDG_CONFIG_HOME"
INSTALL_DIR="$XDG_CONFIG_HOME/opencode"
echo "== isolated HOME: $HOME =="

# ── 3. ide install ───────────────────────────────────────────────────────────
echo "== ide install =="
"$ARCHCTL_BIN" ide install opencode --install-root "$INSTALL_DIR" >/dev/null 2>&1 \
  || { echo "[FAIL] ide install opencode"; FAILURES=$((FAILURES + 1)); }

# ── 4. Skills/agents/plugin present ─────────────────────────────────────────
echo "== verifying installed components =="
for skill in architecture-discovery c4-from-graph class-view-from-graph \
             diagram-review evidence-lifecycle sequence-from-scenario \
             stack-management use-cases-from-graph workbench-view; do
  check "skill: $skill" test -f "$INSTALL_DIR/skills/$skill/SKILL.md"
done
check "agent: diagram-architect" test -f "$INSTALL_DIR/agents/diagram-architect.md"
check "agent: c4-modeler" test -f "$INSTALL_DIR/agents/c4-modeler.md"
check "agent: uml-modeler" test -f "$INSTALL_DIR/agents/uml-modeler.md"
check "agent: architecture-evidence" test -f "$INSTALL_DIR/agents/architecture-evidence.md"
check "agent: diagram-reviewer" test -f "$INSTALL_DIR/agents/diagram-reviewer.md"
check "plugin: archctl-env" test -f "$INSTALL_DIR/plugins/archctl-env.ts"

# ── 5. ide doctor — verify alignment ────────────────────────────────────────
echo "== ide doctor =="
if "$ARCHCTL_BIN" ide doctor opencode >/dev/null 2>&1; then
  echo "[PASS] ide doctor opencode: aligned"
else
  echo "[FAIL] ide doctor opencode: not aligned"
  FAILURES=$((FAILURES + 1))
fi

# ── 6. Idempotency ─────────────────────────────────────────────────────────
echo "== idempotency =="
BEFORE=$(find "$INSTALL_DIR" -type f | sort)
"$ARCHCTL_BIN" ide install opencode --install-root "$INSTALL_DIR" >/dev/null 2>&1
AFTER=$(find "$INSTALL_DIR" -type f | sort)
check "re-install is idempotent" test "$BEFORE" = "$AFTER"

# ── 7. Skill frontmatter valid ─────────────────────────────────────────────
echo "== skill frontmatter =="
for skill in "$INSTALL_DIR"/skills/*/; do
  name=$(basename "$skill")
  fm=$(head -5 "$skill/SKILL.md" | grep -c '^name:\|^description:')
  check "frontmatter: $name" test "$fm" -ge 2
done

# ── 8. doctor (isolated HOME) ──────────────────────────────────────────────
echo "== doctor =="
"$ARCHCTL_BIN" doctor --cwd "$E2E_ROOT" >/dev/null 2>&1 \
  && echo "[PASS] doctor" \
  || { echo "[FAIL] doctor"; FAILURES=$((FAILURES + 1)); }

# ── 9. view serves workbench ───────────────────────────────────────────────
echo "== view serves embedded workbench =="
"$ARCHCTL_BIN" view --port 0 >"$E2E_ROOT/view.log" 2>&1 &
VIEW_PID=$!
sleep 1.5
PORT=$(grep -oP '127\.0\.0\.1:\K[0-9]+' "$E2E_ROOT/view.log" | head -1 || true)
if [[ -n "$PORT" ]] && curl -sf "http://127.0.0.1:$PORT/api/health" 2>/dev/null | grep -q '"status":"ok"'; then
  echo "[PASS] view /api/health (port $PORT)"
else
  echo "[FAIL] view health"
  echo "--- view.log ---"
  cat "$E2E_ROOT/view.log" || true
  FAILURES=$((FAILURES + 1))
fi
kill "$VIEW_PID" 2>/dev/null || true
wait "$VIEW_PID" 2>/dev/null || true

# ── Summary ────────────────────────────────────────────────────────────────
echo
if [[ $FAILURES -eq 0 ]]; then
  echo "INSTALL_E2E PASS: all checks green"
  [[ $KEEP -eq 0 ]] && rm -rf "$E2E_ROOT"
  exit 0
else
  echo "INSTALL_E2E FAIL: $FAILURES checks failed"
  [[ $KEEP -eq 0 ]] && rm -rf "$E2E_ROOT"
  exit 1
fi
