import { test } from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { auditRun, persistAudit, rawToEvidenceRecord } from "./auditor.ts";
import { contentHash } from "../evidence/ledger.ts";
import type { ArchitectureIR } from "../ir/ir.ts";
import type { EvidenceRecord } from "../evidence/ledger.ts";

function sampleRecord(claim: string, path: string): EvidenceRecord {
  const partial: Omit<EvidenceRecord, "id"> = {
    type: "source",
    claim,
    source: {
      path,
      startLine: 1,
      endLine: 1,
      revision: { type: "content-hash", value: "blake3:demo" },
      contentHash: contentHash(claim),
      observedAt: "2026-07-29T12:00:00Z",
    },
    extractor: { name: "go-shape-v1", version: "0.1.0" },
    classification: "fact",
    confidence: 0.92,
    method: "heuristic-v1",
  };
  const id = `ev:${createHash("sha256").update(JSON.stringify(partial)).digest("hex").slice(0, 16)}`;
  return { id, ...partial };
}

function sampleIR(refs: string[]): ArchitectureIR {
  return {
    schemaVersion: 1,
    sourceIdentitySummary: "dir:/tmp/re",
    elements: [
      {
        id: "container:orders-service",
        kind: "container",
        name: "orders-service",
        classification: "fact",
        confidence: 0.92,
        method: "heuristic-v1",
        evidenceRefs: refs,
      },
    ],
    relationships: [],
    generatedAt: "2026-07-29T12:00:00Z",
  };
}

test("auditRun PASSES when IR references are satisfied by the ledger", () => {
  const root = mkdtempSync(join(tmpdir(), "archctl-auditor-"));
  const e = sampleRecord("orders.Service exists", "internal/orders/service.go");
  const ir = sampleIR([e.id]);
  const r = auditRun({ runId: "run", xdgDataDir: root, projectId: "proj", ir, evidence: [e] });
  assert.equal(r.ok, true);
  assert.equal(r.ledger.wrote, 1);
  assert.equal(r.irAudit.unsupported.length, 0);
});

test("auditRun FAILS when IR has unsupported high-confidence claims", () => {
  const root = mkdtempSync(join(tmpdir(), "archctl-auditor-"));
  const ir = sampleIR([]); // zero evidence refs → HARD FAIL
  const r = auditRun({ runId: "run", xdgDataDir: root, projectId: "proj", ir, evidence: [] });
  assert.equal(r.ok, false);
  assert.equal(r.irAudit.unsupported.length, 1);
});

test("auditRun rejects IR references that do not match any ledger record (data-not-instructions test)", () => {
  const root = mkdtempSync(join(tmpdir(), "archctl-auditor-"));
  const e = sampleRecord("orders.Service exists", "internal/orders/service.go");
  const ir = sampleIR(["ev:0000000000000000"]); // bogus ref → dangling
  const r = auditRun({ runId: "run", xdgDataDir: root, projectId: "proj", ir, evidence: [e] });
  // After dangling-filter the element has zero refs → still high-confidence
  // unsupported → HARD FAIL.
  assert.equal(r.ok, false);
});

test("persistAudit writes ir.json under XDG", () => {
  const root = mkdtempSync(join(tmpdir(), "archctl-auditor-"));
  const ir = sampleIR([]);
  const { irPath } = persistAudit({ runId: "run", xdgDataDir: root, projectId: "proj", ir });
  assert.ok(existsSync(irPath));
  const parsed = JSON.parse(readFileSync(irPath, "utf8"));
  assert.equal(parsed.schemaVersion, 1);
});

test("rawToEvidenceRecord preserves structural fields and adds extractor/revision", () => {
  const raw = {
    type: "ast" as const,
    claim: "type Service struct",
    source: { path: "x.go", startLine: 1, endLine: 1 },
    classification: "fact" as const,
    confidence: 0.9,
    method: "heuristic-v1" as const,
  };
  const rec = rawToEvidenceRecord(raw, {
    extractorName: "go-shape-v1",
    extractorVersion: "0.1.0",
    revision: { type: "content-hash" as const, value: "blake3:demo" },
    observedAt: "2026-07-29T12:00:00Z",
    contentHash: "blake3:deadbeef",
  });
  assert.equal(rec.extractor.name, "go-shape-v1");
  assert.equal(rec.source.revision.value, "blake3:demo");
  assert.equal(rec.classification, "fact");
  // No repo text can override these fields downstream.
  assert.equal(rec.method, "heuristic-v1");
});
