# Spec — IDE Adapter Interface (`IdeAdapter` trait)

> **Ciclo:** `m73-distribution-stack-rework`
> **Estado:** Propuesto — 2026-08-10
> **ADR fuente:** [ADR-042](../adr/ADR-042-ide-adapter-abstraction.md)
> **Built-in adapters:** OpenCode, ZCode, Claude Code, Codex

## Objetivo

Definir el contrato `IdeAdapter` que cualquier IDE (OpenCode, ZCode,
Claude Code, Codex, futuros) implementa para que `archctl ide install` /
`doctor` / `list` funcionen uniformemente sin hardcodear paths por IDE.

## Alcance

- Trait `IdeAdapter` (Rust) — interfaz mínima.
- 4 adapters built-in: OpenCode, ZCode, Claude Code, Codex.
- Plugin tap para adapters externos (M76).
- CLI `archctl ide <subcommand>` con subflags.

## Fuera de alcance

- Adapter SDK para terceros (M76).
- Adapter para Cursor / Aider / Windsurf (M77+).

## Trait definition

```rust
// archctl/src/ide/mod.rs
use std::path::{Path, PathBuf};

/// Stable identifier for an IDE (kebab-case, lowercase).
pub trait IdeAdapter: Send + Sync {
    /// Stable id used in CLI flags, config keys, tap names. Never changes.
    /// Examples: "opencode", "claude-code", "codex".
    fn id(&self) -> &'static str;

    /// Human-readable name shown in `archctl ide list`.
    /// Examples: "OpenCode", "Claude Code", "Codex CLI".
    fn name(&self) -> &'static str;

    /// Detect whether this IDE is installed locally.
    /// Used by `ide list --installed` and `ide install` (warn if missing).
    /// Returns `Ok(IdePresence { installed, hint })`.
    fn detect(&self) -> anyhow::Result<IdePresence>;

    /// Root directory for the IDE's config (XDG-style).
    /// Example: OpenCode → `~/.config/opencode`, Claude Code → `~/.claude`.
    fn config_root(&self) -> PathBuf;

    /// Install the embedded StackPayload into this IDE's discovery paths.
    /// Returns `InstallReport { written, skipped, errors }`.
    fn install_stack(&self, payload: &StackPayload) -> anyhow::Result<InstallReport>;

    /// Remove previously installed payload.
    /// Returns `RemoveReport { removed, kept }` (kept = user-customized files).
    fn remove_stack(&self, payload_id: &str) -> anyhow::Result<RemoveReport>;

    /// Report drift between installed files and payload (which are stale / missing / extra).
    fn diff_stack(&self, payload: &StackPayload) -> anyhow::Result<Vec<DriftEntry>>;

    /// Format converter for skills (e.g. SKILL.md → Claude plugin format).
    /// Default impl returns the skill unchanged. Override for IDEs with
    /// non-standard skill representations.
    fn convert_skill(&self, skill_md: &str, skill_name: &str) -> anyhow::Result<String> {
        Ok(skill_md.to_string())
    }

    /// Validate that the IDE's config dir is compatible (no conflicting
    /// versions, no unsupported characters in config, etc.).
    fn validate(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct IdePresence {
    pub installed: bool,
    pub hint: Option<String>,  // "binary 'opencode' not on $PATH"
}

#[derive(Debug, Default)]
pub struct InstallReport {
    pub written: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,  // identical content
    pub errors: Vec<String>,
}

#[derive(Debug, Default)]
pub struct RemoveReport {
    pub removed: Vec<PathBuf>,
    pub kept: Vec<(PathBuf, String)>,  // (path, reason)
}

#[derive(Debug, Clone)]
pub struct DriftEntry {
    pub path: PathBuf,
    pub status: DriftStatus,
}

#[derive(Debug, Clone)]
pub enum DriftStatus {
    Missing,    // payload has it, target doesn't
    Stale,      // payload has it, target has different content
    Extra,      // target has it, payload doesn't (user-installed)
}

#[derive(Debug, Clone)]
pub struct StackPayload {
    /// Stable identifier for the bundle version (matches archctl version).
    pub id: String,           // e.g. "arch-stack-1.33.0"
    pub version: semver::Version,
    pub skills: Vec<SkillFile>,
    pub agents: Vec<AgentFile>,
    pub plugins: Vec<PluginFile>,
}

#[derive(Debug, Clone)]
pub struct SkillFile {
    pub name: String,         // "architecture-discovery"
    pub markdown: String,     // full content including frontmatter
    pub scripts: Vec<ScriptFile>,  // optional executable scripts
}

#[derive(Debug, Clone)]
pub struct AgentFile {
    pub name: String,         // "diagram-architect"
    pub markdown: String,
}

#[derive(Debug, Clone)]
pub struct PluginFile {
    pub name: String,         // "archctl-env"
    pub source: String,       // TypeScript or other source
    pub config: Option<toml::Value>,
}
```

## Built-in adapters (M75)

### OpenCodeAdapter

```rust
pub struct OpenCodeAdapter;
impl IdeAdapter for OpenCodeAdapter {
    fn id(&self) -> &'static str { "opencode" }
    fn name(&self) -> &'static str { "OpenCode" }
    fn detect(&self) -> Result<IdePresence> {
        // Check ~/.config/opencode/ exists OR `opencode` binary on $PATH.
        let config = self.config_root();
        let binary = which("opencode");
        Ok(IdePresence {
            installed: config.exists() || binary.is_some(),
            hint: if binary.is_none() && !config.exists() {
                Some("OpenCode config not found and `opencode` not on $PATH".into())
            } else { None },
        })
    }
    fn config_root(&self) -> PathBuf {
        dirs::config_dir().unwrap_or(PathBuf::from(".")).join("opencode")
    }
    fn install_stack(&self, payload: &StackPayload) -> Result<InstallReport> {
        // payload.skills → ~/.config/opencode/skills/<name>/SKILL.md
        // payload.agents → ~/.config/opencode/agents/<name>.md
        // payload.plugins → ~/.config/opencode/plugins/<name>.ts
        let root = self.config_root();
        let mut report = InstallReport::default();
        for skill in &payload.skills {
            let dir = root.join("skills").join(&skill.name);
            fs::create_dir_all(&dir)?;
            let path = dir.join("SKILL.md");
            if write_if_changed(&path, skill.markdown.as_bytes())? {
                report.written.push(path);
            } else {
                report.skipped.push(path);
            }
        }
        // ... agents + plugins similar
        Ok(report)
    }
    // remove_stack, diff_stack: mirror install_stack with deletion/comparison.
}
```

### ZCodeAdapter

```rust
pub struct ZCodeAdapter;
impl IdeAdapter for ZCodeAdapter {
    fn id(&self) -> &'static str { "zcode" }
    fn name(&self) -> &'static str { "ZCode" }
    fn detect(&self) -> Result<IdePresence> {
        let config = self.config_root();
        let binary = which("zcode");
        Ok(IdePresence {
            installed: config.exists() || binary.is_some(),
            hint: if binary.is_none() && !config.exists() {
                Some("ZCode config not found and `zcode` not on $PATH".into())
            } else { None },
        })
    }
    fn config_root(&self) -> PathBuf {
        dirs::config_dir().unwrap_or(PathBuf::from(".")).join("opencode")
        // ZCode shares OpenCode's discovery path.
    }
    // install_stack: same as OpenCode (ZCode is a fork).
}
```

### ClaudeCodeAdapter

```rust
pub struct ClaudeCodeAdapter;
impl IdeAdapter for ClaudeCodeAdapter {
    fn id(&self) -> &'static str { "claude-code" }
    fn name(&self) -> &'static str { "Claude Code" }
    fn detect(&self) -> Result<IdePresence> {
        let config = self.config_root();
        let binary = which("claude");
        Ok(IdePresence {
            installed: config.exists() || binary.is_some(),
            hint: if binary.is_none() && !config.exists() {
                Some("Claude Code config not found and `claude` not on $PATH".into())
            } else { None },
        })
    }
    fn config_root(&self) -> PathBuf {
        dirs::config_dir().unwrap_or(PathBuf::from(".")).join("claude")
    }
    fn install_stack(&self, payload: &StackPayload) -> Result<InstallReport> {
        // Claude Code plugin format: ~/.claude/plugins/<id>/{skills,commands,agents}/...
        // Each skill becomes a directory with SKILL.md.
        // agents become ~/.claude/agents/<name>.md (with Claude frontmatter).
        let root = self.config_root();
        let mut report = InstallReport::default();
        for skill in &payload.skills {
            let skill_md = self.convert_skill(&skill.markdown, &skill.name)?;
            let dir = root.join("plugins").join("arch-stack").join("skills").join(&skill.name);
            fs::create_dir_all(&dir)?;
            let path = dir.join("SKILL.md");
            if write_if_changed(&path, skill_md.as_bytes())? {
                report.written.push(path);
            } else {
                report.skipped.push(path);
            }
        }
        Ok(report)
    }
    fn convert_skill(&self, skill_md: &str, skill_name: &str) -> Result<String> {
        // Translate frontmatter: OpenCode's `toolName` → Claude's `allowedTools`.
        // Keep `name` and `description` as-is.
        Ok(skill_md.to_string())  // TODO M75: actual translation
    }
}
```

### CodexAdapter

```rust
pub struct CodexAdapter;
impl IdeAdapter for CodexAdapter {
    fn id(&self) -> &'static str { "codex" }
    fn name(&self) -> &'static str { "Codex CLI" }
    fn detect(&self) -> Result<IdePresence> {
        let config = self.config_root();
        let binary = which("codex");
        Ok(IdePresence {
            installed: config.exists() || binary.is_some(),
            hint: if binary.is_none() && !config.exists() {
                Some("Codex config not found and `codex` not on $PATH".into())
            } else { None },
        })
    }
    fn config_root(&self) -> PathBuf {
        dirs::config_dir().unwrap_or(PathBuf::from(".")).join("codex")
    }
    fn install_stack(&self, payload: &StackPayload) -> Result<InstallReport> {
        // Codex uses TOML prompts in ~/.codex/prompts/
        // SKILL.md is markdown; we convert to TOML prompt with `content = """
        for skill in &payload.skills {
            let dir = root.join("prompts");
            fs::create_dir_all(&dir)?;
            let path = dir.join(format!("{}.toml", skill.name));
            let toml = format!(
                r#"prompt = """
{}
"""
"#,
                skill.markdown
            );
            if write_if_changed(&path, toml.as_bytes())? {
                report.written.push(path);
            }
        }
        Ok(report)
    }
}
```

## CLI surface

```
archctl ide list [--installed] [--available]
# Output:
#   ID              NAME                INSTALLED   CONFIG_ROOT
#   opencode        OpenCode            yes         /home/u/.config/opencode
#   zcode           ZCode               no          /home/u/.config/opencode
#   claude-code     Claude Code         yes         /home/u/.claude
#   codex           Codex CLI            no          /home/u/.codex

archctl ide install <ide> [--stack=core|full|none]
#   --stack=core: install only essentials (skills/agents, no plugins)
#   --stack=full: install all (default)
#   --stack=none: dry-run, show what would be installed

archctl ide doctor <ide>
#   Output:
#     OpenCode (opencode) — installed
#       config_root: /home/u/.config/opencode  (exists)
#       binary: /usr/local/bin/opencode          (found)
#       skills: 9 installed, 0 missing, 0 stale
#       agents: 5 installed, 0 missing
#       plugins: 1 installed (archctl-env.ts)
#       drift: none — stack aligned with v1.33.0

archctl ide remove <ide> [--purge]
#   Removes ~/.config/opencode/{skills,agents,plugins}/archctl-* without
#   touching user-installed files.

archctl ide update <ide> [archctl-update-flags...]
#   Convenience wrapper: `archctl ide update claude-code` ==
#   `archctl self update && archctl ide install claude-code`.
```

## Plugin external adapters (M76)

Adapter externos se declaran via `plugin.toml`:

```toml
# ~/.config/archctl/plugins/<author>/<my-ide-adapter>/plugin.toml
[adapter]
id = "windsurf"
name = "Windsurf IDE"
binary = "windsurf"
config_root_template = "$HOME/.config/Windsurf"

[adapter.skill_format]
type = "skill-md"   # or "toml-prompt", "custom"
frontmatter_overrides = { toolName = "tools" }   # OpenCode→Windsurf field rename
```

`archctl ide install windsurf` carga el adapter externo (via `inventory` +
dynamic lib loading via `libloading`) y lo ejecuta.

Para v1 (M75), **no hay adapters externos** — solo los 4 built-in. El
plugin tap model llega en M76.

## Testing strategy

### Unit tests por adapter

Cada adapter tiene fixtures reales (3-5 skills, 2 agents, 1 plugin) y:

- `install_stack_round_trip`: install + diff (debe ser empty) + remove + diff (debe ser empty).
- `install_idempotent`: install 2 veces, segundo InstallReport tiene `written` vacío.
- `convert_skill_*`: cada skill format conversion produce el output esperado.
- `detect_*`: simulated config_root presence/absence scenarios.

### Integration tests

`e2e/install_e2e.sh` extendido con `archctl ide install claude-code` + `archctl
ide install codex` y verificación de los layouts resultantes en
`~/.claude/` y `~/.codex/`.

### Cross-IDE consistency

El mismo payload instalado en 2 IDEs produce 2 layouts correctos independientemente
(los fixtures validan que la suma de bytes difiere porque el adapter incluye
frontmatter distinto, pero los archivos conceptualmente equivalentes están
presentes).

## Verification matrix

| Adapter | `detect()` | `config_root()` | `install_stack()` | `remove_stack()` | `diff_stack()` | `convert_skill()` |
|---|---|---|---|---|---|---|
| OpenCode | ✓ | ✓ | ✓ | ✓ | ✓ | (default) |
| ZCode | ✓ | (alias) | ✓ | ✓ | ✓ | (default) |
| Claude Code | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ (M75 PR3) |
| Codex | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ (M75 PR4) |

## Migration from current `stack.rs`

`archctl/src/stack.rs:48 install()` → se mantiene como wrapper que:

```rust
pub fn install(root: &Path) -> Result<Vec<String>> {
    // Deprecated; use `archctl ide install opencode` instead.
    eprintln!("warning: `archctl stack install` is deprecated, use `archctl ide install opencode`");
    let adapter = OpenCodeAdapter;
    let payload = current_payload()?;
    let report = adapter.install_stack(&payload)?;
    Ok(report.written.iter().map(|p| p.display().to_string()).collect())
}
```

Removal del wrapper en M77 (per ADR-039 anti-roadmap).

## Referencias

- ADR-042 (IDE adapter abstraction)
- OpenCode plugins docs: https://opencode.ai/v2/docs/build/plugins
- OpenCode config docs: https://opencode.ai/docs/config
- Claude Code plugins: https://docs.claude.com/en/docs/claude-code
- Codex CLI: https://github.com/openai/codex
- `archctl/src/stack.rs` (legacy; reemplazado por los adapters)
- `archctl/assets-stack/` (payload actual — fuente de los 9 skills, 5 agents, 1 plugin)
