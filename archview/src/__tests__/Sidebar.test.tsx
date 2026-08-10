// @vitest-environment jsdom
/**
 * Sidebar integration tests (H1, ADR-041 §5–§6).
 *
 * Verifies that the SourceDrawer renders conditionally based on whether
 * the selected node carries a resolvable `file:line` evidence pointer.
 */

import { describe, it, expect, vi } from "vitest";
import { render } from "@solidjs/testing-library";
import { Sidebar } from "../components/Sidebar";
import type { GraphNode } from "../bundle/loader";

const META_NODE: GraphNode = {
  id: "n1",
  label: "main",
  kind: "function",
  level: 0,
  meta: {
    evidence_refs: [{ file: "src/main.rs", line: 10, confidence: 0.95 }],
  },
};

const FILE_ONLY_NODE: GraphNode = {
  id: "n2",
  label: "thing",
  kind: "function",
  file: "src/thing.rs",
  line: 42,
  meta: {},
};

const NO_FILE_NODE: GraphNode = {
  id: "n3",
  label: "orphan",
  kind: "function",
  meta: {},
};

function renderSidebar(node: GraphNode | null) {
  const fetchSource = vi.fn(async () => ({
    file: "src/main.rs",
    start_line: 8,
    total_lines: 20,
    content: ["line8", "line9", "line10", "line11", "line12"],
    truncated: false,
  }));
  const openInEditor = vi.fn(async () => true);
  return {
    fetchSource,
    openInEditor,
    ...render(() => (
      <Sidebar
        node={node}
        bundleMeta={{
          source: "test",
          schemaVersion: "1.0",
          loadedAt: "2026-08-10T00:00:00Z",
          rawKind: "call-graph",
        }}
        onFetchSource={fetchSource}
        onOpenInEditor={openInEditor}
      />
    )),
  };
}

describe("<Sidebar> + SourceDrawer integration", () => {
  it("renders SourceDrawer when node has evidence_refs with file + numeric line", async () => {
    const { findByRole, unmount } = renderSidebar(META_NODE);
    // SourceDrawer renders <section role="region"> — unique across the page.
    expect(await findByRole("region")).toBeTruthy();
    unmount();
  });

  it("renders SourceDrawer when node has file + line directly", async () => {
    const { findByRole, unmount } = renderSidebar(FILE_ONLY_NODE);
    expect(await findByRole("region")).toBeTruthy();
    unmount();
  });

  it("does NOT render SourceDrawer when node has no file/line", () => {
    const { queryByRole, unmount } = renderSidebar(NO_FILE_NODE);
    expect(queryByRole("region")).toBeNull();
    unmount();
  });

  it("does NOT render SourceDrawer when node is null", () => {
    const { queryByRole, unmount } = renderSidebar(null);
    expect(queryByRole("region")).toBeNull();
    unmount();
  });
});
