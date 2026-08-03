import { describe, it, expect } from "vitest";
import {
  buildPackageEdges,
  detectCycles,
  packageForFile,
} from "../views/PackageGraph";

describe("buildPackageEdges (M17.5 view extract)", () => {
  it("returns empty list when no cross-package edges exist", () => {
    const edges = buildPackageEdges(
      [
        { id: "a", file: "src/a.rs" },
        { id: "b", file: "src/b.rs" },
      ],
      [{ source: "a", target: "b" }],
    );
    // Same package (src/), so no inter-package edge
    expect(edges).toEqual([]);
  });

  it("aggregates multiple call sites into one edge with weight", () => {
    const edges = buildPackageEdges(
      [
        { id: "a1", file: "src/a.rs" },
        { id: "a2", file: "src/a.rs" },
        { id: "b1", file: "crates/b/src/lib.rs" },
      ],
      [
        { source: "a1", target: "b1" },
        { source: "a2", target: "b1" },
      ],
    );
    expect(edges).toHaveLength(1);
    expect(edges[0]).toEqual({
      source: "src",
      target: "crates/b",
      weight: 2,
    });
  });

  it("ignores edges where source or target file is unknown", () => {
    const edges = buildPackageEdges(
      [{ id: "a", file: "src/a.rs" }],
      [{ source: "a", target: "ghost" }],
    );
    expect(edges).toEqual([]);
  });

  it("handles Rust workspace crates/*/src/* convention", () => {
    const edges = buildPackageEdges(
      [
        { id: "a", file: "crates/cli/src/main.rs" },
        { id: "b", file: "crates/core/src/lib.rs" },
      ],
      [{ source: "a", target: "b" }],
    );
    expect(edges).toHaveLength(1);
    expect(edges[0].source).toBe("crates/cli");
    expect(edges[0].target).toBe("crates/core");
  });
});

describe("detectCycles (M17.5 view extract)", () => {
  it("returns empty set for acyclic graph", () => {
    const edges = [
      { source: "a", target: "b", weight: 1 },
      { source: "b", target: "c", weight: 1 },
    ];
    const cycles = detectCycles(edges);
    expect(cycles.size).toBe(0);
  });

  it("detects simple 2-node cycle", () => {
    const edges = [
      { source: "a", target: "b", weight: 1 },
      { source: "b", target: "a", weight: 1 },
    ];
    const cycles = detectCycles(edges);
    expect(cycles.size).toBe(2);
    expect(cycles.has("a\0b")).toBe(true);
    expect(cycles.has("b\0a")).toBe(true);
  });

  it("detects 3-node cycle", () => {
    const edges = [
      { source: "a", target: "b", weight: 1 },
      { source: "b", target: "c", weight: 1 },
      { source: "c", target: "a", weight: 1 },
    ];
    const cycles = detectCycles(edges);
    // All 3 edges are part of the cycle
    expect(cycles.size).toBe(3);
  });

  it("detects cycle in subset of edges", () => {
    // a → b → a is a cycle, but a → c is not
    const edges = [
      { source: "a", target: "b", weight: 1 },
      { source: "b", target: "a", weight: 1 },
      { source: "a", target: "c", weight: 1 },
    ];
    const cycles = detectCycles(edges);
    expect(cycles.size).toBe(2); // only a→b and b→a
    expect(cycles.has("a\0c")).toBe(false);
  });
});
