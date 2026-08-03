# ADR-025 — CI post-merge + toolchain fijada + verificación local

**Estado:** Aceptado
**Fecha:** 3 de agosto de 2026
**Aplica a:** `arch-stack` (CI de `main`), `archctl` (toolchain Rust), `archview` (gates web)
**Complementa:** ADR-019 (presupuesto de performance), ADR-024 (ejecutor local-first de workflows)
**Sustituye en la práctica:** el disparador `pull_request` del CI de ADR-019 §1 y la política flotante de toolchain.

> **Decisión clave:** el CI remoto pasa a **detección post-merge** (solo `push` a `main`).
> La **prevención** ocurre localmente: `scripts/verify-local.sh` (pre-push, tier barato)
> y `--full` antes de mergear. El rol del CI remoto ya no es bloquear PRs, sino
> **evidencia de detección y punto de rollback** tras cada merge a `main`.

## Contexto

En julio–agosto de 2026 el CI de `main` se detectó **rojo** (runs `30798408159`, `30799762971`)
bajo Rust flotante 1.97 por un lint `for_kv_map` en `archctl/src/filesystem.rs:337`
(`for (file_path, _) in files.iter()`); local con 1.96 pasaba. Tres defectos de raíz:

1. **Sin pin de toolchain**: cada job instalaba `stable` flotante (`rustup toolchain install stable`),
   así que el lint aparecía solo cuando `stable` avanzaba. No había reproducibilidad.
2. **Lint sin corregir**: `filesystem.rs:337` usaba `iter()` ignorando el valor; clippy
   `for_kv_map` lo señala en 1.97. La solución correcta es `.keys()`, no suprimir el lint.
3. **Trigger `pull_request`**: contradecía ADR-024 (integración local-first) y la preferencia
   del usuario de ejecutar CI remoto **solo al mergear en `main`**.

Además, el usuario prefirió explícitamente (sesión 2026-08-03) que el CI remoto se active
únicamente cuando un cambio se fusione en `main`, abordando los cambios uno por uno.

## Decisión

### 1. CI remoto = post-merge únicamente (detección)

`.github/workflows/ci.yml` dispara **solo** con:

```yaml
on:
  push:
    branches: [main]
```

Sin `pull_request`, `workflow_dispatch`, `schedule` ni triggers por rama. Abrir un PR
**no** lanza CI remoto; el primer run remoto ocurre al mergear a `main`.

Rol del CI remoto: **detección y evidencia de rollback**, no prevención. La prevención
es local (ver §3).

### 2. Jobs post-merge preservados

Cada run de `main` ejecuta los **cuatro grupos de gates** (ADR-019 sin cambios de presupuesto):

| Job | Contenido |
|---|---|
| `rust` | build, test, clippy `-D warnings`, fmt `--check`, doctor (scope `code`) |
| `bench-smoke` | `cargo bench --bench export_pipeline -- --quick` (humo determinista) |
| `bench-compare` | regresión >10% vs **SHA previo de `main`** (`github.event.before`) |
| `web` | `pnpm test` + `pnpm build` + bundle JS gzipped ≤ 2MB |

### 3. Prevención local en niveles

- `scripts/verify-local.sh` (tier barato, default): `cargo test`, `cargo clippy -D warnings`,
  `cargo fmt --check`, doctor. Lo ejecuta el hook `.githooks/pre-push` (vía
  `core.hooksPath`, instalado por `scripts/install-hooks.sh`).
- `scripts/verify-local.sh --full` (pre-merge): añade web test/build/bundle-cap,
  bench smoke y comparación ADR-019 contra `origin/main`.

El script nunca muta fuente versionada: cargo/pnpm escriben solo en `target/` y `dist/`
(gitignored).

### 4. Semántica de baseline «previous-main»

`scripts/bench-compare.sh <baseline-ref>` compara el HEAD mergeado contra el ref pasado:

- CI pasa `"${{ github.event.before }}"` (SHA previo de `main`). **No** `origin/main`
  porque tras el merge `origin/main == HEAD mergeado` y la comparación sería vacía.
- Local `--full` pasa `origin/main` (la rama aún no está mergeada).
- Guardas: SHA todo-ceros (`0000…`, primer push / reescritura de historia) y refs
  ausentes/inválidos/inalcanzables → **exit 2** con error de baseline claro. Un baseline
  inválido **no** puede pasar.

### 5. Toolchain exacta vs MSRV (distinción clave)

- **Toolchain de CI/desarrollo**: `rust-toolchain.toml` en la raíz, `channel = "1.97.1"`,
  profile `minimal`, componentes `rustfmt` + `clippy`. Es la **única versión exacta** en la
  que corren todos los gates. Se eliminaron los pasos flotantes `rustup toolchain install stable`.
- **MSRV (consumidores)**: `archctl/Cargo.toml` declara `rust-version = "1.91"`.
  La spec propuso `1.85`, pero la validación empírica en apply mostró que el árbol de
  dependencias actual exige rustc **1.91** (`cargo-platform@0.3.3`; `idna_adapter`, `ignore`,
  `time` exigen 1.86–1.88). Declarar `1.85` sería falso; el plan de riesgos del proposal
  autorizaba «raise in its own commit». CI corre **por encima** del MSRV (1.97.1),
  deliberadamente.

### 6. Protección de `main` (branch protection)

- PR obligatorio para llegar a `main`; aprobaciones requeridas = 0.
- Bloqueo de push directo, force-push y borrado.
- **Cero** status checks requeridos (el CI es post-merge, no puede bloquear el PR).
- Aplicada vía `gh api` con credenciales ya autenticadas; sin secretos nuevos.

## Trade-offs

| Aspecto | Detección post-merge | Prevención local (antes: PR CI) |
|---|---|---|
| Coste por push de rama | 0 runs remotos | PR CI corría en cada push de rama |
| Regresión detectada | **después** del merge (rollback necesario) | **antes** del merge |
| Riesgo principal | un fallo post-merge marca `main` como insano | el desarrollador debe ejecutar `--full` para cubrir benchmarks/web |
| Evidencia | runs de `main` (históricos, auditables) | salida local, no auditada centralmente |

Mitigación del riesgo de detección tardía: `verify-local.sh --full` es el gate preventivo
de benchmarks/web; el pre-push cubre los gates Rust baratos en cada push. La contención
de fallo (spec) es: si un gate post-merge falla, `main` queda insano y **ningún merge
posterior** procede hasta rollback o corrección verificada localmente con `--full`.

## Consecuencias

### Positivas

- Reproducibilidad total de CI: 1.97.1 fija, lint reparado, doctor con ruta correcta.
- Respeto a ADR-024 y a la preferencia del usuario: sin CI de ramas, sin PR CI.
- ADR-019 intacto: presupuestos y umbral >10% / 2MB se mantienen; solo cambia el disparador.
- Protección de `main` sin checks remotos: el release agent puede mergear vía PR.

### Negativas

- Un fallo post-merge requiere rollback o corrección antes del siguiente merge.
- El desarrollador debe acordarse de `--full` para benchmarks antes de mergear.
- `github.event.before` es `0000…` en el primer push a un repo vacío o en reescritura
  de historia → el gate falla explícitamente (exit 2), nunca pasa en silencio.

## Cómo revertir

| Decisión | Reversión |
|---|---|
| Trigger `push: [main]` | Restaurar `pull_request:` en `ci.yml` |
| Toolchain 1.97.1 | Borrar `rust-toolchain.toml` y volver a `rustup toolchain install stable` |
| MSRV 1.91 | Ajustar `rust-version` al piso real del árbol en ese momento |
| Baseline `github.event.before` | Volver a `origin/main` en el job `bench-compare` |
| Pre-push local | Borrar `.githooks/pre-push` |
| Protección de `main` | `gh api -X DELETE repos/Rubentxu/arch-stack/branches/main/protection` |

Sin migraciones de datos. El presupuesto ADR-019 no cambia.

## Referencias

- ADR-019 — presupuesto de performance (gates >10% y 2MB).
- ADR-024 — workflowctl local-first; CI remoto como evidencia.
- Proposal `sddk/ci-main-gates` (v2) y spec `sddk/ci-main-gates` (obs 5599/5600).
- Scripts: `scripts/bench-compare.sh`, `scripts/verify-local.sh`, `scripts/test-ci-gates.sh`,
  `.githooks/pre-push`, `scripts/install-hooks.sh`.
