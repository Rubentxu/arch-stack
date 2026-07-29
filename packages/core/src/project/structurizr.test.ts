import { test } from "node:test";
import assert from "node:assert/strict";
import { projectIRToStructurizr, projectElement, projectRelationship } from "./structurizr.ts";
import type { ArchitectureIR } from "../ir/ir.ts";

const sampleIR: ArchitectureIR = {
  schemaVersion: 1,
  sourceIdentitySummary: "dir:/tmp/re",
  elements: [
    {
      id: "container:orders-service",
      kind: "container",
      name: "orders-service",
      technology: ["Rust", "Axum"],
      description: "Orders HTTP service",
      classification: "fact",
      confidence: 0.92,
      method: "heuristic-v1",
      evidenceRefs: ["ev:abc"],
    },
    {
      id: "softwareSystem:checkout",
      kind: "softwareSystem",
      name: "checkout",
      classification: "fact",
      confidence: 0.9,
      method: "heuristic-v1",
      evidenceRefs: ["ev:def"],
    },
  ],
  relationships: [
    {
      id: "rel:12345678",
      source: "container:orders-service",
      target: "softwareSystem:checkout",
      via: "imports",
      technology: "HTTP",
      description: "orders calls checkout",
      classification: "fact",
      confidence: 0.9,
      method: "heuristic-v1",
      evidenceRefs: ["ev:abc"],
    },
  ],
  generatedAt: "2026-07-29T12:00:00Z",
};

test("projection emits a workspace DSL with the IR's elements and relationships", () => {
  const { dsl, warnings } = projectIRToStructurizr(sampleIR);
  assert.equal(warnings.length, 0);
  assert.ok(dsl.includes('workspace "archctl"'));
  // Canonical Structurizr DSL grammar: `<kind> <id> "<name>" "<desc>"`.
  // Description carries the technology suffix in brackets so the rendered
  // diagram still shows the technology without inventing a new DSL slot.
  assert.ok(dsl.includes('container container_orders-service "Orders Service" "Orders HTTP service [Rust, Axum]"'));
  assert.ok(dsl.includes('softwaresystem softwareSystem_checkout "Checkout" ""'));
  assert.ok(dsl.includes('container_orders-service -> softwareSystem_checkout "orders calls checkout" "HTTP"'));
  // Stable identifier: re-projecting the same IR yields the same DSL.
  const again = projectIRToStructurizr(sampleIR);
  assert.equal(again.dsl, dsl);
});

test("projection is pure: same IR ⇒ identical DSL byte-for-byte", () => {
  const a = projectIRToStructurizr(sampleIR).dsl;
  const b = projectIRToStructurizr(sampleIR).dsl;
  assert.equal(a, b);
});

test("projection handles empty IR gracefully", () => {
  const empty: ArchitectureIR = {
    schemaVersion: 1,
    sourceIdentitySummary: "dir:/empty",
    elements: [],
    relationships: [],
    generatedAt: "2026-07-29T12:00:00Z",
  };
  const { dsl } = projectIRToStructurizr(empty);
  assert.ok(dsl.includes("workspace"));
  assert.ok(!dsl.includes("container "));
  assert.ok(!dsl.includes(" -> "));
});

test("projection emits tags after description when tags are present", () => {
  const ir: ArchitectureIR = {
    schemaVersion: 1,
    sourceIdentitySummary: "dir:/tmp/re",
    elements: [
      {
        id: "container:tagged",
        kind: "container",
        name: "tagged",
        tags: ["api", "v2"],
        classification: "fact",
        confidence: 0.9,
        method: "heuristic-v1",
        evidenceRefs: ["ev:abc"],
      },
    ],
    relationships: [],
    generatedAt: "2026-07-29T12:00:00Z",
  };
  const { dsl } = projectIRToStructurizr(ir);
  // Tags are emitted as positional quoted strings after the description.
  assert.match(dsl, /container container_tagged "Tagged" "" "api", "v2"/);
});

test("projectElement and projectRelationship are independently callable", () => {
  assert.match(projectElement(sampleIR.elements[0]!), /^container container_orders-service/);
  assert.match(projectRelationship(sampleIR.relationships[0]!), /container_orders-service -> softwareSystem_checkout/);
});
