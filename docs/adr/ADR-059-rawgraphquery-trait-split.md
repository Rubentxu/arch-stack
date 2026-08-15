# ADR-059 — RawGraphQuery trait split and SemanticEdgeRepository boundary

> **Cycle:** `p-38e02210a9f14317/p1-04-raw-graph-query-boundary`
> **Status:** Aceptado — 2026-08-15
> **Supersedes:** ADR-044 §Puertos (persistence ports section)
> **Aplica a:** `archctl/src/store.rs`, `archctl/src/cli.rs`, `archctl/src/diagram/queries.rs`, `archctl/src/code/*.rs`

## Contexto

ADR-044 introdujo el concepto de repositorios semánticos y la frontera de raw query. La auditoría de P1-04 descubrió que `GraphStore::query` era usado por 22 sitios de aplicación (diagram/queries + code/*.rs), violando el principio "0 Cypher en aplicación" de ADR-044 §Puertos.

La situación antes de P1-04:

| Problema | Impacto |
|---|---|
| `GraphStore::query` accesible desde cualquier caller | Cualquier código de aplicación podía escribir Cypher raw |
| No había separación entre query admin y query de aplicación | El gate "Cypher es admin-only" no era forzable estáticamente |
| `is_read_only_query` vivía en `diagram/queries.rs` (的错误位置) | Era un helper de aplicación, no del puerto |
| `SemanticEdgeRepository` no existía | Los writes de SEMANTIC_EDGE iban por `GraphStore::query` con MERGE inline |

## Decisión

### 1. Nuevo `pub trait RawGraphQuery` en `store.rs`

```rust
pub trait RawGraphQuery: Send + Sync {
    fn query(&self, cypher: &str) -> Result<Vec<Row>>;
    fn prepare(&mut self, _: &str) -> Result<PreparedStatementHandle, StoreError> {
        Err(StoreError::Prepare("...".into()))
    }
    fn execute(&mut self, _: &mut PreparedStatementHandle, _: Params) -> Result<Vec<Row>, StoreError> {
        Err(StoreError::Execute("...".into()))
    }
}
```

`LbugStore` implementa `RawGraphQuery` con keyword enforcement via `is_read_only_query` absorbed dentro del `impl`.

### 2. `GraphStore` pierde `query`/`prepare`/`execute`

Los tres métodos se eliminan del trait `GraphStore`. El único camino Cypher raw es `RawGraphQuery`.

### 3. Nuevo `pub trait SemanticEdgeRepository` en `store.rs`

```rust
pub trait SemanticEdgeRepository: Send + Sync {
    fn link_semantic_edge(
        &mut self,
        src_id: &str,
        tgt_id: &str,
        relation_id: &str,
        predicate_id: &str,
        props: &serde_json::Map<String, serde_json::Value>,
        active: bool,
    ) -> Result<()>;
    fn link_call_edge_with_resolution(
        &mut self,
        src_id: &str,
        callee_name: &str,
        relation_id: &str,
        props: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<()>;
}
```

### 4. `CliContext` gana `pub raw_query: Arc<dyn RawGraphQuery>`

Los handlers admin (`graph_query_cmd`, `graph_neighbours_cmd`, MCP `handle_graph_query`) consumen `ctx.raw_query.query(...)`.

### 5. API pública `query_elements`/etc. → `#[deprecated]` re-exports

Por 1 minor (`since = "1.43"`), las funciones públicas delegan a `DiagramRepository::*` vía helper interno.

## Implementación

Ver `archctl/src/store.rs` para el código. Las constantes clave:

- `RawGraphQuery` se re-exporta en `lib.rs`
- `SemanticEdgeRepository` se re-exporta en `lib.rs`
- `is_read_only_query` se mueve a `store.rs` como helper interno del `impl RawGraphQuery for LbugStore`

## Alternativas evaluadas

| Alternativa | Razón de rechazo |
|---|---|
| `GraphStore::query` con flag `is_admin()` | Acopla el puerto de persistencia a RBAC |
| `prepare`/`execute` como métodos de `RawGraphQuery` con default error | La decisión sobre eliminarlos o moverlos se aplaza a P1-05 (UnitOfWork) |
| `link_call_edge_with_resolution` en `ElementRepository` | Producir una relación no es una operación de elemento |

## Dependencias

- P1-01: `CliContext` composition root
- P1-03: `ElementRepository`, `DiagramRepository` traits ya existentes
- M32 BREAK-1: schema migrations para MetaType/Predicate ya activos

## Estado

- 2026-08-15 | Aceptado | P1-04 apply — trait split + SemanticEdgeRepository

## Véase también

- ADR-044 §Puertos (original, se mantiene como referencia pero el texto de "0 Cypher" se refuerza aquí)
- ADR-005 (LadybugDB como grafo canónico)
- ADR-017 (schema migration runner)
