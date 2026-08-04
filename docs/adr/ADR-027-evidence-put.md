# ADR-027 — Evidence Put: Identity Scheme + Separation of Concerns

> **Ciclo:** `b1-source-evaluation-types`
> **Estado:** Aceptado (propuesto)
> **Fecha:** 2026-08-04
> **Complementa:** [ADR-016](ADR-016-activegraph-packs-investigacion.md) §B3 (source_origin)

## Problema

El comando `evidence extract` funciona sobre archivos de código (byte ranges, content_hash). El agente necesita ingestar **hechos semánticos** que no provienen de un archivo — actores definidos por el usuario, use cases declarados verbalmente, transiciones de máquina de estados inferidas sin código fuente.

No existe un subcomando `evidence put` que acepte JSON de hechos con `source_origin: UserInput` y genere identidad estable sin archivo. El gap bloquea las skills de `architecture-discovery` y `c4/from-graph`.

## Decisiones

### D1 — `evidence put` persiste SOLO Evidence + SourceArtifact, NO Elements

**Elección:** `evidence put` ingiere hechos semánticos y crea únicamente:
- Un nodo `Evidence` con `source_origin: UserInput`, `status: drafted`
- Un nodo `SourceArtifact` synthetic (sin archivo real) solo si el hecho lo requiere
- **NO** crea nodos `Element` ni `ElementVersion` semánticos (actor, use_case, state, etc.)

**Rationale:** El modelo de Evidence ya soporta este contrato: `put_with_source()` persiste Evidence + SourceArtifact + Evaluation. Crear Elements desde facts es una **operación de grafo**, no de evidencia — separación de concerns. El agente que quiera crear un `uml.actor` desde un fact usa `graph query` Cypher o un futuro `diagram materialize`.

**Alternativas rechazadas:**
- `evidence put` crea también Elements semánticos — rechazado: la creación de Elements requiere knowledge del grafo (canonical keys existentes, version chains). `evidence put` sería un comando de grafo con capacidad de escritura que excede su ámbito.
- `evidence put` acepta solo facts, no Evidence rows — rechazado: los hechos semánticos **son** Evidence rows con provenance UserInput. No hay distinción.

### D2 — Identity scheme para hechos sin archivo

**Elección:** `evidence_id = "ev:sem:" + blake3(kind + claim + source_origin + sorted_canonical_props)`

```rust
// evidence.rs — SemanticEvidenceId
pub fn semantic_evidence_id(
    kind: &str,
    claim: &str,
    source_origin: SourceOrigin,
    props: &serde_json::Map<String, serde_json::Value>,
) -> String {
    let mut h = blake3::Hasher::new();
    h.update(kind.as_bytes());
    h.update(claim.as_bytes());
    h.update(source_origin.as_str().as_bytes());
    // Props canónicos: sorted by key for determinism
    let mut keys: Vec<_> = props.keys().collect();
    keys.sort();
    for k in &keys {
        h.update(k.as_bytes());
        if let Some(v) = props.get(k) {
            h.update(v.to_string().as_bytes());
        }
    }
    format!("ev:sem:{}", hex::encode(&h.finalize().as_bytes()[..16]))
}
```

**El prefijo `ev:sem:`** distingue de los `evidence_id` existentes (que usan `ev:` + blake3 de path:byte_range:text).

**Propiedades:**
- **Determinista**: dos llamadas con los mismos inputs producen el mismo id
- **Colisión-resistente**: blake3 128-bit (16 bytes hex → 32 chars) da ~2^128 espacio
- **Sin archivo**: no requiere path ni byte range
- **Idempotente**: re-running `evidence put` con los mismos facts no duplica

**Alternativas rechazadas:**
- UUID v4 explícito — rechazado: no es determinista. Re-running produce IDs diferentes.
- Solo `claim` como hash — rechazado: colisiones si dos hechos distintos dicen "actor" con claim ligeramente diferente.
- ID provisto por el usuario — rechazado: el sistema no puede verificar unicidad.

### D3 — SourceArtifact synthetic para hechos sin archivo

**Elección:** Cuando el hecho no tiene archivo fuente asociado, `evidence put` crea un `SourceArtifact` con:
- `relative_path = ""` (vacío)
- `content_hash = ""` (vacío)
- `id = "src:synthetic:" + blake3("synthetic" + kind + claim)` (sigue el patrón ADR-017 §D2)

**Rationale:** El modelo requiere que cada Evidence tenga un `EXTRACTED_FROM` edge a un SourceArtifact. Para hechos sin archivo, un SourceArtifact synthetic con ID predecible permite que el edge se cree sin violar la integridad del grafo.

### D4 — `source_origin: UserInput` → `status: drafted` por defecto

**Elección:** Todo hecho ingestado via `evidence put` recibe `source_origin: UserInput` y `status: drafted`.

**Rationale:** Es el contrato ADR-016 §B3: UserInput → drafted. El agente que quiera que el hecho contribuya a proyecciones debe promoverlo a `accepted` via `evidence accept --id <id>`.

### D5 — JSON input schema para `evidence put`

```json
{
  "facts": [
    {
      "kind": "structural | lexical | config | annotation | other",
      "claim": "OrderState machine models the order lifecycle",
      "props": {
        "element_kind": "state_machine",
        "canonical_key": "rust:src/order.rs:OrderState",
        "language": "rust"
      }
    }
  ]
}
```

**Validaciones:**
- `kind` debe ser uno de los `EvidenceKind` variants
- `claim` no puede estar vacío (255 chars max)
- `props` es opcional; si se provee `canonical_key`, se usa en el `content` del carrier

**Rationale:** Schema mínimo viable. `element_kind` comunica la intención semántica sin crear Elements — el agente la lee para decidir qué relaciones crear después.

---

## Relación con `evidence extract`

| Aspecto | `evidence extract` | `evidence put` |
|---------|-------------------|----------------|
| Source | Archivo de código | Input libre (JSON) |
| Byte range | Sí (del match AST) | No |
| `source_origin` | `UserWorkspace` | `UserInput` |
| `status` default | `Accepted` | `Drafted` |
| `evidence_id` hash input | `path + start + end + text` | `kind + claim + source_origin + props` |
| Requiere archivo | Sí | No |
| Idempotente | Sí (mismo archivo) | Sí (mismos inputs) |

---

## Consequences

### Positivos
- `evidence put` es idempotente y determinista
- No se requiere archivo ni byte range
- Separation of concerns: Evidence = provenance, Elements = dominio
- Reutiliza `put_with_source()` existente sin cambios

### Negativos
- Hechos sin archivo no pueden ser re-verificados contra código fuente
- `canonical_key` en props es opcional — si falta, no hay forma de linkar el hecho a un Element existente

### Riesgos residuales
- **Colisiones de hash**: `ev:sem:` con 16 bytes de blake3 tienen ~2^128 espacio — riesgo despreciable
- **Hechos ambiguos**: `status: drafted` hasta que el agente los acepte — la projections no los incluyen hasta entonces

## Prohibiciones

- `evidence put` NO crea nodos `Element` ni `ElementVersion` — eso es responsabilidad del agente vía grafo queries
- `evidence put` NO debe recibir byte ranges ni content_hash — son exclusivos de archivos reales
- `evidence put` NO debe marcar facts como `accepted` — el lifecycle `drafted → accepted` requiere aceptación explícita
