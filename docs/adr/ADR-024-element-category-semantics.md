# ADR-024 — Element.category Semantics: Diagram Family vs. Projection Kind

> **Ciclo:** `m26-c4-contract-integrity`
> **Estado:** Aceptado
> **Fecha:** 2026-08-05
> **Complementa:** [ADR-007](ADR-007-modelos-y-renderizadores-de-diagramas.md) + [ADR-013](ADR-013-viewer-ortogonal.md)

## Problema

El pipeline de export de C4 (`diagram export`) confunde `Element.category` con el nivel C4 (`container`, `component`, etc.). El código en `export.rs:51` usaba:

```rust
let category = view.kind.to_string(); // "container" ❌
```

Pero `c4_discover` escribe `category = 'c4'` (correcto según el metamodelo). La query resultante:

```cypher
WHERE e.category = 'container'  -- busca en el campo equivocado
```

Encuentra cero resultados porque los datos reales tienen `category = 'c4'`.

## Decisiones

### D1 — `Element.category` = familia de diagrama, nunca nivel C4

**Elección:** `Element.category` es la familia del diagrama (`c4`, `code`, `uml`), no el nivel C4. `Element.kind_id` es el identificador específico de la proyección (e.g., `mt.container`, `mt.component`).

**Implementación en c4_discover:**
```rust
// c4_discover.rs:231-233 ✓ correcto
e.kind_id = '{kind_id}',  // e.g., "mt.container"
e.category = 'c4',         // siempre "c4" para elementos C4
```

**Implementación en export:**
```rust
// export.rs:51-53 ✓ corregido
let category = view.kind.category();  // "c4" ✓
let kind = view.kind.to_string();    // "container" para filtrar kind_id
```

**Rationale:** Esta separación permite que múltiples familias de diagramas coexistan en el mismo grafo. Un elemento puede ser simultáneamente `category='c4', kind_id='mt.container'` y `category='code', kind_id='struct'`. La query de export filtra por ambos campos para evitar falsos positivos.

### D2 — Filtro de kind_id con `STARTS WITH`

**Elección:** El filtro de kind usa `STARTS WITH` en lugar de igualdad exacta.

```cypher
WHERE e.category = 'c4' AND e.kind_id STARTS WITH 'container'
```

**Rationale:** `c4_discover` escribe `kind_id = 'mt.container'` (formato MetaType), no `'container'`. Usar `STARTS WITH 'container'` matchea ambos formatos:
- `'container'` (datos legacy o future)
- `'mt.container'` (formato actual de c4_discover)

**Alternativa rechazada:** Cambio a igualdad exacta `'mt.container'` — requiere migración de datos y es más frágil ante cambios futuros de formato.

### D3 — `C4Kind::category()` como método

**Elección:** Se añade `C4Kind::category(&self) -> &'static str` que retorna `"c4"` para todas las variantes.

```rust
impl C4Kind {
    /// Category for graph queries: always "c4" for C4 diagrams.
    pub fn category(&self) -> &'static str {
        "c4"
    }
}
```

**Rationale:** Mantiene consistencia con `ViewKind::category()` en `project_selector.rs`. Si `C4Kind` se expande en el futuro, el método centraliza la semántica.

## Evidencia del codebase

| Archivo | evidencia |
|---------|-----------|
| `c4_discover.rs:233` | `e.category = 'c4'` — correcto |
| `export.rs:51` (antes) | `view.kind.to_string()` — incorrecto, usaba nivel C4 como category |
| `export.rs:51` (ahora) | `view.kind.category()` — correcto |
| `graph.rs:375` (test) | Crea elemento con `category = 'c4'` — correcto |
| `project_selector.rs:44-46` | `category()` para `ViewKind` → `"c4"` o `"uml"` — correcto |

## Compatibilidad hacia atrás

**Datos existentes con `category = 'c4'` son correctos** según este ADR.

**Datos existentes con `category` establecido a un nivel C4 (bug) deben treatarse como erróneos y corregirse re-ejecutando el writer.**

La corrección es puramente aditiva: la query ahora filtra por ambos `category` Y `kind_id`, así:
- Datos correctos (`category='c4'`): encontrados ✓
- Datos legacy con `category='container'` (bug): no encontrados (correcto, eran ruido)

## Métricas de calidad

| Métrica | Antes | Después |
|---------|-------|---------|
| C4 export query | `WHERE category='container'` → 0 resultados | `WHERE category='c4' AND kind_id STARTS WITH 'container'` → resultados correctos |
| Acoplamiento category-kind | tight (nivel C4 = category) | loose (familia ≠ nivel) |
| False positives en query | posibles (elementos con `category='code'` y `kind_id='container'`) | imposibles (filtro de familia + nivel) |

## Impacto en queries existentes

| Query | Antes | Después |
|-------|-------|---------|
| `query_elements(category, scope, None)` | filtra solo por category | filtra por category + kind (si se pasa kind) |
| `query_elements` en `diagram project` | recibía `category='c4'` via `ProjectSelector` | sin cambio — `ProjectSelector` no usa kind filter |
| `query_semantic_edges` | solo usa `category` | sin cambio |

## Decisiones abiertas

- [ ] ¿Normalizar `kind_id` de `'mt.container'` a `'container'` en futura migración?
  Esto permitiría usar igualdad exacta en lugar de `STARTS WITH`.
- [ ] Los niveles C4 (context, container, component, dynamic, deployment) no comparten prefijos.
  Confirmado: todos son lowercase alphanumérico sin prefijos compartidos. `STARTS WITH` es seguro.

## Commits relacionados

- `fix/m26-c4-contract-integrity` — implementación del fix
- `feat/m8-c4-boundary-inference` — donde se introdujo `c4_discover` con `category='c4'`
