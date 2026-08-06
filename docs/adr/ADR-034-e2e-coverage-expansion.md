# ADR-034: E2E Coverage Expansion — Install, Deploy, Render, Multi-Language

## Status

Proposed — 2026-08-06

## Context

El ecosistema `arch-stack` (binario `archctl` + workbench `archview` + skills
de agente) alcanzó v1.0.1 con gates automáticos y manuales verdes (M27/M28).
Sin embargo, la cobertura E2E actual tiene **gaps verificados** que la
revisión de 2026-08-06 dejó al descubierto:

| Superficie | Estado actual | Gap |
|---|---|---|
| **Instalación** | Solo `stack install` probado manualmente en la máquina del dev | NO hay E2E de instalación desde cero (HOME limpio, binario release, idempotencia, drift) |
| **Despliegue** | Release v1.0.1 verificado manualmente (view 200, stack status) | NO hay verificación post-release automatizada |
| **Render** | E2E de playwright en `/tmp` (NO versionado) | NO hay suite de render versionada con asserts de DOM por tipo de bundle |
| **Multi-lenguaje** | `smoke_real_projects.rs` (4 repos) + bench (11 datasets) | smoke NO cubre vertical completo (accept, call-graph, class-diagram por lenguaje); bench NO renderiza |
| **Sandbox** | Quadlet verificado manualmente una vez | NO hay script reproducible/versionado |

La lección de ADR-031 se repite: los tests unitarios pasan mientras bugs
reales sobreviven en la integración. Cada gap de la tabla es un bug latente.

## Decision

**Ampliar el alcance E2E a 4 superficies, versionadas y ejecutables:**

### 1. E2E de instalación — `e2e/install_e2e.sh`

Prueba el flujo de usuario final contra un HOME limpio (temp dir):

1. Descargar el binario del release (GitHub) o usar `ARCHCTL_BIN`.
2. `archctl stack install` → verificar que skills/agents/plugin existen en
   `$HOME/.config/opencode/{skills,agents,plugins}`.
3. `archctl stack status` → `drift: none`.
4. Idempotencia: re-install → 0 cambios.
5. `archctl doctor` → OK.
6. Validar frontmatter SKILL.md de cada skill instalada (name/description).

Aislamiento: `HOME` y `XDG_CONFIG_HOME` apuntan a temp dirs; nunca toca la
config real del dev.

### 2. E2E de render — `e2e/render_e2e.py` (playwright, versionado)

Por CADA tipo de bundle (C4 context/container, sequence, class-diagram,
call-graph):

1. Arrancar `archctl view --port <n>` (binario release).
2. Cargar el bundle en el workbench (samples + bundles REALES exportados de
   repos multi-lenguaje).
3. **Assert de DOM**: nodes visibles, labels, relaciones, vista activa.
4. Screenshot como artifact.

Criterio: el DOM refleja el contenido del bundle (lo que el usuario VE es
lo que el grafo contiene). Cero errores de consola JS.

### 3. Smoke multi-lenguaje ampliado — `smoke_real_projects.rs`

Cada repo prueba el **vertical completo** según su lenguaje:

| Lenguaje | Vertical |
|---|---|
| rust | c4-discover → accept → export → validate |
| go | call-graph → apply → export → validate |
| python | class-diagram → apply → export → validate |
| javascript | c4-discover (npm-single) → accept → export → validate |

Además: `diagram validate` con asserts de no-vacío cuando hay containers
esperados.

### 4. Sandbox reproducible — `bench/sandbox-e2e.sh`

Reemplaza el one-off manual de 2026-08-06:

1. Build container (`bench/build.sh`).
2. Compilar `archctl` DENTRO del container (glibc nativo ubuntu:24.04).
3. Vertical C4 completo contra un dataset real (axum) con asserts.
4. Veredicto JSON (`PASS`/`FAIL` + métricas) para CI.

## Consequences

Positivas:
- Cada gap de la tabla se cierra con una suite ejecutable.
- Los E2E de render versionados detectan regresiones visuales (el bug de
  `detectKind` de 2026-08-06 se habría detectado con la suite de render).
- La instalación E2E valida el flujo de producto (stack install) que hoy
  solo se probó manualmente.
- El sandbox reproducible permite CI futura sin depender de un dev.

Negativas / trade-offs:
- Coste de mantenimiento: 4 suites nuevas que mantener en verde.
- Render E2E requiere playwright (dependencia de dev, no de runtime).
- Los E2E multi-lenguaje requieren red (clones de GitHub) — se marcan
  `#[ignore]` / `--skip` en CI normal, como los smoke actuales.

## Alternatives considered

- **Solo ampliar smoke Rust**: rechazado — no cubre render ni instalación,
  los dos gaps más peligrosos (producto distribuible vs suite de lib).
- **Un solo script E2E monolítico**: rechazado — 4 superficies con ciclos de
  vida distintos (instalación cambia con stack, render con archview, smoke
  con extractores, sandbox con podman).
- **CI obligatoria en PR**: rechazado para ahora — los E2E requieren red y
  son lentos; se integran como gates manuales (verify-local --full) y
  post-release, no como bloqueo de PR.

## References

- `docs/specs/e2e-installation.md` — especificación de la suite de instalación
- `docs/specs/e2e-render.md` — especificación de la suite de render
- `docs/specs/e2e-sandbox.md` — especificación del sandbox reproducible
- `docs/ROADMAP.md` M29 — milestone de implementación
