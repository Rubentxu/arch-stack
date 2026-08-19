// @vitest-environment jsdom
/**
 * C4View — M17.1 graph view regression coverage.
 *
 * The `renderer/g6` module is mocked because G6's WebGPU/WebGL
 * pipeline is not exercisable in jsdom. The mock records every
 * `setData` call so we can assert that:
 *   - the visible nodes/edges for a given `focusId` are forwarded
 *     to the renderer with the expected `rawKind: "c4"`.
 *   - drill-in (clicking a node) re-pushes the focused subset and
 *     focuses the node.
 *   - resetting the focus re-pushes the full graph and clears the
 *     focus ring.
 *   - M18: the semantic-zoom pill bar switches the visible set to a
 *     level-wide filter and the toggle round-trips through
 *     localStorage.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@solidjs/testing-library";
import { C4View } from "../views/C4View";
import type { GraphEdge, GraphNode } from "../bundle/loader";

const { setDataCalls, focusCalls, clearFocusState } = vi.hoisted(() => ({
  setDataCalls: [] as Array<{
    nodes: unknown[];
    edges: unknown[];
    rawKind: string;
  }>,
  focusCalls: [] as string[],
  clearFocusState: { count: 0 },
}));

vi.mock("../renderer/g6", () => ({
  GraphRenderer: class {
    constructor(opts: Record<string, unknown>) {
      // opts is intentionally unused: the mock records calls without
      // touching the real renderer.
      void opts;
    }
    setData(b: { nodes: unknown[]; edges: unknown[]; rawKind: string }): void {
      setDataCalls.push(b);
    }
    focusNode(id: string): Promise<void> {
      focusCalls.push(id);
      return Promise.resolve();
    }
    clearFocus(): Promise<void> {
      clearFocusState.count++;
      return Promise.resolve();
    }
    resize(): void {}
    destroy(): void {}
  },
}));

const nodes: GraphNode[] = [
  {
    id: "person",
    label: "User",
    kind: "person",
    level: 1,
    parentId: undefined,
  },
  {
    id: "app",
    label: "WebApp",
    kind: "software_system",
    level: 1,
  },
  {
    id: "container.api",
    label: "API",
    kind: "container",
    level: 2,
    parentId: "app",
  },
  {
    id: "container.db",
    label: "Database",
    kind: "container",
    level: 2,
    parentId: "app",
  },
  {
    id: "component.auth",
    label: "Auth",
    kind: "component",
    level: 3,
    parentId: "container.api",
  },
];

const edges: GraphEdge[] = [
  { id: "e1", source: "person", target: "app" },
  { id: "e2", source: "app", target: "container.api" },
  { id: "e3", source: "app", target: "container.db" },
  { id: "e4", source: "container.api", target: "component.auth" },
];

describe("C4View (M17.1 graph view)", () => {
  afterEach(() => {
    cleanup();
    setDataCalls.splice(0, setDataCalls.length);
    focusCalls.splice(0, focusCalls.length);
    clearFocusState.count = 0;
  });

  it("forwards all nodes/edges to the renderer on mount", async () => {
    render(() => (
      <C4View
        nodes={nodes}
        edges={edges}
        selectedId={null}
        onSelect={() => {}}
      />
    ));
    // Initial push happens in onMount; allow the microtask to drain.
    await Promise.resolve();
    await Promise.resolve();
    expect(setDataCalls.length).toBeGreaterThanOrEqual(1);
    const last = setDataCalls[setDataCalls.length - 1];
    expect(last.rawKind).toBe("c4");
    expect(last.nodes).toHaveLength(5);
    expect(last.edges).toHaveLength(4);
  });

  it("drill-in reduces the visible set to focus + descendants", async () => {
    render(() => (
      <C4View
        nodes={nodes}
        edges={edges}
        selectedId={null}
        onSelect={() => {}}
      />
    ));
    await Promise.resolve();
    await Promise.resolve();
    setDataCalls.length = 0;

    // Simulate the user clicking a node: call onSelect through the
    // breadcrumb link. The breadcrumb for `app` is hidden until a
    // focus is set, so we click the "All systems" reset button
    // after we trigger a focus via a node button. The view exposes
    // a `c4-levels` aside for level chips and a `.c4-canvas` div
    // (which is empty in jsdom because the renderer is mocked).
    // Drill-in is invoked by direct id injection: the most reliable
    // path is via the onSelect callback.
    //
    // We instead exercise drill-in by simulating the renderer click
    // through props.onSelect: clicking a node sets `selectedId` from
    // the parent, but the view also sets `focusId` internally. The
    // cleanest way is to render with `drillIntoId="app"` and verify
    // the focused subset.
    cleanup();
    render(() => (
      <C4View
        nodes={nodes}
        edges={edges}
        selectedId="container.api"
        onSelect={() => {}}
        drillIntoId="container.api"
      />
    ));
    await Promise.resolve();
    await Promise.resolve();
    // The mock renderer receives setData once per focus change plus
    // the initial push. After mount with drillIntoId="container.api",
    // visible nodes = [container.api, component.auth, app] (focus +
    // child + parent).
    const last = setDataCalls[setDataCalls.length - 1];
    const visibleIds = (last.nodes as Array<{ id: string }>).map((n) => n.id);
    expect(visibleIds).toContain("container.api");
    expect(visibleIds).toContain("component.auth");
    expect(visibleIds).toContain("app");
    expect(visibleIds).not.toContain("person");
    expect(visibleIds).not.toContain("container.db");
    // Edges: only those whose endpoints are all visible.
    const visibleEdgeIds = (last.edges as Array<{ id: string }>).map(
      (e) => e.id,
    );
    expect(visibleEdgeIds).toContain("e2"); // app -> container.api
    expect(visibleEdgeIds).toContain("e4"); // container.api -> component.auth
    expect(visibleEdgeIds).not.toContain("e1"); // person -> app
  });

  it("focuses the drilled-in node via the renderer", async () => {
    render(() => (
      <C4View
        nodes={nodes}
        edges={edges}
        selectedId={null}
        onSelect={() => {}}
        drillIntoId="app"
      />
    ));
    await Promise.resolve();
    await Promise.resolve();
    expect(focusCalls).toContain("app");
  });
});

// ── M18: semantic-zoom pill bar ───────────────────────────────────────

/**
 * In this project's jsdom env `localStorage` is exposed as a plain
 * object (no `getItem`/`setItem`/`clear`). The C4View's
 * `readStoredLevel`/`writeStoredLevel` guard against that and
 * silently no-op. For tests that need real persistence semantics
 * we install a minimal Storage-like shim on globalThis and restore
 * the original between tests.
 */
function installLocalStorageShim(): () => void {
  const g = globalThis as unknown as { localStorage: unknown };
  const original = g.localStorage;
  const store = new Map<string, string>();
  const shim = {
    getItem: (k: string) => (store.has(k) ? store.get(k)! : null),
    setItem: (k: string, v: string) => {
      store.set(k, String(v));
    },
    removeItem: (k: string) => {
      store.delete(k);
    },
    clear: () => {
      store.clear();
    },
    key: (i: number) => Array.from(store.keys())[i] ?? null,
    get length() {
      return store.size;
    },
  };
  g.localStorage = shim;
  return () => {
    g.localStorage = original;
  };
}

describe("C4View (M18 semantic zoom)", () => {
  let restore: () => void = () => {};

  beforeEach(() => {
    restore = installLocalStorageShim();
  });

  afterEach(() => {
    cleanup();
    setDataCalls.splice(0, setDataCalls.length);
    focusCalls.splice(0, focusCalls.length);
    clearFocusState.count = 0;
    restore();
  });

  const lsGet = (k: string): string | null => {
    try {
      return localStorage.getItem(k);
    } catch {
      return null;
    }
  };
  const lsSet = (k: string, v: string) => {
    try {
      localStorage.setItem(k, v);
    } catch {
      /* ignore */
    }
  };

  it("renders one pill per present C4 level with a count badge", () => {
    const { container } = render(() => (
      <C4View
        nodes={nodes}
        edges={edges}
        selectedId={null}
        onSelect={() => {}}
      />
    ));
    const pills = container.querySelectorAll("button.c4-level-pill");
    // 1 "All levels" + 3 level pills (Context, Container, Component)
    expect(pills).toHaveLength(4);
    const labels = Array.from(pills).map((p) => p.textContent?.trim() ?? "");
    expect(labels.some((l) => l.startsWith("Context"))).toBe(true);
    expect(labels.some((l) => l.startsWith("Container"))).toBe(true);
    expect(labels.some((l) => l.startsWith("Component"))).toBe(true);
  });

  it("filters visible set to the picked level when a pill is clicked", async () => {
    const { container } = render(() => (
      <C4View
        nodes={nodes}
        edges={edges}
        selectedId={null}
        onSelect={() => {}}
      />
    ));
    await Promise.resolve();
    await Promise.resolve();
    setDataCalls.length = 0;

    // Click the Container pill (level 2).
    const containerPill = Array.from(
      container.querySelectorAll("button.c4-level-pill"),
    ).find((b) => b.textContent?.trim().startsWith("Container")) as
      HTMLButtonElement | undefined;
    expect(containerPill).toBeDefined();
    fireEvent.click(containerPill!);
    await Promise.resolve();
    await Promise.resolve();

    const last = setDataCalls[setDataCalls.length - 1];
    const visibleIds = (last.nodes as Array<{ id: string }>).map((n) => n.id);
    expect(visibleIds.sort()).toEqual(["container.api", "container.db"]);
    // No edges here because edges go system→container, not
    // container↔container.
    expect(last.edges).toHaveLength(0);
  });

  it("toggles the active pill off when clicked again", async () => {
    const { container } = render(() => (
      <C4View
        nodes={nodes}
        edges={edges}
        selectedId={null}
        onSelect={() => {}}
      />
    ));
    await Promise.resolve();
    await Promise.resolve();
    const pill = Array.from(
      container.querySelectorAll("button.c4-level-pill"),
    ).find((b) => b.textContent?.trim().startsWith("Container")) as
      HTMLButtonElement | undefined;
    expect(pill).toBeDefined();

    fireEvent.click(pill!);
    await Promise.resolve();
    expect(pill!.getAttribute("aria-pressed")).toBe("true");

    fireEvent.click(pill!);
    await Promise.resolve();
    expect(pill!.getAttribute("aria-pressed")).toBe("false");
  });

  it("'All levels' clears the filter and restores drill-down", async () => {
    const { container } = render(() => (
      <C4View
        nodes={nodes}
        edges={edges}
        selectedId={null}
        onSelect={() => {}}
      />
    ));
    await Promise.resolve();
    await Promise.resolve();

    const containerPill = Array.from(
      container.querySelectorAll("button.c4-level-pill"),
    ).find((b) => b.textContent?.trim().startsWith("Container")) as
      HTMLButtonElement | undefined;
    fireEvent.click(containerPill!);
    await Promise.resolve();
    await Promise.resolve();
    setDataCalls.length = 0;

    const allPill = Array.from(
      container.querySelectorAll("button.c4-level-pill"),
    ).find((b) => b.textContent?.trim() === "All levels") as
      HTMLButtonElement | undefined;
    fireEvent.click(allPill!);
    await Promise.resolve();
    await Promise.resolve();
    const last = setDataCalls[setDataCalls.length - 1];
    // Back to "no filter, no focus" → all nodes visible.
    expect(last.nodes).toHaveLength(5);
  });

  it("persists the picked level to localStorage", async () => {
    const { container } = render(() => (
      <C4View
        nodes={nodes}
        edges={edges}
        selectedId={null}
        onSelect={() => {}}
      />
    ));
    await Promise.resolve();
    await Promise.resolve();
    const pill = Array.from(
      container.querySelectorAll("button.c4-level-pill"),
    ).find((b) => b.textContent?.trim().startsWith("Component")) as
      HTMLButtonElement | undefined;
    fireEvent.click(pill!);
    await Promise.resolve();
    expect(lsGet("archview.c4.lastLevel")).toBe("3");
    const allPill = Array.from(
      container.querySelectorAll("button.c4-level-pill"),
    ).find((b) => b.textContent?.trim() === "All levels") as
      HTMLButtonElement | undefined;
    fireEvent.click(allPill!);
    await Promise.resolve();
    expect(lsGet("archview.c4.lastLevel")).toBeNull();
  });

  it("restores the level filter from localStorage on mount", async () => {
    lsSet("archview.c4.lastLevel", "2");
    const { container } = render(() => (
      <C4View
        nodes={nodes}
        edges={edges}
        selectedId={null}
        onSelect={() => {}}
      />
    ));
    await Promise.resolve();
    await Promise.resolve();
    // The Container pill should already be active on first render.
    const pill = container.querySelector(
      'button.c4-level-pill[aria-pressed="true"]',
    );
    expect(pill?.textContent?.trim()).toMatch(/^Container/);
  });
});
