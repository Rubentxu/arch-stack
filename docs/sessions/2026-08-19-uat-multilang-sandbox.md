# Investigación — UAT completa multi-lenguaje sobre repos famosos con sandbox Podman

> Fecha: 2026-08-19 · Baseline: `main@a264d22` (post-release v1.69.0)
> Objetivo: diseñar pruebas UAT completas de `archctl` sobre repositorios
> famosos de GitHub en distintos lenguajes, ejecutadas en sandbox Podman.

## Resumen ejecutivo

La infraestructura base **ya existe** (sandbox M27 construido el 2026-08-06),
pero está (1) **desactualizada** — imagen de hace 12 días con archctl pre-v1.60,
último report del 06-Ago, antes de fusion engine/strict/redaction/workbench UX — y
(2) **limitada al vertical C4** — la mayoría de datasets corre UN solo
extractor. "UAT completa" requiere ampliar la matriz por lenguaje/comando,
refrescar el binario dentro del sandbox y añadir la capa de workbench (human
loop + Fara). Este documento define la matriz, los criterios de aceptación y
el plan por fases.

## 1. Estado actual del sandbox (M27, ADR-032)

| Componente | Estado |
|---|---|
| `bench/Containerfile` | ubuntu:24.04 + rustup 1.97.1 (toolchain; sin binario) |
| Imagen `archctl-bench:latest` | Construida localmente (12 días — toolchain válido, binario a compilar por run) |
| `bench/quadlets/archctl-bench.container` | Definido (oneshot, rootless, RemapUsers, XDG mapeado); **no instalado** en `~/.config/containers/systemd` |
| `bench/datasets.toml` | 11 repos pinned por SHA: rust×4 (axum, ripgrep, clap, archctl), ts×2 (zustand, vueuse), js×1 (express), go×1 (echo), python×1 (requests), java×1 (javapoet), kotlin×1 (mockk) |
| `bench/run-bench.sh` | Orquestador con métricas (exit, wall median, RSS, JSON validity, determinism baseRevision, baseline >10% bloquea) |
| `bench/sandbox-e2e.sh` | Vertical C4 completo dentro del container con asserts + verdict JSON |
| `e2e/human_loop_sandbox.sh` + `HUMAN_LOOP_TEST.md` | 9 fases de human loop dentro del sandbox (instalación → workbench → skills → errores) |
| Skills `uat-*` | 7 skills + 7 subagentes (planner/discovery/runner/reporter/guide/form-quality/evidence) |
| Fara (agente visual CUA) | **DOWN** — `localhost:8082` cerrado (requerido por uat-discovery/cua-test-orchestrator) |
| Cache datasets | `~/.cache/archctl-smoke/` poblado (11 repos) |

## 2. Gaps para "UAT completa"

1. **Binario stale en el sandbox**: los runs compilan in-container el código
   montado — refrescar a v1.69.0 (post-sweep).
2. **Cobertura por comando**: datasets actuales corren UN extractor cada uno.
   Falta la matriz completa por lenguaje (abajo).
3. **Nuevas capacidades no cubiertas**: strict export + redaction (v1.61–v1.67),
   `architecture explain/coverage/fuse/intent` (v1.49–v1.66), `/api/explain`
   (v1.68) — ninguno en el bench.
4. **Reports de hace 2 semanas**: 2 releases + fusion + strict + workbench UX
   sin datos empíricos.
5. **Capa workbench**: Fara caído; las fases 5 del human loop requieren
   navegador del host (`--network host`).
6. **Quadlet no instalado** en el systemd del usuario.

## 3. Matriz UAT por lenguaje

Capacidades por lenguaje (de `docs/CAPABILITIES.md` + datasets):
`c4-discover` = cargo/npm strategies (rust, ts, js) · `call-graph` = rust, ts,
python, go, java, kotlin · `class-diagram` = rust, ts, python, java ·
`state-machine` = kotlin · `sequence` = sobre datos call-graph (mismos
lenguajes) · `export/validate/strict` = repos C4 (rust, ts, js).

| Repo (lang) | c4-discover → accept → explain → coverage | export + validate | strict + redaction | call-graph | class-diagram | sequence |
|---|---|---|---|---|---|---|
| axum (rust) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| ripgrep (rust) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| clap (rust) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| archctl (rust, dogfood) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| zustand (ts) | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| vueuse (ts) | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| express (js) | ✓ | ✓ | ✓ | — | — | — |
| echo (go) | — | — | — | ✓ | — | ✓ |
| requests (python) | — | — | — | ✓ | ✓ | ✓ |
| javapoet (java) | — | — | — | ✓ | ✓ | ✓ |
| mockk (kotlin) | — | — | — | ✓ | — | ✓ (+state-machine) |

**Criterios de aceptación por celda**:

| Dimensión | Criterio |
|---|---|
| Exit code | 0 (excepciones documentadas: repo sin señal → warning envelope, no crash) |
| Validez | `--json` parseable; bundle `diagram validate` OK |
| No-vacío | extractores producen ≥1 elemento/relación/evidencia |
| Evidencia | tras `evidence accept`: ≥1 accepted con `file:line` resolvible |
| Determinismo | baseRevision estable entre runs; strict checksum estable |
| Redaction | strict bundle: secrets detectados se redactan; default no toca nada |
| Presupuesto | wall/RSS por dataset (baseline + regresión >10% bloquea) |
| Explain | `architecture explain <id>` devuelve subject + provenance (o 404 con id inválido, nunca panic) |

## 4. Capa workbench (human UAT)

- **Automatizable parcialmente**: fases 1–4 y 6–9 de `HUMAN_LOOP_TEST.md`
  corren en `human_loop_sandbox.sh` sin interacción.
- **Fase 5 (workbench)**: requiere navegador del host vía `--network host`
  (cumple ADR-011: bind 127.0.0.1). Verificación visual de zoom C4/action
  palette/relations (v1.68) → humana.
- **Fara CUA** (uat-discovery/uat-runner): requiere arrancar el server
  (`llama.cpp` HTTP, skill `cua-test-orchestrator`) para el flujo
  discovery→plan→run→evidencia sin humano.
- **Evidencia**: skills `uat-evidence` (hash SHA-256 + uat-session.yaml) y
  `uat-dashboard` (HTML self-contained).

## 5. Plan de ejecución

| Fase | Trabajo | Duración estimada |
|---|---|---|
| 0 | Instalar Quadlet + rebuild imagen (toolchain fresco) | 15 min |
| 1 | Smoke: matriz completa sobre 2 repos (axum, echo) | 30 min |
| 2 | Run completo 11 datasets × matriz extendida → `bench/reports/<date>.md` + baseline nuevo | 1–2 h |
| 3 | Fix de cualquier fallo (ciclos SDDK por bug) | variable |
| 4 | Workbench UAT: arrancar Fara + human loop en sandbox | 1 h |
| 5 | Dashboard UAT + evidencia encadenada | 30 min |

**Recomendación**: arrancar por Fase 0+1 (rebuild + smoke axum/echo) para
validar el sandbox con el binario v1.69.0 antes de la matriz completa.

## Referencias

- `docs/adr/ADR-032-bench-methodology.md`, `docs/specs/bench-harness.md`
- `docs/ROADMAP.md` §M27 (objetivo "datos sistemáticos multi-lenguaje pre-v1.0")
- `docs/adr/ADR-031-c4-vertical-validation.md` (precedente axum, 6 bugs)
- `bench/` (Containerfile, datasets.toml, run-bench.sh, sandbox-e2e.sh)
- `e2e/HUMAN_LOOP_TEST.md`, skills `uat-*`, `cua-test-orchestrator`
