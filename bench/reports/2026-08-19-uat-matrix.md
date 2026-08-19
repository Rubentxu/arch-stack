# UAT multi-lenguaje — reporte de matriz completa (2026-08-19)

> Sandbox Podman `archctl-bench:latest` · binario `archctl 1.72.0`
> compilado in-container (`bench/build-in-sandbox.sh`) · datasets
> `~/.cache/archctl-smoke/` (pinned por SHA, `bench/datasets.toml`)
> Plan e investigación: `docs/sessions/2026-08-19-uat-multilang-sandbox.md`.

## Veredicto global: **11/11 datasets PASS**

| # | Dataset (lang) | Celdas | Veredicto | Elementos call-graph | Notas |
|---|---|---|---|---|---|
| 1 | tokio-rs/axum (rust) | 10 | ✅ PASS | 1056 | Smoke inicial de la sesión |
| 2 | BurntSushi/ripgrep (rust) | 10 | ✅ PASS | 807 | |
| 3 | clap-rs/clap (rust) | 10 | ✅ PASS | 2093 | call-graph multi-lenguaje incluye script python en `.github/` |
| 4 | archctl (rust, dogfood) | 10 | ✅ PASS | 5688 | `HEAVY_TIMEOUT=1800` obligatorio (600 marginal); dataset self-dogfood = copia local del checkout |
| 5 | pmndrs/zustand (typescript) | 10 | ✅ PASS | 212 | |
| 6 | vueuse/vueuse (typescript) | 10 | ✅ PASS | 1239 | 4 bugs encontrados → v1.71.0 + v1.72.0 |
| 7 | expressjs/express (javascript) | 10 | ✅ PASS | 125 | |
| 8 | labstack/echo (go) | 5 | ✅ PASS | 1307 | Smoke inicial de la sesión |
| 9 | psf/requests (python) | 6 | ✅ PASS | 545 | |
| 10 | square/javapoet (java) | 6 | ✅ PASS | 662 | |
| 11 | mockk/mockk (kotlin) | 2 | ✅ PASS | 2236 | state-machine NO aplica a kotlin (soporta rust/ts/python/go) — la matriz del doc de sesión se corrige |

## Bugs de producto encontrados y cerrados (9 total)

| # | Bug | Impacto | Fix | Release |
|---|---|---|---|---|
| 1 | canonical_key con `@` (closures Rust) → ids `cg:` inválidos | call-graph --apply fallaba en rust | `graph::sanitize_identifier` + 13 sitios | v1.70.0 (#245) |
| 2 | `batch_link_of_type` tragaba errores (`let _`) | links OF_TYPE perdidos en silencio | propagación con contexto | v1.70.0 (#245) |
| 3 | `EvidenceEntry.status` ausente del schema | `diagram validate` fallaba tras `evidence accept` | schema 1.1.0 → 1.1.1 | v1.70.0 (#245) |
| 4 | `parse_from_selector` sin go/java/kotlin/javascript | `sequence --from <key>` caía a ByName | prefixes añadidos | v1.70.0 (#245) |
| 5 | Categoría `code` ausente en whitelists | repos solo-call-graph invisibles en relevance/coverage/explain | categoría + routing `cg:`/`cd:` | v1.70.0 (#245) |
| 6 | Paths con `@` rechazados por `validate_identifier` | `call-graph --apply` fallaba en vueuse (snapshots scoped, patches) | paths como DATA (quote-escaping) en 5 sitios | v1.71.0 (#247) |
| 7 | `NpmWorkspace` sin pnpm-workspace.yaml + globs como paths literales | **ningún** workspace npm/yarn/pnpm detectaba miembros | parser pnpm + expansión de globs `/*` + exclusiones `!` | v1.72.0 (#249) |
| 8 | `components` trataba dirs ocultos como módulos (`packages..test`) | candidato espurio con id roto | skip de dirs `.`-prefijados (3 detectores) | v1.72.0 (#249) |
| 9 | ids `c4:container:@vueuse/core` sin sanear | batch OF_TYPE fallaba → rollback completo del apply | `sanitize_identifier` en ids c4 | v1.72.0 (#249) |

### Latentes (documentados, sin fix hoy)

- `rel_id` con `→` en class-diagram: no validado hoy; tratar si `validate_identifier` se extiende.
- Fallo de `call-graph --apply` con `--cwd` apuntando AL PADRE del repo (dir org) da
  `write_source_artifact` sin contexto claro — mensaje críptico; mejorar DX.

## Observaciones operativas

- **`HEAVY_TIMEOUT=600` es insuficiente** para el dogfood archctl (multi-lenguaje,
  68K LOC): cada extracción call-graph tarda ~6–8 min in-container. Baseline nuevo:
  `HEAVY_TIMEOUT=1800` para rust/archctl; los demás datasets caben en 600.
- **Celdas vacuas**: `accept`, `export+validate` y `strict+checksum` PASAN con 0
  datos (bundle vacío válido). Un dataset "verde" con 0 elementos es un falso
  positivo — el criterio de no-vacío vive en las celdas de extracción, no en las
  de validación. Para la Fase 2 extendida, añadir assert de `nodes > 0` en
  `export_validate_cell`/`strict_cell`.
- **`verify-local.sh` usa un binario stale** (`archctl/target/release/archctl`,
  legacy v1.45.0): el check de `CAPABILITIES.md` da falso positivo. El binario
  real vive en `~/.cargo/config.toml` → `CARGO_TARGET_DIR=/var/home/rubentxu/cargo-targets`.
  Fix pendiente en el script (fuera de este UAT).
- **Dataset self-dogfood archctl**: `datasets.sh` hace skip del clone (usa checkout
  local) pero `smoke-matrix.sh` espera el dir en `~/.cache/archctl-smoke/archctl`.
  Poblado manualmente con `rsync` (sin `.git`/`target`/`node_modules`). Pendiente:
  formalizar en `datasets.sh`.

## Estado de la matriz extendida (docs/sessions §3)

La columna "state-machine" de mockk (kotlin) se corrige: `state-machine --lang`
soporta rust/typescript/python/go — kotlin no. La celda real de mockk es
call-graph + sequence (2 celdas), ya cubierta por `smoke-matrix.sh`.

## Próximos pasos

1. **Fase 4 (workbench human loop)**: fases 1–4 y 6–9 automatizables con
   `e2e/human_loop_sandbox.sh`; fase 5 (navegador host, zoom C4/action
   palette) requiere `--network host` + humano. Fara CUA caído — requiere
   arrancar `llama.cpp` HTTP y la ruta del CLI `llm` del usuario.
2. Run completo 11 datasets con métricas wall/RSS → baseline formal
   `bench/reports/` (ADR-032).
3. Deuda de script: `verify-local.sh` (binario stale) + assert de no-vacío en
   celdas de validación del smoke.
