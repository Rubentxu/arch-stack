# Estrategia de migración

## Principio

**Strangler refactor interno**, no reescritura. Cada paso conserva CLI/schema salvo
que una spec/ADR declare un cambio.

## A — Freeze de fronteras
- inventariar dependencias;
- golden outputs de CLI;
- baseline de imports y tamaños;
- no mover código aún.

## B — Composition root

```rust
struct Runtime {
    fs: Arc<dyn Filesystem>,
    architecture: Arc<dyn ArchitectureRepository>,
    evidence: Arc<dyn EvidenceRepository>,
    graph_query: Arc<dyn RawGraphQuery>,
    git: Arc<dyn GitRepository>,
    clock: Arc<dyn Clock>,
}
```

El nombre no es contrato; la propiedad importante es un único borde de construcción.

## C — Use cases

```text
clap DTO
  ↓
CLI adapter mapping
  ↓
UseCase::execute(Input)
  ↓
Output DTO
  ↓
human/json formatter
```

## D — Repositories
Introducir ports semánticos delante del store actual. Inicialmente el adapter puede
delegar en `GraphStore`; después se eliminan queries Cypher de usecases.

## E — Module boundaries
Crear `model`, `analysis`, `knowledge`, `projection`, `workbench`, `distribution`.
Añadir dependency tests.

## F — Optional crate extraction
Extraer crate solo cuando:
- boundary estable al menos un ciclo;
- sin ciclos;
- ownership/compile isolation aporta valor;
- o existen ≥2 consumidores.

## Compatibilidad
- mantener comandos;
- mantener JSON schemas;
- versionar contratos cuando realmente cambien;
- aliases deprecated solo un ciclo si son imprescindibles;
- no duplicar almacenamiento.

## Rollback
Cada PR estructural:
1. no mezcla feature nueva;
2. tiene equivalence/golden tests;
3. revierte sin migración de datos salvo schema PR;
4. no borra raw query mientras exista consumidor no migrado.

## Métricas
- imports prohibidos;
- Cypher fuera del adapter;
- `std::fs` fuera de adapters;
- `Command::new` fuera de adapters;
- handlers CLI con negocio;
- archivos >30 KB;
- usecases testeados sin I/O real.
