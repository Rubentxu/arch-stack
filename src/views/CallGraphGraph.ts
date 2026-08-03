/**
 * Pure helpers for call-graph focus expansion.
 *
 * Extracted from CallGraphView so they can be unit-tested without
 * pulling JSX into the test pipeline.
 */

import type { GraphEdge, GraphNode } from "../bundle/loader";

export type CallDirection = "callees" | "callers" | "both";

export interface LevelGroup {
  direction: "callees" | "callers";
  depth: number;
  edges: GraphEdge[];
  nodes: GraphNode[];
}

export const MAX_DEPTH = 5;

/**
 * BFS from focus. At each level, expand nodes by following edges
 * in the chosen direction. Returns one `LevelGroup` per depth.
 * Stops when a level has no new nodes.
 */
export function expandLevels(
  nodes: GraphNode[],
  edges: GraphEdge[],
  focusId: string,
  depth: number,
  direction: CallDirection,
): LevelGroup[] {
  const focus = nodes.find((n) => n.id === focusId);
  if (!focus) return [];

  const result: LevelGroup[] = [];
  const visited = new Set<string>([focusId]);

  const followForward = (from: string): string[] =>
    edges.filter((e) => e.source === from).map((e) => e.target);
  const followBackward = (from: string): string[] =>
    edges.filter((e) => e.target === from).map((e) => e.source);

  let frontier: string[] = [focusId];
  for (let d = 1; d <= depth; d++) {
    const next: string[] = [];
    for (const nodeId of frontier) {
      const targets =
        direction === "callees" || direction === "both"
          ? followForward(nodeId)
          : [];
      const sources =
        direction === "callers" || direction === "both"
          ? followBackward(nodeId)
          : [];
      for (const t of [...targets, ...sources]) {
        if (!visited.has(t)) {
          visited.add(t);
          next.push(t);
        }
      }
    }
    if (next.length === 0) break;
    const levelEdges = edges.filter(
      (e) =>
        visited.has(e.source) &&
        visited.has(e.target) &&
        (frontier.includes(e.source) || frontier.includes(e.target)),
    );
    const levelNodes = nodes.filter((n) => next.includes(n.id));
    result.push({
      direction:
        direction === "both"
          ? d === 1
            ? "callees"
            : "callers"
          : (direction as "callees" | "callers"),
      depth: d,
      edges: levelEdges,
      nodes: levelNodes,
    });
    frontier = next;
  }
  return result;
}

/**
 * Total unique nodes reachable from focus across all levels.
 */
export function blastRadiusOf(groups: LevelGroup[]): number {
  return groups.reduce((acc, g) => acc + g.nodes.length, 0);
}
