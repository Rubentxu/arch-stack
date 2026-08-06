# Spec — E2E Installation Suite (`e2e/install_e2e.sh`)

> **Referencia:** [ADR-034](adr/ADR-034-e2e-coverage-expansion.md) §1
> **Estado:** Propuesta — 2026-08-06
> **Milestone:** M29 (E2E coverage expansion)

## Objetivo

Probar el flujo de **instalación del producto** (no de desarrollo) contra un
entorno limpio y aislado. El producto `arch-stack` es UN todo: binario +
workbench embebido + skills/agents/plugin (ADR-033). La instalación E2E
verifica que un usuario nuevo obtiene el stack completo y funcional.

## Alcance

- Instalación desde cero con HOME aislado (temp dir).
- `archctl stack install` → skills/agents/plugin copiados a los paths de
  descubrimiento OpenCode/ZCode.
- `archctl stack status` → drift none.
- Idempotencia (re-install no cambia nada).
- `archctl doctor` → OK.
- Validación de frontmatter de skills instaladas.
- Verificación de que `archctl view` sirve el workbench embebido.

## Fuera de alcance

- Instalación de dependencias del sistema (podman, node) — prerrequisito.
- Configuración de modelos/providers del agente — del usuario.
- Compatibilidad Windows/macOS — v1.x (ADR-033 asume Linux dev).

## Prerrequisitos

- `ARCHCTL_BIN` apunta al binario release (o se descarga del último release
  GitHub si no se especifica).
- `bash`, `jq`, `curl` disponibles.
- Sin red requerida si `ARCHCTL_BIN` se pasa explícitamente (los clones de
  repos NO forman parte de esta suite).

## Procedimiento (pasos verificables)

```bash
E2E_ROOT=$(mktemp -d)          # HOME aislado
export HOME="$E2E_ROOT/home"
export XDG_CONFIG_HOME="$E2E_ROOT/home/.config"
mkdir -p "$HOME"

# 1. Instalar el stack
"$ARCHCTL_BIN" stack install --dir "$XDG_CONFIG_HOME/opencode"

# 2. Verificar copia
for skill in architecture-discovery c4-from-graph class-view-from-graph \
             diagram-review evidence-lifecycle sequence-from-scenario \
             stack-management use-cases-from-graph workbench-view; do
  test -f "$XDG_CONFIG_HOME/opencode/skills/$skill/SKILL.md" || FAIL "skill $skill"
done
test -f "$XDG_CONFIG_HOME/opencode/agents/diagram-architect.md" || FAIL "agent"
test -f "$XDG_CONFIG_HOME/opencode/plugins/archctl-env.ts" || FAIL "plugin"

# 3. Status sin drift
"$ARCHCTL_BIN" stack status --dir "$XDG_CONFIG_HOME/opencode" | grep "drift: none"

# 4. Idempotencia
BEFORE=$(find "$XDG_CONFIG_HOME/opencode" -type f | sort)
"$ARCHCTL_BIN" stack install --dir "$XDG_CONFIG_HOME/opencode" >/dev/null
AFTER=$(find "$XDG_CONFIG_HOME/opencode" -type f | sort)
[ "$BEFORE" = "$AFTER" ] || FAIL "idempotencia rota"

# 5. Doctor (scope gates) con HOME aislado
"$ARCHCTL_BIN" doctor --cwd "$E2E_ROOT" || FAIL "doctor"

# 6. View sirve el workbench embebido
"$ARCHCTL_BIN" view --port 0 >/tmp/view.log 2>&1 &
sleep 1
curl -sf http://127.0.0.1:*/api/health | grep '"status":"ok"' || FAIL "view health"
kill %1
```

## Criterios de aceptación

| # | Criterio | Método de verificación |
|---|---|---|
| 1 | Skills instaladas en `skills/` | `test -f` por cada skill |
| 2 | Agents instalados en `agents/` | `test -f` |
| 3 | Plugin instalado en `plugins/` | `test -f` |
| 4 | `stack status` reporta drift none | grep en stdout |
| 5 | Re-install es idempotente | diff de lista de archivos |
| 6 | `doctor` pasa con HOME aislado | exit 0 |
| 7 | `view` sirve `/api/health` OK | curl + grep |
| 8 | Frontmatter SKILL.md válido (name/description) | jq/yq sobre el YAML |

## Entregables

1. `e2e/install_e2e.sh` (~80 líneas bash, idempotente, exit != 0 en fallo).
2. `e2e/README.md` — cómo correr la suite.
3. Integración en `verify-local.sh --full` (gate manual).

## Referencias

- ADR-033 (stack distribution), ADR-034 (decisión), M29 (milestone)
