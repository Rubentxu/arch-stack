/**
 * Pure helpers for C4 hierarchical rendering.
 *
 * Extracted from C4View so they can be unit-tested without pulling
 * JSX into the test pipeline. All functions are deterministic
 * transformations over C4 bundle data.
 */

import type { GraphNode } from "../bundle/loader";

/**
 * Compute the visible node set for a C4 drill-down focus.
 *
 * When `focusId` is set, the visible set is: the focus itself +
 * its children (one level down) + the parent context (if any) for
 * orientation. When `focusId` is null/undefined, all nodes are
 * visible.
 */
export function visibleNodesForFocus(
  nodes: GraphNode[],
  focusId: string | null | undefined,
): GraphNode[] {
  if (!focusId) return nodes;
  const focus = nodes.find((n) => n.id === focusId);
  if (!focus) return nodes;
  const children = nodes.filter((n) => n.parentId === focusId);
  const result: GraphNode[] = [focus, ...children];
  if (focus.parentId) {
    const parent = nodes.find((n) => n.id === focus.parentId);
    if (parent) result.push(parent);
  }
  return result;
}

/**
 * Filter edges to those whose endpoints are both in the visible set.
 */
export function visibleEdgesFor(
  edges: { source: string; target: string }[],
  visibleNodes: { id: string }[],
): { source: string; target: string }[] {
  const visible = new Set(visibleNodes.map((n) => n.id));
  return edges.filter((e) => visible.has(e.source) && visible.has(e.target));
}

/**
 * Group nodes by their C4 hierarchy level (1-4). Returns entries
 * sorted by level ascending.
 */
export function groupNodesByLevel(
  nodes: GraphNode[],
): Array<[number, GraphNode[]]> {
  const groups = new Map<number, GraphNode[]>();
  for (const n of nodes) {
    const level = n.level ?? 0;
    const arr = groups.get(level) ?? [];
    arr.push(n);
    groups.set(level, arr);
  }
  return [...groups.entries()].sort(([a], [b]) => a - b);
}

/**
 * Compute the breadcrumb trail from the root to the focus node.
 * Each entry is a node id; the trail is ordered root → focus.
 * Returns [] when focus is unset or not found.
 */
export function breadcrumbTrail(
  nodes: GraphNode[],
  focusId: string | null | undefined,
): string[] {
  if (!focusId) return [];
  const byId = new Map(nodes.map((n) => [n.id, n]));
  let current = byId.get(focusId);
  if (!current) return [];
  const trail: string[] = [];
  const seen = new Set<string>();
  while (current && !seen.has(current.id)) {
    seen.add(current.id);
    trail.unshift(current.id);
    current = current.parentId ? byId.get(current.parentId) : undefined;
  }
  return trail;
}

/**
 * Filter nodes by C4 hierarchy level (1=Context, 2=Container,
 * 3=Component, 4=Code). A null/undefined level returns all nodes
 * unchanged (no filter).
 *
 * M18: powers the semantic-zoom pill bar. The pill click sets the
 * level filter globally; focus remains tracked for sidebar selection
 * but does not narrow the visible set while the pill is active.
 */
export function nodesAtLevel(
  nodes: GraphNode[],
  level: number | null | undefined,
): GraphNode[] {
  if (level === null || level === undefined) return nodes;
  return nodes.filter((n) => (n.level ?? 0) === level);
}

/**
 * Count nodes per C4 level. Returns entries sorted by level
 * ascending. Levels with zero nodes are omitted; out-of-band
 * (`level: 0`) nodes are skipped so the pill bar only shows
 * actual C4 levels present in the bundle.
 */
export function levelCounts(nodes: GraphNode[]): Array<[number, number]> {
  const counts = new Map<number, number>();
  for (const n of nodes) {
    const level = n.level ?? 0;
    if (level === 0) continue;
    counts.set(level, (counts.get(level) ?? 0) + 1);
  }
  return [...counts.entries()].sort(([a], [b]) => a - b);
}

/**
 * Compose the level filter with the drill-down focus. When the
 * level filter is set, it wins (M18 user-picked interaction
 * "Pills de nivel global"): drill-down is suppressed because the
 * user explicitly asked to see the whole level. When the level
 * filter is null, the existing drill-down applies.
 */
export function visibleNodesWithLevel(
  nodes: GraphNode[],
  level: number | null | undefined,
  focusId: string | null | undefined,
): GraphNode[] {
  if (level !== null && level !== undefined) {
    return nodesAtLevel(nodes, level);
  }
  return visibleNodesForFocus(nodes, focusId);
}
