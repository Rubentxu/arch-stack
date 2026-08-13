# Spec — `archctl self` (CLI lifecycle management)

> **Ciclo:** `m73-distribution-stack-rework`
> **Estado:** Propuesto — 2026-08-10
> **ADR fuente:** [ADR-057](../adr/ADR-057-archctl-versioned-distribution.md), [ADR-058](../adr/ADR-058-self-update-github-releases.md)
> **Reemplaza:** parte de `archctl/src/stack.rs` (install/update/status en `~/.config/opencode/`)

## Objetivo

El binario `archctl` se gestiona a sí mismo: instalación, cambio de versión,
actualización desde GitHub Releases, desinstalación, y pin per-project. Sin
estado global mutable fuera de `~/.local/share/archctl/`. Sin tocar el repo
del usuario.

## Alcance

- Multi-version: N versiones de `archctl` instaladas simultáneamente.
- Self-update: descarga + verificación + migración + switch.
- Per-project pin: `.arch-version` walking hasta `$HOME`.
- Uninstall: reverso completo.
- Channels: stable / rc / nightly.

## Fuera de alcance (v1)

- Firma GPG (M76).
- Mirror S3 (M76).
- Homebrew / Scoop / Windows installer (M77).
- Plugin tap para skills de terceros (M76).

## Layout en disco

```
~/.local/share/archctl/
├── current                                 # symlink a installs/<active_version>/
├── installs/
│   ├── v1.32.0/
│   │   ├── archctl                         # binario release
│   │   ├── archctl-shim                    # symlink al binario (legacy)
│   │   └── assets-stack/                   # skills/agents/plugins embebidos (rust-embed extract)
│   ├── v1.33.0/
│   │   ├── archctl
│   │   ├── archctl-shim
│   │   └── assets-stack/
│   └── v1.34.0-rc.1/                       # release candidates
├── cache/
│   ├── release-cache.json                  # cache de GitHub Releases API (TTL 1h)
│   └── downloads/v1.34.0/                  # staging area para downloads en progreso
└── state.json                              # versión activa, último update check, channels

/usr/local/bin/archctl                       # shim binario (≤ 8 KB)
   ↓ (execve) llama a ~/.local/share/archctl/current/archctl "$@"
```

El shim binario es **opcional** pero recomendado: evita que el usuario tenga
que añadir `~/.local/share/archctl/current/` al `$PATH`. Si el usuario no
quiere el shim global, puede usar `archctl-shim` localmente o el PATH
directo.

## Comandos

### `archctl self install [VERSION]`

Instala una versión. Si VERSION se omite, instala la última stable.

```
$ archctl self install           # latest stable
$ archctl self install 1.33.0    # versión específica
$ archctl self install --channel=nightly    # última nightly
$ archctl self install --no-shim --install-root=$HOME/.local/share/archctl   # custom root
```

Procedimiento:

1. Llama a `GET /repos/Rubentxu/arch-stack/releases/latest` (o `/tags/v<VERSION>`).
2. Descarga `archctl-<target-triple>.tar.gz` + `SHA256SUMS`.
3. Verifica sha256. Mismatch → abort con error claro.
4. Extrae a `~/.local/share/archctl/installs/v<VERSION>/`.
5. Ejecuta `~/.local/share/archctl/installs/v<VERSION>/archctl --version`
   para sanity check (exit 0 + versión correcta).
6. Si VERSION == latest stable y no hay `current` → crea symlink.
7. Si `--shim` (default en Linux/macOS) → copia shim a `/usr/local/bin/archctl`
   (o `$HOME/.local/bin/` si no hay permisos para `/usr/local/bin/`).

Exit codes:
- 0: éxito
- 1: error de red
- 2: SHA256 mismatch
- 3: versión no encontrada en GitHub Releases
- 4: fallo de extracción
- 5: fallo de permisos (no se pudo escribir a `/usr/local/bin/`)

### `archctl self list`

```
$ archctl self list
  v1.32.0    2026-08-10    installed
  v1.33.0    2026-08-10    installed, active
  v1.34.0-rc.1  2026-08-09  available (GitHub release)
```

Subflags:
- `--installed` solo muestra las locales.
- `--available` solo muestra las remotas (GitHub Releases).
- `--channel=stable|rc|nightly` filtra.

### `archctl self use VERSION`

Cambia el symlink `current` → `installs/v<VERSION>/`.

```
$ archctl self use 1.32.0
Switched archctl to v1.32.0.
$ archctl --version
archctl 1.32.0
```

Si VERSION no está instalada, ofrece: `not installed. Run archctl self install 1.32.0 first? [y/N]`.

### `archctl self update [FLAGS]`

```
$ archctl self update                  # update a latest stable
$ archctl self update --to 1.33.0     # update a versión específica
$ archctl self update --check         # dry-run, exit 0/1 según haya update
$ archctl self update --no-migrate    # skip migration scripts (CI override)
$ archctl self update --channel=nightly
```

Procedimiento:

1. Resuelve versión target (latest stable / rc / nightly / `--to`).
2. Si target == active version → no-op, exit 0.
3. Descarga target release + SHA256SUMS.
4. Verifica sha256.
5. Si `migration-manifest.json` presente en el release → ejecuta scripts de
   migración desde la active version a target. Si falla → abort + rollback
   symlink (no se cambia `current`).
6. Extrae target a `installs/v<target>/`.
7. Sanity check: ejecuta `archctl --version` en la nueva versión, verifica
   que reporta la versión correcta.
8. Cambia symlink `current` → `installs/v<target>/`.

Exit codes:
- 0: éxito o no-update (con `--check`)
- 1: error de red
- 2: SHA256 mismatch
- 5: fallo de migración (rollback aplicado, current NO cambió)

### `archctl self uninstall [FLAGS]`

```
$ archctl self uninstall                # elimina el active version (binario + assets)
$ archctl self uninstall --version=1.32.0  # elimina versión específica
$ archctl self uninstall --purge         # elimina ~/.local/share/archctl/ completo
$ archctl self uninstall --keep-shim     # no elimina /usr/local/bin/archctl shim
```

Si `--purge` y quedan otras versiones → confirmación interactiva
(`Purge removes ALL installed archctl versions (N found). Continue? [y/N]`).

Si `--purge` y solo hay 1 versión → purga directa.

Exit codes:
- 0: éxito
- 5: fallo de permisos

### `.arch-version` per-project pin

Formato (idéntico a `.tool-versions` de asdf-vm):

```
# .arch-version
1.33.0
```

Walking: desde el cwd, subir hasta encontrar `.arch-version` o llegar a
`$HOME`. Si se llega a `/` sin encontrar, no hay pin (usa global).

Override con env var `ARCHCTL_VERSION` o flag `--archctl-version X.Y.Z`.

Precedencia (mayor a menor):

1. Flag `--archctl-version X.Y.Z`
2. Env var `ARCHCTL_VERSION`
3. `.arch-version` en el proyecto
4. Symlink `~/.local/share/archctl/current`
5. Fallback a `archctl` en `$PATH`

Si el pin requiere una versión no instalada → el shim auto-ejecuta
`archctl self install <version>` (solo si `--auto-install` está activo; off
por defecto para evitar surprise installs).

## State file (`~/.local/share/archctl/state.json`)

```json
{
  "active_version": "1.33.0",
  "channels": {
    "stable": { "last_check": "2026-08-10T17:00:00Z", "latest": "1.33.0" },
    "nightly": { "last_check": "2026-08-10T17:00:00Z", "latest": "nightly-2026-08-10" }
  },
  "shim_path": "/usr/local/bin/archctl",
  "install_root": "/home/user/.local/share/archctl",
  "taps": [
    "https://raw.githubusercontent.com/Rubentxu/arch-stack/main/taps/official.json"
  ]
}
```

`archctl self update --check` actualiza `channels.<chan>.last_check` para
rate-limiting (no spammear GitHub API).

## Shimming

El shim en `/usr/local/bin/archctl` es un script de 8 líneas que ejecuta
`~/.local/share/archctl/current/archctl "$@"`. Si `/usr/local/bin/` no es
escribible (Linux con sandbox), fallback a `~/.local/bin/archctl` (si está
en `$PATH`).

```bash
#!/usr/bin/env bash
# /usr/local/bin/archctl — shim que delega al binario activo.
ARCHCTL_HOME="${ARCHCTL_HOME:-$HOME/.local/share/archctl}"
if [ -L "$ARCHCTL_HOME/current" ]; then
  exec "$ARCHCTL_HOME/current/archctl" "$@"
else
  echo "archctl: no active version installed. Run 'archctl self install' first." >&2
  exit 127
fi
```

Verificación de symlink chain:
- `/usr/local/bin/archctl` → script bash (8 líneas).
- `~/.local/share/archctl/current` → `installs/v1.33.0/`.
- `~/.local/share/archctl/installs/v1.33.0/archctl` → binario real (release artifact).

## Crates Rust necesarios (M73)

| Crate | Uso | Notas |
|---|---|---|
| `reqwest` (o `ureq`) | HTTP client para GitHub Releases API | ADR-011 prohíbe red por defecto; aquí es opt-in (user explicitly runs `update`). |
| `sha2` | SHA256 verify | `sha2 = "0.10"` puro-Rust; no requiere OpenSSL. |
| `flate2` + `tar` | Extract `.tar.gz` | Puro-Rust, no requiere gzip CLI. |
| `semver` | Version comparison | `semver = "1"` para `>`, `<`, `>=`. |
| `serde` + `serde_json` | JSON manifest | Ya en deps. |
| `directories` | XDG paths | Ya en deps (vía `xdg`). |

Total: ~3 crates nuevas. Aceptable.

## Verification (e2e + manual)

### E2E (`e2e/install_e2e.sh` extendido)

```bash
# 1. Install latest stable
"$ARCHCTL_BIN" self install --install-root="$E2E_ROOT/installs"
[ -x "$E2E_ROOT/installs/v"*"/archctl" ] || FAIL "install"

# 2. Multi-version
"$ARCHCTL_BIN" self install 1.32.0 --install-root="$E2E_ROOT/installs"
[ -d "$E2E_ROOT/installs/v1.32.0" ] || FAIL "1.32.0 install"

# 3. Use specific version
"$ARCHCTL_BIN" self use 1.32.0 --install-root="$E2E_ROOT/installs"
INSTALLED=$("$E2E_ROOT/installs/current/archctl" --version)
[ "$INSTALLED" = "archctl 1.32.0" ] || FAIL "use 1.32.0"

# 4. Switch back
"$ARCHCTL_BIN" self use 1.33.0 --install-root="$E2E_ROOT/installs"

# 5. Uninstall
"$ARCHCTL_BIN" self uninstall --version=1.32.0 --install-root="$E2E_ROOT/installs"
[ ! -d "$E2E_ROOT/installs/v1.32.0" ] || FAIL "uninstall 1.32.0"

# 6. Update (mocked; sin red en CI)
"$ARCHCTL_BIN" self update --check --install-root="$E2E_ROOT/installs" || true  # exit 0 o 1, ambos válidos
```

### Manual (Human-in-the-loop)

`docs/HUMAN_LOOP_TEST.md` extendido con:

1. `archctl self install` en una VM limpia, verificar `~/.local/share/archctl/installs/v1.33.0/archctl --version`.
2. `archctl self update` en la misma VM con un release pre-publishado, verificar migración + switch.
3. `archctl self use 1.32.0` y rollback manual a 1.33.0 — verificar que los assets embebidos son diferentes (skills/agents difieren entre versiones).
4. `archctl self uninstall --purge` y verificar que el system queda limpio (`~/.local/share/archctl/` borrado, `/usr/local/bin/archctl` shim queda con `--keep-shim` o se va sin él).

## Riesgos

| Riesgo | Mitigación |
|---|---|
| Rate limit GitHub API | Cache `release-cache.json` con TTL 1h. Auto-retry con backoff. |
| MITM en descarga | HTTPS + SHA256SUMS (M76: firma GPG). |
| Migration script bug → brick | Rollback automático del symlink antes de aplicar migración. Migration scripts son reversibles (idempotent). |
| Disk full en update | Pre-check de espacio disponible (release size × 2). |
| Path collision (`/usr/local/bin/` ocupado) | Fallback a `~/.local/bin/`, warning al usuario. |

## Referencias

- ADR-057 (versioned distribution)
- ADR-058 (self-update via GitHub Releases)
- ADR-042 (IDE adapter abstraction)
- asdf-vm `.tool-versions` format: https://asdf-vm.com/manage/configuration.html#tool-versions
- GitHub Releases API: https://docs.github.com/en/rest/releases
- `archctl/src/stack.rs` (legacy; se mantiene como alias deprecated)
- `scripts/install.sh` (legacy; reemplazado por `archctl self install`)
