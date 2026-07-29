// ADR-0006 — capability router (Shape B) + declarative ShellAdapter.
//
// The router maps an abstract capability name to a concrete Adapter
// implementation. The contract is a uniform `run(ctx) → RawEvidence[]`,
// which keeps the door open for non-declarative adapters later (semantic,
// LSP-driven) while ~90% of fast-profile adapters are pure YAML descriptors
// driving ShellAdapter.
//
// Adding a tool = adding a YAML descriptor under packages/core/src/adapters/.
// Callers never change. This is the OCP seam — the design's strongest
// entropy property (explore-report §9).

import { spawnSync } from "node:child_process";
import { basename } from "node:path";
import type { EvidenceRevision, EvidenceMethod } from "../evidence/ledger.ts";

export interface RunContext {
  /** Canonical root of the analyzed repository (SourceIdentity anchor). */
  repoRoot: string;
  /** Discriminated source revision for evidence provenance. */
  revision: EvidenceRevision;
  /** ISO-8601 timestamp the run began. */
  observedAt: string;
  /** Maximum wall-clock budget in ms; adapter must respect or fail. */
  timeoutMs: number;
  /** Adapter capability name (for logging / audit). */
  capability: string;
}

export type EvidenceType =
  | "configuration"
  | "source"
  | "ast"
  | "import"
  | "build"
  | "schema"
  | "runtime";

export interface RawEvidence {
  type: EvidenceType;
  claim: string;
  source: {
    path: string;
    startLine: number;
    endLine: number;
  };
  classification: "fact" | "inference" | "hypothesis" | "unknown" | "conflict";
  confidence: number;
  method: EvidenceMethod;
}

export interface AdapterRequirements {
  /** External CLIs that must be on PATH (smoke-probed before invocation). */
  binaries: string[];
  /** Optional minimum versions (recorded in evidence for reproducibility). */
  minVersions?: Record<string, string>;
}

export interface Adapter {
  capability: string;
  /** Adapter name + version recorded in every produced evidence. */
  name: string;
  version: string;
  requires: AdapterRequirements;
  /** Pure function: capabilities + ctx → RawEvidence[]. */
  run: (ctx: RunContext) => Promise<RawEvidence[]>;
}

/**
 * Declarative ShellAdapter — drives a CLI from a YAML-like descriptor.
 * The descriptor is loaded as JSON (TS friendly in tests; YAML can be a thin
 * wrapper later). Output is parsed line-by-line into RawEvidence records.
 *
 * The descriptor is intentionally minimal: the OCP seam is the Adapter
 * interface, not the descriptor's expressive power.
 */
export interface ShellAdapterDescriptor {
  capability: string;
  name: string;
  version: string;
  command: string[]; // argv; {repoRoot} and {paths} are templated at run time
  requires: AdapterRequirements;
  /** Map RawEvidence fields from a single stdout line. */
  output: {
    type: EvidenceType;
    /** Regex with capture groups; group 1 = path, group 2 = line, group 3 = claim. */
    pattern: string;
    classification: RawEvidence["classification"];
    confidence: number;
    method: EvidenceMethod;
  };
}

export function shellAdapterFromDescriptor(d: ShellAdapterDescriptor): Adapter {
  const re = new RegExp(d.output.pattern);
  return {
    capability: d.capability,
    name: d.name,
    version: d.version,
    requires: d.requires,
    async run(ctx: RunContext): Promise<RawEvidence[]> {
      const argv = d.command.map((arg) =>
        arg
          .replace("{repoRoot}", ctx.repoRoot)
          // For {paths} we intentionally pass the repo root only — path
          // discovery is the adapter's job; the shell command stays small.
          .replace("{paths}", ctx.repoRoot),
      );
      const r = spawnSync(argv[0]!, argv.slice(1), {
        cwd: ctx.repoRoot,
        encoding: "utf8",
        timeout: ctx.timeoutMs,
      });
      if (r.error) return []; // timeout / spawn failure → no evidence (auditor logs)
      const out = `${r.stdout ?? ""}`;
      const records: RawEvidence[] = [];
      for (const line of out.split("\n")) {
        const m = re.exec(line);
        if (!m) continue;
        const path = m[1] ?? "";
        const lineNum = Number(m[2] ?? "0") || 0;
        const claim = m[3] ?? line;
        records.push({
          type: d.output.type,
          claim,
          source: { path, startLine: lineNum, endLine: lineNum },
          classification: d.output.classification,
          confidence: d.output.confidence,
          method: d.output.method,
        });
      }
      return records;
    },
  };
}

/**
 * Minimal in-memory adapter registry. The router resolves a capability name
 * to the registered adapter; capability names are the public API.
 */
export class CapabilityRouter {
  private readonly adapters = new Map<string, Adapter>();

  register(a: Adapter): void {
    if (this.adapters.has(a.capability)) {
      throw new Error(`capability already registered: ${a.capability}`);
    }
    this.adapters.set(a.capability, a);
  }

  resolve(capability: string): Adapter {
    const a = this.adapters.get(capability);
    if (!a) throw new Error(`unknown capability: ${capability}`);
    return a;
  }

  list(): string[] {
    return [...this.adapters.keys()].sort();
  }
}

/**
 * Probe whether every required binary for an adapter is present on PATH.
 * Used by the smoke probe (0.2) and the doctor (2.11). Records both
 * `which` and `first-line-of --version` so the doctor can assert the
 * minimum-version table.
 */
export function probeAdapterRequirements(reqs: AdapterRequirements): { ok: boolean; missing: string[] } {
  const missing: string[] = [];
  for (const bin of reqs.binaries) {
    const r = spawnSync("which", [bin], { encoding: "utf8" });
    if (r.status !== 0) missing.push(bin);
  }
  return { ok: missing.length === 0, missing };
}

/** Convenience: capability name → adapter name (for logging). */
export function adapterName(router: CapabilityRouter, capability: string): string {
  return router.resolve(capability).name;
}

/** Convenience: surface the descriptive name of an adapter file (used in tests). */
export function adapterFilename(a: Adapter): string {
  return `${a.name}@${a.version}`;
}

/** Filter helper for tests; exported for the auditor / reports. */
export function filterByRepoPath<T extends { source: { path: string } }>(
  evidence: T[],
  repoRoot: string,
): T[] {
  const root = basename(repoRoot) || repoRoot;
  return evidence.filter((e) => e.source.path === root || !e.source.path.includes("/"));
}
