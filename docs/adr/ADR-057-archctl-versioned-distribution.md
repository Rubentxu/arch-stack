# ADR-057 — `archctl` como CLI versionado distribuible (asdf-inspired)

> **Ciclo:** `m73-distribution-stack-rework` (planning)
> **Estado:** Propuesto — 2026-08-10
> **Complementa:** ADR-033 (embedded workbench), ADR-038 (one product, five invariants), ADR-039 (anti-roadmap)
> **Inspira en:** asdf-vm plugin system (100% shell + git repos con `bin/list-all`, `bin/download`, `bin/install`)

## Contexto

`archctl` se construye y distribuye actualmente como un binario único por
release (`v1.32.0`, `v1.33.0`). El ciclo de distribución es:

```
git tag vX.Y.Z
git push --tags       # + GitHub Actions publica el binario en Releases
archctl stack install # el usuario copia skills/agents/plugin a su IDE
```

Lo que **funciona**:

- `archctl stack install/update/status` (M29, M33) ya existe y es idempotente.
- `assets-stack/` se embebe vía `rust-embed` en el binario (ADR-033) → un solo
  artifact distribuye binario + skills + agents + plugin como UNO.
- El manifest gate + doctor + verify-local cubren la calidad del release.

Lo que **NO funciona** (gaps verificados el 2026-08-10):

| Gap | Impacto |
|---|---|
| Sin multi-version | Solo se puede tener 1 versión instalada simultáneamente. `archctl view` requiere el binario corriendo; downgrade requiere reinstalar el binario manualmente. |
| Sin self-update | El usuario debe descargar manualmente de GitHub Releases. No hay forma de `archctl self update`. |
| Sin uninstall | No hay forma limpia de eliminar `archctl` + sus skills/agents. `rm` manual deja `~/.local/share/archctl/` huérfano. |
| Sin pinning per-project | No existe `.arch-version` equivalente a `.tool-versions` de asdf. Si un repo requiere `archctl v1.32.0` y otro `v1.33.0`, no hay mecanismo. |
| Sin tap/plugin | El "stack" (skills + agents + plugin) está embebido en el binario. No se puede distribuir una skill de terceros sin recompilar `archctl`. |
| Solo OpenCode/ZCode | `default_install_root()` está hardcodeado a `~/.config/opencode`. Claude Code usa `~/.claude/`, Codex usa `~/.codex/`. No hay abstracción para IDEs futuros. |
| Sin auto-install post-release | Cuando se publica `vX.Y.Z` el usuario debe enterarse por GitHub watch; no hay notification ni `archctl self update --check`. |

La pregunta estratégica: **¿qué partes del modelo asdf-vm移植amos a `archctl`?**

asdf-vm (100% bash + git plugins) es ideal para herramientas que se compilan
desde fuente (ruby, node, elixir). `archctl` es un binario pre-compilado en
Rust, así que移植amos la **filosofía** (multi-version, tap, install/update/
uninstall, per-project pin) sin la letra completa de asdf.

## Decisión

`archctl stack` se transforma en `archctl self` + `archctl ide` con cuatro
bounded contexts nuevos:

```
archctl self   # ciclo de vida del binario (versión, update, uninstall, pin)
archctl ide    # ciclo de vida del IDE-binding (qué IDEs se instalan, dónde)
archctl stack  # queda como alias deprecated de `archctl self install` + `archctl ide install` (M73.6)
archctl plugin # NUEVO — tap model para skills/agents de terceros (M73.5)
```

### 1. Multi-version: `~/.local/share/archctl/installs/<version>/`

```
/usr/local/bin/archctl              # shim binario (≤ 8 KB, solo resuelve la versión activa)
/usr/local/bin/arch                 # shim del workspace (sigue el mismo patrón)
/usr/local/bin/archctl-shim         # alias interno

~/.local/share/archctl/
├── current                         # symlink a installs/active/vX.Y.Z/  (resuelve $archctl --version)
├── installs/
│   ├── v1.32.0/archctl             # binario release
│   ├── v1.32.0/archview/...        # workbench embedded
│   ├── v1.33.0/archctl
│   └── v1.33.0/archview/...
├── assets-stack/v1.32.0/           # skills/agents/plugin versionados con el binario
└── assets-stack/v1.33.0/

<project>/.arch-version              # pin per-project (overrides ~/.config/archctl/version)
```

**Comando**: `archctl self install <version>` descarga el binario + assets de
GitHub Releases, lo coloca en `installs/v<version>/`. `archctl self use <version>`
cambia el symlink `current`.

### 2. Per-project pin: `.arch-version`

```
# .arch-version (formato idéntico a .tool-versions)
1.33.0
```

`archctl` lee el `.arch-version` del directorio de trabajo (walking up hasta
`$HOME` o `stop=` configurable). Si está presente, usa esa versión **incluso
si está en otro path** (descarga on-demand si falta). Override con env var
`$ARCHCTL_VERSION` o flag `--archctl-version X.Y.Z`.

### 3. Self-update: GitHub Releases API

```
archctl self update                # update a la última estable
archctl self update --to 1.33.0    # update a versión específica
archctl self update --check       # check sin aplicar (CI / cron)
archctl self update --channel=nightly   # suscripción a pre-releases
```

Mecánica:

1. `GET https://api.github.com/repos/Rubentxu/arch-stack/releases/latest`
2. Compara semver con `current → installs/<current>/archctl --version`.
3. Descarga `archctl-x86_64-unknown-linux-gnu.tar.gz` (target triple + sha256).
4. Verifica `SHA256SUMS` (firma GPG opcional en stable).
5. Coloca en `installs/<new_version>/archctl`, ejecuta `archctl self migrate` si hay schema migrations, cambia `current`.

`archctl self uninstall [--purge]` elimina el binario activo + opcionalmente
`~/.local/share/archctl/` completo.

### 4. Distribución: GitHub Releases (binarios pre-compilados) + tap para plugins

**Distribución del core** (binario + skills + agents + plugin): GitHub Releases.

```
arch-stack/releases/download/v1.33.0/archctl-x86_64-unknown-linux-gnu.tar.gz
arch-stack/releases/download/v1.33.0/archctl-aarch64-apple-darwin.tar.gz
arch-stack/releases/download/v1.33.0/archctl-x86_64-apple-darwin.tar.gz
arch-stack/releases/download/v1.33.0/SHA256SUMS
arch-stack/releases/download/v1.33.0/SHA256SUMS.sig
arch-stack/releases/download/v1.33.0/migration-manifest.json  # schema migrations desde vN-1
```

**Distribución de plugins** (skills de terceros, M73.5): tap model.

```
~/.config/archctl/
├── config.toml                          # versión global, tap URLs
├── taps/
│   ├── archctl-official.json            # tap oficial (este repo)
│   └── community/
│       └── <org>/<tap>.json             # taps comunitarios
└── plugins/
    └── <author>/<plugin>@<version>/
        ├── SKILL.md                     # o plugin.toml para plugins multi-file
        └── bin/list-all                 # opcional, si el plugin tiene versiones
```

Plugin = directorio versionado con `SKILL.md` + opcional `plugin.toml`.
Instalación: `archctl plugin install <author>/<plugin>@<version>`. El plugin
se copia a `~/.config/archctl/plugins/<author>/<plugin>/` y se referencia
desde `.arch-stack/plugins` en cada IDE donde aplica (ver ADR-042).

### 5. Comandos de alto nivel (user-facing)

```
archctl self install [version]      # instala versión (default: latest stable)
archctl self list                    # lista versiones instaladas + disponibles
archctl self use <version>           # cambia symlink current
archctl self update [version]        # self-update
archctl self uninstall [--purge]     # elimina

archctl ide install <ide>            # install stack en <ide>
archctl ide list [--installed]       # lista IDEs soportados + instalados
archctl ide doctor <ide>             # diagnóstico específico del IDE

archctl plugin install <spec>        # install plugin (archctl-official/<name>@1.0.0)
archctl plugin list [--installed]    # lista plugins disponibles + instalados
archctl plugin update <spec>|--all   # update plugin(s)
archctl plugin remove <spec>         # remove plugin
```

### Decisiones explícitas

- **Shim binario en `/usr/local/bin/`**: portable, no requiere `$PATH` setup.
  Si el usuario no quiere `/usr/local/`, puede usar `~/.local/bin/` (con
  `$PATH` ajuste).
- **No compilamos desde source en `archctl self install`**: a diferencia de
  asdf (que compila ruby/node), `archctl` es Rust pre-compilado. El "tap"
  solo distribuye assets (skills/agents/plugins), no código fuente.
- **Sin firma GPG obligatoria en v1**: SHA256SUMS es suficiente. Firma GPG
  queda como v2 (M76).
- **Sin Homebrew formula en v1**: Homebrew formula es M77. Mantiene el
  alcance acotado y evita el dependency review de Homebrew.
- **`.arch-version` walking**: solo up-to-`$HOME`. No walking a `/` para evitar
  surprises en monorepos multi-tenant.

## Consecuencias

### Positivas

- Downgrade trivial: `archctl self use 1.32.0` cambia el symlink sin descargar
  (la v1.32.0 ya está en `installs/`). Si falta, `archctl self install 1.32.0`
  la trae.
- Release reproducible: `archctl self update` aplica schema migrations
  desde `migration-manifest.json` (mismo formato que `MIGRATIONS.md` actual,
  solo cambia la fuente del manifest).
- Extensibilidad: un tercero puede publicar `archctl-plugin-<name>` en su
  repo + un tap JSON de un commit, y los usuarios pueden `archctl plugin
  install org/plugin@1.0.0` sin esperar release oficial de arch-stack.
- Testing reproducible: `archctl self install --channel=nightly` permite
  pre-releases en CI sin contaminar el global.

### Negativas

- Complejidad operacional: hay que mantener el shim binario + el resolver
  + el manifest de migrations + la infra de taps. **Mitigation**: scope
  acotado a M73 (multi-version + self-update + uninstall) + M75 (IDE
  adapters) + M76 (plugin tap). Tres milestones, no uno.
- Riesgo de supply chain: distribuir binarios pre-compilados requiere
  hardening (SHA256 + firma + reproducible builds en CI). **Mitigation**:
  SHA256SUMS en M73, reproducible builds + firma GPG en M76.
- Windows / macOS: el shim asume Unix shell. **Mitigation**: scope v1 =
  Linux + macOS. Windows queda para M77+ (ver ADR-039 anti-roadmap).

## Implementation Plan

### M73 — `archctl self` (multi-version, self-update, uninstall)

- PR #1: shim binario + `~/.local/share/archctl/installs/<version>/` layout.
- PR #2: `archctl self install/list/use/uninstall` (sin red — usa `~/.cache/`).
- PR #3: `archctl self update` con GitHub Releases API + SHA256SUMS.
- PR #4: `.arch-version` per-project pin + walking.
- PR #5: `migration-manifest.json` schema migrations on update.

### M75 — `archctl ide` (multi-IDE abstraction)

Ver ADR-042.

### M76 — `archctl plugin` (tap model para skills de terceros)

Scope: tap oficial + 1 tap comunitario de ejemplo + verificación de
firma SHA256.

### Out of scope (v1)

- Firma GPG de releases (M76+).
- Homebrew formula (M77).
- Windows installer (M77+, ADR-039 anti-roadmap).
- Plugin SDK en Rust (M77+; v1 solo acepta plugins declarativos).

## Verificación

- E2E: `e2e/install_e2e.sh` se extiende con `archctl self install --version
  pinned-from-test` + `archctl self update --check` + `archctl self
  uninstall`.
- Multi-version: instalar v1.32.0 + v1.33.0 en worktree, alternar con
  `archctl self use`, verificar que `archctl --version` cambia.
- Pin per-project: en un repo con `.arch-version=1.32.0`, ejecutar el
  binario desde otro path y verificar que arranca con v1.32.0.

## Referencias

- asdf-vm plugins spec: https://asdf-vm.com/plugins/create.html
- asdf-vm core commands: https://asdf-vm.com/manage/commands.html
- ADR-033 (`archctl view` embedded workbench) — patrón de `rust-embed`
- ADR-038 (one product, five invariants) — el binario = el producto
- ADR-039 (renderer reality anti-roadmap) — qué NO incluir
- ADR-042 (IDE adapter abstraction, M73 companion)
- `archctl/src/stack.rs` — base a refactor (no delete)
- `archctl/src/cli.rs:880-940` — comandos `stack install/update/status` a deprecate

## Changelog

- 2026-08-10 | proposed | ADR-057 archctl versioned distribution (asdf-inspired)
