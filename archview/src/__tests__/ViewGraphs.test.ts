import { describe, it, expect } from "vitest";
import {
  visibleNodesForFocus,
  groupNodesByLevel,
  breadcrumbTrail,
  nodesAtLevel,
  levelCounts,
  visibleNodesWithLevel,
} from "../views/C4Graph";
import {
  partitionMembers,
  stereotypeFor,
  groupEdgesByPredicate,
} from "../views/ClassDiagramGraph";
import { extractParticipants, orderInteractions } from "../views/SequenceGraph";
import {
  computeImpact,
  impactCount,
  maxImpactDepth,
} from "../views/ImpactGraph";
import { diffElements, diffRelations, driftCounts } from "../views/DriftGraph";

describe("C4Graph", () => {
  const nodes = [
    {
      id: "sys:1",
      label: "A",
      name: "A",
      kind: "SoftwareSystem",
      level: 1,
      parentId: undefined as string | undefined,
    },
    {
      id: "ctn:1",
      label: "A-C1",
      name: "A-C1",
      kind: "Container",
      level: 2,
      parentId: "sys:1",
    },
    {
      id: "ctn:2",
      label: "A-C2",
      name: "A-C2",
      kind: "Container",
      level: 2,
      parentId: "sys:1",
    },
    {
      id: "sys:2",
      label: "B",
      name: "B",
      kind: "SoftwareSystem",
      level: 1,
      parentId: undefined,
    },
  ];

  it("returns all nodes when focus is unset", () => {
    const visible = visibleNodesForFocus(nodes, null);
    expect(visible).toHaveLength(4);
  });

  it("shows focus + its children + parent when focused", () => {
    const visible = visibleNodesForFocus(nodes, "ctn:1");
    const ids = visible.map((n) => n.id);
    expect(ids).toContain("ctn:1"); // focus
    expect(ids).toContain("sys:1"); // parent
    expect(ids).not.toContain("ctn:2"); // sibling, not child
  });

  it("groups nodes by level sorted ascending", () => {
    const groups = groupNodesByLevel(nodes);
    expect(groups[0][0]).toBe(1);
    expect(groups[1][0]).toBe(2);
    expect(groups[1][1]).toHaveLength(2);
  });

  it("builds breadcrumb trail from root to focus", () => {
    const trail = breadcrumbTrail(nodes, "ctn:2");
    expect(trail).toEqual(["sys:1", "ctn:2"]);
  });

  it("returns empty trail when focus not found", () => {
    expect(breadcrumbTrail(nodes, "ghost")).toEqual([]);
  });

  // ── M18: semantic-zoom helpers ──────────────────────────────────────
  const zoomFixture = [
    { id: "sys:1", label: "A", kind: "context", level: 1 },
    { id: "sys:2", label: "B", kind: "context", level: 1 },
    {
      id: "ctn:1",
      label: "A-C1",
      kind: "container",
      level: 2,
      parentId: "sys:1",
    },
    {
      id: "ctn:2",
      label: "A-C2",
      kind: "container",
      level: 2,
      parentId: "sys:1",
    },
    {
      id: "cmp:1",
      label: "A-C1-X",
      kind: "component",
      level: 3,
      parentId: "ctn:1",
    },
    { id: "orphan", label: "?", kind: "weird", level: 0 },
  ] as never;

  it("nodesAtLevel returns all when level is null/undefined", () => {
    expect(nodesAtLevel(zoomFixture, null)).toHaveLength(6);
    expect(nodesAtLevel(zoomFixture, undefined)).toHaveLength(6);
  });

  it("nodesAtLevel filters by exact level", () => {
    const level1 = nodesAtLevel(zoomFixture, 1);
    expect(level1.map((n) => n.id).sort()).toEqual(["sys:1", "sys:2"]);
    const level2 = nodesAtLevel(zoomFixture, 2);
    expect(level2.map((n) => n.id).sort()).toEqual(["ctn:1", "ctn:2"]);
    const level3 = nodesAtLevel(zoomFixture, 3);
    expect(level3.map((n) => n.id)).toEqual(["cmp:1"]);
  });

  it("levelCounts omits empty levels and out-of-band nodes", () => {
    const counts = levelCounts(zoomFixture);
    expect(counts).toEqual([
      [1, 2],
      [2, 2],
      [3, 1],
    ]);
  });

  it("levelCounts returns [] for empty input", () => {
    expect(levelCounts([])).toEqual([]);
  });

  it("visibleNodesWithLevel prefers the level filter over focus", () => {
    // When level is set, focus is ignored — the user wants the
    // whole level, not a drill-down slice.
    const visible = visibleNodesWithLevel(zoomFixture, 2, "cmp:1");
    expect(visible.map((n) => n.id).sort()).toEqual(["ctn:1", "ctn:2"]);
  });

  it("visibleNodesWithLevel falls back to drill-down when level is null", () => {
    const visible = visibleNodesWithLevel(zoomFixture, null, "ctn:1");
    // Drill-down: focus + children + parent
    const ids = visible.map((n) => n.id);
    expect(ids).toContain("ctn:1");
    expect(ids).toContain("sys:1");
    expect(ids).toContain("cmp:1");
  });

  it("visibleNodesWithLevel returns all when neither level nor focus is set", () => {
    expect(visibleNodesWithLevel(zoomFixture, null, null)).toHaveLength(6);
  });
});

describe("ClassDiagramGraph", () => {
  const node = {
    id: "c1",
    name: "Foo",
    kind: "class",
    meta: {
      members: [
        { name: "x", member_kind: "field", signature: "i32" },
        { name: "new", member_kind: "fn", signature: "fn() -> Self" },
        { name: "do_it", member_kind: "method" },
        { name: "ignored", member_kind: "other" },
      ],
    },
  };

  it("partitions fields and methods", () => {
    const { fields, methods } = partitionMembers(node as never);
    expect(fields.map((m) => m.name)).toEqual(["x"]);
    expect(methods.map((m) => m.name)).toEqual(["new", "do_it"]);
  });

  it("returns empty partition when no members", () => {
    const { fields, methods } = partitionMembers({
      id: "x",
      name: "X",
      kind: "class",
    } as never);
    expect(fields).toEqual([]);
    expect(methods).toEqual([]);
  });

  it("maps stereotypes", () => {
    expect(stereotypeFor("interface")).toBe("<<interface>>");
    expect(stereotypeFor("trait")).toBe("<<trait>>");
    expect(stereotypeFor("enum")).toBe("<<enum>>");
    expect(stereotypeFor("class")).toBeUndefined();
  });

  it("groups edges by predicate", () => {
    const edges = [
      { source: "a", target: "b", kind: "extends" },
      { source: "c", target: "d", kind: "composes" },
      { source: "e", target: "f", kind: "unknown" },
    ];
    const groups = groupEdgesByPredicate(edges);
    expect(groups.extends).toHaveLength(1);
    expect(groups.composes).toHaveLength(1);
    expect(groups.other).toHaveLength(1);
  });
});

describe("SequenceGraph", () => {
  it("extracts unique participants by file:name", () => {
    const interactions = [
      {
        order: 1,
        caller: { name: "a", file: "lib.rs" },
        callee: { name: "b", file: "lib.rs" },
      },
      {
        order: 2,
        caller: { name: "b", file: "lib.rs" },
        callee: { name: "c", file: "db.rs" },
      },
      {
        order: 3,
        caller: { name: "c", file: "db.rs" },
        callee: { name: "a", file: "lib.rs" },
      },
    ] as never;
    const participants = extractParticipants(interactions);
    expect(participants).toHaveLength(3);
    expect(participants.map((p) => p.key).sort()).toEqual([
      "db.rs:c",
      "lib.rs:a",
      "lib.rs:b",
    ]);
  });

  it("orders interactions by order field", () => {
    const interactions = [
      { order: 3, caller: { name: "c" }, callee: { name: "d" } },
      { order: 1, caller: { name: "a" }, callee: { name: "b" } },
      { order: 2, caller: { name: "b" }, callee: { name: "c" } },
    ] as never;
    const ordered = orderInteractions(interactions);
    expect(ordered.map((i) => i.order)).toEqual([1, 2, 3]);
  });
});

describe("ImpactGraph", () => {
  const nodes = [{ id: "a" }, { id: "b" }, { id: "c" }];
  const edges = [
    { source: "a", target: "b" },
    { source: "b", target: "c" },
  ];

  it("computes upstream impact", () => {
    const entries = computeImpact(nodes, edges, "b", "upstream");
    expect(entries.map((e) => e.nodeId)).toEqual(["a"]);
    expect(entries[0].direction).toBe("upstream");
  });

  it("computes downstream impact", () => {
    const entries = computeImpact(nodes, edges, "b", "downstream");
    expect(entries.map((e) => e.nodeId)).toEqual(["c"]);
  });

  it("tracks path from focus", () => {
    const entries = computeImpact(nodes, edges, "a", "downstream");
    const b = entries.find((e) => e.nodeId === "b");
    expect(b?.path).toEqual(["a", "b"]);
    const c = entries.find((e) => e.nodeId === "c");
    expect(c?.path).toEqual(["a", "b", "c"]);
  });

  it("counts impact and max depth", () => {
    const entries = computeImpact(nodes, edges, "a", "downstream");
    expect(impactCount(entries)).toBe(2);
    expect(maxImpactDepth(entries)).toBe(2);
  });

  it("returns empty for isolated node", () => {
    const entries = computeImpact([{ id: "x" }], [], "x", "both");
    expect(entries).toHaveLength(0);
  });
});

describe("DriftGraph", () => {
  const node = (
    id: string,
    name: string,
    extra: Record<string, unknown> = {},
  ) => ({
    id,
    label: name,
    name,
    kind: "Container",
    ...extra,
  });

  it("categorizes added/removed/changed elements", () => {
    const declared = [
      node("a", "A"),
      node("b", "B"),
      node("c", "C", { meta: { description: "old" } }),
    ];
    const actual = [
      node("a", "A"),
      node("d", "D"),
      node("c", "C", { meta: { description: "new" } }),
    ];
    const diffs = diffElements(declared, actual);
    const added = diffs.filter((d) => d.kind === "added");
    const removed = diffs.filter((d) => d.kind === "removed");
    const changed = diffs.filter((d) => d.kind === "changed");
    expect(added.map((d) => d.node.id)).toEqual(["d"]);
    expect(removed.map((d) => d.node.id)).toEqual(["b"]);
    expect(changed).toHaveLength(1);
    expect(changed[0].kind === "changed" && changed[0].changes).toContain(
      "description changed",
    );
  });

  it("diffs relations structurally", () => {
    const declared = [
      { id: "r1", source: "a", target: "b", kind: "uses" },
    ] as never;
    const actual = [
      { id: "r1", source: "a", target: "b", kind: "uses" },
      { id: "r2", source: "b", target: "c", kind: "uses" },
    ] as never;
    const diffs = diffRelations(declared, actual);
    expect(diffs.filter((d) => d.kind === "added")).toHaveLength(1);
    expect(diffs.filter((d) => d.kind === "removed")).toHaveLength(0);
  });

  it("computes drift counts", () => {
    const elements = [
      { kind: "added" },
      { kind: "added" },
      { kind: "removed" },
      { kind: "changed" },
    ] as never;
    const relations = [{ kind: "added" }, { kind: "removed" }] as never;
    const counts = driftCounts(elements, relations);
    expect(counts.added).toBe(2);
    expect(counts.removed).toBe(1);
    expect(counts.changed).toBe(1);
    expect(counts.relAdded).toBe(1);
    expect(counts.relRemoved).toBe(1);
  });
});
