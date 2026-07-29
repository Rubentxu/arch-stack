# Resumen ejecutivo — `archctl`

> **Veredicto: viable, con validación previa obligatoria.** No conviene construir la plataforma completa descrita en `Skills-para-agentes-IA.md`. Conviene empezar por una skill útil, probar la recuperación arquitectónica con evidencia real y ampliar solo si supera umbrales medibles.

## Decisión recomendada

Construir en tres escalones reversibles:

| Hito | Resultado | Ventana | Condición para avanzar |
|---|---|---:|---|
| **M0 — Gate Zero** | Una skill adaptada a OpenCode y una micro-recuperación sobre un fixture no-Git de 5 archivos | 3–4 días | IR producido —no escrito a mano—, proyección válida, render local y cero afirmaciones de alta confianza sin evidencia |
| **M1 — Discovery Spike** | Evidencias, IR y proyecciones probadas en un repositorio Rust pequeño y uno TypeScript mediano | 2–4 semanas | Cobertura ≥ 0,90; render 100 %; Jaccard ≥ 0,95; fixtures adversariales superados |
| **M2 — MVP plugin-first** | Plugin TypeScript, cuatro roles, almacenamiento XDG, auditoría y operación básica | 4–6 semanas | Precisión ≥ 0,85; recall ≥ 0,80; < 50k tokens; primera vista < 10 min |

**Rust queda diferido a M3** y solo se reconsidera si M2 funciona y el coste medido de TypeScript justifica mantener un binario y un contrato IPC.

## Lo valioso del documento original

- Evidencia separada de inferencias, hipótesis, desconocidos y conflictos.
- Architecture IR neutral como única fuente de verdad; los diagramas son proyecciones.
- Separación futura entre grafos declarados, estáticos y observados.
- Preguntas humanas guiadas por impacto × incertidumbre × coste de error.
- Auditor que intenta refutar el modelo, no embellecerlo.
- Persistencia fuera del repositorio analizado.

## Lo que se recortó

- De nueve agentes a **cuatro roles**: orquestador, extractor, sintetizador y auditor.
- Sin núcleo Rust, gemelo temporal, grafo observado, Joern/CodeQL, CI de drift ni SDK headless en el MVP.
- Sin parsers propios: adaptar `ast-grep`, `ctags`, herramientas de build y renderizadores locales.
- Mermaid queda como preview; no es representación canónica.

## Correcciones verificadas

| Tema | Posición final |
|---|---|
| OpenCode MCP | La clave actual es `mcp`; los directorios son `.opencode/agents/`, `skills/`, `commands/` y `plugins/` |
| Structurizr | Lite está EOL; `local` es el visor/workspace local. La validación/export headless usa una herramienta fijada por versión y sigue la evolución de vNext |
| Fuente C4 | El IR es la verdad; Structurizr DSL es una proyección C4 |
| Persistencia | XDG para estado runtime + bundle exportable con `projectId` portable y rebind explícito |
| Identidad | `SourceIdentity` discriminada: repositorio Git o directorio no-Git; Git es opcional |
| Seguridad epistémica | Una afirmación con confianza ≥ 0,9 y sin evidencia es **HARD FAIL** |
| Lenguaje | TypeScript en M0–M2; Node o Bun se decide mediante probe, no por suposición |

## Pros y contras

**Pros:** entrega valor antes de construir plataforma, es reversible, renderer-independent, offline-first y falsificable. Si M0 o M1 fallan, queda una skill disciplinada y reutilizable.

**Contras:** la fiabilidad de ingeniería inversa sigue sin validarse; adaptar skills de Claude Code puede fallar; el toolchain local añade fricción; el IR concentra acoplamiento y debe permanecer mínimo.

## Hipótesis pendiente

Los papers y repositorios citados respaldan patrones multiagente y generación C4/UML, pero **no demuestran** recuperación fiable de arquitectura en repositorios reales arbitrarios. Gate Zero y M1 existen para intentar refutar esa hipótesis antes de invertir más.

## Decisiones pendientes

Los ocho ADRs están **Aceptados** desde 2026-07-29 con reglas operativas concretas (ver índice).
Quedan como experimentales, no bloqueantes, los siguientes puntos:

1. Calibración de confianza — experimento abierto de M1.
2. Elección de runtime TypeScript (Node o Bun) — decidida por Gate Zero.
3. `store-source-snippets: false` es la postura por defecto; puede activarse explícitamente por proyecto.

## Siguiente acción

Ejecutar únicamente **M0 / Gate Zero**. Si falla compatibilidad, confinamiento, recuperación semántica, proyección o renderizado, detener la plataforma y conservar la baseline de skills.

## Documentos

- [ROADMAP](ROADMAP.md)
- [Índice de ADRs](adr/README.md)
- [Exploración](../sddk/architecture-intelligence-platform/explore-report.md)
- [Propuesta](../sddk/architecture-intelligence-platform/proposal.md)
- [Especificación](../sddk/architecture-intelligence-platform/spec.md)
- [Diseño](../sddk/architecture-intelligence-platform/design.md)
- [Tareas](../sddk/architecture-intelligence-platform/tasks.md)
- [Verificación](../sddk/architecture-intelligence-platform/verification-report.md)
