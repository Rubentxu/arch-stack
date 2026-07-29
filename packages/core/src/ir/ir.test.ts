import { test } from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { IR_SCHEMA_VERSION, auditIR, buildIR, migrateToCurrent, relationshipId } from "./ir.ts";
import { contentHash, evidenceId } from "../evidence/ledger.ts";
import type { EvidenceRecord } from "../evidence/ledger.ts";

function ev(claim: string, path = "x.go"): EvidenceRecord {
  const partial = {
    type: "source" as const,
    claim,
    source: {
      path,
      startLine: 1,
      endLine: 1,
      revision: { type: "content-hash" as const, value: "blake3:demo" },
      contentHash: contentHash(claim),
      observedAt: "2026-07-29T12:00:00Z",
    },
    extractor: { name: "go-shape-v1", version: "0.1.0" },
    classification: "fact" as const,
    confidence: 0.95,
    method: "heuristic-v1" as const,
  };
  // Derive the id via the same helper used by EvidenceLedger so the lookup
  // matches what buildIR() does in production.
  const id = evidenceId(partial);
  return { id, ...partial };
}

test("auditIR returns ok on a clean v1 IR", () => {
  const ir = {
    schemaVersion: IR_SCHEMA_VERSION,
    sourceIdentitySummary: "dir:/tmp/re",
    elements: [
      {
        id: "container:main-main",
        kind: "container" as const,
        name: "main-main",
        classification: "fact" as const,
        confidence: 0.95,
        method: "heuristic-v1" as const,
        evidenceRefs: ["ev:abc"],
      },
    ],
    relationships: [],
    generatedAt: "2026-07-29T12:00:00Z",
  };
  const r = auditIR(ir);
  assert.equal(r.ok, true);
  assert.equal(r.unsupported.length, 0);
});

test("auditIR flags high-confidence unsupported claims (HARD FAIL)", () => {
  const ir = {
    schemaVersion: IR_SCHEMA_VERSION,
    sourceIdentitySummary: "dir:/tmp/re",
    elements: [
      {
        id: "container:invented",
        kind: "container" as const,
        name: "invented",
        classification: "fact" as const,
        confidence: 0.99,
        method: "heuristic-v1" as const,
        evidenceRefs: [],
      },
    ],
    relationships: [],
    generatedAt: "2026-07-29T12:00:00Z",
  };
  const r = auditIR(ir);
  assert.equal(r.ok, false);
  assert.equal(r.unsupported.length, 1);
  assert.equal(r.unsupported[0]?.id, "container:invented");
});

test("auditIR records but does not block medium-confidence unsupported claims", () => {
  const ir = {
    schemaVersion: IR_SCHEMA_VERSION,
    sourceIdentitySummary: "dir:/tmp/re",
    elements: [
      {
        id: "container:hypothesis-a",
        kind: "container" as const,
        name: "hypothesis-a",
        classification: "unknown" as const,
        confidence: 0.5,
        method: "heuristic-v1" as const,
        evidenceRefs: [],
      },
    ],
    relationships: [],
    generatedAt: "2026-07-29T12:00:00Z",
  };
  const r = auditIR(ir);
  assert.equal(r.ok, true);
  assert.equal(r.lowConfidenceNoEvidence.length, 1);
});

test("auditIR flags unknown methods", () => {
  const ir = {
    schemaVersion: IR_SCHEMA_VERSION,
    sourceIdentitySummary: "dir:/tmp/re",
    elements: [
      {
        id: "container:no-method",
        kind: "container" as const,
        name: "no-method",
        classification: "fact" as const,
        confidence: 0.95,
        // @ts-expect-error — testing runtime guard
        method: "magic-v9",
        evidenceRefs: ["ev:abc"],
      },
    ],
    relationships: [],
    generatedAt: "2026-07-29T12:00:00Z",
  };
  const r = auditIR(ir);
  assert.equal(r.unknownMethod.length, 1);
});

test("auditIR flags wrong schemaVersion", () => {
  const ir = {
    schemaVersion: 99 as unknown as typeof IR_SCHEMA_VERSION,
    sourceIdentitySummary: "dir:/tmp/re",
    elements: [],
    relationships: [],
    generatedAt: "2026-07-29T12:00:00Z",
  };
  const r = auditIR(ir);
  assert.equal(r.ok, false);
  assert.equal(r.wrongVersion?.found, 99);
});

test("buildIR attaches evidence refs by claim+path", () => {
  const e1 = ev("main is a Container", "main.go");
  const ir = buildIR({
    sourceIdentitySummary: "dir:/tmp/re",
    elements: [
      {
        id: "container:main-main",
        kind: "container",
        name: "main-main",
        classification: "fact",
        confidence: 0.95,
        method: "heuristic-v1",
        description: "main is a Container",
      },
    ],
    relationships: [],
    evidence: [e1],
  });
  // buildIR looks up by description (claim text) and falls back to
  // path-keyed lookup if needed. The evidence record's claim matches the
  // element's description, so the ref should attach.
  assert.equal(ir.elements[0]?.evidenceRefs.length, 1);
  assert.equal(ir.elements[0]?.evidenceRefs[0], e1.id);
});

test("migrateToCurrent applies the migrations and reaches schemaVersion 1", () => {
  const raw = {
    schemaVersion: 0,
    sourceIdentitySummary: "legacy",
    elements: [],
    relationships: [],
  };
  const migrated = migrateToCurrent(raw, [
    {
      from: 0,
      to: 1,
      apply: (ir) => {
        const o = ir as { schemaVersion: number };
        return { ...(ir as object), schemaVersion: 1 };
      },
    },
  ]);
  assert.equal(migrated.schemaVersion, 1);
});

test("migrateToCurrent fails loud when no migration path exists", () => {
  assert.throws(() => migrateToCurrent({ schemaVersion: 99, elements: [], relationships: [] }, []), /cannot reach schemaVersion/);
});

test("relationshipId is deterministic and source/target/via-distinguishing", () => {
  const a = relationshipId("a", "b", "imports");
  const b = relationshipId("a", "b", "imports");
  const c = relationshipId("a", "b", "calls");
  assert.equal(a, b);
  assert.notEqual(a, c);
});
