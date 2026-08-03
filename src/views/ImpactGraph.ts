/**
 * Pure helpers for impact analysis (blast radius).
 *
 * Extracted from ImpactView so they can be unit-tested without
 * pulling JSX into the test pipeline.
 */

export type ImpactDirection = "upstream" | "downstream" | "both";

export interface ImpactEntry {
  nodeId: string;
  depth: number;
  direction: "upstream" | "downstream";
  path: string[];
}

const MAX_DEPTH = 5;

/**
 * Compute the blast radius of a proposed change at `focusId`.
 *
 * BFS in both directions (configurable) from the focus, with path
 * tracking. The focus itself is excluded from results. Depth is
 * capped at MAX_DEPTH (5).
 */
export function computeImpact(
  nodes: { id: string }[],
  edges: { source: string; target: string }[],
  focusId: string,
  direction: ImpactDirection,
  maxDepth = MAX_DEPTH,
): ImpactEntry[] {
  const focus = nodes.find((n) => n.id === focusId);
  if (!focus) return [];

  const visited = new Map<string, ImpactEntry>();
  const entries: ImpactEntry[] = [];

  const traverse = (
    startId: string,
    dir: "upstream" | "downstream",
  ) => {
    let frontier: Array<{ id: string; path: string[] }> = [
      { id: startId, path: [startId] },
    ];
    let depth = 0;
    while (frontier.length > 0 && depth < maxDepth) {
      depth++;
      const next: typeof frontier = [];
      for (const { id, path } of frontier) {
        const neighbors = edges
          .filter((e) => (dir === "upstream" ? e.target === id : e.source === id))
          .map((e) => (dir === "upstream" ? e.source : e.target));
        for (const n of neighbors) {
          if (n === focusId) continue; // don't loop back to focus
          const newPath = [...path, n];
          const existing = visited.get(n);
          if (!existing || existing.depth > depth) {
            const entry: ImpactEntry = {
              nodeId: n,
              depth,
              direction: dir,
              path: newPath,
            };
            visited.set(n, entry);
            if (!existing) entries.push(entry);
            next.push({ id: n, path: newPath });
          }
        }
      }
      frontier = next;
    }
  };

  if (direction === "upstream" || direction === "both") {
    traverse(focusId, "upstream");
  }
  if (direction === "downstream" || direction === "both") {
    traverse(focusId, "downstream");
  }
  return entries;
}

/**
 * Count distinct impacted nodes (excludes the focus).
 */
export function impactCount(entries: ImpactEntry[]): number {
  return entries.filter((e) => e.depth > 0).length;
}

/**
 * Max depth reached across entries (0 when empty).
 */
export function maxImpactDepth(entries: ImpactEntry[]): number {
  return entries.reduce((m, e) => Math.max(m, e.depth), 0);
}
