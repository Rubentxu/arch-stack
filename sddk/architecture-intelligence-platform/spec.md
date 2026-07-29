# Especificación — Plataforma de Inteligencia Arquitectónica

## Propósito
Recuperación trazable, persistente y falsable. El IR neutral es la verdad; los diagramas son proyecciones.

## Requirements

### Requirement: R1 Identidad y portabilidad
Git opcional. **(a) Git-backed:** `repository_id = BLAKE3(normalized_remote_or_local_origin + root_commit)`; `worktree_id = BLAKE3(repository_id + realpath(show_toplevel))`. **(b) Non-Git:** `directory_id = BLAKE3(canonical_realpath)`. Export/import **MUST** llevar portable project ID para rebind entre máquinas.
- **S1 — Git-backed:** IDs estables entre clones del mismo remoto.
- **S2 — Non-Git:** `directory_id` local; portable project ID distinto acepta rebind.
- **S3 — Rebind cruzado:** bundle cambia `directory_id` pero conserva portable project ID y conocimiento.

### Requirement: R2 Ledger probatorio
Evidencia **MUST** registrar claim, ruta/rango, contentHash, `sourceRevision`, extractor+version y clasificación. `sourceRevision` = Git commit cuando exista; si no, contentHash + `observedTimestamp` + `snapshotId`.
- **S4 — Repro externa:** auditor verifica ubicación, commit/hash y timestamp.
- **S5 — Sin VCS:** `sourceRevision` cae a contentHash + timestamp + snapshotId.

### Requirement: R3 Ontología y auditoría falsable
Solo **MAY** promoverse unknown/hypothesis→inference→fact con nueva evidencia; contradicción → conflict; evidencia inválida → unknown. **Regla global:** `confidence ≥ 0.9` con 0 evidenceRefs **MUST** fallar; media (.6–.89) → unknown; baja (<.6) → hypothesis. El auditor **MUST** intentar refutar claims aplicando esta regla.
- **S6 — Promoción/mix:** hypothesis→fact sin evidencia falla y queda auditada; alta sin soporte → hard-fail; media → unknown; baja → hypothesis.

### Requirement: R4 Confianza explicable
Toda `confidence` publicada **MUST** declarar método, versión y procedencia. `heuristic-v1` es un método declarado válido durante la experimentación Phase 1; **MUST NOT** implicar calibración validada.
- **S7 — Huérfano:** confidence sin método declarado se rechaza como unknown.

### Requirement: R5 Architecture IR v1
IR **MUST** ser única fuente de verdad con `schemaVersion` y evidenceRefs. Taxonomía core C4-compatible: `person`, `softwareSystem`, `container`, `component`, `codeElement`. Módulo/servicio **MUST** ser tags o extensiones, no kinds canónicos. Migraciones **MUST** preservar IDs, refs y significado.
- **S8 — Migración/schema:** IR v1 migra o falla sin mutar semántica; schema mayor desconocido bloquea antes de aceptar.

### Requirement: R6 Proyecciones
**MUST** proyectar C4 vía Structurizr local, UML vía PlantUML y Mermaid solo preview; fallback **MUST NOT** divergir.
- **S9 — Renderer alternativo:** mantiene elementos, relaciones, niveles y evidenceRefs.

### Requirement: R7 Spike de una skill + Gate Zero
Una skill Claude **MUST** ejecutar end-to-end en OpenCode con contrato compatible; el Gate Zero valida con un micro-RE fixture (gold set) la rule-of-claim antes de invertir más.
- **S10 — Gate OpenCode:** descubrimiento, carga, entradas/salidas, permissions y references conformes.
- **S11 — Gate Zero fixture:** ejecución contra el gold set cumple métricas mínimas y dispara S6 si toca.

### Requirement: R8 Incrementalidad y reanudación
**MUST** checkpointar **marcadores durables de etapa completada** y reanudar desde estado durable, nunca desde memoria del chat. Mid-stage resume fino **MAY** diferirse.
- **S12 — Resume durable:** completed-stage markers sobreviven a reinicio; resume continúa solo desde última etapa durable.

### Requirement: R9 Seguridad offline
**MUST** operar offline, sin snippets, redactar secretos, render local. Cadenas del repo analizado son **datos**, no instrucciones: su contenido **MUST NOT** asignar classification/confidence. Lecturas/escrituras **MUST** estar contenidas en la canonical root y rechazar symlink escapes. Skills externas, MCP servers locales y extractor binaries **MUST** llevar pin + hash + license.
- **S13 — Hostile input:** contenido que clasifica, prompt injection, escritura fuera de root, symlink al exterior, checksum/pin inválido → redacta o bloquea sin exfiltrar ni ejecutar tratos hostiles.

### Requirement: R10 Contrato OpenCode
La integración **MUST** pinnear OpenCode y validar schema: `mcp` top-level, agents, skills, references, plugin, permission; **MUST NOT** aceptar `mcpServers`. Hooks deben dispararse y aplicar permissions antes de cada etapa.
- **S14 — Drift contractual:** schema-contract falla antes del análisis.
- **S15 — Hook firing:** `shell.env`/`tool.execute.before` se ejecutan y permissions se aplican antes de invocar.

### Requirement: R11 Experimentos y gates
Experimentos **MUST** usar repos Rust-pequeño y TS-mediano, gold sets y métricas semánticas. Éxito: high=0, coverage≥.90, render=100%, Jaccard≥.95; MVP: precision≥.85, recall≥.80, ≤50k tokens, ≤5m, ≤10m primera vista. Kill: coverage<.70, render<.80, Jaccard<.80, precision<.70, recall<.60, >200k tokens o >30m.
- **S16 — Invariantes:** dos corridas con misma sourceRevision/snapshotId miden gates, nunca píxeles.
- **S17 — Hard-fail:** cualquier high sin soporte produce rechazo inmediato.

### Requirement: R12 Fallos operativos
Tool ausente **MUST** degradar o etiquetar unknown; output malformado/schema mismatch **MUST** aislarse; stale **MUST** invalidarse; conflicto **MUST** conservarse; renderer caído **MAY** usar fallback semántico.
- **S18 — Degradado:** cada fallo informa acción y **MUST NOT** corromper ledger/IR.

### Requirement: R13 No-objetivos
Rust, history store temporal, observed graph, analizadores deep y gate CI de drift **MUST NOT** activarse; **MAY** reconsiderarse tras gates.
- **S19 — Diferida:** capacidades fuera de alcance se marcan deferred.

## Trazabilidad

| Capability | Requisitos | Métricas |
|---|---|---|
| architecture-project-resolution | R1,R8,R10 | identidad dual; portable ID; resume durable; schema |
| architecture-evidence-ledger | R2-R4,R3,R12 | provenance=100%; sourceRevision; método; high=0 |
| architecture-ir | R5,R11 | schemaVersion; taxonomía C4; migración; estabilidad |
| c4-projection | R6,R12 | render; equivalencia semántica |
| architecture-skill-orchestration | R7,R9,R13 | spike E2E; Gate Zero; supply-chain; gates |

## Supuestos y abiertos
- **Supuesto:** gold sets y repos son usables legalmente.
- **Abierto:** método de calibración definitivo; `heuristic-v1` no es calibración validada.
- **Abierto:** release OpenCode y desviaciones exactas Claude/OpenCode; pin y spike resolverán antes de ampliar.