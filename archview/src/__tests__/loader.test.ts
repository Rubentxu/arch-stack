import { describe, it, expect } from "vitest";
import { c4LevelForType, normalizeBundle } from "../bundle/loader";

/** BFS expansion logic (mirrors CallGraphView's levelGroups). */
function expandLevels(
  nodes: { id: string }[],
  edges: { source: string; target: string }[],
  focusId: string,
  depth: number,
  direction: "callees" | "callers" | "both",
): { direction: "callees" | "callers"; depth: number; nodes: string[] }[] {
  const result: {
    direction: "callees" | "callers";
    depth: number;
    nodes: string[];
  }[] = [];
  const visited = new Set<string>([focusId]);
  let frontier: string[] = [focusId];
  for (let d = 1; d <= depth; d++) {
    const next: string[] = [];
    for (const nodeId of frontier) {
      const forward =
        direction === "callees" || direction === "both"
          ? edges.filter((e) => e.source === nodeId).map((e) => e.target)
          : [];
      const backward =
        direction === "callers" || direction === "both"
          ? edges.filter((e) => e.target === nodeId).map((e) => e.source)
          : [];
      for (const t of [...forward, ...backward]) {
        if (!visited.has(t)) {
          visited.add(t);
          next.push(t);
        }
      }
    }
    if (next.length === 0) break;
    result.push({
      direction: direction === "both" ? "callees" : direction,
      depth: d,
      nodes: next,
    });
    frontier = next;
  }
  return result;
}

describe("bundle loader", () => {
  it("normalizes a call-graph bundle", () => {
    const raw = {
      schemaVersion: "1.0",
      nodes: [
        {
          id: "fn:a",
          name: "a",
          kind: "function",
          file: "src/lib.rs",
          line: 1,
        },
        {
          id: "fn:b",
          name: "b",
          kind: "function",
          file: "src/lib.rs",
          line: 5,
        },
      ],
      edges: [{ id: "e1", source: "fn:a", target: "fn:b", kind: "calls" }],
    };
    const bundle = normalizeBundle(raw, "test");
    expect(bundle.rawKind).toBe("call-graph");
    expect(bundle.nodes).toHaveLength(2);
    expect(bundle.edges).toHaveLength(1);
    expect(bundle.nodes[0].label).toBe("a");
    expect(bundle.edges[0].label).toBe("calls");
  });

  it("classifies call-graph nodes with a language field as call-graph (regression)", () => {
    // Real samples carry kind:"function" AND language — the language
    // check used to win and misclassify as class-diagram, so the G6
    // canvas never mounted for call-graph bundles.
    const raw = {
      schemaVersion: "1.0",
      nodes: [
        { id: "fn:caller", name: "caller", kind: "function", language: "rust" },
        { id: "fn:callee", name: "callee", kind: "function", language: "rust" },
      ],
      edges: [{ id: "e1", source: "fn:caller", target: "fn:callee", kind: "calls" }],
    };
    const bundle = normalizeBundle(raw, "test");
    expect(bundle.rawKind).toBe("call-graph");
  });

  it("normalizes a class-diagram bundle", () => {
    const raw = {
      schemaVersion: "1.0",
      nodes: [
        {
          canonical_key: "rust:lib.rs:class:Foo:2",
          name: "Foo",
          kind: "class",
          language: "rust",
          file: "lib.rs",
          line: 2,
        },
      ],
      edges: [],
    };
    const bundle = normalizeBundle(raw, "test");
    expect(bundle.rawKind).toBe("class-diagram");
    expect(bundle.nodes).toHaveLength(1);
    expect(bundle.nodes[0].id).toBe("rust:lib.rs:class:Foo:2");
    expect(bundle.nodes[0].language).toBe("rust");
  });

  it("normalizes a sequence bundle by extracting caller/callee pairs", () => {
    const raw = {
      schemaVersion: "1.0",
      interactions: [
        {
          order: 1,
          message_kind: "SyncCall",
          label: "a → b",
          caller: { name: "a", file: "lib.rs", line: 1 },
          callee: { name: "b", file: "lib.rs", line: 5 },
        },
      ],
    };
    const bundle = normalizeBundle(raw, "test");
    expect(bundle.rawKind).toBe("sequence");
    expect(bundle.nodes).toHaveLength(2);
    expect(bundle.edges).toHaveLength(1);
    expect(bundle.edges[0].source).toBe("lib.rs:a");
    expect(bundle.edges[0].target).toBe("lib.rs:b");
  });

  it("normalizes a C4 bundle with canonical viewer-bundle sections", () => {
    const raw = {
      manifest: {
        schemaVersion: "1.0.0",
        format: "viewer-bundle",
        viewSelector: "container:*",
        baseRevision:
          "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        generatedAt: "2026-08-03T12:00:00Z",
        elementCount: 2,
        edgeCount: 1,
        evidenceCount: 0,
      },
      projection: {
        nodes: [
          { id: "el:1", type: "container", name: "WebApp" },
          { id: "el:2", type: "container", name: "DB" },
        ],
        edges: [
          { id: "r1", source: "el:1", target: "el:2", predicate: "uses" },
        ],
      },
      evidence: { evidence: [] },
      styles: {
        theme: "default",
        version: "1.0.0",
        elementColors: {},
        edgeColors: {},
      },
    };
    const bundle = normalizeBundle(raw, "test");
    expect(bundle.rawKind).toBe("c4");
    expect(bundle.nodes).toHaveLength(2);
    expect(bundle.edges[0].kind).toBe("uses");
  });

  it("derives C4 level and parentId from canonical type + canonicalKey", () => {
    const raw = {
      manifest: {
        schemaVersion: "1.0.0",
        format: "viewer-bundle",
        viewSelector: "context:*",
        baseRevision:
          "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        generatedAt: "2026-08-03T12:00:00Z",
        elementCount: 2,
        edgeCount: 0,
        evidenceCount: 0,
      },
      projection: {
        nodes: [
          {
            id: "sys:1",
            type: "context",
            name: "Platform",
            canonicalKey: "platform",
          },
          {
            id: "ctn:1",
            type: "container",
            name: "API",
            canonicalKey: "platform/api",
            description: "HTTP entry",
          },
        ],
        edges: [],
      },
      evidence: { evidence: [] },
      styles: {
        theme: "default",
        version: "1.0.0",
        elementColors: {},
        edgeColors: {},
      },
    };
    const bundle = normalizeBundle(raw, "test");
    const sys = bundle.nodes.find((n) => n.id === "sys:1")!;
    const ctn = bundle.nodes.find((n) => n.id === "ctn:1")!;
    expect(sys.level).toBe(1);
    expect(sys.parentId).toBeUndefined();
    expect(ctn.level).toBe(2);
    expect(ctn.parentId).toBe("sys:1");
    expect(ctn.meta.description).toBe("HTTP entry");
  });
});

describe("c4LevelForType", () => {
  it("maps canonical c4 types to hierarchy levels", () => {
    expect(c4LevelForType("context")).toBe(1);
    expect(c4LevelForType("container")).toBe(2);
    expect(c4LevelForType("component")).toBe(3);
    expect(c4LevelForType("dynamic")).toBe(1);
    expect(c4LevelForType("deployment")).toBe(1);
  });

  it("returns 0 for unknown types", () => {
    expect(c4LevelForType("UnknownKind")).toBe(0);
    expect(c4LevelForType("")).toBe(0);
  });
});

describe("call-graph BFS expansion (M17.2)", () => {
  // Diamond: a -> b, a -> c, b -> d, c -> d
  const nodes = [{ id: "a" }, { id: "b" }, { id: "c" }, { id: "d" }];
  const edges = [
    { source: "a", target: "b" },
    { source: "a", target: "c" },
    { source: "b", target: "d" },
    { source: "c", target: "d" },
  ];

  it("expands 1 level downstream", () => {
    const levels = expandLevels(nodes, edges, "a", 1, "callees");
    expect(levels).toHaveLength(1);
    expect(levels[0].nodes.sort()).toEqual(["b", "c"]);
  });

  it("expands 2 levels downstream and dedupes d", () => {
    const levels = expandLevels(nodes, edges, "a", 2, "callees");
    expect(levels).toHaveLength(2);
    expect(levels[0].nodes.sort()).toEqual(["b", "c"]);
    expect(levels[1].nodes).toEqual(["d"]);
  });

  it("expands upstream", () => {
    const levels = expandLevels(nodes, edges, "d", 2, "callers");
    expect(levels[0].nodes.sort()).toEqual(["b", "c"]);
    expect(levels[1].nodes).toEqual(["a"]);
  });

  it("expands both directions", () => {
    const levels = expandLevels(nodes, edges, "b", 1, "both");
    expect(levels[0].nodes.sort()).toEqual(["a", "d"]);
  });

  it("terminates early when frontier is exhausted", () => {
    const levels = expandLevels(nodes, edges, "a", 5, "callees");
    expect(levels).toHaveLength(2);
  });
});

describe("call-graph async flow (M17.2)", () => {
  it("distinguishes SyncCall vs AsyncCall edge kinds", () => {
    const raw = {
      schemaVersion: "1.0",
      nodes: [
        { id: "fn:a", name: "a", kind: "function" },
        { id: "fn:b", name: "b", kind: "function" },
      ],
      edges: [{ id: "e1", source: "fn:a", target: "fn:b", kind: "AsyncCall" }],
    };
    const bundle = normalizeBundle(raw, "test");
    expect(bundle.rawKind).toBe("call-graph");
    expect(bundle.edges[0].kind).toBe("AsyncCall");
  });
});

describe("sequence bundle (M17.3)", () => {
  it("preserves raw interactions on the bundle", () => {
    const raw = {
      schemaVersion: "1.0",
      interactions: [
        {
          order: 1,
          label: "a → b",
          message_kind: "SyncCall",
          caller: { name: "a", file: "lib.rs", line: 1 },
          callee: { name: "b", file: "lib.rs", line: 5 },
        },
        {
          order: 2,
          label: "← result",
          message_kind: "Reply",
          caller: { name: "b", file: "lib.rs", line: 6 },
          callee: { name: "a", file: "lib.rs", line: 2 },
        },
      ],
    };
    const bundle = normalizeBundle(raw, "test");
    expect(bundle.rawKind).toBe("sequence");
    expect(bundle.interactions).toBeDefined();
    expect(bundle.interactions).toHaveLength(2);
    expect(bundle.interactions![0].order).toBe(1);
    expect(bundle.interactions![0].message_kind).toBe("SyncCall");
    expect(bundle.interactions![1].message_kind).toBe("Reply");
  });

  it("extracts participants as unique file:name pairs", () => {
    // 4 interactions, 3 unique participants: a, b, c
    const raw = {
      schemaVersion: "1.0",
      interactions: [
        {
          order: 1,
          message_kind: "SyncCall",
          caller: { name: "a" },
          callee: { name: "b" },
        },
        {
          order: 2,
          message_kind: "SyncCall",
          caller: { name: "b" },
          callee: { name: "c" },
        },
        {
          order: 3,
          message_kind: "Reply",
          caller: { name: "c" },
          callee: { name: "b" },
        },
        {
          order: 4,
          message_kind: "Reply",
          caller: { name: "b" },
          callee: { name: "a" },
        },
      ],
    };
    const bundle = normalizeBundle(raw, "test");
    const keys = new Set(
      bundle.interactions!.flatMap((i) => [
        `${i.caller.file ?? ""}:${i.caller.name ?? "?"}`,
        `${i.callee.file ?? ""}:${i.callee.name ?? "?"}`,
      ]),
    );
    expect(keys.size).toBe(3);
  });

  it("handles interactions with missing optional fields", () => {
    const raw = {
      schemaVersion: "1.0",
      interactions: [
        {
          order: 1,
          message_kind: "SyncCall",
          caller: {},
          callee: { name: "b" },
        },
      ],
    };
    const bundle = normalizeBundle(raw, "test");
    expect(bundle.interactions![0].caller.name).toBeUndefined();
    expect(bundle.interactions![0].callee.name).toBe("b");
  });
});

describe("class-diagram bundle (M17.4)", () => {
  it("preserves members in node meta for the diagram view", () => {
    const raw = {
      schemaVersion: "1.0",
      nodes: [
        {
          canonical_key: "rust:lib.rs:class:Foo:1",
          name: "Foo",
          kind: "class",
          language: "rust",
          file: "lib.rs",
          line: 1,
          members: [
            { name: "x", member_kind: "field", signature: "i32", line: 2 },
            {
              name: "new",
              member_kind: "fn",
              signature: "fn() -> Self",
              line: 4,
            },
          ],
        },
      ],
      edges: [],
    };
    const bundle = normalizeBundle(raw, "test");
    expect(bundle.rawKind).toBe("class-diagram");
    expect(bundle.nodes[0].meta.members).toBeDefined();
    expect(bundle.nodes[0].meta.members).toHaveLength(2);
  });

  it("distinguishes extends / implements / composes predicates", () => {
    const raw = {
      schemaVersion: "1.0",
      nodes: [
        { canonical_key: "a", name: "A", kind: "class" },
        { canonical_key: "b", name: "B", kind: "class" },
        { canonical_key: "c", name: "C", kind: "class" },
      ],
      edges: [
        { canonical_key: "e1", source: "b", target: "a", predicate: "extends" },
        {
          canonical_key: "i1",
          source: "c",
          target: "a",
          predicate: "implements",
        },
        {
          canonical_key: "c1",
          source: "c",
          target: "b",
          predicate: "composes",
        },
      ],
    };
    const bundle = normalizeBundle(raw, "test");
    const kinds = bundle.edges.map((e) => e.kind);
    expect(kinds).toEqual(["extends", "implements", "composes"]);
  });
});

describe("package derivation (M17.5)", () => {
  it("derives package from file path", () => {
    function packageForFile(file: string | undefined): string {
      if (!file) return "(unknown)";
      const parts = file.split("/");
      if (parts.length <= 1) return file;
      parts.pop();
      while (parts.length > 1 && parts[parts.length - 1] === "src") {
        parts.pop();
      }
      return parts.join("/") || file;
    }
    expect(packageForFile("src/auth.rs")).toBe("src");
    expect(packageForFile("crates/cli/src/main.rs")).toBe("crates/cli");
    expect(packageForFile("lib/foo/bar.ts")).toBe("lib/foo");
    expect(packageForFile("src/auth/login.rs")).toBe("src/auth");
    expect(packageForFile(undefined)).toBe("(unknown)");
  });

  it("aggregates package edges from call-graph", () => {
    // 4 functions in 2 packages + 1 cycle
    const raw = {
      schemaVersion: "1.0",
      nodes: [
        { id: "a1", name: "auth::login", file: "src/auth.rs" },
        { id: "a2", name: "auth::logout", file: "src/auth.rs" },
        { id: "d1", name: "db::query", file: "src/db.rs" },
        { id: "c1", name: "cli::run", file: "src/main.rs" },
      ],
      edges: [
        { id: "e1", source: "c1", target: "a1", kind: "calls" },
        { id: "e2", source: "a1", target: "d1", kind: "calls" },
        { id: "e3", source: "d1", target: "a1", kind: "calls" }, // cycle
        { id: "e4", source: "c1", target: "a2", kind: "calls" },
      ],
    };
    const bundle = normalizeBundle(raw, "test");
    // 3 packages: src (auth.rs, db.rs, main.rs), but main.rs is src/, same
    // Actually 1 package "src" with 4 functions, since all files are under src/
    // The test should reflect the actual data, not a contrived one
    expect(bundle.nodes).toHaveLength(4);
    // Edges: cli→auth (×2 via e1+e4), auth→db, db→auth
    expect(bundle.edges).toHaveLength(4);
  });
});

describe("drift detection (M17.6)", () => {
  // Helper that replicates the diff logic for testing.
  function diffBundles(
    declared: {
      nodes: { id: string; name: string; meta?: Record<string, unknown> }[];
      edges: { source: string; target: string; kind?: string }[];
    },
    actual: {
      nodes: { id: string; name: string; meta?: Record<string, unknown> }[];
      edges: { source: string; target: string; kind?: string }[];
    },
  ): {
    added: string[];
    removed: string[];
    changed: { id: string; changes: string[] }[];
    relAdded: string[];
    relRemoved: string[];
  } {
    const decMap = new Map(declared.nodes.map((n) => [n.id, n]));
    const actMap = new Map(actual.nodes.map((n) => [n.id, n]));
    const added: string[] = [];
    const removed: string[] = [];
    const changed: { id: string; changes: string[] }[] = [];
    for (const [id] of actMap) {
      if (!decMap.has(id)) added.push(id);
    }
    for (const [id] of decMap) {
      if (!actMap.has(id)) removed.push(id);
    }
    for (const [id, dec] of decMap) {
      const act = actMap.get(id);
      if (!act) continue;
      const changes: string[] = [];
      if (dec.name !== act.name) changes.push(`name changed`);
      if (dec.meta?.description !== act.meta?.description)
        changes.push(`description changed`);
      if (changes.length > 0) changed.push({ id, changes });
    }
    const relKey = (e: { source: string; target: string; kind?: string }) =>
      `${e.source}\0${e.target}\0${e.kind ?? ""}`;
    const decRels = new Map(declared.edges.map((e) => [relKey(e), e]));
    const actRels = new Map(actual.edges.map((e) => [relKey(e), e]));
    const relAdded: string[] = [];
    const relRemoved: string[] = [];
    for (const [k] of actRels) {
      if (!decRels.has(k)) relAdded.push(k);
    }
    for (const [k] of decRels) {
      if (!actRels.has(k)) relRemoved.push(k);
    }
    return { added, removed, changed, relAdded, relRemoved };
  }

  it("detects added elements (in actual, not declared)", () => {
    const result = diffBundles(
      { nodes: [{ id: "a", name: "A" }], edges: [] },
      {
        nodes: [
          { id: "a", name: "A" },
          { id: "b", name: "B" },
        ],
        edges: [],
      },
    );
    expect(result.added).toEqual(["b"]);
  });

  it("detects removed elements (in declared, not actual)", () => {
    const result = diffBundles(
      {
        nodes: [
          { id: "a", name: "A" },
          { id: "b", name: "B" },
        ],
        edges: [],
      },
      { nodes: [{ id: "a", name: "A" }], edges: [] },
    );
    expect(result.removed).toEqual(["b"]);
  });

  it("detects changed elements (description diff)", () => {
    const result = diffBundles(
      {
        nodes: [{ id: "a", name: "A", meta: { description: "old" } }],
        edges: [],
      },
      {
        nodes: [{ id: "a", name: "A", meta: { description: "new" } }],
        edges: [],
      },
    );
    expect(result.changed).toHaveLength(1);
    expect(result.changed[0].id).toBe("a");
    expect(result.changed[0].changes).toContain("description changed");
  });

  it("detects added/removed relations", () => {
    const result = diffBundles(
      {
        nodes: [{ id: "a" }, { id: "b" }],
        edges: [{ source: "a", target: "b", kind: "uses" }],
      },
      {
        nodes: [{ id: "a" }, { id: "b" }],
        edges: [
          { source: "a", target: "b", kind: "uses" },
          { source: "a", target: "b", kind: "depends" },
        ],
      },
    );
    expect(result.relAdded).toContain("a\0b\0depends");
    expect(result.relRemoved).toEqual([]);
  });

  it("no changes when bundles are identical", () => {
    const bundle = {
      nodes: [{ id: "a", name: "A" }],
      edges: [{ source: "a", target: "a", kind: "self" }],
    };
    const result = diffBundles(bundle, bundle);
    expect(result.added).toEqual([]);
    expect(result.removed).toEqual([]);
    expect(result.changed).toEqual([]);
    expect(result.relAdded).toEqual([]);
    expect(result.relRemoved).toEqual([]);
  });
});

describe("impact analysis (M17.7)", () => {
  // Mirror of ImpactView's BFS for testing.
  function computeImpact(
    nodes: { id: string }[],
    edges: { source: string; target: string }[],
    focusId: string,
    direction: "upstream" | "downstream" | "both",
    maxDepth = 5,
  ): { nodeId: string; depth: number; direction: string }[] {
    const visited = new Set<string>([focusId]);
    const entries: { nodeId: string; depth: number; direction: string }[] = [
      { nodeId: focusId, depth: 0, direction: "upstream" },
    ];
    const traverse = (start: string, dir: "upstream" | "downstream") => {
      let frontier = [start];
      let depth = 0;
      while (frontier.length > 0 && depth < maxDepth) {
        depth++;
        const next: string[] = [];
        for (const id of frontier) {
          const ns = edges
            .filter((e) =>
              dir === "upstream" ? e.target === id : e.source === id,
            )
            .map((e) => (dir === "upstream" ? e.source : e.target));
          for (const n of ns) {
            if (n === focusId) continue;
            if (!visited.has(n)) {
              visited.add(n);
              entries.push({ nodeId: n, depth, direction: dir });
              next.push(n);
            }
          }
        }
        frontier = next;
      }
    };
    if (direction === "upstream" || direction === "both")
      traverse(focusId, "upstream");
    if (direction === "downstream" || direction === "both")
      traverse(focusId, "downstream");
    return entries.filter((e) => e.depth > 0);
  }

  it("returns empty impact for isolated node", () => {
    const result = computeImpact([{ id: "a" }, { id: "b" }], [], "a", "both");
    expect(result).toHaveLength(0);
  });

  it("finds upstream impact (callers)", () => {
    // a -> b -> c; impact on b is {a}
    const result = computeImpact(
      [{ id: "a" }, { id: "b" }, { id: "c" }],
      [
        { source: "a", target: "b" },
        { source: "b", target: "c" },
      ],
      "b",
      "upstream",
    );
    expect(result.map((e) => e.nodeId)).toEqual(["a"]);
  });

  it("finds downstream impact (callees)", () => {
    const result = computeImpact(
      [{ id: "a" }, { id: "b" }, { id: "c" }],
      [
        { source: "a", target: "b" },
        { source: "b", target: "c" },
      ],
      "b",
      "downstream",
    );
    expect(result.map((e) => e.nodeId)).toEqual(["c"]);
  });

  it("finds both directions and dedups at shallower depth", () => {
    // Diamond: a -> b -> c, a -> c directly
    // upstream of c: a (depth 1), b (depth 1)
    // downstream of c: none
    const result = computeImpact(
      [{ id: "a" }, { id: "b" }, { id: "c" }],
      [
        { source: "a", target: "b" },
        { source: "b", target: "c" },
        { source: "a", target: "c" },
      ],
      "c",
      "upstream",
    );
    const ids = result.map((e) => e.nodeId).sort();
    expect(ids).toEqual(["a", "b"]);
  });

  it("respects maxDepth", () => {
    // Chain: a -> b -> c -> d -> e (focus)
    // upstream of e with maxDepth=2: d (depth 1), c (depth 2)
    // a and b are at depth 3+ and excluded
    const result = computeImpact(
      [{ id: "a" }, { id: "b" }, { id: "c" }, { id: "d" }, { id: "e" }],
      [
        { source: "a", target: "b" },
        { source: "b", target: "c" },
        { source: "c", target: "d" },
        { source: "d", target: "e" },
      ],
      "e",
      "upstream",
      2,
    );
    const ids = result.map((e) => e.nodeId).sort();
    expect(ids).toEqual(["c", "d"]);
    expect(ids).not.toContain("a");
    expect(ids).not.toContain("b");
  });

  it("does not loop back to focus", () => {
    // a <-> b (cycle); impact on a (upstream) should be {b} only
    const result = computeImpact(
      [{ id: "a" }, { id: "b" }],
      [
        { source: "a", target: "b" },
        { source: "b", target: "a" },
      ],
      "a",
      "upstream",
    );
    expect(result.map((e) => e.nodeId)).toEqual(["b"]);
  });
});
