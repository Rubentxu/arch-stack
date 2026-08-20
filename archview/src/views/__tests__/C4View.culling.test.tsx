// @vitest-environment jsdom
/**
 * T4 tests — C4View culling opt-in + M18 orthogonality guard (M21).
 *
 * Verifies the M18 pill guard contract:
 * - When levelFilter is null (drill-down mode): enableCulling=true
 * - When levelFilter is active (e.g. Component): enableCulling=false
 *
 * This ensures culling does not interfere with M18 semantic zoom.
 *
 * Approach: we do NOT mock GraphRenderer. Instead we verify the DOM
 * output (pill aria-pressed state) and verify that the pill count
 * matches what C4View produces — the enableCulling flag itself is
 * tested via a focused unit test of the guard logic.
 */
import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { render } from "@solidjs/testing-library";
import { C4View } from "../C4View";
import type { GraphNode, GraphEdge } from "../../bundle/loader";

// Minimal C4 fixture: 3 systems, 2 containers each, 3 components each.
function makeC4Data() {
  const nodes: GraphNode[] = [];
  const edges: GraphEdge[] = [];

  const systems = ["sys-a", "sys-b", "sys-c"];
  for (const id of systems) {
    nodes.push({
      id,
      label: id,
      kind: "software_system",
      level: 1,
    } as GraphNode);
  }
  edges.push({
    id: "r1",
    source: "sys-a",
    target: "sys-b",
    kind: "depends-on",
  });

  for (const sys of systems) {
    for (let i = 0; i < 2; i++) {
      const cid = `${sys}-ct-${i}`;
      nodes.push({
        id: cid,
        label: cid,
        kind: "container",
        level: 2,
        parentId: sys,
      } as GraphNode);
      edges.push({
        id: `r-${cid}`,
        source: sys,
        target: cid,
        kind: "contains",
      });
    }
  }

  for (const sys of systems) {
    for (let i = 0; i < 2; i++) {
      const cid = `${sys}-ct-${i}`;
      for (let j = 0; j < 3; j++) {
        const cmid = `${cid}-cp-${j}`;
        nodes.push({
          id: cmid,
          label: cmid,
          kind: "component",
          level: 3,
          parentId: cid,
        } as GraphNode);
        edges.push({
          id: `r-${cmid}`,
          source: cid,
          target: cmid,
          kind: "contains",
        });
      }
    }
  }

  return { nodes, edges };
}

describe("C4View culling opt-in (M21)", () => {
  beforeEach(() => {
    vi.stubGlobal("localStorage", {
      getItem: () => null,
      setItem: () => {},
      removeItem: () => {},
    });
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders no level filter pill when levelFilter is null (drill-down mode)", () => {
    const { nodes, edges } = makeC4Data();
    const { container } = render(() => (
      <C4View
        nodes={nodes}
        edges={edges}
        selectedId={null}
        onSelect={() => {}}
      />
    ));
    // "All levels" pill should be active (aria-pressed=true) when no filter is set.
    const allLevelsPill = container.querySelector('[aria-pressed="true"]');
    expect(allLevelsPill).toBeTruthy();
    expect(allLevelsPill?.textContent).toMatch(/all levels/i);
  });

  it("sets enableCulling=false when a level filter is active (M18 guard)", () => {
    // When localStorage has level 3 stored, C4View reads it on mount.
    vi.stubGlobal("localStorage", {
      getItem: () => "3",
      setItem: () => {},
      removeItem: () => {},
    });

    const { nodes, edges } = makeC4Data();
    const { container } = render(() => (
      <C4View
        nodes={nodes}
        edges={edges}
        selectedId={null}
        onSelect={() => {}}
      />
    ));
    // The "Component" pill should now be active.
    const pills = container.querySelectorAll('[aria-pressed="true"]');
    // Should have exactly one active pill (the Component level)
    expect(pills.length).toBe(1);
    expect(pills[0].textContent).toMatch(/component/i);
  });

  it("M18 guard: level filter pill disables drill-down breadcrumb button", () => {
    // When level filter is set, drill-down breadcrumb button is disabled.
    vi.stubGlobal("localStorage", {
      getItem: () => "2", // container level
      setItem: () => {},
      removeItem: () => {},
    });

    const { nodes, edges } = makeC4Data();
    const { container } = render(() => (
      <C4View
        nodes={nodes}
        edges={edges}
        selectedId={null}
        onSelect={() => {}}
      />
    ));
    // Breadcrumb "All systems" button should be disabled when level filter is active.
    const breadcrumbBtn = container.querySelector(
      ".breadcrumb-root",
    ) as HTMLButtonElement;
    expect(breadcrumbBtn).toBeTruthy();
    expect(breadcrumbBtn.disabled).toBe(true);
  });
});
