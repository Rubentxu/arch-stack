import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync, readFileSync } from "node:fs";
import { join, basename, dirname } from "node:path";
import { existsSync } from "node:fs";

export type RenderFormat = "structurizr" | "plantuml";

export interface RenderOptions {
  sourcePath: string;
  format: RenderFormat;
  outDir: string;
  krokiUrl?: string;
  structurizrUrl?: string;
}

export interface RenderResult {
  ok: boolean;
  sourcePath: string;
  format: RenderFormat;
  outputPath: string;
  bytes: number;
  details: string;
}

export function render(opts: RenderOptions): RenderResult {
  if (!existsSync(opts.sourcePath)) {
    return {
      ok: false,
      sourcePath: opts.sourcePath,
      format: opts.format,
      outputPath: "",
      bytes: 0,
      details: `source file not found: ${opts.sourcePath}`,
    };
  }
  mkdirSync(opts.outDir, { recursive: true });

  if (opts.format === "structurizr") {
    const krokiUrl = opts.krokiUrl ?? "http://localhost:18000";
    const body = readFileSync(opts.sourcePath, "utf8");
    const outPath = join(opts.outDir, `${basename(opts.sourcePath, ".dsl")}.svg`);
    const r = spawnSync(
      "curl",
      [
        "-sS",
        "-o", outPath,
        "-w", "%{http_code}",
        "-X", "POST",
        "-H", "Content-Type: text/plain",
        "--data-binary", "@-",
        `${krokiUrl}/structurizr/svg`,
      ],
      { input: body, encoding: "utf8" },
    );
    const code = Number((r.stdout ?? "0").trim()) || 0;
    const ok = code >= 200 && code < 300;
    return {
      ok,
      sourcePath: opts.sourcePath,
      format: opts.format,
      outputPath: outPath,
      bytes: Buffer.byteLength(body),
      details: ok ? `POST ${krokiUrl}/structurizr/svg → ${code}` : `HTTP ${code}`,
    };
  }

  const krokiUrl = opts.krokiUrl ?? "http://localhost:18000";
  const body = readFileSync(opts.sourcePath, "utf8").trim();
  const outPath = join(opts.outDir, `${basename(opts.sourcePath, ".puml")}.svg`);
  const r = spawnSync(
    "curl",
    [
      "-sS",
      "-o", outPath,
      "-w", "%{http_code}",
      "-X", "POST",
      "-H", "Content-Type: text/plain",
      "--data-binary", "@-",
      `${krokiUrl}/plantuml/svg`,
    ],
    { input: body, encoding: "utf8" },
  );
  const code = Number((r.stdout ?? "0").trim()) || 0;
  const ok = code >= 200 && code < 300;
  return {
    ok,
    sourcePath: opts.sourcePath,
    format: opts.format,
    outputPath: outPath,
    bytes: Buffer.byteLength(body),
    details: ok ? `POST ${krokiUrl}/plantuml/svg → ${code}` : `HTTP ${code}`,
  };
}

const args = process.argv.slice(2);
function arg(name: string, fallback?: string): string | undefined {
  const i = args.indexOf(`--${name}`);
  return i >= 0 ? args[i + 1] : fallback;
}
const sourcePath = args.find((a) => !a.startsWith("--"));
if (!sourcePath) {
  console.error("usage: archctl render <source.dsl|source.puml> [--format structurizr|plantuml] [--out <dir>]");
  process.exit(2);
}
const format = (arg("format", sourcePath.endsWith(".puml") ? "plantuml" : "structurizr") as RenderFormat);
const outDir = arg("out", join(dirname(sourcePath), ".archctl-rendered")) ?? ".archctl-rendered";
const r = render({ sourcePath, format, outDir });
console.log(JSON.stringify(r, null, 2));
process.exit(r.ok ? 0 : 1);
