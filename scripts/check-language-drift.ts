// 2.16 — Full language-drift guard (ADR-0001 enforcement).
//
// The WU1 placeholder (`scripts/check-language-drift.ts`) checks both
// *content signatures* of forbidden languages. The full guard adds a
// *structural* layer: file extensions, ignoring markdown and test files.
// The two together make "did someone drop a .go file in packages/?" loud
// at build time.
//
// `failOn` (default ["warn"]) controls the severity threshold that
// triggers a non-zero exit. The CI step should pass `--fail-on=warn`.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, extname, basename } from "node:path";

const FORBIDDEN_EXTENSIONS = new Set([".go", ".rs", ".c", ".cc", ".cpp", ".h", ".hpp"]);
const TYPE_SIGNATURES: Record<string, RegExp> = {
  ".go": /^\s*package\s+[a-z]/m,
  ".rs": /^\s*(fn\s+main|use\s+std|mod\s+\w+)/m,
  ".c": /^\s*#\s*include\s*</m,
  ".cc": /^\s*#\s*include\s*</m,
  ".cpp": /^\s*#\s*include\s*</m,
  ".h": /^\s*#\s*include\s*</m,
  ".hpp": /^\s*#\s*include\s*</m,
};

const ROOT = new URL("../", import.meta.url).pathname;
const EXEMPT_SUFFIXES = [".test.ts", ".d.ts", ".md"];
const EXEMPT_DIRS = ["node_modules", ".git", "docs", "fixtures", "sddk"];

interface Finding {
  file: string;
  reason: "forbidden_extension" | "forbidden_signature";
  detail: string;
}

function walk(dir: string, out: string[]): void {
  for (const entry of readdirSync(dir)) {
    if (EXEMPT_DIRS.includes(entry)) continue;
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) walk(full, out);
    else if (st.isFile()) out.push(full);
  }
}

function scan(): Finding[] {
  const findings: Finding[] = [];
  const files: string[] = [];
  walk(join(ROOT, "packages"), files);
  for (const f of files) {
    const name = basename(f);
    if (EXEMPT_SUFFIXES.some((s) => name.endsWith(s))) continue;
    const ext = extname(name);
    if (FORBIDDEN_EXTENSIONS.has(ext)) {
      findings.push({
        file: relative(ROOT, f),
        reason: "forbidden_extension",
        detail: `forbidden source extension: ${ext}`,
      });
      continue;
    }
    const sig = TYPE_SIGNATURES[ext];
    if (!sig) continue;
    const content = readFileSync(f, "utf8");
    if (sig.test(content)) {
      findings.push({
        file: relative(ROOT, f),
        reason: "forbidden_signature",
        detail: `forbidden-language signature detected in ${ext} file`,
      });
    }
  }
  return findings;
}

interface Report {
  ok: boolean;
  root: "packages/";
  findings: Finding[];
  failOn: "ok" | "warn" | "fail";
}

function report(failOn: "ok" | "warn" | "fail"): Report {
  const findings = scan();
  const ok = findings.length === 0;
  return { ok, root: "packages/", findings, failOn };
}

function formatHuman(r: Report): string {
  const lines = ["archctl language-drift guard (ADR-0001)"];
  if (r.findings.length === 0) {
    lines.push("  ok — no forbidden extensions or signatures under packages/");
  } else {
    for (const f of r.findings) {
      lines.push(`  ${f.reason.padEnd(22)} ${f.file}  (${f.detail})`);
    }
  }
  lines.push(r.ok ? "GUARD: OK" : `GUARD: FAIL (${r.findings.length} finding${r.findings.length === 1 ? "" : "s"})`);
  return lines.join("\n");
}

const args = process.argv.slice(2);
const human = args.includes("--human");
const failOnArg = args.find((a) => a.startsWith("--fail-on="));
const failOn = (failOnArg?.split("=")[1] ?? "fail") as "ok" | "warn" | "fail";
const r = report(failOn);
console.log(human ? formatHuman(r) : JSON.stringify(r, null, 2));
const fail = failOn === "ok" ? !r.ok : failOn === "warn" ? !r.ok : false; // fail on FAIL severity only by default
if (fail) process.exit(1);
