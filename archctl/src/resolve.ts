import { existsSync } from "node:fs";
import { join } from "node:path";

interface ProjectInfo {
  projectId: string;
  projectDir: string;
  sourceIdentity: string;
}

function resolve(cwd: string): ProjectInfo {
  const home = process.env.HOME ?? "/tmp";
  const projectDir = join(home, ".local", "share", "archctl", "projects", "default");
  return {
    projectId: "default",
    projectDir,
    sourceIdentity: `dir:${cwd}`,
  };
}

const args = process.argv.slice(2);
const cwdIdx = args.indexOf("--cwd");
const cwd = cwdIdx >= 0 ? args[cwdIdx + 1] ?? process.cwd() : process.cwd();
const json = args.includes("--json");

const info = resolve(cwd);
if (json) {
  console.log(JSON.stringify(info, null, 2));
} else {
  console.log(`projectId:      ${info.projectId}`);
  console.log(`projectDir:     ${info.projectDir}`);
  console.log(`sourceIdentity: ${info.sourceIdentity}`);
}
process.exit(existsSync(info.projectDir) ? 0 : 2);
