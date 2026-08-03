// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@solidjs/testing-library";
import { App } from "../App";
import type { GraphBundle } from "../bundle/loader";

/**
 * R5 — Navigation regression coverage.
 *
 * These tests drive the real App shell: load a bundle through the
 * top-bar (sample buttons or the URL input), activate the Call graph /
 * Package triggers, and assert the rendered view plus representative
 * data. They also guard R1's "sequence does not collide" rule.
 *
 * `loadBundle` is mocked so tests are deterministic and offline;
 * `renderer/g6` is mocked because the generic GraphView fallback
 * instantiates the G6 canvas, which is not exercised here.
 */

const { loadBundleMock } = vi.hoisted(() => {
  const callGraphBundle: GraphBundle = {
    schemaVersion: "0.14.3",
    source: "test://call-graph",
    loadedAt: "2026-01-01T00:00:00Z",
    rawKind: "call-graph",
    nodes: [
      {
        id: "main",
        label: "main",
        kind: "function",
        file: "src/main.rs",
        line: 1,
      },
      {
        id: "auth_login",
        label: "auth_login",
        kind: "function",
        file: "src/auth.rs",
        line: 5,
      },
      {
        id: "db_query",
        label: "db_query",
        kind: "function",
        file: "crates/db/src/lib.rs",
        line: 10,
      },
    ],
    edges: [
      { id: "e1", source: "main", target: "auth_login", kind: "SyncCall" },
      { id: "e2", source: "auth_login", target: "db_query", kind: "SyncCall" },
    ],
  };

  const deepCallGraphBundle: GraphBundle = {
    schemaVersion: "0.14.3",
    source: "test://call-graph-deep",
    loadedAt: "2026-01-01T00:00:00Z",
    rawKind: "call-graph",
    nodes: [
      {
        id: "a",
        label: "alpha",
        kind: "function",
        file: "src/alpha.rs",
        line: 1,
      },
      {
        id: "b",
        label: "beta",
        kind: "function",
        file: "src/beta.rs",
        line: 1,
      },
      {
        id: "c",
        label: "gamma",
        kind: "function",
        file: "src/gamma.rs",
        line: 1,
      },
      {
        id: "d",
        label: "delta",
        kind: "function",
        file: "crates/io/src/lib.rs",
        line: 1,
      },
      {
        id: "e",
        label: "epsilon",
        kind: "function",
        file: "crates/io/src/lib.rs",
        line: 2,
      },
    ],
    edges: [
      { id: "e1", source: "a", target: "b", kind: "SyncCall" },
      { id: "e2", source: "a", target: "c", kind: "SyncCall" },
      { id: "e3", source: "b", target: "d", kind: "SyncCall" },
      { id: "e4", source: "c", target: "e", kind: "SyncCall" },
    ],
  };

  const emptyCallGraphBundle: GraphBundle = {
    ...callGraphBundle,
    source: "test://empty",
    nodes: [],
    edges: [],
  };

  const sequenceBundle: GraphBundle = {
    schemaVersion: "0.14.3",
    source: "test://sequence",
    loadedAt: "2026-01-01T00:00:00Z",
    rawKind: "sequence",
    nodes: [
      { id: "lib.rs:a", label: "a", kind: "function", file: "lib.rs", line: 1 },
      { id: "lib.rs:b", label: "b", kind: "function", file: "lib.rs", line: 2 },
    ],
    edges: [
      { id: "e1", source: "lib.rs:a", target: "lib.rs:b", kind: "SyncCall" },
    ],
    interactions: [
      {
        order: 1,
        label: "a → b",
        message_kind: "SyncCall",
        caller: { name: "a", file: "lib.rs", line: 1 },
        callee: { name: "b", file: "lib.rs", line: 2 },
      },
    ],
  };

  const loadBundleMock = vi.fn(async (url: string): Promise<GraphBundle> => {
    if (url.includes("empty")) return emptyCallGraphBundle;
    if (url.includes("deep")) return deepCallGraphBundle;
    if (url.includes("sequence")) return sequenceBundle;
    return callGraphBundle;
  });

  return {
    loadBundleMock,
    callGraphBundle,
    deepCallGraphBundle,
    emptyCallGraphBundle,
    sequenceBundle,
  };
});

vi.mock("../renderer/g6", () => ({
  GraphRenderer: class {
    constructor() {
      /* noop — canvas not exercised in these tests */
    }
    setData() {
      /* noop */
    }
    destroy() {
      /* noop */
    }
  },
}));

vi.mock("../bundle/loader", () => ({
  loadBundle: loadBundleMock,
}));

afterEach(() => {
  cleanup();
  loadBundleMock.mockClear();
});

function loadSample(label: string) {
  fireEvent.click(screen.getByRole("button", { name: label }));
}

async function loadUrl(url: string) {
  const input = screen.getByPlaceholderText(/bundle URL/) as HTMLInputElement;
  input.value = url;
  fireEvent.keyDown(input, { key: "Enter" });
}

describe("App navigation — call-graph bundles (R1/R3/R5)", () => {
  it("keeps Impact as the default call-graph outcome", async () => {
    render(() => <App />);
    loadSample("Sample call-graph (rust)");

    await waitFor(() =>
      expect(document.querySelector(".impact-view")).not.toBeNull(),
    );
    // Sole specialized view: Call graph and Package are absent by default.
    expect(document.querySelector(".callgraph-view")).toBeNull();
    expect(document.querySelector(".package-view")).toBeNull();
  });

  it("navigates to Call graph and shows bundle function data", async () => {
    render(() => <App />);
    loadSample("Sample call-graph (rust)");
    fireEvent.click(await screen.findByRole("button", { name: "Call graph" }));

    await waitFor(() =>
      expect(document.querySelector(".callgraph-view")).not.toBeNull(),
    );
    // Focus detail shows the first function of the bundle.
    expect(
      document.querySelector(".callgraph-focus-detail")?.textContent,
    ).toContain("main");
    // Level 1 downstream from main reaches auth_login (blast radius 1).
    expect(document.querySelector(".callgraph-levels")?.textContent).toContain(
      "auth_login",
    );
    expect(document.querySelector(".callgraph-stats")?.textContent).toContain(
      "1",
    );
    // Competing specialized views are absent.
    expect(document.querySelector(".impact-view")).toBeNull();
    expect(document.querySelector(".package-view")).toBeNull();
  });

  it("triangulates: deep bundle renders extra reachable levels in Call graph", async () => {
    render(() => <App />);
    loadSample("Sample call-graph (deep, 5 nodes)");
    fireEvent.click(await screen.findByRole("button", { name: "Call graph" }));

    await waitFor(() =>
      expect(
        document.querySelector(".callgraph-levels")?.textContent,
      ).toContain("beta"),
    );
    expect(document.querySelector(".callgraph-levels")?.textContent).toContain(
      "gamma",
    );
    expect(document.querySelector(".callgraph-stats")?.textContent).toContain(
      "2",
    );
    // delta/epsilon are depth-2 only — not yet visible.
    expect(
      document.querySelector(".callgraph-levels")?.textContent,
    ).not.toContain("delta");

    fireEvent.click(screen.getByRole("button", { name: "Increase depth" }));
    await waitFor(() =>
      expect(
        document.querySelector(".callgraph-levels")?.textContent,
      ).toContain("delta"),
    );
    expect(document.querySelector(".callgraph-levels")?.textContent).toContain(
      "epsilon",
    );
  });
});

describe("App navigation — Package (R2/R5)", () => {
  it("navigates to Package and shows derived packages + inter-package deps", async () => {
    render(() => <App />);
    loadSample("Sample call-graph (rust)");
    fireEvent.click(await screen.findByRole("button", { name: "Package" }));

    await waitFor(() =>
      expect(document.querySelector(".package-view")).not.toBeNull(),
    );
    // Header summarizes derived packages and edges.
    expect(document.querySelector(".pkg-header")?.textContent).toContain(
      "2 packages · 1 inter-package edges",
    );
    // Package cards are derived from node files: src/ and crates/db.
    const cardTexts = screen
      .getAllByRole("article")
      .map((card) => card.textContent ?? "");
    expect(cardTexts.some((t) => t.includes("src"))).toBe(true);
    expect(cardTexts.some((t) => t.includes("crates/db"))).toBe(true);
    // Inter-package dependency src → crates/db with weight 1.
    expect(document.querySelector(".pkg-relations")?.textContent).toContain(
      "src",
    );
    expect(document.querySelector(".pkg-relations")?.textContent).toContain(
      "crates/db",
    );
    expect(screen.getByText("─→(1)→")).toBeTruthy();
  });

  it("keeps Package reachable and shows its empty state for a bundle with no functions", async () => {
    render(() => <App />);
    await loadUrl("test://empty");
    fireEvent.click(await screen.findByRole("button", { name: "Package" }));

    await waitFor(() =>
      expect(
        screen.getByText("No functions to derive packages from."),
      ).toBeTruthy(),
    );
  });
});

describe("App navigation — sequence must not collide (R1)", () => {
  it("renders Sequence and never Call graph for a sequence bundle", async () => {
    render(() => <App />);
    loadSample("Sample sequence diagram");

    await waitFor(() =>
      expect(document.querySelector(".sequence-view")).not.toBeNull(),
    );
    expect(document.querySelector(".sequence-header")?.textContent).toContain(
      "Sequence diagram",
    );
    expect(document.querySelector(".callgraph-view")).toBeNull();
    expect(document.querySelector(".impact-view")).toBeNull();
  });
});
