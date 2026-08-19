# Auditoría — Roadmap, ADRs, implementación y deuda (2026-08-19)

> Post-release v1.68.0 (`wave-3-workbench-ux`). Baseline: `main@f9ffc7f`
> (PR #233 incluido). Binario de referencia: build fresco vía
> `cargo run` (target dir real: `/var/home/rubentxu/cargo-targets`).

## 1. Roadmap — pendiente

### Wave 3 (catálogo 2026-08-13)

| Item | Estado | Gate |
|---|---|---|
| 19, 22, 27, 28+29, 31–33 | ✅ Cerrados v1.60.0–v1.68.0 | — |
| 30 (session token, P3-03) | ⏸ Pendiente | ADR-051 deferido (hijack vector disclosed) |
| 34 (lens recommendation, P3-05) | ⏸ Pendiente | ADR-056/062 (≥2 consumers OR measured need) |

### Horizons (ROADMAP)

- **H0/H1/H2/H4** — cerrados (v1.31.0–v1.39.x).
- **H3 (moldabilidad)** — **parcial**: items 31–33 shipped v1.68.0;
  LensSpec (P3-05) sigue en entry criteria por diseño (ADR-062).

### Anti-roadmap (ADR-039 §tabla, 10 decisiones deferred)

WGPU, Rust/WASM compute, Apache Arrow, cosmos.gl, SceneGraph, WIT SDK,
Event sourcing, Architecture Lab, 9-agent catalog, Tauri — todas con
triggers medibles. Ningún trigger disparado.

### Otros pendientes con trigger

- ADR-051 loopback session security (deferido).
- ADR-016 B2 (manifest+content_hash+static gates) / B3 (trust-by-origin).
- ADR-014 Ola 2 (SparrowDB adapter — opcional, port listo).
- ADR-046 B2/B3 (gating por plugin origin, attestation firmada,
  per-plugin scopes).
- ADR-007 ViewEdge (diferido a archview 1.x).
- Nivel "Code" C4→class-diagram (ADR-062 reopen trigger propio).
- ELK layout + virtualización >1k nodos (M17.1 opcional).
- Menores: redaction report, cutoff staleness por proyecto (XDG),
  bump lbug.

## 2. Verificación de implementaciones (exhaustiva)

### Coherencia tags ↔ CHANGELOG ↔ código

- Tags v1.57.0–v1.68.0: 12/12 con sección CHANGELOG ✓.
- Claims v1.60–v1.67 verificados en código: `--cutoff-days`/`--expire-stale`
  (fusion.rs), entropía Shannon + allowlist (redact.rs), seams fuse-on-write
  (c4_discover.rs + call_graph.rs), `ExportProfile::Strict` (export.rs),
  backfill v5 (migrations.rs), `PutEvidenceResult` (dual-write), `ide_doctor`,
  policy metamodel ✓.
- Claims v1.68: navigation.ts (NavStack/zoomTargetFor/c4SelectorFor, 12
  símbolos), `/api/explain` (view.rs, 3), wiring strict (App.tsx, 4) ✓.
- CLI live (binario fresco 1.68.0): **12/12 subcomandos** de
  `archctl architecture` presentes (create/list/gc/diff/explain/coverage/
  policy/relevance/context/observe/fuse/intent) ✓.
- Archview embebido: CSS de assets-view contiene `nav-history`,
  `breadcrumbs`, `node-actions` (workbench nuevo) ✓.
- Gates: doctor 30/30 OK · `capabilities --check` OK · ADR integrity
  0 errores · verify-local cheap PASS · 1107 tests Rust + 147 TS ·
  clippy `--all-targets -D warnings` limpio · fmt limpio.

### Gaps encontrados DURANTE la auditoría (ya cerrados)

1. **CAPABILITIES.md stale** (nueva capability `cli.agent` + header de
   versión) — el check embebido lo detectó; regenerado.
2. **Cargo.lock en 1.59.0** tras el bump — el workspace lockfile embebe
   la versión. Fix: PR #233 (precedente v1.42.0: fix forward, sin
   re-tag; el source del tag v1.68.0 es correcto).

### Falsa alarma documentada

`archctl/target/debug/archctl` es un artefacto **stale del 2026-08-17**
(no se usa: el target dir real es `/var/home/rubentxu/cargo-targets` por
config de cargo del usuario). Usarlo para smoke-tests muestra un CLI sin
context/observe/fuse/intent. Recomendación: `rm -rf archctl/target` o
documentar el target dir en AGENTS.md.

## 3. Calidad

| Métrica | Valor | Nota |
|---|---|---|
| Tests Rust | 1107 (baseline 872 @ v1.48) | +235 en ~2 semanas |
| Tests TS | 147 (incluye 19 nuevos del ciclo) | — |
| TODOs abiertos | 0 | M60 cerró los 2 de M55 |
| Comentarios `ponytail:` | 0 | ledger de deuda vacío |
| Clippy/fmt | limpio | — |
| Deprecated APIs | 5 | 2 desde 0.2.0, 3 desde 1.43.0 — candidatos a barrido |
| Archivos >1000 LOC | 9 | store.rs 4662, cli.rs 4464 lideran |

## 4. Deuda técnica activa

| # | Deuda | Severidad | Acción |
|---|---|---|---|
| D1 | `store.rs` (4662) / `cli.rs` (4464) — SRP | MEDIUM | Split en ciclo natural (P1-01 solo cubrió composition root) |
| D2 | 5 APIs `#[deprecated]` fuera de ventana | LOW | Barrido chore (re-exports queries.rs + shims evidence.rs) |
| D3 | `archctl/target/` legacy dir stale | LOW | `rm -rf` + nota en AGENTS.md sobre target dir real |
| D4 | lbug 0.18.x sin implicit cast STRING→TIMESTAMP | LOW | Workaround documentado (parse_observed_at); evaluar bump |
| D5 | Binario `archctl` instalado del usuario: **1.45.0** | MEDIUM (ops) | `archctl self update` (23 minors atrás) |
| D6 | 2 LOW del cycle debt-verify (relationsFor ×2, explain sin caché) | LOW | Aceptadas por diseño |

## Conclusiones

1. **Todo lo mergeado está implementado y verificado** — los únicos gaps
   reales fueron los 2 de sync de release (PR #233), cerrados en esta
   auditoría.
2. **El roadmap restante está 100% gateado** — no hay trabajo unblocked
   salvo los pendientes menores y el barrido de deprecated.
3. **La calidad subió en el ciclo**: +33 tests, 0 deuda nueva material,
   2 deudas pre-existentes cerradas.
4. Recomendación operativa inmediata: `archctl self update` (D5).
