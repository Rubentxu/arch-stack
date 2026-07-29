import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));

const HELP = `archctl — M0 minimal CLI (replaced by Rust in M2).

Commands:
  archctl doctor [--human]
        Check XDG directories, renderers, and OpenCode CLI presence.
  archctl project resolve [--cwd <path>] [--json]
        Resolve SourceIdentity for the current directory.
  archctl render <source.dsl|source.puml> [--format structurizr|plantuml] [--out <dir>]
        Render a DSL/PUML source via local Kroki.

See docs/ROADMAP.md for M0 → M11 milestones.`;

const args = process.argv.slice(2);
const command = args[0];

if (!command || command === "help" || command === "--help" || command === "-h") {
  console.log(HELP);
  process.exit(command ? 0 : 2);
}

const subcommands: Record<string, string> = {
  doctor: "doctor.ts",
  project: "resolve.ts",
  render: "render.ts",
};

const target = subcommands[command];
if (!target) {
  console.error(`archctl: unknown command '${command}'`);
  console.error("run `archctl help` for the list of available commands");
  process.exit(2);
}

const forwarded = args.slice(1);
const child = spawnSync(process.execPath, ["--import", "tsx", join(here, target), ...forwarded], {
  stdio: "inherit",
});
process.exit(child.status ?? 1);
