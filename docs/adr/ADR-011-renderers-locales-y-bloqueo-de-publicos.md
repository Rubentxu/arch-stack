# ADR-011 — Renderers locales y bloqueo de servicios públicos

**Estado:** Aceptado (alcance reducido a `archctl` tras ADR-013)
**Fecha:** 29 de julio de 2026
**Relacionado:** [ADR-013](ADR-013-viewer-ortogonal.md) (viewer ortogonal), [ADR-007](ADR-007-modelos-y-renderizadores-de-diagramas.md) (modos de render)

## Contexto

Ni ADR-007 (diagramas como proyecciones) ni ADR-005 (LadybugDB) explicitan la política sobre renderers públicos. El documento inicial `Skills-para-agentes-IA.md` (§187-192) lo dice explícitamente:

> Para repositorios corporativos usaría siempre `PlantUML local` o `Kroki desplegado internamente`. No enviaría código, nombres de sistemas ni diagramas a un Kroki público. La propia skill distingue entre el backend público y las alternativas locales, y avisa de si el contenido sale de la máquina.

Esta laguna existía en la v2 de la documentación. La cerramos aquí.

Tras ADR-013, el rendering se divide en dos proyectos con políticas separadas pero coherentes. Este ADR cubre **solo** el lado de `archctl`. El viewer `archview` hereda el mismo principio pero lo aplica sobre su propio bundle; ver ADR-013 § Seguridad local.

## Decisión

### `archctl` (este ADR)

`archctl render` y los renderers invocados por las skills deben usar:

- **PlantUML** vía `plantuml-little 1.2026.2-4` (librería Rust pure-Rust, multi-licensada MIT-compatible; sustituye al `plantuml.jar` local que era la opción previa). Adoptado en M9.
- **Structurizr DSL** renderizado vía `petgraph` + `dagre-rs` + `svg` crate (renderer propio, pure-Rust, alcance C4 Context/Container/Component). POC validado en `/tmp/structurizr_poc`. Adoptado en M9.
- **Mermaid** vía `merman 0.8.0-alpha.3` (librería Rust parity-focused headless, MIT/Apache-2.0). Adoptado en M9.
- **Kroki interno** solo si está desplegado localmente (loopback `127.0.0.1:18000`), como fallback opcional para tipos no soportados por las librerías pure-Rust.

`plantuml.com` y `kroki.io` quedan **bloqueados por defecto**. Activar un servicio público requiere:

- Flag explícito por run (`--allow-public-renderer`).
- Justificación obligatoria en un campo de metadatos que se registra en el `AnalysisRun`.
- Warning visible en consola antes del envío.

El bloqueo se aplica tanto al render directo desde `archctl` como al render desde las skills (los wrappers `c4-from-graph`, `use-cases-from-graph`, `class-view-from-graph`, `sequence-from-scenario` y `diagram-review` deben verificar el flag).

### `archview` (proyecto separado)

El viewer ortogonal, definido en [ADR-013](ADR-013-viewer-ortogonal.md), consume bundles locales generados por `archctl`. El bundle es la **única** fuente de datos del viewer. Por construcción:

- `archview` no abre conexiones de red salientes.
- `archview` no carga scripts desde CDNs (CSP estricto).
- `archview` no envía telemetría.
- Los assets (iconos, temas) se incluyen localmente en el bundle.

El viewer cumple el principio de este ADR **por construcción**, no necesita verificaciones adicionales.

### Política compartida

| Aspecto | `archctl` | `archview` |
|---|---|---|
| Renderers locales | obligatorio (este ADR) | obligatorio (ADR-013) |
| Bloqueo de servicios públicos | por flag explícito | por construcción (no puede acceder a red) |
| Telemetría saliente | prohibida | prohibida |
| Recursos remotos (CDN, fuentes) | prohibidos | prohibidos (CSP) |
| Datos al disco local | sí (`--output`) | sí (file watcher del bundle) |

## Consecuencias

### Positivas

- El código del repositorio y los nombres de sistemas nunca abandonan la máquina sin voluntad explícita del usuario cuando se usa `archctl`.
- `archview`, al ser estrictamente local y offline, garantiza el mismo principio sin necesidad de flags.
- Un análisis puede compartirse sin filtraciones accidentales.
- `plantuml-little` + `merman` + el renderer Rust propio eliminan la dependencia de JRE/Node en `archctl` (M9).
- ADR-013 consolida el bundle contract: `archctl` escribe, `archview` lee. No hay fugas posibles por canal de transporte.

### Negativas y trade-offs

- M9 introduce un renderer Rust propio (Structurizr) que requiere 3-4 meses de trabajo para alcanzar paridad razonable con Structurizr-Lite.
- La paridad pixel-perfect con Structurizr-Lite no es objetivo (diferentes motores de layout).
- Si el usuario necesita paridad exacta, debe usar Structurizr-Lite directamente (fuera de archctl).
- ADR-013 exige disciplina: cualquier feature que intente "abrir un canal" entre `archctl` y `archview` debe rechazarse en revisión.

### Métricas de éxito

- `archctl doctor` reporta el conjunto de renderers integrado y verifica que las versiones esperadas están disponibles.
- `archctl render --output` en una red con acceso bloqueado a `plantuml.com` y `kroki.io` produce el mismo output que en una red abierta.
- `archview` abre un bundle sin hacer ninguna petición de red saliente (verificable con DevTools).

## Cómo revertir

| Decisión | Reversión |
|---|---|
| Renderer Rust propio para Structurizr | Volver a `vizoxide` (C lib linkeada estáticamente, sin fork+exec) o aceptar dependencia de Structurizr-Lite |
| `plantuml-little` / `merman` | Volver a `plantuml.jar` / `mmdc` (CLIs externos); reintroduce dependencia de JRE/Node |
| `archview` ortogonal | Fusionar con `archctl` (rompe ADR-001, ADR-010). Aceptable solo si la presión de integración supera la simplicidad sidecar |
| Bloqueo de servicios públicos | Cambiar el default a "permitir" — esto requeriría justificación ética documentada y revertir ADR-011 entero |
