# ADR-012 — Política "descartar CLIs" + ciclo M5–M8 de adopción incremental + renderers como librerías

**Estado:** Aceptado
**Fecha:** 29 de julio de 2026
**Sustituye:** ADR-006 (deprecado íntegramente — ver ADR-006 § Por qué se deprecó)
**Complementado por:** [ADR-013](ADR-013-viewer-ortogonal.md) (separación del viewer interactivo)
**Relacionado:** ADR-005 (LadybugDB), ADR-011 (renderers locales), ADR-007 (modos de render)

## Contexto

Tras cerrar M4 (`ast-grep-core` + grafo de evidencias + persistencia
MERGE), se propuso una adopción más profunda del ecosistema de análisis
estático en Rust como librerías en lugar de CLIs envueltos. La
propuesta original listaba:

- `ast-grep-core` y `ast-grep-language` (ya integrado parcialmente).
- `tree-sitter-graph` para extracción declarativa de grafos.
- `tree-sitter-stack-graphs` para resolución de nombres.
- `scip` para consumir índices semánticos externos.
- `gix` para reemplazar el CLI de Git.
- `cargo_metadata` para reemplazar el parseo manual de `Cargo.toml`.
- `lsp-types` para clientes LSP.
- `ra_ap_*` (rust-analyzer snapshots) para análisis profundo de Rust.
- `oxc_parser` + `oxc_semantic` para análisis de JS/TS.
- `swc_ecma_parser` como alternativa a Oxc.
- Workspace multi-crate de 18 paquetes con `archctl-analysis-*` y
  `archctl-language-*` separados por concern.

Además, una segunda revisión endureció la postura:
"descartamos todo los cli" — archctl debe evitar invocar CLIs
externos siempre que exista una librería Rust mantenida que cumpla
la misma función. Y una tercera iteración extendió la misma lógica a
los **renderers**, motivando la separación entre `archctl` (sidecar
CLI con rendering estático) y `archview` (proyecto separado para
rendering interactivo, ver [ADR-013](ADR-013-viewer-ortogonal.md)).

## Decisión

Adoptamos **dos cosas** en este ADR:

1. **Una política**: archctl descarta CLIs externos cuando existe una
   librería Rust mantenida activamente. Los CLIs se invocan solo
   cuando no hay alternativa razonable en Rust.
2. **Un ciclo concreto de adopción**: M5–M8 introduce 4 crates de
   análisis ahora, y M9 introduce los renderers como librerías.

### Crates adoptados en este ciclo (M5–M8)

| Crate | Versión | Función | Sustituye |
|---|---|---|---|
| `gix` | 0.86 | API Git in-process | `Command::new("git")` en `identity.rs` |
| `cargo_metadata` | 0.23 | JSON estable de `cargo metadata` | Parseo manual de `Cargo.toml` |
| `ast-grep-language` | 0.45 | Lenguajes pre-cableados (con `builtin-parser`) | Boilerplate `impl Language` por gramática |
| `tree-sitter-graph` | 0.12 | DSL declarativo CST → grafo | Extracción ad-hoc en `evidence.rs` |

### Renderers adoptados en M9 (renderers como librerías, dentro de `archctl`)

| Renderer | Crate sustituto | Estado |
|---|---|---|
| PlantUML (`plantuml.jar`) | `plantuml-little 1.2026.2-4` — "byte-exact SVG parity with Java PlantUML", multi-licensado MIT-compatible | **M9, win claro** |
| Mermaid (`mmdc`) | `merman 0.8.0-alpha.3` (parity-focused) o `mermaid-render 0.10.0` | **M9, win claro** |
| Structurizr (CLI / Lite) | Renderer propio Rust con `petgraph` + `dagre-rs` + `svg` crate, alcance C4 Context/Container/Component, sin icons, sin paridad pixel-perfect con Lite | **M9, POC validado en `/tmp/structurizr_poc`** |

### Rendering interactivo (NO en `archctl`, proyecto separado `archview`)

El viewer interactivo (drill-down, pan/zoom, hover, comparación temporal, edición visual) **no es responsabilidad de `archctl`**. Es el proyecto separado `archview`, definido íntegramente en [ADR-013](ADR-013-viewer-ortogonal.md).

`archview` consume bundles `DiagramProjection` JSON que `archctl` exporta vía `archctl diagram export`. La comunicación es por sistema de archivos. No hay servidor, no hay WebSocket, no hay daemon. ADR-001 y ADR-010 quedan intactos.

### Crates evaluados y diferidos (Fase 2 — M14 o posterior)

| Crate | Motivo del diferimiento |
|---|---|
| `oxc_parser`, `oxc_semantic` | Valor real pero implementación como segundo parser (paralelo a tree-sitter) requiere estabilizar contrato. Reservado para M14. |
| `scip` | Requiere indexador externo produciendo `index.scip`. Útil cuando tengamos pipelines que lo generen. |
| `lsp-types`, cliente LSP | Requiere servidor corriendo por sesión; incompatible con el modelo one-shot del MVP (ADR-010). Reservado para fase daemon. |
| `ra_ap_syntax`, `ra_ap_hir`, `ra_ap_load-cargo` | Snapshots frecuentes; tree-sitter-rust + ast-grep cubren el 90% actual. Esperar demanda real. |
| `tree-sitter-stack-graphs` | Requiere reglas por lenguaje y framework; sin ellas no aporta. |
| `swc_ecma_parser` | Oxc es preferible; swc solo se justifica si Oxc no cumple. |
| `ctrs` (Universal Ctags port) | Sin demanda concreta todavía. |
| `tree-sitter` gramática Kotlin | Diferido hasta que el ecosistema tree-sitter-kotlin actualice a binding ≥ 0.23. |
| Workspace multi-crate | Sin presión de boundaries (proyectos `archctl` y `archview` son los dos repositorios reales). |

### Política operativa

1. **Un crate por commit.** Cada crate entra con su propio commit
   aislado para mantener la trazabilidad y revertir ante problemas.
2. **No breaking changes a la API pública de `archctl`.** Si un crate
   nuevo mejora la API interna, se actualizan los call-sites pero
   `lib.rs` no rompe el contrato externo.
3. **Default-features desactivado cuando sea posible.** Solo se
   activan las features mínimas necesarias.
4. **Cada crate se mide contra el sustituto que reemplaza.** Antes de
   añadir un crate se demuestra que elimina código o añade capacidad
   concreta.

### Política "descartar CLIs" (criterio operativo)

Para cada herramienta del Núcleo o Opcionales:

1. ¿Existe una librería Rust mantenida que cumple la misma función?
2. Si sí → adoptar la librería, documentar sustitución en ADR.
3. Si no → adaptar el CLI con envoltorio mínimo + salida normalizada.
4. Si en el futuro aparece la librería → ADR de sustitución.

**Excepciones explícitas donde el CLI se mantiene** (ver tabla completa
en ADR-006, ahora deprecado pero preservado históricamente):

- **Build metadata no-Cargo**: cada herramienta tiene su propio protocolo.
- **Infraestructura como código** (Terraform, Helm, kubectl, Syft):
  sin parser Rust mantenido equivalente a la herramienta oficial.
- **Análisis profundo opcional** (Semgrep, Joern) si se introducen en M14: mantener CLI hasta que aparezca port maduro.

**Renderers**: la decisión de split entre `archctl` (estático, pure-Rust) y `archview` (interactivo, Sprotty+ELK.js) está consolidada en [ADR-013](ADR-013-viewer-ortogonal.md).

## Consecuencias

### Positivas

- `gix` elimina fork+exec del path crítico de `archctl project resolve`.
- `cargo_metadata` permite resucitar `archctl inventory depends` con un
  JSON estable producido por el propio Cargo.
- `ast-grep-language` añade **kotlin** sin coste adicional de
  integración (su `builtin-parser` ya lo trae).
- `tree-sitter-graph` convierte las reglas de extracción en artefactos
  versionados independientes del binario.
- Política explícita "no CLIs" cierra la puerta a futuras
  justificaciones de fork+exec para análisis.
- M9 cierra el último hueco de rendering en `archctl`:
  `plantuml-little` + `merman` + el renderer Structurizr propio eliminan
  también los renderers como CLI. ADR-011 (renderers locales) se
  cumple sin requerir Java/Node.
- ADR-013 consolida la separación viewer/sidecar sin romper ADR-001 ni
  ADR-010.

### Negativas y riesgos

- Mayor superficie de dependencias y de versiones a trackear.
- `gix 0.86` introduce cambios de API no triviales.
- `tree-sitter-graph` requiere escribir el DSL por framework; sin
  rule packs mínimos por lenguaje no aporta valor.
- `ast-grep-language` con `builtin-parser` arrastra ~25 gramáticas;
  tamaño del binario crece. Si molesta, desactivar `default-features`.
- `merman 0.8` es alpha; `mermaid-render 0.10` es más estable pero
  menos completo. Adoptamos `merman` por parity-focused.
- `plantuml-little` es versión `1.2026.2-4` (year.month.rev) siguiendo
  la cadencia de PlantUML upstream; aceptar pin a esa versión o
  seguir upstream.
- El renderer Structurizr propio (M9) requiere 3-4 meses de trabajo
  dedicado; alcance recortado a C4 Context/Container/Component, sin
  icons, sin paridad pixel-perfect con Lite.

### Métrica de éxito

Tras M9:

- `archctl doctor` reporta el conjunto de crates integrado.
- Ningún crate introducido está documentado como "planeado"; o se usa
  o se quita.
- Binario ≤ 80 MB (sin `vizoxide`) o ≤ 90 MB (con `vizoxide`).
- 0 invocaciones fork+exec en el path de operaciones declaradas en
  Núcleo (`archctl project resolve`, `archctl evidence extract`,
  `archctl inventory`, `archctl render`).
- `archview` (proyecto separado) abre un bundle generado por
  `archctl diagram export` sin errores.

## Cómo revertir

Cualquier crate de este subset puede eliminarse sin tocar los demás:

- `gix`: revertir a `Command::new("git")`; `identity.rs` es el único
  consumidor.
- `cargo_metadata`: revertir a parseo manual de `Cargo.toml`.
- `ast-grep-language`: revertir al `impl Language for Lang` por
  gramática que ya tenemos en `astgrep.rs`.
- `tree-sitter-graph`: revertir a `evidence.rs` ad-hoc; el módulo se
  puede aislar en su propio archivo para borrarlo limpiamente.
- `plantuml-little`: revertir a `plantuml.jar` (reintroduce dependencia
  de JRE).
- `merman`: revertir a `mmdc` (reintroduce dependencia de Node).
- Renderer Structurizr propio: reemplazar por `vizoxide` (C lib linkeada
  estáticamente, sin fork+exec) o aceptar dependencia de
  Structurizr-Lite fuera de archctl.

La política "descartar CLIs" misma se revierte cambiando ADR-012 de
vuelta a "adaptadores CLI", pero no debería hacerse salvo que
aparezca un caso donde una librería sustituta tenga bugs graves y el
CLI sea claramente superior.

## Notas operativas

- `tree-sitter-graph 0.12` requiere el binario de Python del paquete
  `tree-sitter` para generar parsers; en runtime no es necesario.
- `ast-grep-language 0.45` con `default-features` trae 25+ gramáticas.
  Si el bloat es inaceptable, custom-features: seleccionar
  explícitamente solo las nuestras.
- `gix 0.86` requiere Rust ≥ 1.85 (ya cumplido, 1.96 disponible).
- `cargo_metadata 0.23` requiere Rust ≥ 1.86 (ya cumplido).
- `archview` es **proyecto separado** (ver ADR-013). No añadir nada
  relacionado con viewer al crate de `archctl`.
