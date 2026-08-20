// @vitest-environment jsdom
/**
 * Integration tests for sidebar tabs (M22).
 *
 * Verifies:
 *   - Default tab is "evidence" when a node is selected
 *   - Clicking "Relations" tab shows the VirtualList
 *   - Badge counts are correct (omitted when 0)
 *   - Switching node resets tab back to "evidence"
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, fireEvent } from "@solidjs/testing-library";
import { Sidebar } from "../components/Sidebar";
import type { GraphNode } from "../bundle/loader";
import type { RendererEdge } from "../types";

const NODE_WITH_EVIDENCE_AND_RELATIONS: GraphNode = {
  id: "n1",
  label: "TestNode",
  kind: "function",
  level: 0,
  meta: {
    evidence_refs: [
      { file: "src/main.rs", line: 10, confidence: 0.9 },
      { file: "src/main.rs", line: 20, confidence: 0.85 },
      { file: "src/main.rs", line: 30, confidence: 0.8 },
    ],
  },
};

function makeEdges(sourceId: string, _targetId: string, count: number): RendererEdge[] {
  const edges: RendererEdge[] = [];
  for (let i = 0; i < count; i++) {
    edges.push({
      id: `e${i}`,
      source: sourceId,
      target: `other${i}`,
      label: "calls",
    });
  }
  return edges;
}

function getTabs(container: HTMLElement) {
  return Array.from(container.querySelectorAll('[role="tab"]')) as HTMLButtonElement[];
}

afterEach(() => {
  cleanup();
});

describe("Sidebar tabs (M22)", () => {
  it("defaults to evidence tab when a node with evidence and relations is selected", () => {
    const edges = makeEdges("n1", "other", 12);
    const { container } = render(() => (
      <Sidebar
        node={NODE_WITH_EVIDENCE_AND_RELATIONS}
        bundleMeta={{
          source: "test",
          schemaVersion: "1.0",
          loadedAt: "2026-08-01T00:00:00Z",
          rawKind: "call-graph",
        }}
        edges={edges}
      />
    ));
    const tabs = getTabs(container);
    expect(tabs).toHaveLength(2);
    // Tab 0 (Evidence) should be active
    expect(tabs[0].getAttribute("aria-selected")).toBe("true");
    expect(tabs[0].textContent).toContain("Evidence");
    // Badge shows evidence count (3)
    expect(tabs[0].querySelector(".tab-badge")?.textContent).toContain("3");
    // Tab 1 (Relations) should not be active
    expect(tabs[1].getAttribute("aria-selected")).toBe("false");
  });

  it("clicking Relations tab shows the VirtualList", () => {
    const edges = makeEdges("n1", "other", 12);
    const { container } = render(() => (
      <Sidebar
        node={NODE_WITH_EVIDENCE_AND_RELATIONS}
        bundleMeta={{
          source: "test",
          schemaVersion: "1.0",
          loadedAt: "2026-08-01T00:00:00Z",
          rawKind: "call-graph",
        }}
        edges={edges}
      />
    ));
    const tabs = getTabs(container);
    expect(tabs).toHaveLength(2);
    // Click Relations tab
    fireEvent.click(tabs[1]);
    // Now Relations should be active
    expect(tabs[1].getAttribute("aria-selected")).toBe("true");
    // VirtualList should be visible
    const vl = container.querySelector('[aria-label="Node relations"]');
    expect(vl).toBeTruthy();
  });

  it("badge counts are correct — omitted when 0", () => {
    // Node with evidence (3) but no relations (empty edge list for this node)
    const { container } = render(() => (
      <Sidebar
        node={NODE_WITH_EVIDENCE_AND_RELATIONS}
        bundleMeta={{
          source: "test",
          schemaVersion: "1.0",
          loadedAt: "2026-08-01T00:00:00Z",
          rawKind: "call-graph",
        }}
        edges={[]}
      />
    ));
    const tabs = getTabs(container);
    expect(tabs).toHaveLength(2);
    // Tab 0 (Evidence) should show badge 3
    expect(tabs[0].querySelector(".tab-badge")?.textContent).toContain("3");
    // Tab 1 (Relations) should NOT show a badge (no relations)
    expect(tabs[1].querySelector(".tab-badge")).toBeNull();
  });

  it("selecting a different node resets tab back to evidence", () => {
    const nodeA = { ...NODE_WITH_EVIDENCE_AND_RELATIONS, id: "n1" };
    const nodeB: GraphNode = {
      id: "n2",
      label: "NodeB",
      kind: "function",
      level: 0,
      meta: {},
    };
    const { container: containerA } = render(() => (
      <Sidebar
        node={nodeA}
        bundleMeta={{
          source: "test",
          schemaVersion: "1.0",
          loadedAt: "2026-08-01T00:00:00Z",
          rawKind: "call-graph",
        }}
        edges={makeEdges("n1", "other", 5)}
      />
    ));
    const tabsA = getTabs(containerA);
    expect(tabsA).toHaveLength(2);
    // Switch to Relations tab
    fireEvent.click(tabsA[1]);
    expect(tabsA[1].getAttribute("aria-selected")).toBe("true");

    // Now render with nodeB (simulating node change via props)
    const { container: containerB } = render(() => (
      <Sidebar
        node={nodeB}
        bundleMeta={{
          source: "test",
          schemaVersion: "1.0",
          loadedAt: "2026-08-01T00:00:00Z",
          rawKind: "call-graph",
        }}
        edges={makeEdges("n2", "other", 8)}
      />
    ));
    // Tab should reset to evidence (default state for new node)
    const tabsB = getTabs(containerB);
    expect(tabsB[0].getAttribute("aria-selected")).toBe("true");
    expect(tabsB[1].getAttribute("aria-selected")).toBe("false");
  });
});
