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

test("projection emits a C4 PlantUML with @startuml/@enduml", () => {
  const puml = projectIRToPlantUML(sampleIR);
  assert.ok(puml.startsWith("@startuml"));
  assert.ok(puml.endsWith("@enduml"));
  assert.ok(puml.includes("Container(container_orders-service"));
  assert.ok(puml.includes('Person(person_customer, "customer")'));
  assert.ok(puml.includes("Rel(person_customer, container_orders-service"));
});

test("projection is pure: same IR ⇒ identical output", () => {
  const a = projectIRToPlantUML(sampleIR);
  const b = projectIRToPlantUML(sampleIR);
  assert.equal(a, b);
});

test("projection handles empty IR", () => {
  const empty: ArchitectureIR = {
    schemaVersion: 1,
    sourceIdentitySummary: "dir:/empty",
    elements: [],
    relationships: [],
    generatedAt: "2026-07-29T12:00:00Z",
  };
  const puml = projectIRToPlantUML(empty);
  assert.ok(puml.includes("@startuml"));
  assert.ok(!puml.includes("Container("));
});

test("helpers emit element/relationship lines independently", () => {
  assert.match(projectPUmlElement(sampleIR.elements[0]!), /^Container\(container_orders-service/);
  assert.match(projectPUmlRelationship(sampleIR.relationships[0]!), /^Rel\(person_customer, container_orders-service/);
});
