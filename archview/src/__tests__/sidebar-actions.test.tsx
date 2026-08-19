// @vitest-environment jsdom
/**
 * Sidebar action palette tests (ADR-062, items 32–33).
 * Covers spec scenarios S2 (no zoom out at root), S3 (actions offered),
 * S4 (copy id without server), S13 (strict hides explain), and the
 * client-side wiring of zoom + explain.
 */

import { afterEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@solidjs/testing-library";
import { Sidebar } from "../components/Sidebar";
import type { GraphNode } from "../bundle/loader";
import type { RendererEdge } from "../types";
import type { ExplainResult } from "../lib/workspace";

const CONTAINER_NODE: GraphNode = {
  id: "elm:container-a",
  label: "Core API",
  kind: "container",
  level: 2,
  parentId: "elm:ctx",
  meta: {},
};

const ROOT_NODE: GraphNode = {
  id: "elm:ctx",
  label: "System",
  kind: "context",
  level: 1,
  meta: {},
};

const EDGES: RendererEdge[] = [
  {
    id: "e1",
    source: "elm:container-a",
    target: "elm:container-b",
    label: "calls",
  },
  {
    id: "e2",
    source: "elm:container-c",
    target: "elm:container-a",
    kind: "Uses",
  },
];

const EXPLAIN_REPORT: ExplainResult = {
  schemaVersion: "1.0",
  capability: "explain",
  subject: {
    kind: "element",
    id: "elm:container-a",
    statement: "Core API is a container backed by 2 evidence entries",
  },
  provenance: { evidence: [], unsubstantiated: true },
  warnings: [],
};

function renderSidebar(opts: {
  node: GraphNode;
  onZoom?: (dir: "in" | "out") => void;
  onExplain?: (id: string) => Promise<ExplainResult>;
  edges?: RendererEdge[];
}) {
  return render(() => (
    <Sidebar
      node={opts.node}
      bundleMeta={{
        source: "test",
        schemaVersion: "1.0",
        loadedAt: "2026-01-01T00:00:00Z",
        rawKind: "c4",
      }}
      onZoom={opts.onZoom}
      onExplain={opts.onExplain}
      edges={opts.edges ?? []}
    />
  ));
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("Sidebar action palette", () => {
  it("offers zoom in/out, copy id and explain for a C4 container (S3)", () => {
    renderSidebar({
      node: CONTAINER_NODE,
      onExplain: async () => EXPLAIN_REPORT,
    });
    expect(screen.getByRole("button", { name: "copy id" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "zoom in" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "zoom out" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "explain" })).toBeTruthy();
  });

  it("lists only the relations touching the selected node (S3)", () => {
    renderSidebar({ node: CONTAINER_NODE, edges: EDGES });
    const list = document.querySelector(".relations-list");
    expect(list).not.toBeNull();
    expect(list!.textContent).toContain("elm:container-b");
    expect(list!.textContent).toContain("elm:container-c");
    // Direction markers: out → container-b, in ← container-c.
    expect(list!.querySelectorAll(".rel.out").length).toBe(1);
    expect(list!.querySelectorAll(".rel.in").length).toBe(1);
  });

  it("fires onZoom with the requested direction (S1/S5 wiring)", () => {
    const onZoom = vi.fn();
    renderSidebar({ node: CONTAINER_NODE, onZoom });
    fireEvent.click(screen.getByRole("button", { name: "zoom in" }));
    fireEvent.click(screen.getByRole("button", { name: "zoom out" }));
    expect(onZoom).toHaveBeenCalledWith("in");
    expect(onZoom).toHaveBeenCalledWith("out");
  });

  it("does not offer zoom out for a root-level node (S2)", () => {
    renderSidebar({ node: ROOT_NODE });
    expect(screen.getByRole("button", { name: "zoom in" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "zoom out" })).toBeNull();
  });

  it("renders the explain statement after resolving (S7 client side)", async () => {
    const onExplain = vi.fn(async (id: string) => {
      expect(id).toBe("elm:container-a");
      return EXPLAIN_REPORT;
    });
    renderSidebar({ node: CONTAINER_NODE, onExplain });
    fireEvent.click(screen.getByRole("button", { name: "explain" }));
    await waitFor(() =>
      expect(document.querySelector(".explain-result")?.textContent).toContain(
        "Core API is a container",
      ),
    );
    expect(onExplain).toHaveBeenCalledOnce();
  });

  it("copies the canonical id to the clipboard without a server (S4)", async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText },
      configurable: true,
    });
    renderSidebar({ node: ROOT_NODE });
    fireEvent.click(screen.getByRole("button", { name: "copy id" }));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith("elm:ctx"));
    expect(screen.getByText("copied ✓")).toBeTruthy();
  });

  it("hides explain for strict bundles but keeps copy id and zoom (S13)", () => {
    renderSidebar({ node: CONTAINER_NODE }); // no onExplain prop = strict
    expect(screen.queryByRole("button", { name: "explain" })).toBeNull();
    expect(screen.getByRole("button", { name: "copy id" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "zoom in" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "zoom out" })).toBeTruthy();
  });
});
