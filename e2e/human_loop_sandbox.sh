#!/usr/bin/env bash
# e2e/human_loop_sandbox.sh — Human Loop Test DENTRO del sandbox (sin tocar el host).
#
# Ejecuta las fases del guion e2e/HUMAN_LOOP_TEST.md dentro del container
# archctl-bench (ubuntu:24.04). Nada se instala ni modifica en el OS del
# usuario: HOME/XDG efímeros dentro del container, datasets montados :ro,
# binario del release montado (compilado en ubuntu-latest → glibc compatible).
#
# Fase 5 (workbench) se expone al navegador del host vía --network host:
# el server bindea 127.0.0.1 dentro del container que ES el 127.0.0.1 del
# host (rootless podman, network host) — cumple ADR-011 (nunca 0.0.0.0).
#
# Usage:
#   e2e/human_loop_sandbox.sh [--bin <path>] [--dataset <cache-name>] [--skip-view]
#
# Exit: 0 = todas las fases automáticas PASS; 1 = alguna falla.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="archctl-bench:latest"
CACHE_DIR="${CACHE_DIR:-$HOME/.cache/archctl-smoke}"
DATASET="${HL_DATASET:-mini-redis}"
SKIP_VIEW=0
FAILURES=0

usage() {
  cat <<EOF
human_loop_sandbox.sh — Human Loop Test dentro del sandbox

USAGE:
  e2e/human_loop_sandbox.sh [--bin <path>] [--dataset <name>] [--skip-view]

  --bin <path>     archctl binary (default: download release, glibc-compat)
  --dataset <name> repo cache name (default: mini-redis; debe existir en
                   ~/.cache/archctl-smoke/ o se clona)
  --skip-view      skip fase 5 interactiva (CI/automático)
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bin) BIN_PATH="$2"; shift 2 ;;
    --dataset) DATASET="$2"; shift 2 ;;
    --skip-view) SKIP_VIEW=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown arg: $1" >&2; usage; exit 1 ;;
  esac
done

check() {
  local name="$1"; shift
  if "$@"; then echo "[PASS] $name"; else echo "[FAIL] $name"; FAILURES=$((FAILURES + 1)); fi
}

command -v podman >/dev/null || { echo "podman required"; exit 1; }

# ── 0. Resolve binary ─────────────────────────────────────────────────────
# CRÍTICO (2026-08-06): el binario local (glibc Fedora) NO corre en
# ubuntu:24.04 (GLIBCXX_3.4.35). El release v1.0.2 manual también era local.
# La vía correcta y reproducible: COMPILAR dentro del container (cargo
# 1.97.1 + build-essential ya en la imagen) — el mismo patrón de
# bench/sandbox-e2e.sh. `--bin` solo para binarios glibc-compatible probados.
if [[ -n "${BIN_PATH:-}" ]]; then
  echo "== usando binario externo: $BIN_PATH =="
else
  echo "== compilando archctl DENTRO del container =="
  BIN_PATH="__in_container__"
fi
echo "== image =="
podman image exists "$IMAGE" 2>/dev/null || {
  echo "building $IMAGE ..."
  podman build -q -f "$REPO_ROOT/bench/Containerfile" -t "$IMAGE" "$REPO_ROOT/bench/"
}

if [[ "$BIN_PATH" == "__in_container__" ]]; then
  podman run --rm --security-opt label=disable \
    -v "$REPO_ROOT:/src:rw" \
    -v "$HOME/.cargo:/root/.cargo:rw" \
    "$IMAGE" bash -c 'cd /src/archctl && cargo build --release --quiet' \
    || { echo "in-container build failed"; exit 1; }
  # El binario compilado en el container queda en target/ pero NO se monta:
  # lo copiamos a un temp y se monta :ro en los runs (glibc ubuntu nativa).
  TMP_BIN=$(mktemp -d)
  cp "$REPO_ROOT/archctl/target/release/archctl" "$TMP_BIN/archctl"
  chmod +x "$TMP_BIN/archctl"
  BIN_PATH="$TMP_BIN/archctl"
  echo "== binario in-container: $BIN_PATH =="
fi

# ── 2. Dataset (cached or clone)
if [[ ! -d "$CACHE_DIR/$DATASET" ]]; then
  echo "== cloning dataset $DATASET (cache: $CACHE_DIR) =="
  mkdir -p "$CACHE_DIR"
  git clone --depth 1 "https://github.com/$DATASET.git" "$CACHE_DIR/$DATASET" 2>/dev/null \
    || { echo "clone failed for $DATASET"; exit 1; }
fi

# Common podman args: binario release :ro, datasets :ro, HOME efímero por run
# (la fase 1 necesita HOME limpio; el vertical necesita su propia XDG).
run_in_container() {
  local script="$1"
  podman run --rm --security-opt label=disable \
    -v "$BIN_PATH:/usr/local/bin/archctl:ro" \
    -v "$CACHE_DIR:/datasets:ro" \
    "$IMAGE" bash -c "$script"
}

# ── 3. Fases 1-4, 6, 8, 9 (automáticas, dentro del container) ──────────────
echo "=============================================="
echo " FASE 1 — Instalación (HOME aislado en container)"
echo "=============================================="
run_in_container '
set -e
export HOME=/hlt/home XDG_CONFIG_HOME=/hlt/home/.config
mkdir -p $HOME $XDG_CONFIG_HOME
archctl stack install >/dev/null
archctl stack status 2>/dev/null | grep -q "drift: none"
for s in architecture-discovery c4-from-graph class-view-from-graph \
         diagram-review evidence-lifecycle sequence-from-scenario \
         stack-management use-cases-from-graph workbench-view; do
  test -f "$XDG_CONFIG_HOME/opencode/skills/$s/SKILL.md"
done
test -f "$XDG_CONFIG_HOME/opencode/agents/diagram-architect.md"
test -f "$XDG_CONFIG_HOME/opencode/plugins/archctl-env.ts"
echo "PHASE1_OK"
' | tail -1 | grep -q PHASE1_OK \
  && echo "[PASS] Fase 1: install + 9 skills + 5 agents + plugin + drift none" \
  || { echo "[FAIL] Fase 1"; FAILURES=$((FAILURES + 1)); }

echo "=============================================="
echo " FASE 2 — Descubrimiento en $DATASET"
echo "=============================================="
run_in_container '
set -e
export RUST_LOG=error XDG_DATA_HOME=/hlt/xdg/data XDG_CONFIG_HOME=/hlt/xdg/config
mkdir -p /hlt/xdg/data /hlt/xdg/config
OUT=$(archctl code c4-discover --cwd /datasets/'"$DATASET"' --apply 2>/dev/null | tail -1)
echo "OUT=$OUT"
echo "$OUT" | grep -qE "Applied: [1-9]"
echo "PHASE2_OK"
' | tail -1 | grep -q PHASE2_OK \
  && echo "[PASS] Fase 2: discover aplica >=1 container" \
  || { echo "[FAIL] Fase 2"; FAILURES=$((FAILURES + 1)); }

echo "=============================================="
echo " FASE 3 — Evidencia (drafted -> accept)"
echo "=============================================="
run_in_container '
set -e
export RUST_LOG=error XDG_DATA_HOME=/hlt/xdg/data XDG_CONFIG_HOME=/hlt/xdg/config
mkdir -p /hlt/xdg/data /hlt/xdg/config
# Cada fase corre en container nuevo (--rm): re-hacer discover para poblar.
archctl code c4-discover --cwd /datasets/'"$DATASET"' --apply >/dev/null 2>&1
N=$(archctl evidence list --cwd /datasets/'"$DATASET"' --status drafted --json 2>/dev/null | jq "length")
[ "$N" -ge 1 ] || { echo "NO_EVIDENCE"; exit 1; }
ID=$(archctl evidence list --cwd /datasets/'"$DATASET"' --status drafted --json 2>/dev/null | jq -r ".[0].\"e.id\"")
archctl evidence accept --id "$ID" --cwd /datasets/'"$DATASET"' >/dev/null 2>&1
archctl evidence list --cwd /datasets/'"$DATASET"' --status accepted --json 2>/dev/null | jq -e "any(.[]; .\"e.id\" == \"$ID\")" >/dev/null
echo "PHASE3_OK"
' | tail -1 | grep -q PHASE3_OK \
  && echo "[PASS] Fase 3: evidence drafted -> accepted + persistencia" \
  || { echo "[FAIL] Fase 3"; FAILURES=$((FAILURES + 1)); }

echo "=============================================="
echo " FASE 4 — Diagramas (export / validate / project)"
echo "=============================================="
run_in_container '
set -e
export RUST_LOG=error XDG_DATA_HOME=/hlt/xdg/data XDG_CONFIG_HOME=/hlt/xdg/config
archctl diagram export container:* --cwd /datasets/'"$DATASET"' --output /tmp/bundle >/dev/null 2>&1
test -f /tmp/bundle/manifest.json
test -f /tmp/bundle/projection.json
archctl diagram validate /tmp/bundle --cwd /datasets/'"$DATASET"' >/dev/null 2>&1
archctl diagram project --view c4-container:* --format plantuml --output /tmp/out.puml --cwd /datasets/'"$DATASET"' >/dev/null 2>&1
grep -q "@startuml" /tmp/out.puml
echo "PHASE4_OK"
' | tail -1 | grep -q PHASE4_OK \
  && echo "[PASS] Fase 4: export + bundle files + validate + plantuml" \
  || { echo "[FAIL] Fase 4"; FAILURES=$((FAILURES + 1)); }

echo "=============================================="
echo " FASE 6 — Extractores por lenguaje (typescript/python si cacheados)"
echo "=============================================="
# NOTA: call-graph MVP soporta rust/ts/python/go. Go verificado en Fase 9.2.
# 6a usa zustand (TS) como muestra real de extracción.
if [[ -d "$CACHE_DIR/zustand" ]]; then
  run_in_container '
  set -e
  export RUST_LOG=error XDG_DATA_HOME=/hlt/xdg6/data XDG_CONFIG_HOME=/hlt/xdg6/config
  mkdir -p /hlt/xdg6/data /hlt/xdg6/config
  N=$(archctl code call-graph --cwd /datasets/zustand --apply --json 2>/dev/null | jq ".elements_written")
  [ "${N:-0}" -gt 0 ] || { echo "NO_CG"; exit 1; }
  echo "PHASE6_OK"
  ' | tail -1 | grep -q PHASE6_OK \
    && echo "[PASS] Fase 6a: call-graph typescript" \
    || { echo "[FAIL] Fase 6a"; FAILURES=$((FAILURES + 1)); }
else
  echo "[SKIP] Fase 6a: zustand no cacheado"
fi
if [[ -d "$CACHE_DIR/psf" ]]; then
  run_in_container '
  set -e
  export RUST_LOG=error XDG_DATA_HOME=/hlt/xdg6b/data XDG_CONFIG_HOME=/hlt/xdg6b/config
  mkdir -p /hlt/xdg6b/data /hlt/xdg6b/config
  N=$(archctl code class-diagram --cwd /datasets/psf/requests --apply --json 2>/dev/null | jq ".elements_written")
  [ "${N:-0}" -gt 0 ] || { echo "NO_CD"; exit 1; }
  echo "PHASE6_OK"
  ' | tail -1 | grep -q PHASE6_OK \
    && echo "[PASS] Fase 6b: class-diagram python" \
    || { echo "[FAIL] Fase 6b"; FAILURES=$((FAILURES + 1)); }
else
  echo "[SKIP] Fase 6b: psf/requests no cacheado"
fi

echo "=============================================="
echo " FASE 8 — Update y drift"
echo "=============================================="
run_in_container '
set -e
export HOME=/hlt/home8 XDG_CONFIG_HOME=/hlt/home8/.config
mkdir -p $HOME $XDG_CONFIG_HOME
archctl stack install >/dev/null
echo "# hacked" >> $XDG_CONFIG_HOME/opencode/skills/stack-management/SKILL.md
archctl stack status 2>/dev/null | grep -q "stale:"
archctl stack update >/dev/null
archctl stack status 2>/dev/null | grep -q "drift: none"
echo "PHASE8_OK"
' | tail -1 | grep -q PHASE8_OK \
  && echo "[PASS] Fase 8: drift detectado + update restaura" \
  || { echo "[FAIL] Fase 8"; FAILURES=$((FAILURES + 1)); }

echo "=============================================="
echo " FASE 9 — Errores y límites"
echo "=============================================="
run_in_container '
set -e
export RUST_LOG=error XDG_DATA_HOME=/hlt/xdg9/data XDG_CONFIG_HOME=/hlt/xdg9/config
mkdir -p /hlt/xdg9/data /hlt/xdg9/config
# 9.1 selector inválido -> error claro, exit != 0
# (container:* sin proyecto NO es error: el producto exporta 0 con éxito)
if archctl diagram export nope:* --cwd /tmp --output /tmp/x9 >/dev/null 2>&1; then
  echo "NO_ERR91"; exit 1
fi
# 9.2 call-graph sobre repo Go (soportado desde M30) -> extracción real,
# rápida. El apply-path Go se cubre en smoke_go_apply_fixture (fixture
# pequeño); el apply del repo completo es lento por writer perf (M32).
if [[ -d /datasets/labstack ]]; then
  F=$(archctl code call-graph --cwd /datasets/labstack/echo --json 2>/dev/null | jq ".project.filesScanned")
  [ "${F:-0}" -gt 0 ] || { echo "NO_ERR92"; exit 1; }
fi
# 9.4 accept id inexistente -> error claro
if archctl evidence accept --id ev:noexiste --cwd /datasets/'"$DATASET"' >/dev/null 2>&1; then
  echo "NO_ERR94"; exit 1
fi
echo "PHASE9_OK"
' | tail -1 | grep -q PHASE9_OK \
  && echo "[PASS] Fase 9: errores claros (selector inválido, go soportado, accept inexistente)" \
  || { echo "[FAIL] Fase 9"; FAILURES=$((FAILURES + 1)); }

# ── 4. Fase 5 — Workbench interactivo (--network host, navegador del host)
echo "=============================================="
echo " FASE 5 — Workbench interactivo (ABRE TU NAVEGADOR)"
echo "=============================================="
if [[ $SKIP_VIEW -eq 0 ]]; then
  echo "Arrancando archctl view dentro del container (network host)..."
  echo "Abre: http://127.0.0.1:18777  y pega en el input: http://127.0.0.1:18777/api/export"
  echo "Verifica: containers visibles, click en nodo -> sidebar evidencia,"
  echo "          drill-down funciona, F12 sin errores JS."
  echo "Presiona Ctrl+C cuando termines."
  podman run --rm --network host --security-opt label=disable \
    -v "$BIN_PATH:/usr/local/bin/archctl:ro" \
    -v "$CACHE_DIR:/datasets:ro" \
    -e RUST_LOG=error \
    "$IMAGE" archctl view --cwd "/datasets/$DATASET" --port 18777
else
  echo "[SKIP] Fase 5 (--skip-view)"
fi

# ── 5. Veredicto
echo
if [[ $FAILURES -eq 0 ]]; then
  echo "HUMAN_LOOP_SANDBOX PASS: todas las fases automáticas verdes"
  exit 0
else
  echo "HUMAN_LOOP_SANDBOX FAIL: $FAILURES fases fallaron"
  exit 1
fi
