# Human Loop Test Guide — arch-stack v1.0.x

> **Propósito:** probar el producto como lo usaría una persona real, paso a
> paso, con resultado esperado verificable en cada paso. Complementa las
> suites automáticas (M29): estas prueban que el sistema FUNCIONA; este
> guion prueba que el sistema se **SIENTE correcto** (UX, tiempos,
> outputs legibles, sin sorpresas).
>
> **Cuándo:** antes de cada release, tras cambios de UX, o en un entorno
> nuevo (otra máquina, otro OS).
>
> **Duración estimada:** 30-45 min.
>
> **Cómo usar:** sigue cada fase en orden. Marca `[PASS]`/`[FAIL]`/`[WARN]`
> por paso y anota observaciones. Al final, registra el veredicto.

---

## Preparación

```bash
# 1. Descarga el binario release (no el de desarrollo)
mkdir -p /tmp/hlt && cd /tmp/hlt
gh release download --repo Rubentxu/arch-stack --pattern archctl --clobber
chmod +x archctl
./archctl --version   # esperado: archctl 1.0.x
```

| # | Check | Resultado esperado | Verdicto |
|---|---|---|---|
| P0 | `--version` | `archctl 1.0.x` | [ ] |

---

## Fase 1 — Instalación como usuario nuevo

> Prueba el flujo de instalación del producto contra un HOME aislado
> (NO tocar la config real del dev).

```bash
export HOME=/tmp/hlt/home && mkdir -p $HOME
export XDG_CONFIG_HOME=$HOME/.config
/tmp/hlt/archctl stack install
/tmp/hlt/archctl stack status
```

| # | Check | Resultado esperado | Verdicto |
|---|---|---|---|
| 1.1 | `stack install` | "installed N components" | [ ] |
| 1.2 | `stack status` | "drift: none — stack aligned" | [ ] |
| 1.3 | Skills instaladas | `ls $XDG_CONFIG_HOME/opencode/skills/` → 9 dirs | [ ] |
| 1.4 | Agents instalados | `ls .../agents/` → 5 files | [ ] |
| 1.5 | Plugin instalado | `ls .../plugins/archctl-env.ts` existe | [ ] |
| 1.6 | Re-install idempotente | segunda ejecución: "stack is current" | [ ] |
| 1.7 | `doctor` | `DOCTOR: OK` | [ ] |

**Observaciones Fase 1:**
```
```

---

## Fase 2 — Descubrimiento en un proyecto real

> Prueba el vertical C4 en un repo real. Usa uno clonado o clona uno
> pequeño (mini-redis ≈ 3MB).

```bash
cd /tmp/hlt
git clone --depth 1 https://github.com/tokio-rs/mini-redis.git 2>/dev/null || true
cd mini-redis
/tmp/hlt/archctl code c4-discover --apply
/tmp/hlt/archctl evidence list --status drafted
```

| # | Check | Resultado esperado | Verdicto |
|---|---|---|---|
| 2.1 | `c4-discover --apply` | "Applied: N elements" con N ≥ 1 | [ ] |
| 2.2 | Output legible | lista de containers con nombre + ruta, no ruido | [ ] |
| 2.3 | Evidencias drafted | ≥ 1 evidencia con claim + file:line | [ ] |
| 2.4 | Tiempo percibido | < 3s en repo pequeño (sin compilación) | [ ] |

**Observaciones Fase 2:**
```
```

---

## Fase 3 — Evidencia: aceptar y verificar

```bash
# Acepta la primera evidencia drafted
FIRST=$(/tmp/hlt/archctl evidence list --status drafted --json | jq -r '.[0]."e.id"')
/tmp/hlt/archctl evidence accept --id "$FIRST"
/tmp/hlt/archctl evidence list --status accepted
```

| # | Check | Resultado esperado | Verdicto |
|---|---|---|---|
| 3.1 | `accept` | "accepted: ev:..." | [ ] |
| 3.2 | Lista accepted | la evidencia aparece con status accepted | [ ] |
| 3.3 | Persistencia | re-ejecutar `evidence list` → sigue ahí | [ ] |

**Observaciones Fase 3:**
```
```

---

## Fase 4 — Diagramas: exportar, validar, proyectar

```bash
cd /tmp/hlt/mini-redis
/tmp/hlt/archctl diagram export container:* --output /tmp/hlt/bundle
/tmp/hlt/archctl diagram validate /tmp/hlt/bundle
/tmp/hlt/archctl diagram project --view c4-container:* --format plantuml --output /tmp/hlt/out.puml
cat /tmp/hlt/out.puml | head -20
```

| # | Check | Resultado esperado | Verdicto |
|---|---|---|---|
| 4.1 | `export` | "Exported N elements... to /tmp/hlt/bundle" | [ ] |
| 4.2 | Bundle files | manifest/projection/evidence/styles.json existen | [ ] |
| 4.3 | `validate` | "Bundle ... is valid" | [ ] |
| 4.4 | `project --format plantuml` | archivo .puml con @startuml...@enduml | [ ] |
| 4.5 | DSL legible | nombres de containers reales (mini-redis, server, etc.) | [ ] |

**Observaciones Fase 4:**
```
```

---

## Fase 5 — Workbench interactivo (archctl view)

> La prueba MÁS importante de UX: el humano ve el diagrama renderizado.

```bash
cd /tmp/hlt/mini-redis
/tmp/hlt/archctl view --cwd . --port 18777 &
sleep 2
# Abre en el navegador: http://127.0.0.1:18777
```

| # | Check | Resultado esperado | Verdicto |
|---|---|---|---|
| 5.1 | Servidor arranca | "archctl view — http://127.0.0.1:18777" | [ ] |
| 5.2 | Workbench carga | landing con samples + input bundle URL | [ ] |
| 5.3 | **Carga bundle real** | pega `http://127.0.0.1:18777/api/export` en el input → Enter | [ ] |
| 5.4 | **Diagrama visible** | se ven los containers de mini-redis como tarjetas/nodos | [ ] |
| 5.5 | **Interacción** | click en un nodo → sidebar muestra evidencia | [ ] |
| 5.6 | **Navegación** | drill-down (All systems → container) funciona | [ ] |
| 5.7 | Sin errores JS | F12 → console: sin errores rojos | [ ] |
| 5.8 | Render fiel | lo que se ve coincide con `evidence list` (mismos containers) | [ ] |

```bash
# No olvides parar el server al terminar
kill %1
```

**Observaciones Fase 5 (LA MÁS IMPORTANTE — describe qué viste):**
```
```

---

## Fase 6 — Extractor por lenguaje (call-graph / class-diagram)

> call-graph MVP soporta **rust/ts/python/go**. class-diagram soporta **python**.

```bash
cd /tmp/hlt
git clone --depth 1 https://github.com/pmndrs/zustand.git 2>/dev/null || true
cd zustand
/tmp/hlt/archctl code call-graph --apply --json | jq '.elements_written'
git clone --depth 1 https://github.com/psf/requests.git 2>/dev/null || true
cd ../requests
/tmp/hlt/archctl code class-diagram --apply --json | jq '.elements_written'
```

| # | Check | Resultado esperado | Verdicto |
|---|---|---|---|
| 6.1 | call-graph (typescript) | número de funciones/edges > 0 | [ ] |
| 6.2 | class-diagram (python) | número de clases > 0 | [ ] |
| 6.3 | JSON válido | jq no da error (sin logs contaminando stdout) | [ ] |

**Observaciones Fase 6:**
```
```

---

## Fase 7 — Skills en el agente (OpenCode/ZCode)

> Prueba que las skills instaladas son descubiertas y útiles en el agente.

```bash
# En el agente (ZCode/OpenCode), con el HOME del paso 1:
# 1. Lista de skills disponibles → deben aparecer las 9 del stack
# 2. Invoca: "descubre la arquitectura de /tmp/hlt/mini-redis"
# 3. Invoca: "dame el diagrama C4 container de mini-redis"
# 4. Invoca: "revisa si el diagrama es válido"
```

| # | Check | Resultado esperado | Verdicto |
|---|---|---|---|
| 7.1 | Skills descubiertas | 9 skills del stack en el registro de skills | [ ] |
| 7.2 | `architecture-discovery` | el agente ejecuta `archctl code c4-discover` | [ ] |
| 7.3 | `c4-from-graph` | el agente exporta + proyecta sin inventar | [ ] |
| 7.4 | `diagram-review` | el agente valida antes de entregar | [ ] |
| 7.5 | `stack-management` | el agente sabe de `stack status/update` | [ ] |
| 7.6 | Sin comandos inventados | el agente usa SOLO comandos reales del CLI | [ ] |

**Observaciones Fase 7:**
```
```

---

## Fase 8 — Actualización y drift

```bash
# Simula una skill modificada a mano (drift)
echo "# hacked" >> $HOME/.config/opencode/skills/stack-management/SKILL.md
/tmp/hlt/archctl stack status
/tmp/hlt/archctl stack update
/tmp/hlt/archctl stack status
```

| # | Check | Resultado esperado | Verdicto |
|---|---|---|---|
| 8.1 | Drift detectado | "stale: skills/stack-management/SKILL.md" | [ ] |
| 8.2 | `update` restaura | "updated N components" | [ ] |
| 8.3 | Drift resuelto | "drift: none" tras update | [ ] |

**Observaciones Fase 8:**
```
```

---

## Fase 9 — Errores y límites (UX de fallo)

| # | Escenario | Resultado esperado | Verdicto |
|---|---|---|---|
| 9.1 | `archctl diagram export nope:*` (selector inválido); `container:*` sobre /tmp (vacío) | error claro, exit ≠ 0, NO panic; empty-graph: exit 0, JSON con `empty: true` | [ ] |
| 9.2 | `call-graph` sobre repo Go (soportado desde M30) | extracción real: `project.filesScanned > 0` (rápida; apply-path cubierto por `smoke_go_apply_fixture`) | [ ] |
| 9.3 | `archctl view` sin assets (binario mal build) | "view assets not embedded — run: ..." | [ ] |
| 9.4 | `stack install` con HOME sin permisos | error claro de filesystem | [ ] |
| 9.5 | `evidence accept` con id inexistente | "not found" claro | [ ] |
| 9.6 | Ctrl+C en `view` | server para limpiamente, sin proceso zombie | [ ] |

**Observaciones Fase 9:**
```
```

---

## Veredicto final

| Fase | PASS | FAIL | WARN |
|---|---|---|---|
| 1 Instalación | | | |
| 2 Descubrimiento | | | |
| 3 Evidencia | | | |
| 4 Diagramas | | | |
| 5 Workbench | | | |
| 6 Multi-lenguaje | | | |
| 7 Skills agente | | | |
| 8 Update/drift | | | |
| 9 Errores | | | |

**Veredicto global:** [ ] READY FOR RELEASE  [ ] BLOQUEADO

**Bloqueadores (FAIL con impacto):**
```
1. ...
2. ...

```

**Mejoras sugeridas (WARN, no bloqueantes):**
```
1. ...
2. ...

```

**Tester:** ______________  **Fecha:** ______________  **Binario probado:** v________

---

## Notas de mantenimiento

- Este guion es MANUAL a propósito: las suites automáticas (M29) ya cubren
  que el sistema funcione; aquí se prueba la experiencia subjetiva.
- Los checks 5.3-5.6 (workbench) y 7.x (skills) son los de mayor valor —
  son los que un script no puede juzgar bien.
- Actualizar los comandos si el CLI cambia (verificar contra `--help`).
- Un FAIL en 5.x o 7.x es bloqueante de release SIEMPRE.
