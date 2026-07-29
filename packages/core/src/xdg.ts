// XDG layout helper — resolves the four archctl directories from XDG env vars
// with safe defaults. Created in 0.1; reused by probe (0.2), doctor (2.11),
// and the export/import CLI (2.12).
import { mkdirSync } from "node:fs";
import { join } from "node:path";

export interface XdgLayout {
  data: string; // ${XDG_DATA_HOME:-$HOME/.local/share}/archctl
  state: string; // ${XDG_STATE_HOME:-$HOME/.local/state}/archctl
  cache: string; // ${XDG_CACHE_HOME:-$HOME/.cache}/archctl
  config: string; // ${XDG_CONFIG_HOME:-$HOME/.config}/archctl
  projectsRoot(): string; // data/projects/
  runsRoot(): string; // state/runs/
}

export function resolveXdg(env: NodeJS.ProcessEnv = process.env): XdgLayout {
  const home = env.HOME ?? "/tmp";
  const data = join(env.XDG_DATA_HOME ?? `${home}/.local/share`, "archctl");
  const state = join(env.XDG_STATE_HOME ?? `${home}/.local/state`, "archctl");
  const cache = join(env.XDG_CACHE_HOME ?? `${home}/.cache`, "archctl");
  const config = join(env.XDG_CONFIG_HOME ?? `${home}/.config`, "archctl");
  return {
    data,
    state,
    cache,
    config,
    projectsRoot: () => join(data, "projects"),
    runsRoot: () => join(state, "runs"),
  };
}

export function ensureXdg(layout: XdgLayout): void {
  for (const dir of [layout.data, layout.state, layout.cache, layout.config, layout.projectsRoot(), layout.runsRoot()]) {
    mkdirSync(dir, { recursive: true });
  }
}
