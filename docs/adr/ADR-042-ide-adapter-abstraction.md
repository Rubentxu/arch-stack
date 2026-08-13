# ADR-042 — IDE adapter abstraction (multi-IDE plugin target)

> **Ciclo:** `m73-distribution-stack-rework` (planning)
> **Estado:** Propuesto — 2026-08-10
> **Complementa:** ADR-057 (versioned distribution), ADR-058 (self-update)
> **Reemplaza:** la hardcoded `default_install_root()` en `archctl/src/stack.rs:18`

## Contexto

`archctl stack install` actualmente escribe a `~/.config/opencode/` (per
`archctl/src/stack.rs:18-27`). Esto asume que el IDE es OpenCode (o ZCode,
que comparte la convención de discovery). Pero el ecosistema de coding
agents en 2026 tiene **al menos 4 IDEs con discovery paths distintos**:

| IDE | Skills/Agents/Plugins path | Config |
|---|---|---|
| **OpenCode** | `~/.config/opencode/{skills,agents,commands,plugins}/` | `~/.config/opencode/opencode.json` |
| **ZCode** | `~/.config/opencode/` (mismo path que OpenCode — fork) | igual |
| **Claude Code** | `~/.claude/{skills,agents,commands,plugins}/` | `~/.claude/settings.json` |
| **OpenAI Codex** | `~/.codex/{skills,agents,prompts}/` | `~/.codex/config.toml` |
| **Cursor** | `~/.cursor/rules/` + `.cursorrules` | `~/.cursor/config.json` |
| **Aider** | `~/.aider.conf.yml` + `.aider/*` | ninguno (file-based) |

(verificado vía web search 2026-08-10 sobre OpenCode v2 plugins docs,
Claude Code plugins docs, Codex plugins docs).

El problema: **un solo `archctl stack install` no puede escribir a todos
estos paths simultáneamente** porque:

1. Los formatos de skill difieren (OpenCode SKILL.md vs Claude Code plugin
   manifest con frontmatter distinto).
2. Las convenciones de naming difieren (OpenCode lowercase-kebab; Claude
   Code acepta Title-Case; Codex usa snake_case).
3. Las relaciones entre skills/agents/plugins difieren (OpenCode tiene
   sub-agent concept; Claude Code tiene MCP server bundles).

Si intentamos un solo payload universal, perdemos fidelity por IDE.
Si escribimos N payloads hardcoded, mantenemos N ramas.

## Decisión

Introducir **`archctl ide <subcommand>`** con un **adapter pattern**:

```
archctl ide install <ide> [--stack=core|full|none]
archctl ide list                  # IDEs soportados (con --installed, los detectados en $PATH)
archctl ide doctor <ide>          # diagnóstico específico del IDE
archctl ide remove <ide> [--purge]
```

Cada IDE se representa por un **adapter** que implementa:

```rust
pub trait IdeAdapter: Send + Sync {
    /// Stable identifier (e.g. "opencode", "claude-code", "codex").
    fn id(&self) -> &'static str;
    /// Human-readable name (e.g. "OpenCode", "Claude Code").
    fn name(&self) -> &'static str;
    /// Detect whether this IDE is installed locally (binary on $PATH,
    /// config dir exists, etc.). Used by `ide list --installed`.
    fn detect(&self) -> Result<IdePresence>;
    /// Root directory for skills/agents/plugins (XDG-style).
    fn config_root(&self) -> PathBuf;
    /// Install a `StackPayload` into this IDE's discovery paths.
    fn install_stack(&self, payload: &StackPayload) -> Result<InstallReport>;
    /// Remove previously installed payload.
    fn remove_stack(&self, payload_id: &str) -> Result<RemoveReport>;
    /// Report drift between installed and payload.
    fn diff_stack(&self, payload: &StackPayload) -> Result<Vec<DriftEntry>>;
}
```

### Built-in adapters (M75)

| Adapter | ID | Skill format | Notes |
|---|---|---|---|
| **OpenCodeAdapter** | `opencode` | SKILL.md + agents/*.md | Default. Equivalente al `default_install_root()` actual. |
| **ZCodeAdapter** | `zcode` | SKILL.md (mismo formato que OpenCode) | ZCode es fork de OpenCode; alias directo. |
| **ClaudeCodeAdapter** | `claude-code` | Plugin bundle format (`.claude-plugin.json` + commands/agents/skills dirs) | Mapping de SKILL.md → Claude plugin spec. |
| **CodexAdapter** | `codex` | Codex prompts format (`prompts/*.toml`) | Mapping de SKILL.md → TOML prompt. |

### Adapter discovery

Adapters built-in se cargan en `archctl` via `inventory`-style registration:

```rust
// archctl/src/ide/mod.rs
pub fn builtin_adapters() -> Vec<Box<dyn IdeAdapter>> {
    vec![
        Box::new(OpenCodeAdapter::new()),
        Box::new(ZCodeAdapter::new()),
        Box::new(ClaudeCodeAdapter::new()),
        Box::new(CodexAdapter::new()),
    ]
}
```

Adapters externos (terceros) se registran vía plugin tap (ADR-057 §4):

```toml
# ~/.config/archctl/plugins/<author>/<my-ide-adapter>/plugin.toml
[adapter]
id = "windsurf"
name = "Windsurf IDE"
binary = "windsurf"
config_root_template = "$HOME/.config/Windsurf"
skill_format = "skill-md"
```

`archctl ide install windsurf` carga el adapter externo y lo ejecuta.

### Plugin format conversion

Para IDEs que no usan SKILL.md nativo (Claude Code, Codex), el adapter
implementa la conversión:

```rust
// ClaudeCodeAdapter::install_stack:
fn install_stack(&self, payload: &StackPayload) -> Result<InstallReport> {
    // payload.skills/*.md → ~/.claude/plugins/<id>/skills/<name>/SKILL.md
    // payload.agents/*.md → ~/.claude/agents/<name>.md (frontmatter translation)
    // payload.plugins/*.ts → bundled into ~/.claude/plugins/<id>/commands/
    Ok(InstallReport::default())
}
```

Esto significa que el **mismo payload** (embedded en el binario archctl) se
presenta en formato nativo de cada IDE. No se requieren binarios separados
por IDE.

### CLI changes

```bash
# v1 (M75): OpenCode + ZCode + Claude Code + Codex como built-in
archctl ide list
# Output:
#   opencode       (installed: ~/.config/opencode)
#   zcode          (not installed)
#   claude-code     (installed: ~/.claude)
#   codex          (not installed)

archctl ide install claude-code
# → copies skills/agents/plugin to ~/.claude/ in Claude Code plugin format

archctl ide doctor claude-code
# → diagnostic: 8 skills installed, 0 plugins missing, MCP config valid
```

### Backward compatibility

`archctl stack install` queda como **alias deprecated** de
`archctl ide install opencode` durante un ciclo de release, con un warning
emitido la primera vez. Removal en M77 (ADR-039 anti-roadmap: no acumular
APIs deprecated).

## Consecuencias

### Positivas

- **Extensible**: añadir soporte para un nuevo IDE = implementar `IdeAdapter`
  + registrar en `builtin_adapters()` o publicar un plugin externo.
- **No más N hardcoded paths**: el adapter encapsula las convenciones del IDE.
- **Testeable**: cada adapter tiene tests unitarios + tests integration con
  fixtures de skills/agents/plugins reales.

### Negativas

- **Conversión de formato puede perder información**: SKILL.md de OpenCode
  tiene campos que Claude Code plugin no soporta (`toolName`, etc.).
  Mitigation: el adapter documenta los campos ignorados por IDE y emite un
  warning al usuario.
- **Plugins TypeScript solo se instalan donde aplica**: el plugin
  `archctl-env.ts` solo tiene sentido en OpenCode/ZCode (TypeScript runtime).
  En Claude Code, se descarta con un warning.

## Implementation Plan (M75)

- PR #1: `IdeAdapter` trait + `StackPayload` struct (extraído de `stack.rs`).
- PR #2: `OpenCodeAdapter` (extract current `stack.rs` logic) + `ZCodeAdapter` (alias).
- PR #3: `ClaudeCodeAdapter` con plugin format conversion.
- PR #4: `CodexAdapter` con prompts TOML conversion.
- PR #5: `archctl ide list/install/doctor/remove` CLI + adapter registry.

## Verificación

- Unit: cada adapter tiene fixtures (3-5 skills, 2 agents, 1 plugin) y
  verifica round-trip (install → diff → remove → diff).
- Integration: `e2e/install_e2e.sh` extendido con `archctl ide install
  claude-code` + verificación de `~/.claude/` contents.
- Cross-IDE: el mismo payload instalado en 2 IDEs produce los 2 layouts
  correctos independientemente.

## Referencias

- OpenCode plugins docs: https://opencode.ai/v2/docs/build/plugins
- OpenCode config docs: https://opencode.ai/docs/config
- Claude Code plugins: https://docs.claude.com/en/docs/claude-code (commands, plugins)
- Codex CLI: https://github.com/openai/codex
- `archctl/src/stack.rs:18` — `default_install_root()` a eliminar
- ADR-057 §4 (plugin tap model)
- ADR-058 (GitHub Releases distribution)

## Changelog

- 2026-08-10 | proposed | ADR-042 IDE adapter abstraction
