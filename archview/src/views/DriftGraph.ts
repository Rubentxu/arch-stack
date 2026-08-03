/**
 * Pure helpers for C4 drift detection.
 *
 * Extracted from DriftView so they can be unit-tested without
 * pulling JSX into the test pipeline.
 */

import type { GraphEdge, GraphNode } from "../bundle/loader";

export type ElementDiff =
  | { kind: "added"; node: GraphNode }
  | { kind: "removed"; node: GraphNode }
  | { kind: "changed"; node: GraphNode; changes: string[] };

export type RelationDiff =
  | { kind: "added"; edge: GraphEdge }
  | { kind: "removed"; edge: GraphEdge }
  | { kind: "changed"; edge: GraphEdge; changes: string[] };

/**
 * Diff declared vs actual element lists.
 *
 * - added: in actual, not in declared
 * - removed: in declared, not in actual
 * - changed: in both, with property diffs
 */
export function diffElements(
  declared: GraphNode[],
  actual: GraphNode[],
): ElementDiff[] {
  const decMap = new Map<string, GraphNode>();
  const actMap = new Map<string, GraphNode>();
  for (const n of declared) decMap.set(n.id, n);
  for (const n of actual) actMap.set(n.id, n);

  const result: ElementDiff[] = [];
  for (const [id, node] of actMap) {
    if (!decMap.has(id)) result.push({ kind: "added", node });
  }
  for (const [id, node] of decMap) {
    if (!actMap.has(id)) result.push({ kind: "removed", node });
  }
  for (const [id, decNode] of decMap) {
    const actNode = actMap.get(id);
    if (!actNode) continue;
    const changes = diffElementProps(decNode, actNode);
    if (changes.length > 0) {
      result.push({ kind: "changed", node: actNode, changes });
    }
  }
  return result;
}

/**
 * Diff declared vs actual relation lists. Relations are matched
 * structurally by (source, target, kind) — no per-relation prop
 * diff in MVP.
 */
export function diffRelations(
  declared: GraphEdge[],
  actual: GraphEdge[],
): RelationDiff[] {
  const keyOf = (e: GraphEdge): string =>
    `${e.source}\0${e.target}\0${e.kind ?? ""}`;
  const decMap = new Map<string, GraphEdge>();
  const actMap = new Map<string, GraphEdge>();
  for (const e of declared) decMap.set(keyOf(e), e);
  for (const e of actual) actMap.set(keyOf(e), e);

  const result: RelationDiff[] = [];
  for (const [k, edge] of actMap) {
    if (!decMap.has(k)) result.push({ kind: "added", edge });
  }
  for (const [k, edge] of decMap) {
    if (!actMap.has(k)) result.push({ kind: "removed", edge });
  }
  return result;
}

/**
 * Return a list of human-readable property diffs between two nodes.
 */
export function diffElementProps(dec: GraphNode, act: GraphNode): string[] {
  const changes: string[] = [];
  if (dec.label !== act.label) {
    changes.push(`label: "${dec.label}" → "${act.label}"`);
  }
  if (dec.kind !== act.kind) {
    changes.push(`kind: ${dec.kind} → ${act.kind}`);
  }
  if ((dec.meta?.description as string) !== (act.meta?.description as string)) {
    changes.push(`description changed`);
  }
  if ((dec.meta?.technology as string) !== (act.meta?.technology as string)) {
    changes.push(`technology changed`);
  }
  return changes;
}

/**
 * Summary counts for the drift header.
 */
export function driftCounts(
  elements: ElementDiff[],
  relations: RelationDiff[],
) {
  return {
    added: elements.filter((d) => d.kind === "added").length,
    removed: elements.filter((d) => d.kind === "removed").length,
    changed: elements.filter((d) => d.kind === "changed").length,
    relAdded: relations.filter((d) => d.kind === "added").length,
    relRemoved: relations.filter((d) => d.kind === "removed").length,
  };
}
