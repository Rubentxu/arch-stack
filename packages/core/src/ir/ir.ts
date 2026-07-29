// ADR-0002 — Architecture IR v1.
//
// Source of truth. Diagrams are pure projections of this. Every element
// and relationship MUST carry `evidenceRefs: [...]` and a `method` enum
// (ADR-0004 acceptance). The IR is forward-compatible: temporal fields
// (validFrom/validTo) and an `meta` extension bag are reserved but only
// added when a real consumer exists (avoid premature fields).
//
// Schema versioning is mandatory. Migrations live in ./migrations/ and are
// applied in order. The current `schemaVersion: 1` shape is the only one
// archctl ever reads; unknown versions fail loud.

import { createHash } from "node:crypto";
import type { EvidenceMethod, EvidenceClassification, EvidenceRecord } from "../evidence/ledger.ts";
export type { EvidenceMethod, EvidenceClassification, EvidenceRecord };

export const IR_SCHEMA_VERSION = 1 as const;

export type IRElementKind =
  | "person"
  | "softwareSystem"
  | "container"
  | "component"
  | "codeElement";

export type IREvidenceRef = string; // ev:<sha256(16)>

export interface IRElement {
  id: string; // <kind>:<kebab-name>
  kind: IRElementKind;
  name: string;
  technology?: string[];
  tags?: string[]; // non-canonical extensions
  description?: string;
  classification: EvidenceClassification;
  confidence: number;
  method: EvidenceMethod;
  evidenceRefs: IREvidenceRef[];
}

export interface IRRelationship {
  id: string; // rel:<sha256(8)>
  source: string; // IRElement.id
  target: string; // IRElement.id
  via?: string; // "imports", "calls", "reads", "writes"
  technology?: string;
  classification: EvidenceClassification;
  confidence: number;
  method: EvidenceMethod;
  evidenceRefs: IREvidenceRef[];
}

export interface ArchitectureIR {
  schemaVersion: typeof IR_SCHEMA_VERSION;
  sourceIdentitySummary: string; // free-form: "git:abc..def /tmp/foo" or "dir:/tmp/foo"
  elements: IRElement[];
  relationships: IRRelationship[];
  generatedAt: string; // ISO-8601
}

export interface Migration {
  from: number;
  to: number;
  apply: (ir: unknown) => unknown;
}

/**
 * Lightweight auditor. Returns:
 *  - `unsupported`: high-confidence claims (confidence ≥ 0.9, classification
 *    `fact` or `inference`) with zero evidence refs.
 *  - `lowConfidenceNoEvidence`: confidence < 0.6 with zero refs (recorded as
 *    `unknown`/`hypothesis` per ADR-0004; never blocks).
 *  - `unknownMethod`: any record missing a valid `method`.
 *  - `wrongVersion`: schemaVersion is not `IR_SCHEMA_VERSION`.
 */
export interface AuditResult {
  ok: boolean;
  unsupported: { kind: "element" | "relationship"; id: string }[];
  lowConfidenceNoEvidence: { kind: "element" | "relationship"; id: string }[];
  unknownMethod: { kind: "element" | "relationship"; id: string }[];
  wrongVersion?: { found: number };
}

export function auditIR(ir: ArchitectureIR): AuditResult {
  const result: AuditResult = {
    ok: true,
    unsupported: [],
    lowConfidenceNoEvidence: [],
    unknownMethod: [],
  };
  if (ir.schemaVersion !== IR_SCHEMA_VERSION) {
    result.ok = false;
    result.wrongVersion = { found: ir.schemaVersion };
  }
  const validMethods = new Set<EvidenceMethod>(["heuristic-v1", "calibrated-v1", "human-overridden"]);
  for (const e of ir.elements) {
    if (!validMethods.has(e.method)) result.unknownMethod.push({ kind: "element", id: e.id });
    if (e.evidenceRefs.length === 0) {
      if (e.confidence >= 0.9 || e.classification === "fact" || e.classification === "inference") {
        result.unsupported.push({ kind: "element", id: e.id });
        result.ok = false;
      } else if (e.confidence < 0.6) {
        result.lowConfidenceNoEvidence.push({ kind: "element", id: e.id });
      }
    }
  }
  for (const r of ir.relationships) {
    if (!validMethods.has(r.method)) result.unknownMethod.push({ kind: "relationship", id: r.id });
    if (r.evidenceRefs.length === 0) {
      if (r.confidence >= 0.9 || r.classification === "fact" || r.classification === "inference") {
        result.unsupported.push({ kind: "relationship", id: r.id });
        result.ok = false;
      } else if (r.confidence < 0.6) {
        result.lowConfidenceNoEvidence.push({ kind: "relationship", id: r.id });
      }
    }
  }
  return result;
}

/**
 * Build an ArchitectureIR v1 from a flat list of evidence records.
 * Records with the same claim/path produce a single element with merged
 * evidence refs. The function is intentionally simple at M1 — Phase 4's
 * temporal store will reuse it without changes.
 */
export function buildIR(opts: {
  sourceIdentitySummary: string;
  elements: Omit<IRElement, "evidenceRefs">[];
  relationships: Omit<IRRelationship, "evidenceRefs">[];
  evidence: Pick<EvidenceRecord, "id" | "type" | "claim" | "source">[];
}): ArchitectureIR {
  // Index evidence by claim (the IR's text matches the ledger's claim) and
  // also by path so the lookup is robust to slight phrasing differences in
  // production. The first match per claim wins (deterministic by order).
  const byClaim = new Map<string, string[]>();
  const byClaimPath = new Map<string, string[]>();
  for (const ev of opts.evidence) {
    if (!ev.id) continue;
    const claimRefs = byClaim.get(ev.claim) ?? [];
    if (!claimRefs.includes(ev.id)) claimRefs.push(ev.id);
    byClaim.set(ev.claim, claimRefs);
    const cpKey = `${ev.claim}::${ev.source.path}`;
    const cpRefs = byClaimPath.get(cpKey) ?? [];
    if (!cpRefs.includes(ev.id)) cpRefs.push(ev.id);
    byClaimPath.set(cpKey, cpRefs);
  }
  const attach = (description: string | undefined): string[] => {
    if (!description) return [];
    return byClaim.get(description) ?? byClaimPath.get(description + "::") ?? [];
  };
  const elements: IRElement[] = opts.elements.map((e) => ({
    ...e,
    evidenceRefs: e.evidenceRefs ?? attach(e.description),
  }));
  const relationships: IRRelationship[] = opts.relationships.map((r) => ({
    ...r,
    evidenceRefs: r.evidenceRefs ?? [],
  }));
  return {
    schemaVersion: IR_SCHEMA_VERSION,
    sourceIdentitySummary: opts.sourceIdentitySummary,
    elements,
    relationships,
    generatedAt: new Date().toISOString(),
  };
}

/**
 * Apply migrations in order to bring an unknown IR into schemaVersion 1.
 * Unknown incoming versions fail loud (the caller decides what to do).
 */
export function migrateToCurrent(raw: unknown, migrations: Migration[]): ArchitectureIR {
  if (!raw || typeof raw !== "object" || !("schemaVersion" in raw)) {
    throw new Error("IR migration: input is not an object");
  }
  const start = (raw as { schemaVersion: unknown }).schemaVersion;
  if (typeof start !== "number") {
    throw new Error("IR migration: schemaVersion is not a number");
  }
  let current = raw;
  for (const m of migrations) {
    if (m.from === start) current = m.apply(current);
  }
  const result = current as ArchitectureIR;
  if (result.schemaVersion !== IR_SCHEMA_VERSION) {
    throw new Error(`IR migration: cannot reach schemaVersion ${IR_SCHEMA_VERSION} from ${start}`);
  }
  return result;
}

export function relationshipId(source: string, target: string, via = ""): string {
  // 8-hex digest of (source|via|target) — enough to dedupe within a run.
  // (Caller responsibility: avoid collisions by including `via` for parallel edges.)
  return `rel:${createHash("sha256").update(`${source}|${via}|${target}`).digest("hex").slice(0, 8)}`;
}

// Re-export so downstream callers can build a self-contained IR/ledger cycle.
export type { EvidenceRecord };
export type { EvidenceClassification as EvidenceClassificationT };
export { evidenceId } from "../evidence/ledger.ts";
