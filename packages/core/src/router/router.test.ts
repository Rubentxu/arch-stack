import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, chmodSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  CapabilityRouter,
  shellAdapterFromDescriptor,
  probeAdapterRequirements,
  type ShellAdapterDescriptor,
} from "./router.ts";

test("capability router resolves a registered capability", () => {
  const r = new CapabilityRouter();
  const a = shellAdapterFromDescriptor({
    capability: "extract.imports",
    name: "noop",
    version: "0.1.0",
    command: ["true"],
    requires: { binaries: [] },
    output: { type: "import", pattern: "(.+)", classification: "fact", confidence: 0.9, method: "heuristic-v1" },
  });
  r.register(a);
  assert.equal(r.list().length, 1);
  assert.equal(r.resolve("extract.imports").name, "noop");
});

test("router rejects duplicate capability registration", () => {
  const r = new CapabilityRouter();
  const a = shellAdapterFromDescriptor({
    capability: "x",
    name: "a",
    version: "0",
    command: ["true"],
    requires: { binaries: [] },
    output: { type: "import", pattern: "(.+)", classification: "fact", confidence: 0.9, method: "heuristic-v1" },
  });
  r.register(a);
  assert.throws(() => r.register({ ...a, name: "b" }), /already registered/);
});

test("router throws on unknown capability", () => {
  const r = new CapabilityRouter();
  assert.throws(() => r.resolve("nope"), /unknown capability/);
});

test("shell adapter parses stdout lines into RawEvidence", async () => {
  const dir = mkdtempSync(join(tmpdir(), "archctl-router-"));
  writeFileSync(join(dir, "run.sh"), `#!/bin/sh\nprintf 'main.go:1:package main\\nmain.go:2:func main() {}\\n'`);
  chmodSync(join(dir, "run.sh"), 0o755);
  const desc: ShellAdapterDescriptor = {
    capability: "extract.outline",
    name: "fake-outline",
    version: "0.1.0",
    command: [join(dir, "run.sh")],
    requires: { binaries: [] },
    output: {
      type: "ast",
      pattern: "^([^:]+):(\\d+):(.+)$",
      classification: "fact",
      confidence: 0.9,
      method: "heuristic-v1",
    },
  };
  const a = shellAdapterFromDescriptor(desc);
  const out = await a.run({
    repoRoot: dir,
    revision: { type: "content-hash", value: "blake3:demo" },
    observedAt: "2026-07-29T12:00:00Z",
    timeoutMs: 5000,
    capability: "extract.outline",
  });
  assert.equal(out.length, 2);
  assert.equal(out[0]?.source.path, "main.go");
  assert.equal(out[0]?.source.startLine, 1);
});

test("probeAdapterRequirements reports missing binaries", () => {
  const r = probeAdapterRequirements({ binaries: ["definitely-not-a-real-binary-xyz"] });
  assert.equal(r.ok, false);
  assert.ok(r.missing.includes("definitely-not-a-real-binary-xyz"));
});
