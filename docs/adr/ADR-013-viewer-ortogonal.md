# ADR-013 — Viewer ortogonal basado en DiagramProjection

**Estado:** Aceptado
**Fecha:** 29 de julio de 2026
**Proyecto:** `archview` (repositorio separado, NO parte de `archctl`)
**Refuerza:** ADR-001 (archctl como sidecar), ADR-010 (sin daemon), ADR-011 (renderers locales)
**Relacionado:** ADR-007 (diagramas como proyecciones), ADR-012 (política descart-CLIs), ADR-005 (LadybugDB)

## Contexto

`archctl` emite artefactos estáticos (`.svg`, `.dsl`, `.puml`) que se pueden commitear, embeber en Markdown, o renderizar en CI. Esto cubre el caso del agente de OpenCode (genera un diagrama y devuelve el path) y el caso de revisión estática.

Sin embargo, ciertos workflows humanos se benefician de interactividad:

- Drill-down Context → Container → Component sin re-render.
- Pan/zoom sobre diagramas grandes.
- Hover sobre un nodo para ver sus evidencias.
- Comparación temporal entre dos snapshots.
- Edición visual de vistas (mover nodos, colapsar grupos) que vuelve como ChangeSet al grafo.

Construir esta interactividad dentro de `archctl` como servidor HTTP rompería ADR-001 (sidecar) y ADR-010 (sin daemon). Construirla como librería de rendering en Rust añadiría miles de LOC para reinventar lo que ya existe en el ecosistema JS.

## Decisión

El rendering interactivo vive en un **proyecto ortogonal** llamado `archview`, separado de `archctl`:

```text
archctl (sidecar CLI, Rust)
   │
   ├── consulta y actualiza LadybugDB
   ├── genera proyecciones (DiagramProjection JSON)
   └── exporta bundles autocontenidos
                │
                ▼
         Diagram Bundle (directorio o ZIP)
                │
                ▼
       archview (proyecto HTML/TypeScript)
                │
                ├── Sprotty (modelo → SVG interactivo)
                ├── ELK.js (layout en Web Worker)
                ├── Cytoscape.js (explorador libre del grafo)
                └── TypeScript (UI, paneles, navegación)
```

`archview` es **estrictamente ortogonal**:

- No accede a LadybugDB.
- No ejecuta Cypher.
- No interpreta el repositorio.
- No abre puertos.
- No mantiene conexiones de larga duración.
- Carga bundles desde el sistema de archivos (o ZIP).

`archctl` es **estrictamente sidecar**:

- No sirve HTML.
- No ejecuta Sprotty ni ELK.js.
- No abre puertos para el viewer.
- No mantiene LadybugDB abierta más allá de la duración de un comando.

## Contrato: DiagramProjection bundle

`archctl` exporta un bundle autocontenido:

```text
diagram-bundle/
├── manifest.json          # metadata del bundle (schema version, source, snapshot)
├── projection.json        # nodos + aristas + grupos (sin layout)
├── evidence.json          # evidencias vinculadas a nodos/aristas
├── styles.json            # tema + colores por tipo
└── assets/                # iconos C4 (PNG/SVG embebidos)
```

Cada comando que lo genera:

```bash
archctl diagram export \
  diagram:orders-container \
  --format viewer-bundle \
  --output ~/.local/share/archctl/exports/orders-container/
```

El bundle es la **única superficie de contrato** entre `archctl` y `archview`. El esquema JSON vive en `schemas/diagram-projection.schema.json` y se versiona independientemente de los binarios.

`archview` no debe:

- Asumir que el bundle fue generado por una versión concreta de `archctl`.
- Intentar leer LadybugDB directamente.
- Mantener estado sincronizado con `archctl` automáticamente.

## Cambios de vuelta: ChangeSet

Cuando el usuario edita visualmente en `archview` (mueve un nodo, colapsa un grupo, edita una etiqueta), `archview` exporta un ChangeSet JSON:

```json
{
  "schemaVersion": "1.0",
  "diagramId": "diagram:orders-container",
  "baseRevision": "revision:42",
  "commands": [
    { "type": "move-member", "memberId": "view-member:orders-api", "x": 240, "y": 160 },
    { "type": "collapse-group", "groupId": "view-group:orders-system" }
  ]
}
```

El usuario invoca:

```bash
archctl diagram apply --changes viewer-changes.json
```

`archctl` valida:

- `baseRevision` contra la revisión actual en LadybugDB (control de concurrencia).
- Tipos de comandos permitidos (mover/colapsar/etiquetar son OK; crear/eliminar elementos canónicos NO — eso pasa por el grafo, no por el viewer).
- Adquiere el lock por proyecto (ADR-010).
- Aplica las mutaciones como una nueva revisión de la vista.
- Libera el lock.

El ciclo de actualización es **explícito**, no automático:

```text
OpenCode modifica el grafo
   ↓
archctl diagram export
   ↓
archview detecta cambio (file watcher) y recarga
   ↓
usuario edita visualmente
   ↓
archctl diagram apply
   ↓
archview recarga para ver el resultado
```

No hay WebSocket. No hay servidor. No hay conexión persistente.

## Stack de `archview`

| Pieza | Crate / librería | Razón |
|---|---|---|
| Framework de diagramación | Sprotty | Modelo JSON → SVG interactivo, separan modelo/vista/comando |
| Layout | ELK.js (en Web Worker) | Layout jerárquico de Eclipse, soporta ports y jerarquía C4 |
| Lenguaje | TypeScript | Tipado estricto para el contrato `DiagramProjection` |
| Build | Vite | Build rápido, ESM nativo, output estático |
| UI shell | Svelte o Lit | Sin framework pesado; el canvas es Sprotty, los paneles son HTML directo |
| Explorador libre | Cytoscape.js (opcional) | Para navegar el grafo completo sin restricción de vista |
| Secuencias | Layout propio en TS | `SequenceLayout` determinista (no Sugiyama); animación por paso |

## Stack de `archctl` (cambios mínimos)

Para soportar el bundle, `archctl` añade:

- `archctl diagram export <id> --format viewer-bundle --output <dir>`.
- `archctl diagram apply --changes <file>` con validación de `baseRevision`.
- `archctl diagram validate <bundle-dir>` que verifica el bundle contra el schema.

Estos comandos siguen siendo one-shot. No añaden estado persistente en `archctl`.

## Decisiones explícitas que este ADR cierra

| Pregunta | Respuesta |
|---|---|
| ¿`archctl` se convierte en servidor? | **No.** ADR-001 y ADR-010 quedan intactos. |
| ¿`archview` accede a LadybugDB? | **No.** Solo lee bundles del sistema de archivos. |
| ¿`archview` se distribuye con `archctl`? | **No.** Repositorio separado, build pipeline separado. |
| ¿Hay WebSocket entre ambos? | **No.** File watcher + recarga explícita. |
| ¿`archview` reemplaza el renderer Rust de M9? | **No.** M9 (SVG estático) sigue activo. `archview` es complemento, no sustituto. |
| ¿`archctl render` se mantiene? | **Sí.** Para CI, agentes, embedding en Markdown. El renderer Rust produce SVG. |

## Consecuencias

### Positivas

- `archctl` mantiene su rol sidecar; los agentes de OpenCode no cambian.
- `archview` se puede desarrollar en paralelo sin tocar el binario de `archctl`.
- El bundle es un artefacto testeable, inspeccionable, committable.
- Si `archview` fracasa, `archctl` sigue produciendo SVG estático funcional.
- La separación permite terceras herramientas (scripts de CI, otros viewers) que consuman el mismo bundle.

### Negativas

- Dos proyectos a mantener con pipelines separados.
- Drift entre versiones de `archctl` y `archview` puede causar incompatibilidades de bundle (mitigado por versionado de schema).
- Workflow humano requiere alternar entre `archctl diagram export` y `archview open`. No es transparente.
- Sin actualizaciones en tiempo real; el watcher refresca con latencia del filesystem.

### Métricas de éxito

- `archview` abre un bundle exportado por `archctl <v1.0>` y renderiza correctamente sin importar la versión del binario que lo generó.
- El ChangeSet se aplica en menos de 1 segundo sobre un bundle de 100 nodos.
- El bundle de un diagrama C4 Container completo (10 nodos, 15 aristas) cabe en <50 KB.

## Cómo revertir

Si el viewer ortogonal resulta inadecuado:

- Eliminar `archview` (proyecto independiente).
- Mantener `archctl render` (SVG estático) como única salida de diagramas.
- Marcar este ADR como `Rechazado` y mover `DiagramProjection` a "formato interno, no expuesto".

`archctl` no necesita cambios para revertir este ADR.
