---
name: stack-management
description: Install, update, and verify the arch-stack product (archctl binary + archview workbench + agent skills). Use when setting up a new machine, upgrading versions, checking component alignment, or onboarding to OpenCode/ZCode. Drives `archctl stack *` + `archctl doctor`.
license: MIT
compatibility: opencode
metadata:
  version: "1.0.0"
  maturity: stable
---

# Objective

Treat `archctl` + `archview` + the agent skills as ONE product that is
installed, versioned, and updated as a unit (stack distribution model).

# Required process

1. Health check first — always start here:
   ```bash
   archctl doctor
   ```
   Reports scope gates + environment. If it fails, the install is
   broken; do not proceed to feature work.
2. See what's installed vs embedded in the binary:
   ```bash
   archctl stack status
   ```
   Shows: binary version, workbench embedded version, skills installed
   (per OpenCode/ZCode discovery path) and whether they match the
   embedded set.
3. Install or update the full stack (skills + agents + plugin into
   `~/.config/opencode/{skills,agents,plugins}`):
   ```bash
   archctl stack install
   # Idempotent update (same command re-runs safely)
   archctl stack update
   # Non-interactive (no prompts):
   archctl stack install --yes
   ```
4. Verify skills are discoverable by the agent runtime:
   ```bash
   archctl skills list
   archctl skills verify
   ```

# Platform notes

- OpenCode and ZCode share the discovery paths
  (`~/.config/opencode/skills/`, `~/.agents/skills/`,
  `~/.claude/skills/`) — one install serves both.
- The SKILL.md format is portable: the same skills work in Claude
  Code / Codex / Cursor if copied to their skill dirs.
- Agents install to `~/.config/opencode/agents/`; the env plugin to
  `~/.config/opencode/plugins/`.

# Forbidden

- Patching skill files by hand in the install target (they get
  overwritten on update — edit the repo sources instead).
- Claiming the stack is current when `stack status` shows drift.
