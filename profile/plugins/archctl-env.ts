import type { Plugin } from "@opencode-ai/plugin";
import { spawnSync } from "node:child_process";

interface ProjectEnv {
  projectId: string;
  projectDir: string;
  sourceIdentity: string;
}

function resolveProjectEnv(cwd: string): ProjectEnv | null {
  const r = spawnSync("archctl", ["project", "resolve", "--cwd", cwd, "--json"], {
    encoding: "utf8",
  });
  if (r.status !== 0) return null;
  try {
    const out = JSON.parse(`${r.stdout ?? ""}`) as {
      projectId: string;
      projectDir: string;
      sourceIdentity: string;
    };
    return out;
  } catch {
    return null;
  }
}

export const ArchctlEnvPlugin: Plugin = async ({ directory }) => {
  const env = resolveProjectEnv(directory);
  return {
    "shell.env": async (_input, output) => {
      if (env) {
        output.env.ARCHCTL_PROJECT_ID = env.projectId;
        output.env.ARCHCTL_PROJECT_DIR = env.projectDir;
        output.env.ARCHCTL_SOURCE_IDENTITY = env.sourceIdentity;
      }
      output.env.ARCHCTL_CONFIG_DIR = `${process.env.HOME ?? ""}/.config/archctl`;
    },
  };
};
