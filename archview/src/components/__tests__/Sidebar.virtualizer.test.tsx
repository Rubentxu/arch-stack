// @vitest-environment jsdom
/**
 * Verifies the Sidebar's relations list is virtualized end-to-end
 * (M20). Constructs a Sidebar with a selected hub node that has
 * 1000+ relations and asserts the rendered DOM count is bounded
 * (~14 rows regardless of the underlying edge count).
 */
import { describe, expect, it } from "vitest";
import { render } from "@solidjs/testing-library";
import { Sidebar } from "../Sidebar";
import type { GraphNode } from "../../bundle/loader";
import type { RendererEdge } from "../../types";

function makeHubNode(id: string): GraphNode {
  return {
    id,
    label: id,
    kind: "system",
    level: 1,
  } as GraphNode;
}

function makeManyRelations(hubId: string, n: number): RendererEdge[] {
  const edges: RendererEdge[] = [];
  for (let i = 0; i < n; i++) {
    edges.push({
      id: `rel:${i}`,
      source: `container:peer-${i}`,
      target: hubId,
      label: "depends-on",
    });
  }
  return edges;
}

describe("Sidebar virtualized relations list (M20)", () => {
  it("renders a bounded DOM slice even with 1000 relations on the hub", () => {
    const hub = makeHubNode("system:core");
    const edges = makeManyRelations("system:core", 1000);
    const { container } = render(() => <Sidebar node={hub} edges={edges} />);
    // The virtual list's role="list" with aria-label="Node relations"
    // is the host. Inside it, the rendered rows are .rel items.
    const vl = container.querySelector(
      '[aria-label="Node relations"]',
    ) as HTMLElement;
    expect(vl).toBeTruthy();
    // The visible listitem count is bounded (visible rows + overscan).
    const listitems = vl.querySelectorAll('[role="listitem"]');
    // itemHeight 28, viewport 220 → ~8 visible + 2*4 overscan = 16, but
    // at scrollTop=0 the top overscan is clamped, so ~12.
    expect(listitems.length).toBeLessThan(20);
    expect(listitems.length).toBeGreaterThan(0);
    // The spacer height reflects the total (1000 * 28 = 28000px).
    const spacer = vl.querySelector("div > div") as HTMLElement;
    expect(spacer.style.height).toBe("28000px");
  });

  it("renders ALL relations if the count is small (no virtualizer overhead)", () => {
    const hub = makeHubNode("system:core");
    const edges = makeManyRelations("system:core", 5);
    const { container } = render(() => <Sidebar node={hub} edges={edges} />);
    const vl = container.querySelector(
      '[aria-label="Node relations"]',
    ) as HTMLElement;
    const listitems = vl.querySelectorAll('[role="listitem"]');
    // All 5 should be rendered (they fit comfortably in the viewport).
    expect(listitems.length).toBe(5);
  });
});
