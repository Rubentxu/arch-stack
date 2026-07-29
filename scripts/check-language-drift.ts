// ADR-0001 enforcement — M2 will create the real script and wire it into CI.
// Until then, this tiny in-test shim is the *only* script referenced by the
// language-drift guard acceptance criteria (tasks 1.2). It must:
//   - emit JSON+human output;
//   - exit non-zero on any forbidden extension under `packages/`.
//
// The shim checks the *content* of each file for compilation-unit signatures
// of forbidden languages (e.g. `package main` only counts as Go when the file
// is actually named `*.go`). Markdown is always allowed.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, extname, basename } from "node:path";

const FORBIDDEN: Record<string, RegExp> = {
  ".go": /^\s*package\s+[a-z]/m,
  ".rs": /^\s*(fn\s+main|use\s+std|mod\s+\w+)/m,
  ".c": /^\s*#\s*include\s*</m,
  ".cc": /^\s*#\s*include\s*</m,
  ".cpp": /^\s*#\s*include\s*</m,
};
const ROOT = new URL("../", import.meta.url).pathname;

type Finding = { file: string; ext: string };

function walk(dir: string, out: string[]): void {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) walk(full, out);
    else if (st.isFile()) out.push(full);
  }
}

const files: string[] = [];
walk(join(ROOT, "packages"), files);
const findings: Finding[] = [];
for (const f of files) {
  const name = basename(f);
  // Test files (*.test.ts) and markdown documentation are exempt — they are
  // explicitly part of the TypeScript toolchain.
  if (name.endsWith(".test.ts") || name.endsWith(".md")) continue;
  const ext = extname(f);
  const sig = FORBIDDEN[ext];
  if (!sig) continue;
  const content = readFileSync(f, "utf8");
  if (sig.test(content)) {
    findings.push({ file: relative(ROOT, f), ext });
  }
}

const report = {
  ok: findings.length === 0,
  root: "packages/",
  forbidden: Object.keys(FORBIDDEN),
  findings,
  note: "Placeholder enforcement created in WU1; replaced by the full language-drift guard in task 2.16.",
};

console.log(JSON.stringify(report, null, 2));
if (!report.ok) process.exit(1);
