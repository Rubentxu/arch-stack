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
