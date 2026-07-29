#!/usr/bin/env tsx
// ADR-0005 — archctl render CLI.
//
// Headless render entry point: writes the IR projections (workspace.dsl,
// diagram.puml) into the per-run XDG diagrams directory and, when --images
// is set, renders the PUML via the local Kroki /plantuml/png endpoint.
import { renderDiagrams } from "./render.ts";
import type { ArchitectureIR } from "../../core/src/ir/ir.ts";
import { resolveXdg } from "../../core/src/xdg.ts";

function arg(name: string, fallback?: string): string | undefined {
  const i = process.argv.indexOf(`--${name}`);
  return i >= 0 ? process.argv[i + 1] : fallback;
}

const runId = arg("runId") ?? "default-run";
const projectId = arg("project", "default");
const renderImages = process.argv.includes("--images");
const structurizrUrl = arg("structurizr-url", "http://localhost:18080");
const krokiUrl = arg("kroki-url", "http://localhost:18000");

const ir: ArchitectureIR = {
  schemaVersion: 1,
  sourceIdentitySummary: "dir:/tmp/cli-render-demo",
  elements: [
    { id: "container:api", kind: "container", name: "api", technology: ["Rust"], classification: "fact", confidence: 0.92, method: "heuristic-v1", evidenceRefs: ["ev:a"] },
    { id: "container:db", kind: "container", name: "db", technology: ["PostgreSQL"], classification: "fact", confidence: 0.9, method: "heuristic-v1", evidenceRefs: ["ev:b"] },
  ],
  relationships: [{ id: "rel:1", source: "container:api", target: "container:db", description: "reads/writes", classification: "fact", confidence: 0.9, method: "heuristic-v1", evidenceRefs: ["ev:a"] }],
  generatedAt: new Date().toISOString(),
};

const layout = resolveXdg();
const r = renderDiagrams({
  ir,
  xdgDataDir: layout.data,
  projectId,
  runId,
  renderImages,
  structurizrUrl,
  krokiUrl,
});
console.log(JSON.stringify(r, null, 2));
process.exit(renderImages && r.images.plantumlPng?.ok === false ? 1 : 0);
