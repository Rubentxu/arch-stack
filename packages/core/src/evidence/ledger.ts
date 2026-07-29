// ADR-0004 — evidence ledger (append-only, single-writer per run).
//
// The ledger is the project's source of provenance. Every element/relationship
// in the Architecture IR MUST carry a `evidenceRefs: [...]` pointing at ledger
// records. To prevent the "draw a Mermaid" failure mode (ADR-0004 hard fail),
// every record here is reproducible: it carries the extractor version, the
// source revision (git commit OR content snapshot), the observed-at timestamp,
// and a content hash of the observed slice.

import { createHash } from "node:crypto";
import { mkdirSync, appendFileSync, writeFileSync, readFileSync, existsSync } from "node:fs";
import { join } from "node:path";

export type EvidenceClassification =
  | "fact"
  | "inference"
  | "hypothesis"
  | "unknown"
  | "conflict";

export type EvidenceMethod =
  | "heuristic-v1"
  | "calibrated-v1"
  | "human-overridden";

export interface EvidenceRevision {
  type: "git-commit" | "content-hash";
  value: string;
}

export interface EvidenceRecord {
  id: string; // ev:<sha256(8)>
  type: "configuration" | "source" | "ast" | "import" | "build" | "schema" | "runtime";
  claim: string;
  source: {
    path: string;
    startLine: number;
    endLine: number;
    revision: EvidenceRevision;
    contentHash: string; // blake3:<sha256> of the slice
    observedAt: string; // ISO-8601
  };
  extractor: {
    name: string; // e.g. "go-shape-v1"
    version: string; // e.g. "0.1.0"
  };
  classification: EvidenceClassification;
  confidence: number; // 0..1
  method: EvidenceMethod;
}

export interface LedgerOptions {
  runId: string;
  xdgDataDir: string; // e.g. ~/.local/share/archctl
  projectId: string; // portable projectId
}

/** Compute the ledger record id deterministically from its content. */
export function evidenceId(rec: Omit<EvidenceRecord, "id">): string {
  const h = createHash("sha256").update(JSON.stringify(rec)).digest("hex").slice(0, 16);
  return `ev:${h}`;
}

/** Compute a content hash for an arbitrary source slice. */
export function contentHash(content: string): string {
  return `blake3:${createHash("sha256").update(content).digest("hex")}`;
}

/**
 * Open the per-run evidence segment. The cross-run ledger is append-only and
 * sequential; this API is the ONLY writer — there is no public append method
 * beyond `record()`, which writes one line of JSONL to the per-run segment.
 */
export class EvidenceLedger {
  readonly segmentPath: string;

  constructor(opts: LedgerOptions) {
    const projectDir = join(opts.xdgDataDir, "projects", opts.projectId);
    const runDir = join(projectDir, "runs", opts.runId);
    mkdirSync(runDir, { recursive: true });
    this.segmentPath = join(runDir, "evidence.jsonl");
    // Initialise an empty segment file so callers can rely on its presence.
    writeFileSync(this.segmentPath, "", { flag: "a" });
  }

  record(rec: Omit<EvidenceRecord, "id">): EvidenceRecord {
    const id = evidenceId(rec);
    const full: EvidenceRecord = { ...rec, id };
    appendFileSync(this.segmentPath, JSON.stringify(full) + "\n");
    return full;
  }
}

/**
 * Read all evidence records for a (projectId, runId). Used by the auditor
 * (1.8) and the spike report (1.12).
 */
export function readLedger(opts: LedgerOptions): EvidenceRecord[] {
  const projectDir = join(opts.xdgDataDir, "projects", opts.projectId);
  const runDir = join(projectDir, "runs", opts.runId);
  const segmentPath = join(runDir, "evidence.jsonl");
  if (!existsSync(segmentPath)) return [];
  return readFileSync(segmentPath, "utf8")
    .split("\n")
    .filter((line) => line.trim().length > 0)
    .map((line) => JSON.parse(line) as EvidenceRecord);
}
