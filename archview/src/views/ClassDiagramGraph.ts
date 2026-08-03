/**
 * Pure helpers for UML class diagram rendering.
 *
 * Extracted from ClassDiagramView so they can be unit-tested without
 * pulling JSX into the test pipeline.
 */

import type { GraphNode } from "../bundle/loader";

export interface ClassMember {
  name: string;
  member_kind: string;
  signature?: string;
  line?: number;
}

export interface MemberPartition {
  fields: ClassMember[];
  methods: ClassMember[];
}

/**
 * Partition a class node's members by kind. Fields are members with
 * `member_kind === "field"`. Methods are members with
 * `member_kind === "fn"` or `"method"`. Other kinds are ignored.
 */
export function partitionMembers(node: GraphNode): MemberPartition {
  const raw = (node.meta?.members as ClassMember[] | undefined) ?? [];
  const fields: ClassMember[] = [];
  const methods: ClassMember[] = [];
  for (const m of raw) {
    if (m.member_kind === "field") fields.push(m);
    else if (m.member_kind === "fn" || m.member_kind === "method")
      methods.push(m);
  }
  return { fields, methods };
}

/**
 * Stereotype for a class kind. Returns undefined for plain classes.
 *   interface → <<interface>>
 *   trait     → <<trait>>
 *   enum      → <<enum>>
 */
export function stereotypeFor(kind: string): string | undefined {
  if (kind === "interface") return "<<interface>>";
  if (kind === "trait") return "<<trait>>";
  if (kind === "enum") return "<<enum>>";
  return undefined;
}

/**
 * Group edges by predicate kind for the relations panel.
 * Known kinds get their own bucket; everything else goes to "other".
 */
export function groupEdgesByPredicate(
  edges: { kind?: string; source: string; target: string }[],
): Record<string, { kind?: string; source: string; target: string }[]> {
  const groups: Record<
    string,
    { kind?: string; source: string; target: string }[]
  > = {
    extends: [],
    implements: [],
    composes: [],
    other: [],
  };
  for (const e of edges) {
    const k = e.kind ?? "other";
    if (k in groups) groups[k].push(e);
    else groups.other.push(e);
  }
  return groups;
}
