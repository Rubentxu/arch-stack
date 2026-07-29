# ADR-012 — Adopción incremental de crates de análisis como librerías

**Estado:** Aceptado
**Fecha:** 29 de julio de 2026
**Relacionado:** ADR-006 (adaptadores CLI), ADR-005 (LadybugDB)

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

La propuesta tiene dirección correcta pero sobra en alcance para el
estado actual del proyecto.

## Decisión

Adoptamos un **subset incremental**, manteniendo `archctl` como crate
único y posponiendo el split en workspace hasta que aparezca presión
concreta (p. ej. un `archctl-server` para cliente LSP o un daemon
estable).

### Crates adoptados en este ciclo

| Crate | Versión | Función | Sustituye |
|---|---|---|---|
| `gix` | 0.86 | API Git in-process | `Command::new("git")` en `identity.rs` |
| `cargo_metadata` | 0.23 | JSON estable de `cargo metadata` | Parseo manual de `Cargo.toml` |
| `ast-grep-language` | 0.45 | Lenguajes pre-cableados (con `builtin-parser`) | Boilerplate `impl Language` por gramática |
| `tree-sitter-graph` | 0.12 | DSL declarativo CST → grafo | Extracción ad-hoc en `evidence.rs` |

### Crates diferidos

| Crate | Motivo del diferimiento |
|---|---|
| `oxc_parser`, `oxc_semantic` | Valor real pero implementación como segundo parser (paralelo a tree-sitter) requiere estabilizar contrato. Reservado para M9 o posterior. |
| `scip` | Requiere indexador externo produciendo `index.scip`. Útil cuando tengamos pipelines que lo generen; hoy ninguno. |
| `lsp-types`, cliente LSP | Requiere servidor corriendo por sesión; incompatible con el modelo one-shot del MVP (ADR-010). Reservado para fase daemon. |
| `ra_ap_syntax`, `ra_ap_hir`, `ra_ap_load-cargo` | Snapshots frecuentes, encapsularlos en `archctl-analysis-rust` es razonable pero tree-sitter-rust + ast-grep cubren el 90% actual. Diferido hasta que un caso real lo demande. |
| `tree-sitter-stack-graphs` | Requiere reglas por lenguaje y framework; sin ellas no aporta. Esperar demanda concreta. |
| `swc_ecma_parser` | Oxc es preferible; swc solo se justifica si Oxc no cumple. |
| Workspace multi-crate | Sin presión de boundaries. |

### Política de adopción

1. **Un crate por commit.** Cada crate entra con su propio commit
   aislado para mantener la trazabilidad y revertir ante problemas.
2. **No breaking changes a la API pública de `archctl`.** Si un crate
   nuevo mejora la API interna, se actualizan los call-sites pero
   `lib.rs` no rompe el contrato externo.
3. **Default-features desactivado cuando sea posible.** Solo se
   activan las features mínimas necesarias; por ejemplo `gix` con
   `max-performance-safe` pero sin `blocking-io` mientras no se use.
4. **Cada crate se mide contra el sustituto que reemplaza.** Antes de
   añadir `tree-sitter-graph` se demuestra que reduce LOC en
   `evidence.rs`; antes de añadir `cargo_metadata` se demuestra que
   elimina parseo manual de TOML.

## Consecuencias

### Positivas

- `gix` elimina fork+exec del path crítico de `archctl project resolve`
  (operación más frecuente: cada llamada de un agente).
- `cargo_metadata` permite resucitar `archctl inventory depends` con un
  JSON estable producido por el propio Cargo, en lugar de parsear
  TOML a mano.
- `ast-grep-language` añade **kotlin** sin coste adicional de
  integración (su `builtin-parser` ya lo trae), satisfaciendo un
  hueco conocido del conjunto de lenguajes.
- `tree-sitter-graph` convierte las reglas de extracción en artefactos
  versionados independientes del binario, abrindo la puerta a rule
  packs distribuibles.
- 4 commits pequeños, revisables y revertibles.

### Negativas y riesgos

- Mayor superficie de dependencias y de versiones a trackear.
- `gix` 0.86 introduce cambios de API no triviales (muchos feature
  flags, API `Repository::discover` en `unsafe`-marked modules);
  encapsular siempre detrás de `identity.rs`.
- `tree-sitter-graph` requiere escribir el DSL por framework (no hay
  todavía reglas para Rust/Spring/Express/etc.). El MVP necesita al
  menos un rule pack mínimo por lenguaje para demostrar valor.
- `ast-grep-language` con `builtin-parser` arrastra ~20 gramáticas no
  usadas; el tamaño del binario crece. Si molesta, desactivar
  `default-features` y seleccionar solo las nuestras.
- M9–M12 (los originales use cases / secuencias / clases / vistas)
  se desplazan una posición; el "primer MVP útil" pasa de M0–M6 a
  M0–M4 + M5–M8.

### Métrica de éxito

Tras M8 (cierre del subset):

- `archctl doctor` reporta el conjunto de crates integrado.
- Cada crate tiene un test de smoke que falla si el crate se degrada.
- Ningún crate introducido está documentado como "planeado"; o se usa
  o se quita.
- El binario no supera los 80 MB (límite operativo para
  distribución).

## Cómo revertir

Cualquier crate de este subset puede eliminarse sin tocar los demás:

- `gix`: revertir a `Command::new("git")`; `identity.rs` es el único
  consumidor.
- `cargo_metadata`: revertir a parseo manual de `Cargo.toml` (el
  eliminado en M3).
- `ast-grep-language`: revertir al `impl Language for Lang` por
  gramática que ya tenemos en `astgrep.rs`.
- `tree-sitter-graph`: revertir a `evidence.rs` ad-hoc; el módulo se
  puede aislar en su propio archivo para borrarlo limpiamente.

## Notas operativas

- `tree-sitter-graph 0.12` requiere el binario de Python del paquete
  `tree-sitter` para generar parsers; en runtime no es necesario.
- `ast-grep-language 0.45` con `default-features` trae 25+ gramáticas.
  Si el bloat es inaceptable, custom-features: seleccionar
  explícitamente solo las nuestras.
- `gix 0.86` requiere Rust ≥ 1.85 (ya cumplido, 1.96 disponible).
- `cargo_metadata 0.23` requiere Rust ≥ 1.86 (ya cumplido).

