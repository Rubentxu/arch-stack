# ADR-029 — C4 Component Light: Estrategia de detección de componentes con revisión de agente

> **Ciclo:** `diagram-authoring-toolchain`
> **Estado:** Aceptado (propuesto)
> **Fecha:** 2026-08-04
> **Complementa:** [ADR-012](ADR-012-adopcion-incremental-crates-analisis.md) + [ADR-026](ADR-026-state-machine-metamodel.md)

## Problema

`archctl code c4-discover` detecta **Containers** (estrategias `cargo`/`npm`), pero el nivel **Component** de C4 (ADR-007) no tiene extractor:

- `mt.component` existe en el metamodelo (`metamodel-core.json`) pero nada lo puebla.
- Las skills y agentes de `profile/` no pueden proyectar una vista de componentes (`diagram project --view c4-component`) porque el grafo no tiene componentes.
- El agente podría crearlos manualmente vía `evidence put` + operaciones de grafo, pero eso es tedioso y sin evidencia reproducible.

El nivel Component es inherentemente **semántico**: un "componente" es una unidad cohesionada con boundary explícito, algo que un AST no puede decidir con certeza. Pero un extractor puede ofrecer **candidatos** con evidencia de módulo, dejando la decisión al agente.

## Decisiones

### D1 — Nueva estrategia `components` en el framework existente de c4-discover

**Elección:** Se añade una estrategia `components` al registro de `c4_discover.rs` (mismo framework `Strategy` con `detect()`, `id()`, confidence, merge cross-strategy). NO se crea un comando nuevo.

**Detección (heurística conservadora):**
- Módulos internos de primer nivel bajo el source root del proyecto (ej: `src/orders/`, `src/payments/` en Rust; carpetas top-level en TS/Python).
- Paquetes del workspace (Cargo workspace members, npm workspaces) que NO son containers (excluye los ya detectados por `cargo`/`npm` strategy).
- Cada candidato: `canonical_key`, `name`, evidencia de archivos (paths + líneas de mod), `confidence < 1.0` (heurística: el boundary es inferido, no declarado).

**Rationale:** Reutiliza el framework probado de strategies (merge cross-strategy, determinismo, `--apply`, `--json`). Un comando nuevo duplicaría infraestructura. El `--strategy components` se registra como estrategia adicional, no rompe las existentes.

**Alternativas rechazadas:**
- Comando nuevo `code c4-components` — rechazado: duplica el framework Strategy y el merge; la audiencia es la misma (descubrimiento C4).
- Extractor autoritativo con confidence 1.0 — rechazado: el boundary de componente no es decidible por AST; un confidence alto produciría falsos positivos dañinos.

### D2 — El agente revisa y promueve (misma filosofía que ADR-026)

**Elección:** Los candidatos se persisten como `Element` de tipo `mt.component` con `current_confidence < 1.0` (via `--apply`). El agente (skill `c4-from-graph`) revisa cada candidato y:
- Lo **acepta** (la evidencia del candidato se promueve a `accepted` vía `evidence accept`), o
- Lo **rechaza** (la evidencia se marca `superseded`), o
- Lo **renombra/ajusta** (edición de view spec — fuera de alcance de este ciclo, futuro `diagram materialize`).

**Rationale:** Consistente con ADR-026 (state machine): el extractor detecta lo detectable con confidence < 1.0; el agente da el sentido final. La evidencia con procedencia (`rule_id: c4-discover:components`) hace la revisión reproducible.

**Prohibiciones:**
- Promover candidatos a `accepted` automáticamente (siempre requiere agente/usuario).
- Detectar componentes en código de terceros (solo módulos del repo).
- Duplicar containers ya detectados (merge excluye canonical_keys ya existentes de containers).

## Consecuencias

- **Positivas:** el grafo puede poblarse con componentes candidatos reproducibles; `diagram project --view c4-component` funciona; las skills tienen evidencia que revisar; cero cambios en estrategias existentes.
- **Negativas:** heurística puede producir candidatos ruidosos en monorepos sin estructura clara; requiere revisión humana/agente (es el diseño, no un defecto).
- **Trazabilidad:** cada candidato lleva `strategy: components`, `rule_id: c4-discover:components`, confidence y paths de evidencia.

## Archivos afectados

| Archivo | Cambio |
|---------|--------|
| `archctl/src/code/strategies/components.rs` | **Nuevo** — estrategia `components` (~250 LOC) |
| `archctl/src/code/strategies/mod.rs` | Registrar estrategia |
| `archctl/src/code/c4_discover.rs` | Sin cambios funcionales (framework ya registra strategies) |
| `archctl/src/code/mod.rs` | Exportar estrategia |
| `archctl/tests/` | Tests de la estrategia (módulos → candidatos, exclusión de containers) |
