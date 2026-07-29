// Smoke probe — Phase 0.2 / Task 0.2.
// Validates environment for archctl M0/M1/M2 without pretending to be the full
// `archctl doctor` (that lives in task 2.11). This probe exists to fail fast
// *before* any skill is adapted or any extraction is attempted. JSON+human output.
import { existsSync, mkdirSync, realpathSync, writeFileSync, readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { join } from "node:path";
import { ensureXdg, resolveXdg } from "../../core/src/xdg.ts";

type Severity = "ok" | "warn" | "fail";

interface ProbeFinding {
  id: string;
  title: string;
  severity: Severity;
  detail: string;
}

interface ProbeReport {
  ok: boolean;
  findings: ProbeFinding[];
  runner: { node: string | null; bun: string | null };
  pinnedOpenCode: string;
  xdgWritable: boolean;
  schemasFound: boolean;
  renderer: { structurizrCli: boolean; plantuml: boolean };
  hooks: { shellEnv: "ok" | "unverified"; toolExecuteBefore: "ok" | "unverified" };
  notes: string[];
}

function probeBinary(name: string): { ok: boolean; version: string | null } {
  const r = spawnSync(name, ["--version"], { encoding: "utf8" });
  if (r.status !== 0) return { ok: false, version: null };
  const out = `${r.stdout ?? ""}${r.stderr ?? ""}`.trim();
  const first = out.split("\n")[0] ?? "";
  return { ok: true, version: first };
}

function probeOpenCodeCli(): { ok: boolean; version: string | null } {
  const r = spawnSync("opencode", ["--version"], { encoding: "utf8" });
  if (r.status !== 0) return { ok: false, version: null };
  return { ok: true, version: `${r.stdout ?? ""}`.trim() };
}

function tryWriteProbeFile(dir: string): boolean {
  try {
    mkdirSync(dir, { recursive: true });
    const probe = join(dir, ".archctl-probe");
    writeFileSync(probe, "ok");
    return existsSync(probe);
  } catch {
    return false;
  }
}

export function runProbe(opts: { cwd: string; pinnedOpenCode: string }): ProbeReport {
  const findings: ProbeFinding[] = [];
  const notes: string[] = [];

  // Runner — never assume which one is present. Gate Zero needs both probed.
  const node = process.versions.node ?? null;
  const bunProc = spawnSync("bun", ["--version"], { encoding: "utf8" });
  const bun = bunProc.status === 0 ? (bunProc.stdout ?? "").trim() : null;
  if (!node) findings.push({ id: "runner.node", title: "Node runtime", severity: "fail", detail: "Node not detected; archctl M0–M2 requires Node ≥ 20 (or Bun)." });
  else findings.push({ id: "runner.node", title: "Node runtime", severity: "ok", detail: `Node ${node}` });
  if (!bun) findings.push({ id: "runner.bun", title: "Bun runtime", severity: "warn", detail: "Bun not detected; not required but supported. Runners are interchangeable." });
  else findings.push({ id: "runner.bun", title: "Bun runtime", severity: "ok", detail: `Bun ${bun}` });

  // OpenCode pin + vendored snapshot.
  const pinPath = join(opts.cwd, ".opencode-version");
  const snapshotDir = join(opts.cwd, "schemas", "opencode", opts.pinnedOpenCode);
  const snapshotFile = join(snapshotDir, "config.json");
  if (!existsSync(pinPath)) findings.push({ id: "opencode.pin-file", title: "OpenCode pin", severity: "fail", detail: ".opencode-version missing" });
  else findings.push({ id: "opencode.pin-file", title: "OpenCode pin", severity: "ok", detail: readFileSync(pinPath, "utf8").split("\n").find((l) => l.startsWith("opencode-version")) ?? "" });
  const schemasFound = existsSync(snapshotFile);
  if (!schemasFound) findings.push({ id: "opencode.schema-snapshot", title: "Vendored schema snapshot", severity: "fail", detail: `missing ${snapshotFile}` });
  else findings.push({ id: "opencode.schema-snapshot", title: "Vendored schema snapshot", severity: "ok", detail: snapshotFile });

  // OpenCode CLI availability — install is a runtime concern, not a build blocker.
  const oc = probeOpenCodeCli();
  if (!oc.ok) {
    findings.push({
      id: "opencode.cli",
      title: "OpenCode CLI",
      severity: "warn",
      detail: "opencode CLI not on PATH. Gate Zero can validate the schema/hook shape via the vendored snapshot but cannot exercise the runtime until OpenCode is installed locally.",
    });
  } else {
    findings.push({ id: "opencode.cli", title: "OpenCode CLI", severity: "ok", detail: oc.version ?? "unknown version" });
  }

  // XDG writability.
  const layout = resolveXdg();
  const xdgWritable = tryWriteProbeFile(layout.projectsRoot());
  ensureXdg(layout);
  if (!xdgWritable) findings.push({ id: "xdg.writability", title: "XDG writability", severity: "fail", detail: `cannot write to ${layout.projectsRoot()}` });
  else findings.push({ id: "xdg.writability", title: "XDG writability", severity: "ok", detail: layout.data });

  // Renderers — Structurizr `local` and PlantUML are local-first only.
  const structurizr = probeBinary("structurizr");
  const plantuml = probeBinary("plantuml");
  if (!structurizr.ok) {
    findings.push({
      id: "renderer.structurizr",
      title: "Structurizr CLI (headless)",
      severity: "warn",
      detail: "Structurizr CLI not on PATH. Gate Zero's render step will fail until a pinned CLI is installed (podman recommended; vNext tracked).",
    });
  } else findings.push({ id: "renderer.structurizr", title: "Structurizr CLI (headless)", severity: "ok", detail: structurizr.version ?? "" });
  if (!plantuml.ok) {
    findings.push({
      id: "renderer.plantuml",
      title: "PlantUML (local)",
      severity: "warn",
      detail: "PlantUML not on PATH. Optional at Gate Zero (UML is not required for the 5-file fixture).",
    });
  } else findings.push({ id: "renderer.plantuml", title: "PlantUML (local)", severity: "ok", detail: plantuml.version ?? "" });

  // Static analyzer dependencies.
  const sg = probeBinary("ast-grep");
  const ctags = probeBinary("ctags");
  if (!sg.ok && !ctags.ok) {
    findings.push({
      id: "extractor.tools",
      title: "Static extractor tools",
      severity: "warn",
      detail: "Neither ast-grep nor ctags is on PATH. Phase 1 fixtures will fail until one is installed.",
    });
  } else {
    const which = sg.ok ? `ast-grep ${sg.version ?? ""}` : `ctags ${ctags.version ?? ""}`;
    findings.push({ id: "extractor.tools", title: "Static extractor tools", severity: "ok", detail: which });
  }

  // Hook firing + permission ordering — runtime probes. Without OpenCode on PATH we
  // cannot exercise hooks; mark as unverified and warn.
  const hooks: ProbeReport["hooks"] = oc.ok
    ? { shellEnv: "unverified", toolExecuteBefore: "unverified" }
    : { shellEnv: "unverified", toolExecuteBefore: "unverified" };
  if (oc.ok) notes.push("OpenCode CLI present but hook firing was not exercised by this probe; defer to M0.4 end-to-end runner.");
  findings.push({
    id: "opencode.hooks",
    title: "Hook firing + permission ordering",
    severity: "warn",
    detail: `shell.env + tool.execute.before not exercised (status: ${hooks.shellEnv}/${hooks.toolExecuteBefore}).`,
  });

  const ok = findings.every((f) => f.severity !== "fail");

  return {
    ok,
    findings,
    runner: { node, bun },
    pinnedOpenCode: opts.pinnedOpenCode,
    xdgWritable,
    schemasFound,
    renderer: { structurizrCli: structurizr.ok, plantuml: plantuml.ok },
    hooks,
    notes,
  };
}

function formatHuman(r: ProbeReport): string {
  const lines: string[] = [];
  lines.push(`archctl smoke probe (runner=${r.runner.node ?? r.runner.bun ?? "?"})`);
  lines.push(`pinned opencode: ${r.pinnedOpenCode}  xdg: ${r.xdgWritable ? "ok" : "FAIL"}  schemas: ${r.schemasFound ? "ok" : "FAIL"}`);
  for (const f of r.findings) {
    lines.push(`  [${f.severity.toUpperCase().padEnd(4)}] ${f.id}: ${f.title} — ${f.detail}`);
  }
  if (r.notes.length) {
    lines.push("Notes:");
    for (const n of r.notes) lines.push(`  - ${n}`);
  }
  lines.push(r.ok ? "PROBE: OK (no FAIL)" : "PROBE: FAIL — fix FAIL items before M0.4");
  return lines.join("\n");
}

// CLI entry: tsx packages/cli/src/probe.ts
const args = process.argv.slice(2);
const human = args.includes("--human");
const cwd = process.cwd();
const pinPath = join(cwd, ".opencode-version");
let pinned = "1.18.x";
try {
  const first = readFileSync(pinPath, "utf8").split("\n").find((l) => l.startsWith("opencode-version:"));
  if (first) pinned = first.split(":")[1]?.trim() ?? pinned;
} catch {
  // ignore — probe will report
}
const report = runProbe({ cwd, pinnedOpenCode: pinned });
console.log(human ? formatHuman(report) : JSON.stringify(report, null, 2));
process.exit(report.ok ? 0 : 1);
