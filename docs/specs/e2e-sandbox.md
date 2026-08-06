# Spec — Sandbox E2E (`bench/sandbox-e2e.sh`)

> **Referencia:** [ADR-034](adr/ADR-034-e2e-coverage-expansion.md) §4
> **Estado:** Propuesta — 2026-08-06
> **Milestone:** M29 (E2E coverage expansion)

## Objetivo

Ejecutar el vertical C4 completo **dentro del sandbox Quadlet** de forma
reproducible y verificable — reemplaza el one-off manual de 2026-08-06 (que
validó el vertical una vez pero no quedó versionado).

El sandbox demuestra la **reproducibilidad del entorno**: el mismo archctl,
el mismo ubuntu:24.04, el mismo toolchain — en cualquier máquina con podman.

## Alcance

1. Build de la imagen (`bench/build.sh` o `podman build` directo).
2. Compilar `archctl` DENTRO del container (glibc nativa ubuntu:24.04 — el
   binario host NO es portable al container, ver ADR-033/lección 2026-08-06).
3. Vertical C4 completo contra un dataset real (default: `tokio-rs/axum`):
   - `code c4-discover --apply` → ≥1 container
   - `evidence list --status drafted` → ≥1
   - `evidence accept` (todos los drafted)
   - `diagram export container:*` → bundle
   - `diagram validate <bundle>` → exit 0
4. Veredicto JSON (`{"verdict":"PASS"|"FAIL","checks":[...]}`).

## Fuera de alcance

- Benchmarks de rendimiento (eso es `bench/run-bench.sh`).
- Múltiples datasets (la suite es 1 dataset + dogfood; el bench cubre los 11).
- CI en GitHub Actions (podman no disponible en runners ubuntu estándar;
  gate manual vía `verify-local.sh --full`).

## Prerrequisitos

- `podman` ≥ 4 (rootless).
- Red para clonar el dataset si no está cacheado
  (`~/.cache/archctl-smoke/<name>`).
- Imagen base descargable (`ubuntu:24.04`).

## Procedimiento

```bash
bench/sandbox-e2e.sh [--dataset tokio-rs/axum] [--keep-container]

# 1. Build imagen (cacheada)
podman build -f bench/Containerfile -t archctl-bench:latest bench/

# 2. Compilar archctl DENTRO del container (montando el source)
podman run --rm --security-opt label=disable \
  -v "$REPO:/src" -v "$HOME/.cargo:/root/.cargo" \
  archctl-bench:latest bash -c 'cd /src/archctl && cargo build --release'

# 3. Vertical C4 con asserts (script interno heredoc)
podman run --rm --security-opt label=disable \
  -v "$REPO/archctl/target/release/archctl:/usr/local/bin/archctl" \
  -v "$CACHE:/datasets" -v "$XDG_DATA:/xdg/data" \
  -e XDG_DATA_HOME=/xdg/data -e XDG_CONFIG_HOME=/xdg/config \
  archctl-bench:latest bash -e <<'VERTICAL'
    archctl code c4-discover --cwd /datasets/<name> --apply | grep -q "Applied: [1-9]"
    N=$(archctl evidence list --cwd /datasets/<name> --status drafted --json | jq 'length')
    [ "$N" -ge 1 ]
    for id in $(archctl evidence list ... | jq -r '.[].id'); do
      archctl evidence accept --id "$id" --cwd /datasets/<name>
    done
    archctl diagram export container:* --cwd /datasets/<name> --output /tmp/b
    archctl diagram validate /tmp/b --cwd /datasets/<name>
VERTICAL

# 4. Emitir veredicto JSON
echo '{"verdict":"PASS","checks":{"discover":true,"accept":true,"export":true,"validate":true}}'
```

## Criterios de aceptación

| # | Criterio | Método |
|---|---|---|
| 1 | Imagen builda sin error | exit 0 podman build |
| 2 | archctl compila in-container | cargo build exit 0 |
| 3 | `c4-discover --apply` detecta ≥1 container | grep "Applied: [1-9]" |
| 4 | Evidencias drafted ≥1 | jq length ≥ 1 |
| 5 | Todas aceptadas | accept exit 0 por id |
| 6 | Export genera bundle | exit 0 + files existen |
| 7 | Validate exit 0 | exit 0 |
| 8 | Veredicto JSON válido | jq -e .verdict |

## Entregables

1. `bench/sandbox-e2e.sh` (~100 líneas bash, idempotente).
2. Veredicto JSON a stdout (para tooling/CI futura).
3. Integración en `verify-local.sh --full` (condicional a podman presente).

## Nota de compatibilidad (2026-08-06)

El binario compilado en el host Fedora NO corre en ubuntu:24.04
(`GLIBCXX_3.4.35` missing). Por eso la suite compila DENTRO del container —
nunca monta el binario host. El Containerfile ya incluye `build-essential`
+ `libssl-dev` + `pkg-config` (PR #57) para soportar el build in-container.

## Referencias

- ADR-032 (metodología bench), ADR-033 (view), ADR-034 (decisión), M29
