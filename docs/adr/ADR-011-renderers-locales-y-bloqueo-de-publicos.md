# ADR-011 — Renderers locales; bloqueo de servicios públicos

**Estado:** Aceptado
**Fecha:** 29 de julio de 2026

## Contexto

Ni ADR-007 (diagramas como proyecciones) ni ADR-005 (LadybugDB) explicitan
la política sobre renderers públicos. El documento inicial
`Skills-para-agentes-IA.md` (§187-192) lo dice explícitamente:

> Para repositorios corporativos usaría siempre `PlantUML local` o `Kroki
> desplegado internamente`. No enviaría código, nombres de sistemas ni
> diagramas a un Kroki público. La propia skill distingue entre el backend
> público y las alternativas locales, y avisa de si el contenido sale de
> la máquina.

Esta laguna existía en la v2 de la documentación. La cerramos aquí.

## Decisión

`archctl render` y los renderers invocados por las skills deben usar:

- **PlantUML** vía `plantuml.jar` local (descargado en
  `~/.local/share/archctl/tools/` por el instalador).
- **Structurizr** vía `structurizr-cli` pinneado localmente o el viewer
  `structurizr/lite` en podman sobre el puerto `127.0.0.1:18080`.
- **Kroki** interno si está desplegado localmente (podman sobre
  `127.0.0.1:18000`).

`plantuml.com` y `kroki.io` quedan **bloqueados por defecto**. Activar
un servicio público requiere:

- Flag explícito por run (`--allow-public-renderer`).
- Justificación obligatoria en un campo de metadatos que se registra en
  el `AnalysisRun`.
- Warning visible en consola antes del envío.

El bloqueo se aplica tanto al render directo desde `archctl` como al
render desde las skills (los wrappers `c4-from-graph`,
`use-cases-from-graph`, `class-view-from-graph`,
`sequence-from-scenario` y `diagram-review` deben verificar el flag).

## Consecuencias

- El código del repositorio y los nombres de sistemas nunca abandonan la
  máquina sin voluntad explícita del usuario.
- Un análisis puede compartirse sin filtraciones accidentales.
- Los renderers locales son requisito de instalación (ADR-005 + M11).
  Sin `plantuml.jar` o `structurizr-cli`, el render falla con un
  mensaje claro.
- `Mermaid` C4 sigue siendo no-canónico (su sintaxis es
  oficial-experimental, según la propia documentación de Mermaid).
- `draw.io` mantiene su rol de salida editable opcional.
