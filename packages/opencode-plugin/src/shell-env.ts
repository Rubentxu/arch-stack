// 2.1 — SourceIdentity → `$ARCHCTL_PROJECT_DIR`.
//
// Resolves the discriminated SourceIdentity at session start (ADR-0003),
// then exposes the resulting `$ARCHCTL_PROJECT_DIR` env var so OpenCode
// agents run under that directory. The resolver is *deterministic and
// fallback-only*: it never throws and always picks a writable XDG path.
//
// This is the thin TypeScript shim that the OpenCode plugin installs via
// `shell.env` (ADR-0007). The same logic can be invoked from a CLI.
import { resolveSourceIdentity, portableProjectId } from "../../core/src/resolver/identity.ts";
import { resolveXdg } from "../../core/src/xdg.ts";
import { join } from "node:path";
import { mkdirSync } from "node:fs";

export interface ProjectEnv {
  sourceIdentity: ReturnType<typeof resolveSourceIdentity>;
  projectId: string;
  projectDir: string;
  env: Record<string, string>;
}

/**
 * Resolve the project's `archctl` env and ensure the project directory exists.
 * Returns the env map the plugin should inject via `shell.env`.
 */
export function resolveProjectEnv(opts: { cwd?: string } = {}): ProjectEnv {
  const id = resolveSourceIdentity({ cwd: opts.cwd });
  const layout = resolveXdg();
  const projectId = portableProjectId(id);
  const projectDir = join(layout.projectsRoot(), projectId);
  mkdirSync(projectDir, { recursive: true });
  return {
    sourceIdentity: id,
    projectId,
    projectDir,
    env: {
      ARCHCTL_PROJECT_DIR: projectDir,
      ARCHCTL_PROJECT_ID: projectId,
      ARCHCTL_SOURCE_IDENTITY: JSON.stringify(id),
    },
  };
}

/**
 * Pure helper exposed for tests and CLI: produce the env map for a given cwd
 * without writing to disk.
 */
export function projectEnvFor(opts: { cwd?: string } = {}): Record<string, string> {
  return resolveProjectEnv(opts).env;
}
