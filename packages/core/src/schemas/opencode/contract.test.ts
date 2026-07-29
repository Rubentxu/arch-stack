// ADR-0007 — TS schema-contract test against the vendored OpenCode snapshot.
// The contract enforces two drift axes:
//   1. OpenCode schema drift  — used keys must remain in the pinned snapshot.
//   2. Implementation drift  — no forbidden compilation-unit signatures under
//      `packages/` (ADR-0001, language-drift guard).
//
// Both checks fail the build on regression. The full language-drift script
// (task 2.16) replaces this guard; until then the inline signature check is
// the single source of truth and is referenced by task 1.2 acceptance.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative, extname, basename } from "node:path";

const ROOT = new URL("../../../../../", import.meta.url).pathname;

interface Snapshot {
  $schema?: string;
  mcp?: unknown;
  skills?: unknown;
  references?: unknown;
  plugin?: unknown;
  subagent_depth?: number;
  permission?: unknown;
  compaction?: unknown;
  experimental?: unknown;
}

function readSnapshot(): Snapshot {
  const dir = readdirSync(join(ROOT, "schemas/opencode"));
  const pin = dir.find((d) => d.startsWith("1.")) ?? "1.18.x";
  const path = join(ROOT, "schemas/opencode", pin, "config.json");
  return JSON.parse(readFileSync(path, "utf8")) as Snapshot;
}

test("vendored OpenCode schema snapshot exposes the keys archctl relies on", () => {
  const s = readSnapshot();
  // Top-level `mcp` MUST exist (NOT `mcpServers`).
  assert.ok("mcp" in s, "vendored snapshot must declare top-level 'mcp'");
  assert.ok(!("mcpServers" in s), "vendored snapshot must NOT use the legacy 'mcpServers' key");
  // The seven archctl-relevant top-level keys must all be present.
  for (const key of ["mcp", "skills", "references", "plugin", "permission", "compaction", "experimental"]) {
    assert.ok(key in s, `vendored snapshot missing key '${key}'`);
  }
  // subagent_depth must be an integer >= 0.
  assert.equal(typeof s.subagent_depth, "number");
  assert.ok((s.subagent_depth ?? 0) >= 0);
});

test("compaction is top-level, not inside experimental", () => {
  const s = readSnapshot();
  assert.ok("compaction" in s, "compaction must be a top-level key");
  assert.ok(s.experimental && typeof s.experimental === "object", "experimental must be an object");
  assert.ok(!("session.compacting" in (s.experimental as Record<string, unknown>)), "experimental.session.compacting must NOT be a config key; it is a plugin hook");
});

const FORBIDDEN_SIGS: Record<string, RegExp> = {
  ".go": /^\s*package\s+[a-z]/m,
  ".rs": /^\s*(fn\s+main|use\s+std|mod\s+\w+)/m,
  ".c": /^\s*#\s*include\s*</m,
  ".cc": /^\s*#\s*include\s*</m,
  ".cpp": /^\s*#\s*include\s*</m,
};

function walkPackages(): string[] {
  const out: string[] = [];
  function walk(dir: string): void {
    for (const entry of readdirSync(dir)) {
      const full = join(dir, entry);
      const st = statSync(full);
      if (st.isDirectory()) walk(full);
      else if (st.isFile()) out.push(full);
    }
  }
  walk(join(ROOT, "packages"));
  return out;
}

test("no forbidden compilation-unit signatures under packages/ (ADR-0001 drift)", () => {
  const offenders: string[] = [];
  for (const f of walkPackages()) {
    const name = basename(f);
    if (name.endsWith(".test.ts") || name.endsWith(".md")) continue;
    const ext = extname(f);
    const sig = FORBIDDEN_SIGS[ext];
    if (!sig) continue;
    const content = readFileSync(f, "utf8");
    if (sig.test(content)) offenders.push(relative(ROOT, f));
  }
  assert.deepEqual(offenders, [], `forbidden-language signatures found: ${offenders.join(", ")}`);
});
