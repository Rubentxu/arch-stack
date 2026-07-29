// 1.8 — Auditor.
//
// Connects the IR-side strict boundary (auditIR) to the runtime boundary:
// every RawEvidence becomes an EvidenceRecord in the ledger, and every
// IR passed to audit() is rejected at HARD FAIL when high-confidence
// claims lack evidence. This module is the runnable enforcement of ADR-0004
// and ADR-0008 (data-not-instructions is implicit: only structural fields
// of records flow into IR; repo text never assigns classification/confidence).
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { RawEvidence } from "../router/router.ts";
import type { ArchitectureIR, IRElement, IRRelationship } from "../ir/ir.ts";
import { auditIR } from "../ir/ir.ts";
import { EvidenceLedger } from "../evidence/ledger.ts";
import type { EvidenceRecord } from "../evidence/ledger.ts";

export interface AuditRunOptions {
  runId: string;
  xdgDataDir: string;
  projectId: string;
  ir: ArchitectureIR;
  evidence: EvidenceRecord[];
}

export interface AuditRunResult {
  ok: boolean;
  ledger: { wrote: number };
  irAudit: ReturnType<typeof auditIR>;
  hook: { unsupportedEmitted: number };
  recorder: (rec: Omit<EvidenceRecord, "id">) => EvidenceRecord;
}

/**
 * Run the auditor end-to-end:
 *   1. Persist every evidence record via the ledger (records are the input to
 *      `auditIR`; the ledger is the only store of provenance).
 *   2. Verify every IR element/relationship references an existing ledger
 *      record (the data-not-instructions test for hook layers downstream).
 *   3. Run `auditIR` — the strict boundary.
 *
 * Returns `ok = false` if the IR has unsupported high-confidence claims.
 */
export function auditRun(opts: AuditRunOptions): AuditRunResult {
  const ledger = new EvidenceLedger({ runId: opts.runId, xdgDataDir: opts.xdgDataDir, projectId: opts.projectId });
  let wrote = 0;
  const recorder = (rec: Omit<EvidenceRecord, "id">): EvidenceRecord => {
    wrote++;
    return ledger.record(rec);
  };
  for (const ev of opts.evidence) recorder(ev);

  const known = new Set(opts.evidence.map((e) => e.id));
  let dangling = 0;
  const elements: IRElement[] = opts.ir.elements.map((e) => ({ ...e }));
  const relationships: IRRelationship[] = opts.ir.relationships.map((r) => ({ ...r }));
  for (const e of elements) {
    e.evidenceRefs = e.evidenceRefs.filter((ref) => {
      const ok = known.has(ref);
      if (!ok) dangling++;
      return ok;
    });
  }
  for (const r of relationships) {
    r.evidenceRefs = r.evidenceRefs.filter((ref) => {
      const ok = known.has(ref);
      if (!ok) dangling++;
      return ok;
    });
  }

  const irAudit = auditIR({ ...opts.ir, elements, relationships });
  return {
    ok: irAudit.ok && dangling === 0,
    ledger: { wrote },
    irAudit,
    hook: { unsupportedEmitted: irAudit.unsupported.length },
    recorder,
  };
}

/**
 * Convenience: convert RawEvidence into the minimal EvidenceRecord form.
 * The router produces `RawEvidence` (no id, no contentHash) — the auditor
 * adds contentHash + id and persists via the ledger.
 */
export function rawToEvidenceRecord(
  raw: RawEvidence,
  opts: {
    extractorName: string;
    extractorVersion: string;
    revision: { type: "git-commit" | "content-hash"; value: string };
    observedAt: string;
    contentHash: string;
  },
): Omit<EvidenceRecord, "id"> {
  return {
    type: raw.type,
    claim: raw.claim,
    source: {
      path: raw.source.path,
      startLine: raw.source.startLine,
      endLine: raw.source.endLine,
      revision: opts.revision,
      contentHash: opts.contentHash,
      observedAt: opts.observedAt,
    },
    extractor: { name: opts.extractorName, version: opts.extractorVersion },
    classification: raw.classification,
    confidence: raw.confidence,
    method: raw.method,
  };
}

/**
 * Persist IR under XDG (the per-run segment). Idempotent.
 */
export function persistAudit(opts: {
  xdgDataDir: string;
  projectId: string;
  runId: string;
  ir: ArchitectureIR;
}): { irPath: string } {
  const projectDir = join(opts.xdgDataDir, "projects", opts.projectId);
  const runDir = join(projectDir, "runs", opts.runId);
  mkdirSync(runDir, { recursive: true });
  const irPath = join(runDir, "ir.json");
  writeFileSync(irPath, JSON.stringify(opts.ir, null, 2));
  return { irPath };
}
