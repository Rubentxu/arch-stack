type Severity = "ok" | "warn" | "fail";

interface Finding {
  id: string;
  title: string;
  severity: Severity;
  detail: string;
}

export interface DoctorReport {
  ok: boolean;
  findings: Finding[];
  profilOpencode: string;
  xdg: { data: string; config: string };
  renderers: { structurizr: boolean; plantuml: boolean; kroki: boolean };
}

function probe(url: string, timeoutSec = 2): boolean {
  const r = spawnSync(
    "curl",
    ["-sS", "-o", "/dev/null", "-w", "%{http_code}", "--max-time", String(timeoutSec), url],
    { encoding: "utf8" },
  );
  if (r.status !== 0) return false;
  const code = Number((r.stdout ?? "0").trim());
  return code >= 200 && code < 400;
}

function probeBinary(name: string): boolean {
  const r = spawnSync(name, ["--version"], { encoding: "utf8" });
  return r.status === 0;
}

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";

function xdgPaths(): { data: string; config: string } {
  const home = process.env.HOME ?? "/tmp";
  const data = process.env.XDG_DATA_HOME ?? `${home}/.local/share`;
  const config = process.env.XDG_CONFIG_HOME ?? `${home}/.config`;
  return { data: join(data, "archctl"), config: join(config, "archctl") };
}

export function runDoctor(): DoctorReport {
  const findings: Finding[] = [];
  const xdg = xdgPaths();
  const xdgOk = existsSync(xdg.data) || (() => {
    try {
      const r = spawnSync("mkdir", ["-p", xdg.data]);
      return r.status === 0;
    } catch {
      return false;
    }
  })();
  findings.push({
    id: "xdg.data",
    title: "XDG data directory",
    severity: xdgOk ? "ok" : "warn",
    detail: xdg.data,
  });
  findings.push({
    id: "xdg.config",
    title: "XDG config directory",
    severity: existsSync(xdg.config) ? "ok" : "warn",
    detail: xdg.config,
  });

  const structurizr = probe("http://localhost:18080/") || probeBinary("structurizr");
  findings.push({
    id: "renderer.structurizr",
    title: "Structurizr (local)",
    severity: structurizr ? "ok" : "warn",
    detail: structurizr
      ? "reachable (HTTP localhost:18080 or structurizr CLI on PATH)"
      : "not reachable; run `podman run -d --rm --name archctl-structurizr -p 18080:8080 structurizr/structurizr:latest`",
  });

  const plantuml = probeBinary("plantuml");
  const kroki = probe("http://localhost:18000/");
  const plantumlOk = plantuml || kroki;
  findings.push({
    id: "renderer.plantuml",
    title: "PlantUML (local)",
    severity: plantumlOk ? "ok" : "warn",
    detail: plantuml
      ? "plantuml CLI on PATH"
      : kroki
      ? "local Kroki on localhost:18000"
      : "not reachable; install `plantuml` or `podman run -d --rm --name archctl-kroki -p 18000:8000 yuzutech/kroki:latest`",
  });

  const opencode = probeBinary("opencode");
  const archctl = probeBinary("archctl");
  findings.push({
    id: "opencode.cli",
    title: "OpenCode CLI",
    severity: opencode ? "ok" : "warn",
    detail: opencode ? "opencode on PATH" : "opencode not on PATH",
  });
  findings.push({
    id: "archctl.cli",
    title: "archctl CLI",
    severity: archctl ? "ok" : "warn",
    detail: archctl ? "archctl on PATH" : "archctl not on PATH (run from `archctl/` via npm run)",
  });

  const ok = findings.every((f) => f.severity !== "fail");
  return {
    ok,
    findings,
    profilOpencode: process.env.OPENCODE_CONFIG_DIR ?? "<unset>",
    xdg,
    renderers: { structurizr, plantuml: plantumlOk, kroki },
  };
}

const args = process.argv.slice(2);
const human = args.includes("--human");
const r = runDoctor();
if (human) {
  console.log("archctl doctor");
  for (const f of r.findings) console.log(`  [${f.severity.toUpperCase().padEnd(4)}] ${f.id}: ${f.title} — ${f.detail}`);
  console.log(r.ok ? "DOCTOR: OK" : "DOCTOR: FAIL");
} else {
  console.log(JSON.stringify(r, null, 2));
}
process.exit(r.ok ? 0 : 1);
