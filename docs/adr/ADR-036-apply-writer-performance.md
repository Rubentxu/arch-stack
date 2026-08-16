# ADR-036 — Apply Writer Performance: Transaction + Bulk Import

> **Ciclo:** `m32-apply-writer-performance` (planificado — decisión registrada 2026-08-06)
> **Estado:** Aceptado
> **Fecha:** 2026-08-06
> **Decisor:** Ruben Dario (plan aprobado para ejecución)
> **Alternativas consideradas:** (a) documentar el límite y no tocar el writer; (b) solo transacción; (c) bypass de Kùzu con import directo

## Contexto

El writer `--apply` de call-graph tarda **~0.43s por elemento** en todos los
lenguajes (problema preexistente desde m11, expuesto por M30 al activar Go):

| Dataset | Lenguaje | Elementos | Tiempo `--apply` |
|---|---|---|---|
| pmndrs/zustand | typescript | 212 | 92s |
| labstack/echo | go | 1307 | 483s |

Causa raíz (verificada en lbug 0.18.3 source):
- **lbug 0.18.3 NO es SQLite — es Kùzu** (embedded graph DB, FFI C++, Arrow
  batches, threads internos — por eso `user 37min` vs `real 8min`).
- `apply()` (`call_graph.rs` L1298) abre el store UNA vez pero emite
  **~5-6 queries por nodo** (write_function_version, write_function_element,
  3× link_function_edges) + ~2 por edge → **~10.500 queries** para echo.
- Cada `GraphStore::query()` = ejecución completa vía FFI con su propio
  commit/checkpoint → el overhead por query domina.
- Error documental: `store.rs:420` dice "lbug 0.18.3 has no parameter
  binding" — **FALSO**: `Connection::prepare()` + `execute(&mut prepared,
  params)` existen (connection.rs L318-354). El comentario hay que corregirlo.

## Decisiones

### D1 — Transacción única (BEGIN/COMMIT) alrededor del apply

**Elección:** Envolver los loops de nodes + edges en UNA transacción
(`BEGIN TRANSACTION` … `COMMIT`), un solo checkpoint al final.

**Rationale:**
- Elimina el commit/checkpoint por query (hoy 1 por query ≈ 10.500 commits).
- Impacto esperado: **10-100x** (echo 483s → ~5-15s).
- Bajo riesgo: semántica MERGE idempotente y skip de existentes intactos.

**Trade-offs:**
- Un fallo a mitad aborta TODO el lote (hoy deja escritos parciales). Aceptado:
  el apply es atómico por diseño; verificar el manejo de error para reportar
  "0 written" en vez de estado corrupto.
- Kùzu: confirmar sintaxis `BEGIN TRANSACTION`/`COMMIT` en su Cypher en el
  ciclo de implementación.

### D2 — Bulk import con UNWIND (patrón nativo Kùzu)

**Elección:** Reemplazar las queries por elemento con `UNWIND $batch AS row
CREATE ...` con parámetros, lotes de ~500.

**Rationale:**
- Kùzu está optimizado para carga masiva; 10.500 queries → **~6 queries**.
- Combinado con D1: objetivo **echo < 3s**.
- Es el patrón documentado de Kùzu para import de grafos.

**Trade-offs:**
- Riesgo medio: hay que reescribir el writer a forma batch y mantener la
  semántica (dedup por canonical_key, version_id por contenido, edge matching
  por current_name).
- La dedup de existentes (existing_keys) se mantiene en memoria, no en SQL.

### D3 — Prepared statements + parameter binding

**Elección:** `prepare()` una vez por forma de query, `execute(&mut prepared,
params)` con parámetros; extender `GraphStore` con API transaccional y
prepared.

**Rationale:**
- Elimina recompilar el Cypher por elemento (parse + plan por query vía FFI).
- lbug 0.18.3 YA lo soporta (connection.rs L318-354) — solo falta exponerlo.
- Impacto esperado: 2-5x adicional.

**Trade-offs:**
- Cambio de trait público (`GraphStore`): los writers hermanos y tests
  dependen de `query(&str)` — mantener `query` como wrapper y añadir la API
  nueva (no breaking).
- Interpolación actual con escape de strings (ADR-009 legacy) se mantiene
  para compat, pero el path nuevo usa parámetros.

### D4 — Gate de regresión (bench) + corrección documental

**Elección:** Añadir bench criterion de call-graph apply al bench existente;
corregir los comentarios erróneos.

**Rationale:**
- Hoy NO hay ningún benchmark del writer — por eso el problema era invisible.
- Criterios: echo < 10s, zustand < 5s, smoke fixture Go < 5s.
- Corregir `store.rs:420` ("no parameter binding" → Kùzu sí lo tiene) y
  ROADMAP M32 (decía "SQLite-backed" → Kùzu).

**Trade-offs:**
- Bench con datos reales requiere dataset cacheado (ya existe
  `~/.cache/archctl-smoke/`); marcado `--ignored`/manual como el resto.

### D5 — Alcance a writers hermanos

**Elección:** Aplicar el mismo patrón (transacción + batch) a
class-diagram, state-machine y sequence si comparten la estructura del
writer.

**Rationale:**
- Mismo defecto de fondo (queries por elemento con commit individual).
- Un solo patrón para todo el code-gen.

**Trade-offs:**
- Aumenta el alcance del ciclo M32; verificar primero si comparten el
  patrón antes de incluirlos (si no, quedan como follow-up).

## Resultado esperado

| Escenario | Hoy | Objetivo |
|---|---|---|
| echo 1307 elementos | 483s | **< 10s** (D1) / **< 3s** (D1+D2) |
| zustand 212 elementos | 92s | **< 5s** |
| Proyección 10k elementos | ~70min | ~10-40s |

## Amendment — M32 D2 Re-ship + D5 Extension (2026-08-16)

**Reason:** D2 of this ADR was **regressed** by [P1-04 T3 commit `599c863`](https://github.com/Rubentxu/arch-stack/commit/599c863) which accidentally removed the UNWIND bulk import from `call_graph.rs` during a refactor. The regression went undetected until the M32 cycle re-audited writer performance post-P1-05 UnitOfWork merge.

### D2 — UNWIND Bulk Import: Re-shipped

**What changed:** D2 was re-implemented across 4 writers on 2026-08-16:

| Writer | Commit | Notes |
|--------|--------|-------|
| `call_graph` | `a92ee0e` | UNWIND restore via `apply_common::batch_upsert_element` |
| `class_diagram` | `3ab707c` | UNWIND restore + N+1 hoist (prerequisite) |
| `state_machine` | `df174bb` | UNWIND on 3 nested loops (machines, states, transitions) |
| `c4_discover` | `9e05b81` | UNWIND for container nodes + versions |

**BATCH_SIZE:** `500` (echo 1307 / 500 ≈ 3 batches, rationale documented in `apply_common.rs`).

**Trade-offs:** Same as original D2. Per-edge `write_call_edge` / `link_semantic_edge` / `link_transition_*` kept as per-row `OPTIONAL MATCH` — D2 only covers Element + ElementVersion bulk import.

### D3 — Prepared Statements + Parameter Binding: STAYs DEFERRED

**Status:** No change. D3 remains deferred from v1.21.0 (M51 cycle).

**Re-open trigger:** lbug ships typed bindings (`Value::String` direct, not `Value::Json`) OR Kùzu documents `CAST($p AS STRING)` semantics that match typed String columns. Without one of these, the JSON-wrapper binding fails `WHERE canonical_key = $p` on String-typed columns.

**Path forward:** New `BatchedWriter` port (out of scope for M32; would supersede the inline `UNWIND` helpers).

### Bonus Fix — class_diagram N+1 on `existing_canonical_keys`

**Problem:** `class_diagram::apply` called `ElementRepository::existing_canonical_keys(s)` **inside** the per-node loop at L1394-1395, causing 1 query per node (the N+1 bug that made D2 batching impossible).

**Fix:** Hoisted `existing_canonical_keys` out of the loop to a single pre-pass. Verified by `class_diagram_existing_keys_not_n_plus_one` test (grep assertion + instrumentation).

## Referencias

- `archctl/src/code/call_graph.rs` (apply L1298, writer helpers L1050-1290)
- `archctl/src/store.rs` (L384-391 query; L420-421 comentario erróneo)
- lbug 0.18.3 source: `src/connection.rs` (prepare L318, execute L332)
- ROADMAP M32 (este plan), debt-report M30 (W: apply perf)
