import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { EvidenceLedger, contentHash, readLedger } from "./ledger.ts";

function sampleEvidence() {
  return {
    type: "source" as const,
    claim: "orders.Service is a Container candidate",
    source: {
      path: "internal/orders/service.go",
      startLine: 1,
      endLine: 7,
      revision: { type: "content-hash" as const, value: "blake3:deadbeef" },
      contentHash: contentHash("package orders\n\ntype Service struct{}\n"),
      observedAt: "2026-07-29T12:00:00Z",
    },
    extractor: { name: "go-shape-v1", version: "0.1.0" },
    classification: "fact" as const,
    confidence: 0.92,
    method: "heuristic-v1" as const,
  };
}

test("record() persists a single JSONL line and assigns a deterministic id", () => {
  const root = mkdtempSync(join(tmpdir(), "archctl-ledger-"));
  const ledger = new EvidenceLedger({ runId: "run-001", xdgDataDir: root, projectId: "proj" });
  const rec = ledger.record(sampleEvidence());
  assert.ok(rec.id.startsWith("ev:"));
  assert.equal(rec.id.length, "ev:".length + 16);
  // Reading the segment directly confirms the on-disk shape.
  const lines = readFileSync(ledger.segmentPath, "utf8").split("\n").filter((l) => l.length > 0);
  assert.equal(lines.length, 1);
  const parsed = JSON.parse(lines[0]!);
  assert.equal(parsed.id, rec.id);
});

test("contentHash is stable across calls (BLAKE-style tag)", () => {
  assert.equal(contentHash("hello"), contentHash("hello"));
  assert.notEqual(contentHash("hello"), contentHash("world"));
});

test("readLedger returns the same records written by record()", () => {
  const root = mkdtempSync(join(tmpdir(), "archctl-ledger-"));
  const ledger = new EvidenceLedger({ runId: "run-002", xdgDataDir: root, projectId: "proj" });
  const a = ledger.record(sampleEvidence());
  const b = ledger.record({ ...sampleEvidence(), claim: "another claim" });
  const all = readLedger({ runId: "run-002", xdgDataDir: root, projectId: "proj" });
  assert.equal(all.length, 2);
  assert.deepEqual(all.map((r) => r.id).sort(), [a.id, b.id].sort());
});

test("readLedger on a non-existent run returns []", () => {
  const root = mkdtempSync(join(tmpdir(), "archctl-ledger-"));
  const all = readLedger({ runId: "never", xdgDataDir: root, projectId: "proj" });
  assert.deepEqual(all, []);
  assert.ok(!existsSync(join(root, "projects", "proj", "runs", "never", "evidence.jsonl")));
});

test("confidence 0.95 with no evidence refs would fail downstream — ledger does not gate writes (auditor does)", () => {
  // The ledger records *what was observed*. The auditor (1.8) is the gate that
  // rejects high-confidence claims with zero evidence refs. We document the
  // boundary here so future readers do not expect the ledger to enforce it.
  const root = mkdtempSync(join(tmpdir(), "archctl-ledger-"));
  const ledger = new EvidenceLedger({ runId: "run-003", xdgDataDir: root, projectId: "proj" });
  const rec = ledger.record({ ...sampleEvidence(), confidence: 0.97, classification: "fact" });
  assert.equal(rec.confidence, 0.97);
  // Auditor responsibility — exercised in 1.8.
});
