import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { renderDiagrams } from "./render.ts";
import type { ArchitectureIR } from "../ir/ir.ts";

function sampleIR(): ArchitectureIR {
  return {
    schemaVersion: 1,
    sourceIdentitySummary: "dir:/tmp/render-test",
    elements: [
      { id: "container:api", kind: "container", name: "api", technology: ["Rust"], classification: "fact", confidence: 0.92, method: "heuristic-v1", evidenceRefs: ["ev:a"] },
      { id: "container:db", kind: "container", name: "db", technology: ["PostgreSQL"], classification: "fact", confidence: 0.9, method: "heuristic-v1", evidenceRefs: ["ev:b"] },
    ],
    relationships: [{ id: "rel:1", source: "container:api", target: "container:db", via: "SQL", description: "reads/writes", classification: "fact", confidence: 0.9, method: "heuristic-v1", evidenceRefs: ["ev:a"] }],
    generatedAt: "2026-07-29T12:00:00Z",
  };
}

test("renderDiagrams writes DSL + PlantUML even without renderers running", () => {
  const root = mkdtempSync(join(tmpdir(), "archctl-render-"));
  const r = renderDiagrams({ ir: sampleIR(), xdgDataDir: root, projectId: "proj", runId: "run-001" });
  assert.ok(existsSync(r.dsl.path), "workspace.dsl must be written");
  assert.ok(existsSync(r.puml.path), "diagram.puml must be written");
  assert.ok(r.dsl.bytes > 0);
  assert.ok(r.puml.bytes > 0);
  const dsl = readFileSync(r.dsl.path, "utf8");
  assert.ok(dsl.includes("container container_api"));
  const puml = readFileSync(r.puml.path, "utf8");
  // PlantUML sequence diagram: `participant "label [tech]" as alias`.
  assert.match(puml, /participant "api \[Rust\]" as container_api/);
});

test("renderDiagrams produces a render-success 100% when the PlantUML renderer is reachable", () => {
  // The Structurizr `local` HTTP server is a viewer (no DSL upload endpoint).
  // Headless C4 rendering goes through Kroki's `/plantuml/png` endpoint
  // (well-supported) and kroki's bundled structurizr (strict grammar). For
  // the test we only require the PUML render to succeed; the DSL render is
  // asserted separately if Kroki's structurizr endpoint can parse our DSL.
  const krokiOk = spawnSync("curl", ["-sf", "-o", "/dev/null", "http://localhost:18000/"], { encoding: "utf8" }).status === 0;
  if (!krokiOk) {
    console.log("[skip] kroki not reachable; skipping render-image test");
    return;
  }
  const root = mkdtempSync(join(tmpdir(), "archctl-render-"));
  const r = renderDiagrams({
    ir: sampleIR(),
    xdgDataDir: root,
    projectId: "proj",
    runId: "run-002",
    renderImages: true,
    structurizrUrl: "http://localhost:18080",
    krokiUrl: "http://localhost:18000",
  });
  assert.ok(r.images.plantumlPng?.ok, "kroki plantuml render must succeed");
  assert.ok(r.images.plantumlPng?.bytes && r.images.plantumlPng.bytes > 0);
});
