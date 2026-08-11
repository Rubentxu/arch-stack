---
name: workbench-view
description: Serve the interactive archview workbench for a project. Use when the user asks to "serve the workbench", "open in browser", "interactive view", "pan zoom", or "explore the graph interactively". Drives `archctl view`.
license: MIT
compatibility: opencode
metadata:
  version: "1.0.0"
  maturity: stable
---

# Objective

Launch the local archview workbench (embedded in the archctl binary,
ADR-033) against a project so the user can interact with C4/call-graph/
class/sequence views in the browser.

# Required process

1. Confirm the project has a graph (run discovery first if not:
   `architecture-discovery` skill).
2. Start the server:
   ```bash
   archctl view --cwd <dir>
   # Fixed port for integration:
   archctl view --cwd <dir> --port 18765
   ```
3. The server prints the URL (`http://127.0.0.1:<port>`). Tell the
   user to open it.
4. The workbench loads bundles from `/api/export` (graph-backed).
   If the user opens it without a project, tell them to re-run with
   `--cwd`.

# Contract

- Server binds ONLY to 127.0.0.1 (never public, ADR-011).
- One-shot: it stops when the process is interrupted (Ctrl+C).
- `/api/health` → `{"status":"ok","version":...}` — use it to confirm
  the server is up.
- If the error `view assets not embedded` appears, the binary was
  built without the workbench (run `scripts/embed-view.sh` + rebuild,
  or install a release binary).

# Forbidden

- Exposing the server to the network (no port forwarding, no `0.0.0.0`).
- Leaving the server running in background without telling the user.
