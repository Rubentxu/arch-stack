import { describe, it, expect } from "vitest";
import { c4LevelForKind, normalizeBundle } from "../bundle/loader";

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
