# ADR-058 — Self-update via GitHub Releases (binarios pre-compilados)

> **Ciclo:** `m73-distribution-stack-rework` (planning)
> **Estado:** Propuesto — 2026-08-10
> **Complementa:** ADR-057 (versioned distribution)
> **Aplica a:** `archctl self update` / `archctl self install`

## Contexto

ADR-040 introduce el concepto de multi-version + self-update pero no
especifica el **canal de distribución**. Las opciones evaluadas:

| Canal | Pros | Contras |
|---|---|---|
| GitHub Releases | Ya publica binarios (asume GH Actions workflow); URL estable; API pública; sha256 fácil. | Acopla a GitHub; rate limits en API pública (60/h sin token). |
| crates.io | Descarga vía `cargo install` (familiar para Rust devs). | `archctl` no es una lib Rust pública; requiere `cargo` instalado; metadata `Cargo.toml` no es trivial fuera de un crate. |
| Docker Hub / GHCR | Imagen inmutable + multi-arch (linux/amd64, linux/arm64). | 200MB+ por tag; requiere Docker para correr; contradice "single binary" del ADR-038. |
| Self-hosted S3 | Control total. | Coste operacional; no discovery orgánico. |

GitHub Releases gana porque **ya es la fuente actual** (M70 publica v1.31.0
ahí; M71 v1.32.0; M72 v1.33.0). Adoptar un canal paralelo introduce
duplicación operacional sin beneficio.

## Decisión

`archctl` se distribuye exclusivamente vía **GitHub Releases**. Cada
release produce:

```
archctl-x86_64-unknown-linux-gnu.tar.gz      # Linux x86_64 (target Tier 1)
archctl-aarch64-unknown-linux-gnu.tar.gz    # Linux ARM64 (Graviton, M-series en Asahi)
archctl-x86_64-apple-darwin.tar.gz          # macOS Intel
archctl-aarch64-apple-darwin.tar.gz        # macOS Apple Silicon
SHA256SUMS                                  # sha256 por archivo
migration-manifest.json                     # schema migrations desde vN-1
```

### URL pattern

```
https://github.com/Rubentxu/arch-stack/releases/download/vX.Y.Z/<asset>
https://api.github.com/repos/Rubentxu/arch-stack/releases/latest
https://api.github.com/repos/Rubentxu/arch-stack/releases?per_page=20
```

### Release manifest (en cada tag)

`migration-manifest.json` documenta schema migrations desde la versión
anterior:

```json
{
  "from_version": "1.32.0",
  "to_version": "1.33.0",
  "migrations": [
    {
      "id": "M72-workspace-error-is-directory",
      "description": "WorkspaceError gained IsDirectory variant",
      "applies_to": ["workspace_state"],
      "migration_script": "migrate-1.32-to-1.33.py",
      "rollback_supported": true
    }
  ]
}
```

El `migration_script` se descarga y ejecuta **antes** de cambiar el symlink
`current`. Si falla → `archctl self update` aborta con exit code != 0 y el
symlink NO cambia (rollback automático). `archctl self update --no-migrate`
es el escape hatch para CI / scripts que prefieren rollback manual.

### Verificación de integridad

```
archctl self update
  1. GET /repos/.../releases/latest      → JSON con tag_name, assets[]
  2. Para cada asset_target (linux-x86_64):
     - Download .tar.gz
     - Download SHA256SUMS
     - Verifica sha256; mismatch → abort con error claro
  3. Extract a ~/.local/share/archctl/installs/v<new>/
  4. Run migration scripts si aplica
  5. Cambia symlink current → installs/v<new>/
  6. Verifica archctl --version reporta la nueva versión
```

### Versioning channels

```
archctl self update                  # stable (default)
archctl self update --channel=stable
archctl self update --channel=rc     # release candidates (vX.Y.Z-rc.N)
archctl self update --channel=nightly # pre-releases etiquetados nightly-YYYY-MM-DD
```

`nightly` se construye desde `main` en cada push, con tag `nightly-2026-08-10`.
Útil para CI reproducibilidad (test contra nightly antes de promover a stable).

### Firmas

- **v1 (M73)**: SHA256SUMS sin firma. Adecuado para releases de un equipo
  pequeño; MITM risk es bajo (HTTPS + GitHub es la fuente).
- **v2 (M76)**: firma GPG con key pública commited en el repo
  (`MAINTAINERS-GPG-KEY.asc`). `archctl self update` verifica
  `SHA256SUMS.sig` antes de aplicar. SHA256-only queda como fallback para
  mirrors sin key.

## Consecuencias

### Positivas

- **CI reproducible**: `archctl self install --channel=nightly-2026-08-10`
  pin a una fecha exacta.
- **Downgrade trivial**: si v1.34.0 rompe, `archctl self use 1.33.0` revierte
  en <1s (sin re-descarga si el binario ya está).
- **Auditoría**: GitHub Releases tiene timestamp + signer + asset hashes;
  cumple requirements de supply-chain transparency para enterprise.

### Negativas

- **GitHub API rate limits**: 60 req/h sin token. Mitigación: cache del JSON
  del release en `~/.cache/archctl/release-cache.json` con TTL de 1h.
- **Acoplamiento a GitHub**: si GitHub cae, `archctl self update` falla. El
  usuario puede descargar manualmente de un mirror (M76) o vía `cargo install
  --git` (Rust devs).
- **CI cost**: GitHub Actions runners para 4 targets × ~3 min = ~12 min por
  release. Acceptable para releases semanales. Si pasa a diario, evaluar
  build matrix más agresiva.

## Implementation Plan (parte de M73)

- PR #3: GitHub Releases API client (sin firma) + SHA256SUMS verify.
- PR #4: `migration-manifest.json` schema + auto-migrate on update.
- PR #5: channels (stable/rc/nightly) + version detection.
- M76 (futuro): GPG firma + mirror S3.

## Verificación

- E2E: `e2e/install_e2e.sh` extendido con `archctl self update --check`
  (debe retornar exit 0 si hay update, exit 0 también si no hay — el flag es
  dry-run).
- Mock server: tests unitarios con `wiremock-rs` o un `tiny_http` server
  local que sirva un fake GitHub Releases API.
- Downgrade: instalar v1.32.0 + v1.33.0, `archctl self use 1.32.0`,
  verificar que `archctl --version` reporta 1.32.0 sin re-descarga.

## Referencias

- GitHub Releases API: https://docs.github.com/en/rest/releases
- ADR-040 (versioned distribution)
- ADR-042 (IDE adapter abstraction)
- `archctl/.github/workflows/release.yml` (existente — emite los binarios)

## Changelog

- 2026-08-10 | proposed | ADR-058 self-update via GitHub Releases
