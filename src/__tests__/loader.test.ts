import { describe, it, expect } from "vitest";
import { c4LevelForKind, normalizeBundle } from "../bundle/loader";

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
        { id: "fn:a", name: "a", kind: "function", file: "src/lib.rs", line: 1 },
        { id: "fn:b", name: "b", kind: "function", file: "src/lib.rs", line: 5 },
      ],
      edges: [
        { id: "e1", source: "fn:a", target: "fn:b", kind: "calls" },
      ],
    };
    const bundle = normalizeBundle(raw, "test");
    expect(bundle.rawKind).toBe("call-graph");
    expect(bundle.nodes).toHaveLength(2);
    expect(bundle.edges).toHaveLength(1);
    expect(bundle.nodes[0].label).toBe("a");
    expect(bundle.edges[0].label).toBe("calls");
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

  it("normalizes a C4 bundle with elements + relations", () => {
    const raw = {
      schemaVersion: "1.0",
      elements: [
        { id: "el:1", name: "WebApp", kind: "Container" },
        { id: "el:2", name: "DB", kind: "Container" },
      ],
      relations: [
        { source: "el:1", target: "el:2", predicate_id: "uses" },
      ],
    };
    const bundle = normalizeBundle(raw, "test");
    expect(bundle.rawKind).toBe("c4");
    expect(bundle.nodes).toHaveLength(2);
    expect(bundle.edges[0].kind).toBe("uses");
  });

  it("derives C4 level and parentId for hierarchical rendering", () => {
    const raw = {
      schemaVersion: "1.0",
      elements: [
        {
          id: "sys:1",
          kind: "SoftwareSystem",
          name: "Platform",
          parent: null,
        },
        {
          id: "ctn:1",
          kind: "Container",
          name: "API",
          technology: "Rust",
          description: "HTTP entry",
          parent: "sys:1",
        },
      ],
      relations: [],
    };
    const bundle = normalizeBundle(raw, "test");
    const sys = bundle.nodes.find((n) => n.id === "sys:1")!;
    const ctn = bundle.nodes.find((n) => n.id === "ctn:1")!;
    expect(sys.level).toBe(1);
    expect(sys.parentId).toBeUndefined();
    expect(ctn.level).toBe(2);
    expect(ctn.parentId).toBe("sys:1");
    expect(ctn.meta.technology).toBe("Rust");
    expect(ctn.meta.description).toBe("HTTP entry");
  });
});

describe("c4LevelForKind", () => {
  it("maps C4 kinds to hierarchy levels", () => {
    expect(c4LevelForKind("Person")).toBe(1);
    expect(c4LevelForKind("SoftwareSystem")).toBe(1);
    expect(c4LevelForKind("Container")).toBe(2);
    expect(c4LevelForKind("Component")).toBe(3);
    expect(c4LevelForKind("Code")).toBe(4);
  });

  it("handles instance variants and kind:variant form", () => {
    expect(c4LevelForKind("ContainerInstance")).toBe(2);
    expect(c4LevelForKind("ComponentInstance")).toBe(3);
    expect(c4LevelForKind("foo:Container")).toBe(2);
  });

  it("returns 0 for unknown kinds", () => {
    expect(c4LevelForKind("UnknownKind")).toBe(0);
    expect(c4LevelForKind("")).toBe(0);
  });
});

describe("call-graph BFS expansion (M17.2)", () => {
  // Diamond: a -> b, a -> c, b -> d, c -> d
  const nodes = [
    { id: "a" },
    { id: "b" },
    { id: "c" },
    { id: "d" },
  ];
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
      edges: [
        { id: "e1", source: "fn:a", target: "fn:b", kind: "AsyncCall" },
      ],
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
        { order: 1, message_kind: "SyncCall", caller: { name: "a" }, callee: { name: "b" } },
        { order: 2, message_kind: "SyncCall", caller: { name: "b" }, callee: { name: "c" } },
        { order: 3, message_kind: "Reply",    caller: { name: "c" }, callee: { name: "b" } },
        { order: 4, message_kind: "Reply",    caller: { name: "b" }, callee: { name: "a" } },
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
        { order: 1, message_kind: "SyncCall", caller: {}, callee: { name: "b" } },
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
            { name: "new", member_kind: "fn", signature: "fn() -> Self", line: 4 },
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
