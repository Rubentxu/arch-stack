---
name: stack-management
description: Install, update, and verify the arch-stack product (archctl binary + archview workbench + agent skills). Use when asked to "install archctl", "update archctl", "stack status", "doctor check", "setup a new machine", or "onboarding". Drives `archctl ide install` + `archctl self *` + `archctl doctor`.
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
2. See which IDEs have the stack installed:
   ```bash
   archctl ide list --installed
   ```
   Shows: supported IDEs, which are currently installed, and the
   install root for each.
3. Install or update the full stack (skills + agents + plugin into
   the IDE discovery path):
   ```bash
   archctl ide install opencode --install-root ~/.config/opencode
   # Idempotent update — same command re-runs safely
   archctl ide update opencode --install-root ~/.config/opencode
   ```
4. Verify the installation is healthy:
   ```bash
   archctl ide doctor opencode
   ```
   Checks that all embedded skills and agents are present at the
   install root and match what the binary ships.

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
- Claiming the stack is current without running `archctl ide doctor <ide>`
  to verify alignment.
