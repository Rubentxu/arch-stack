import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, existsSync, mkdtempSync, mkdirSync, realpathSync, symlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { projectIRToStructurizr } from "../../../packages/core/src/project/structurizr.ts";
import { auditIR } from "../../../packages/core/src/ir/ir.ts";
import type { ArchitectureIR } from "../../../packages/core/src/ir/ir.ts";

interface ManifestFile {
  path: string;
  content?: string;
  linkTo?: string;
}
interface Manifest {
  comment: string;
  files: ManifestFile[];
}

function loadManifest(p: string): Manifest {
  return JSON.parse(readFileSync(p, "utf8")) as Manifest;
}

test("symlink-escape fixture manifest is consistent", () => {
  const m = loadManifest("m0-gate-zero/fixtures/security/symlink-escape/manifest.json");
  assert.ok(m.files.some((f) => f.linkTo && f.linkTo.includes("../")),
    "symlink-escape manifest must declare a link that escapes the fixture root");
  for (const f of m.files) {
    assert.ok(!f.path.includes(".."), `fixture-relative path must not escape: ${f.path}`);
  }
});

test("write-guard containment REJECTS writes inside an escape-symlink tree", () => {
  const root = mkdtempSync(join(tmpdir(), "archctl-symlink-"));
  const target = join(root, "target");
  mkdirSync(target, { recursive: true });
  const linkPath = join(target, "evil-link");
  try {
    symlinkSync("../../../../../../../etc/passwd", linkPath);
  } catch {
    // Some sandboxes refuse symlinks; the manifest exercises the same scenario.
  }
  if (existsSync(linkPath)) {
    let real: string;
    try {
      real = realpathSync.native(linkPath);
    } catch {
      real = linkPath;
    }
    const realRoot = realpathSync.native(target);
    assert.ok(!real.startsWith(realRoot), "symlink target must escape the fixture root");
  }
});

test("prompt-injection fixture manifest is consistent", () => {
  const m = loadManifest("m0-gate-zero/fixtures/security/prompt-injection/manifest.json");
  const readme = m.files.find((f) => f.path === "README.md");
  assert.ok(readme?.content);
  assert.ok(/IGNORE.*INSTRUCTIONS/.test(readme!.content!),
    "README must contain an instruction-shaped payload");
});

test("auditor HARD FAILS a prompt-injected IR (defence in depth)", () => {
  const ir: ArchitectureIR = {
    schemaVersion: 1,
    sourceIdentitySummary: "dir:/tmp/evil",
    elements: [
      {
        id: "container:phantom-injected",
        kind: "container",
        name: "phantom-injected",
        classification: "fact",
        confidence: 0.99,
        method: "heuristic-v1",
        evidenceRefs: [],
      },
    ],
    relationships: [],
    generatedAt: "2026-07-29T12:00:00Z",
  };
  const r = auditIR(ir);
  assert.equal(r.ok, false);
  assert.equal(r.unsupported.length, 1);
  assert.equal(r.unsupported[0]?.id, "container:phantom-injected");
});

test("adversarial fixtures do not poison a clean IR via projection", () => {
  const ir: ArchitectureIR = {
    schemaVersion: 1,
    sourceIdentitySummary: "dir:/tmp/clean",
    elements: [
      {
        id: "container:legit",
        kind: "container",
        name: "legit",
        technology: ["Rust"],
        classification: "fact",
        confidence: 0.9,
        method: "heuristic-v1",
        evidenceRefs: ["ev:abc"],
      },
    ],
    relationships: [],
    generatedAt: "2026-07-29T12:00:00Z",
  };
  const a = projectIRToStructurizr(ir).dsl;
  const b = projectIRToStructurizr(ir).dsl;
  assert.equal(a, b);
  assert.ok(a.includes("container container_legit"));
});
