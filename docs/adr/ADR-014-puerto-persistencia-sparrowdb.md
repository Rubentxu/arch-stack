# ADR-014 — Puerto de persistencia hexagonal + SparrowDB como adapter alternativo

**Estado:** Aceptado (Ola 1)
**Fecha:** 30 de julio de 2026
**Sustituye:** (parcialmente) ninguno — añade un boundary que antes no existía.
**Refuerza:** ADR-005 (grafo canónico), ADR-010 (sin daemon).
**Relacionado:** ADR-006 (descart-CLIs, ya deprecado), ADR-012 (descart-CLIs, política operativa).

## Contexto

`archctl` hasta M4 ha estado atado a **LadybugDB** (`lbug = "0.18.3"`) en
dos sitios:

- `src/graph.rs` — `Database::new(...)`, `Connection::query(...)`,
  conversión de `lbug::Value` a `serde_json::Value`, y la lógica de
  schema bootstrap.
- `src/evidence.rs::put` — string-interpolation de Cypher
  identificador-por-identificador (porque `lbug 0.18.3` no soporta
  parámetros preparados).

Cada call site (`cli.rs::graph_init_cmd`, `graph_query_cmd`,
`graph_neighbours_cmd`, `evidence_list_cmd`, `evidence::put`) importa
`crate::graph::*` y depende transitivamente del shape de la API de
`lbug`. El adaptador es invisible y está enredado en todo el dominio.

### Por qué importa

1. **Migración de motor.** Cuando aparezca un motor mejor — o
   cuando `lbug` rompa compatibilidad en una release — el costo de
   cambio es proporcional al número de call sites. Hoy son ~6
   funciones con lógica Cypher embebida.

2. **Bindings multi-lenguaje.** El pivote que motivó este ADR es la
   posibilidad de usar **SparrowDB** (`sparrowdb = "0.1.16"` en
   crates.io) que ofrece Cypher embebido con bindings prometidos para
   Python, Node.js y Ruby. Si adoptamos SparrowDB, queremos que la
   herramienta de agentes escrita en Python pueda abrir la misma base
   que `archctl` escribió.

3. **Testabilidad.** Sin un port, los tests del dominio
   (`evidence::put_is_idempotent`, `evidence_list_cmd` indirecto) están
   acoplados a tener `lbug` funcionando en disco. Un port permite
   fixtures en memoria para tests delgados.

### Alternativas consideradas

- **(A) Status quo.** Mantener todo atado a `lbug`. Se descarta por
  los tres puntos de arriba.

- **(B) Swap directo a SparrowDB.** Cambiar `lbug` por `sparrowdb` sin
  tocar la estructura. Funciona pero deja la misma fragilidad bajo
  otra marca. Si SparrowDB madura, romperá la API, y volveremos a
  empezar.

- **(C) Encapsular `lbug` en un módulo pero sin trait.** Esconder el
  detalle tras un wrapper concreto. Más simple que un port pero no
  testeable: sigue habiendo un solo motor real.

- **(D) Puerto hexagonal + adapter actual.** **Elegida.** Trait
  `GraphStore` con métodos `init`, `stat`, `query`, `put_evidence`,
  `list_evidence`. Adapter actual `LbugStore` implementa el port.
  SparrowDB entra como un segundo adapter (`SparrowStore`) cuando se
  decida adoptarlo.

## Decisión

### El puerto: `crate::store::GraphStore`

```rust
pub trait GraphStore: Send + Sync {
    fn open(project_dir: &Path) -> Result<Self> where Self: Sized;
    fn init(&mut self) -> Result<()>;
    fn stat(&self) -> Result<GraphStat>;
    fn query(&self, cypher: &str) -> Result<Vec<Json>>;
    fn put_evidence(&mut self, evidence: &[Evidence]) -> Result<usize>;
    fn list_evidence(&self, path: Option<&str>) -> Result<Vec<Json>>;
}
```

Cinco métodos. Sin `Connection`, sin `Database`, sin handles — el
adaptador gestiona su ciclo de vida internamente. La firma `fn
list_evidence(&self, path: Option<&str>)` devuelve filas ya
materializadas como `serde_json::Value`, lo que elimina la conversión
de tipos driver-específicos del dominio.

### El adapter actual: `crate::store::LbugStore`

Contiene todo el código que antes vivía en `graph.rs` (open session,
schema statements, value_to_json, value_to_i64, count_match, etc.).
`graph.rs` se queda con la **API pública** (`init`, `stat`, `query`,
`neighbours`, `open_session`, `database_path`, `validate_identifier`,
`GraphStat`) como shims delgados que delegan al port. Esto preserva
backward compat con `pub use graph::*` en `lib.rs` y con los tests
legado que llaman `open_session` directamente.

### Lo que NO entra en el puerto

Tres cosas se quedan explícitamente fuera del trait y viven en el
dominio:

1. **`validate_identifier`.** Validación de strings contra una
   allowlist antes de interpolar en Cypher. Es protección contra
   injection, no detalle del motor. La lógica es ortogonal a qué
   backend use `archctl`.

2. **Forma del query string.** El trait recibe Cypher plano. Si
   mañana queremos GQL o SQL/PGQ, eso es un refactor del query
   strings en `evidence::put` y `cli::*` — no del port.

3. **Cross-engine migration.** Si adoptamos SparrowDB mañana y
   queremos leer un archivo `architecture.lbdb` legacy, eso es un
   `SparrowStore::import_lbug(path) -> Result<()>`, no un método del
   trait.

### Selección del adapter en runtime

Una sola función factory hoy:

```rust
pub fn open_default(project_dir: &Path) -> Result<Box<dyn GraphStore>> {
    Ok(Box::new(LbugStore::open(project_dir)?))
}
```

Mañana, cuando llegue SparrowDB:

```rust
pub fn open_default(project_dir: &Path) -> Result<Box<dyn GraphStore>> {
    let backend = std::env::var("ARCHCTL_STORE").unwrap_or_else(|_| "lbug".into());
    match backend.as_str() {
        "lbug" => Ok(Box::new(LbugStore::open(project_dir)?)),
        "sparrowdb" => Ok(Box::new(SparrowStore::open(project_dir)?)),
        other => anyhow::bail!("unknown store backend: {other}"),
    }
}
```

El flag CLI `--store sparrowdb|lbug` puede ir a `xdg/config.yaml`
para que sea persistente sin tocar CLI parsing.

## Consecuencias

### Positivas

- **Migración de motor cuesta O(1) archivos.** Cambiar de `lbug` a
  `sparrowdb` = escribir `SparrowStore` (~200 LOC, mismo patrón que
  `LbugStore`) y un match arm en `open_default`. El dominio no se
  entera.

- **Tests delgados.** `store::tests` prueba el trait directamente
  con un `LbugStore` real. Tests del dominio (`evidence::*`) ya no
  abren `lbug::Database`; reciben un `&mut dyn GraphStore`.

- **Bindings Python/Node/Ruby vía SparrowDB.** Cuando SparrowDB
  estabilice sus bindings, abrimos la puerta a que un wrapper en
  Python o Node consuma el mismo `.sparrow` que `archctl` produce.

- **Compat layer intacto.** El `pub use graph::*` no cambia. Tests
  legado (`graph::tests::*`) siguen funcionando. La transición es
  aditiva.

### Negativas y riesgos

- **Doble vía hasta que se elimine `graph.rs`.** El shim layer
  (`graph::init` etc.) duplica nombres con el port. Mantenerlo es
  decisión consciente (compat) pero crea dos formas de hacer lo
  mismo. La limpieza final — eliminar el shim — es Ola 3, sin fecha.

- **`open_default` aún no elige runtime.** Hoy es hardcoded a
  `LbugStore`. La decisión de cuándo añadir el match real es
  Ola 2 (cuando llegue SparrowStore).

- **El port no oculta el query language.** Migrar de Cypher a otra
  cosa sigue siendo un refactor de strings. Es una limitación
  aceptada: el trait describe comportamiento, no sintaxis.

- **`lbug` sigue como dependencia directa.** El refactor elimina el
  acoplamiento en el código del dominio, pero no quita la dependencia
  de `Cargo.toml`. Eso es deliberado: `LbugStore` la necesita. Cuando
  llegue `SparrowStore`, ambas vivirán como deps detrás de features
  (default = `lbug`).

## Cómo revertir

Reversión es trivial: borrar `crate::store`, volver a mover la
lógica de `LbugStore` a `graph.rs`, restaurar `evidence::put` con su
string-interpolación inline. El refactor fue puramente estructural,
no cambió semántica. Diff sería ~600 líneas eliminadas.

## SparrowDB — research notes (Ola 2)

Lo que investigamos para validar que el port tiene sentido:

- `sparrowdb = "0.1.16"` (julio 2026), licencia MIT, un solo
  maintainer (`ryaker`).
- API: `GraphDb::open(path)`, `db.execute("MATCH … RETURN …")`,
  `db.execute_with_params(cypher, params)`, `db.execute_batch(&[cyphers])`,
  `begin_read()`, `begin_write()`, `checkpoint()`, `optimize()`,
  `export_json()`, `import_json()`.
- Modelo de transacción: **SWMR** (single writer + multiple
  readers), snapshot isolation. Encaja con ADR-010 (lock por
  proyecto).
- Query language: **Cypher** (mismo que `lbug`). Las cadenas
  Cypher escritas hoy se reutilizan tal cual.
- Bindings multi-lenguaje prometidos: Rust (oficial). Python/Node/Ruby
  anunciados pero **no verificados en el research inicial** — son
  claim del README, no releases en crates.io/pypi. Pendiente
  verificar antes de Ola 2.
- Versión 0.1.16 es **muy joven**. API puede romper. El port
  mitiga esto al costo de una implementación inicial mayor.

## Próximos pasos

1. ✅ Ola 1 — port hexagonal implementado, 41 tests verdes, binario
   end-to-end verificado.
2. **Ola 2** (cuando se decida) — escribir `SparrowStore`,
   añadir `ARCHCTL_STORE` env var, suite de tests paralelos
   lbug-vs-sparrowdb contra fixtures idénticos.
3. **Ola 3** (sin fecha) — eliminar `graph::init/stat/query/neighbours`
   shims, dejar solo `validate_identifier` y `database_path` como
   utilidades puras del dominio.
