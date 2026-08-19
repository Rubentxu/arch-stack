// @vitest-environment jsdom
/**
 * Verifies the C4 view's relations footer is virtualized (M20).
 *
 * The footer renders ALL visible edges from the current focus
 * node. For a stress sample with 1000+ edges this would be 1000+
 * `<li>` elements. The VirtualList caps the rendered DOM to a
 * bounded window (visible + overscan).
 */
import { describe, expect, it } from "vitest";
import { render } from "@solidjs/testing-library";
import { C4View } from "../C4View";
import type { GraphNode, GraphEdge } from "../../bundle/loader";

function makeData(nodeCount: number, edgesPerNode: number) {
  const nodes: GraphNode[] = [];
  const edges: GraphEdge[] = [];
  for (let i = 0; i < nodeCount; i++) {
    nodes.push({
      id: `system:sys-${i}`,
      label: `sys-${i}`,
      kind: "system",
      level: 1,
    } as GraphNode);
  }
  for (let i = 0; i < nodeCount; i++) {
    for (let j = 0; j < edgesPerNode; j++) {
      const target = (i + 1 + j) % nodeCount;
      edges.push({
        id: `rel:${i}-${j}`,
        source: `system:sys-${i}`,
        target: `system:sys-${target}`,
        kind: "depends-on",
      });
    }
  }
  return { nodes, edges };
}

describe("C4View virtualized relations footer (M20)", () => {
  it("keeps the rendered DOM bounded even with 1000+ edges", () => {
    // 100 nodes, 10 edges each = 1000 edges. The VirtualList should
    // cap the rendered rows to ~18 (180px viewport / 24px itemHeight
    // = 7.5 → 8 visible + 5+5 overscan = 18).
    const { nodes, edges } = makeData(100, 10);
    const { container } = render(() => (
      <C4View
        nodes={nodes}
        edges={edges}
        selectedId={null}
        onSelect={() => {}}
      />
    ));
    const vl = container.querySelector(
      '[aria-label="Visible C4 relations"]',
    ) as HTMLElement;
    expect(vl).toBeTruthy();
    const listitems = vl.querySelectorAll('[role="listitem"]');
    // Should be bounded (not 1000).
    expect(listitems.length).toBeLessThan(25);
    expect(listitems.length).toBeGreaterThan(0);
    // Spacer height reflects total (1000 * 24 = 24000px).
    const spacer = vl.querySelector("div > div") as HTMLElement;
    expect(spacer.style.height).toBe("24000px");
  });
});
