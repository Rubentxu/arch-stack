import { describe, it, expect } from "vitest";
import {
  expandLevels,
  blastRadiusOf,
  MAX_DEPTH,
} from "../views/CallGraphGraph";

// Diamond: a -> b, a -> c, b -> d, c -> d
const nodes = [
  { id: "a", label: "a" },
  { id: "b", label: "b" },
  { id: "c", label: "c" },
  { id: "d", label: "d" },
];
const edges = [
  { id: "e1", source: "a", target: "b" },
  { id: "e2", source: "a", target: "c" },
  { id: "e3", source: "b", target: "d" },
  { id: "e4", source: "c", target: "d" },
];

describe("expandLevels (CallGraphGraph)", () => {
  it("expands 1 level downstream", () => {
    const groups = expandLevels(
      nodes as never,
      edges as never,
      "a",
      1,
      "callees",
    );
    expect(groups).toHaveLength(1);
    expect(groups[0].nodes.map((n) => n.id).sort()).toEqual(["b", "c"]);
  });

  it("expands 2 levels downstream and dedupes d", () => {
    const groups = expandLevels(
      nodes as never,
      edges as never,
      "a",
      2,
      "callees",
    );
    expect(groups).toHaveLength(2);
    expect(groups[0].nodes.map((n) => n.id).sort()).toEqual(["b", "c"]);
    expect(groups[1].nodes.map((n) => n.id)).toEqual(["d"]);
  });

  it("expands upstream", () => {
    const groups = expandLevels(
      nodes as never,
      edges as never,
      "d",
      2,
      "callers",
    );
    expect(groups[0].nodes.map((n) => n.id).sort()).toEqual(["b", "c"]);
    expect(groups[1].nodes.map((n) => n.id)).toEqual(["a"]);
  });

  it("expands both directions", () => {
    const groups = expandLevels(nodes as never, edges as never, "b", 1, "both");
    expect(groups[0].nodes.map((n) => n.id).sort()).toEqual(["a", "d"]);
  });

  it("terminates early when frontier is exhausted", () => {
    const groups = expandLevels(
      nodes as never,
      edges as never,
      "a",
      5,
      "callees",
    );
    expect(groups).toHaveLength(2);
  });

  it("returns empty when focus not found", () => {
    const groups = expandLevels(
      nodes as never,
      edges as never,
      "ghost",
      1,
      "callees",
    );
    expect(groups).toHaveLength(0);
  });

  it("exposes MAX_DEPTH for UI controls", () => {
    expect(MAX_DEPTH).toBe(5);
  });

  it("computes blast radius as sum of level nodes", () => {
    const groups = expandLevels(
      nodes as never,
      edges as never,
      "a",
      2,
      "callees",
    );
    expect(blastRadiusOf(groups)).toBe(3); // b, c, d
  });
});
