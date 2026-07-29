import { test } from "node:test";
import assert from "node:assert/strict";
import { projectIRToPlantUML, projectPUmlElement, projectPUmlRelationship } from "./plantuml.ts";
import type { ArchitectureIR } from "../ir/ir.ts";

const sampleIR: ArchitectureIR = {
  schemaVersion: 1,
  sourceIdentitySummary: "dir:/tmp/re",
  elements: [
    {
      id: "container:orders-service",
      kind: "container",
      name: "orders-service",
      technology: ["Rust"],
      description: "Orders HTTP service",
      classification: "fact",
      confidence: 0.92,
      method: "heuristic-v1",
      evidenceRefs: ["ev:abc"],
    },
    {
      id: "person:customer",
      kind: "person",
      name: "customer",
      classification: "fact",
      confidence: 0.95,
      method: "heuristic-v1",
      evidenceRefs: ["ev:def"],
    },
  ],
  relationships: [
    {
      id: "rel:12345678",
      source: "person:customer",
      target: "container:orders-service",
      via: "HTTP",
      description: "uses",
      classification: "fact",
      confidence: 0.95,
      method: "heuristic-v1",
      evidenceRefs: ["ev:def"],
    },
  ],
  generatedAt: "2026-07-29T12:00:00Z",
};

test("projection emits a PlantUML sequence diagram with @startuml/@enduml", () => {
  const puml = projectIRToPlantUML(sampleIR);
  assert.ok(puml.startsWith("@startuml"));
  assert.ok(puml.endsWith("@enduml"));
  // Participant display label must be quoted; the alias follows `as`.
  assert.match(puml, /participant "orders-service \[Rust\]" as container_orders-service/);
  assert.match(puml, /participant "customer" as person_customer/);
  assert.match(puml, /person_customer -> container_orders-service : uses/);
});

test("projection uses standard PlantUML only (no remote !include, no C4 macros)", () => {
  const puml = projectIRToPlantUML(sampleIR);
  assert.doesNotMatch(puml, /^!include .*$/m);
  assert.doesNotMatch(puml, /!define\s+/);
});
