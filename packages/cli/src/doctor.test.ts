import { test } from "node:test";
import assert from "node:assert/strict";
import { runDoctor } from "./doctor.ts";

test("doctor returns a structured report with the expected finding ids", () => {
  const r = runDoctor();
  assert.equal(typeof r.ok, "boolean");
  assert.ok(Array.isArray(r.findings));
  for (const id of [
    "renderer.structurizr",
    "renderer.kroki",
    "opencode.snapshot",
    "opencode.cli",
  ]) {
    assert.ok(r.findings.some((f) => f.id === id), `finding ${id} must be present`);
  }
  assert.ok(Array.isArray(r.inventory.adapters));
  assert.ok(Array.isArray(r.inventory.binaries));
  // Default router registers the two fast-profile descriptors shipped in WU4.
  assert.ok(r.inventory.adapters.some((a) => a.capability === "extract.outline"));
  assert.ok(r.inventory.adapters.some((a) => a.capability === "extract.symbols"));
});

test("doctor never throws even when the environment is missing binaries", () => {
  // The function must be best-effort and never throw on missing tools.
  const r = runDoctor();
  assert.ok(r.findings.length > 0);
});
