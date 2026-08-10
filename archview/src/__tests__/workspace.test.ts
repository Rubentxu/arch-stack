// @vitest-environment jsdom
/**
 * workspace hook tests (H1, ADR-041).
 *
 * Tests use `createRoot` directly (no JSX render) to avoid esbuild JSX
 * transformation issues in test mode. The hook returns plain values
 * (signals, functions) that we assert against after flushing
 * microtasks/macrotasks.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { createRoot, createEffect } from "solid-js";
import { __setFetchForTests, useWorkspaceState } from "../lib/workspace";

interface FetchCall {
  url: string;
  init?: RequestInit;
}

function makeFetchMock(
  handler: (call: FetchCall) => Response | Promise<Response>,
) {
  return vi.fn(async (input: string, init?: RequestInit) =>
    handler({ url: input, init }),
  );
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

beforeEach(() => {
  __setFetchForTests(
    makeFetchMock(() => jsonResponse({ workspace: null, version: "1.0" })),
  );
});

afterEach(() => {
  __setFetchForTests(null);
});

/** Flush all pending microtasks + macrotasks (resources + debounce). */
// (Intentionally no helper — each test uses its own await + setTimeout.)

describe("useWorkspaceState", () => {
  it("restores workspace from GET /api/workspace on mount", async () => {
    const stored = {
      workspace: {
        camera: { x: 12.5, y: -7.25 },
        zoom: 80,
        filters: [{ kind: "c4" as const, predicate: "Container" }],
        selection: null,
      },
      version: "1.0",
    };
    const fetchMock = makeFetchMock(() => jsonResponse(stored));
    __setFetchForTests(fetchMock);

    // Sanity: the stub is wired before the hook runs.
    expect(fetchMock).toBeDefined();

    let captured: { zoom: number; camX: number; filters: number } | null = null;
    await new Promise<void>((resolve) => {
      createRoot((dispose) => {
        const ws = useWorkspaceState();
        createEffect(() => {
          captured = {
            zoom: ws.workspace().zoom,
            camX: ws.workspace().camera.x,
            filters: ws.workspace().filters.length,
          };
        });
        setTimeout(() => {
          dispose();
          resolve();
        }, 100);
      });
    });
    expect(fetchMock).toHaveBeenCalled();
    expect(captured).not.toBeNull();
    expect(captured!.zoom).toBe(80);
    expect(captured!.camX).toBeCloseTo(12.5);
    expect(captured!.filters).toBe(1);
  });

  it("keeps defaults when backend returns workspace: null", async () => {
    let captured: number | null = null;
    await new Promise<void>((resolve) => {
      createRoot((dispose) => {
        const ws = useWorkspaceState();
        createEffect(() => {
          captured = ws.workspace().zoom;
        });
        setTimeout(() => {
          dispose();
          resolve();
        }, 100);
      });
    });
    expect(captured).toBe(50);
  });

  it("setWorkspace fires PUT after debounce", async () => {
    vi.useFakeTimers();
    let putCall: FetchCall | null = null;
    __setFetchForTests(
      makeFetchMock((call) => {
        if (call.init?.method === "PUT") {
          putCall = call;
          return new Response(null, { status: 204 });
        }
        return jsonResponse({ workspace: null, version: "1.0" });
      }),
    );
    await new Promise<void>((resolve) => {
      createRoot((dispose) => {
        const ws = useWorkspaceState();
        ws.setWorkspace({
          camera: { x: 100, y: 200 },
          zoom: 33,
          filters: [],
          selection: null,
        });
        expect(putCall).toBeNull();
        vi.advanceTimersByTime(500);
        expect(putCall).not.toBeNull();
        dispose();
        resolve();
      });
    });
    vi.useRealTimers();
    expect(putCall!.url).toBe("/api/workspace");
    expect(putCall!.init?.method).toBe("PUT");
    const body = JSON.parse(putCall!.init?.body as string);
    expect(body.workspace.zoom).toBe(33);
    expect(body.workspace.camera.x).toBe(100);
    expect(body.version).toBe("1.0");
  });

  it("debounce coalesces rapid changes", async () => {
    vi.useFakeTimers();
    const putCalls: FetchCall[] = [];
    __setFetchForTests(
      makeFetchMock((call) => {
        if (call.init?.method === "PUT") {
          putCalls.push(call);
          return new Response(null, { status: 204 });
        }
        return jsonResponse({ workspace: null, version: "1.0" });
      }),
    );
    await new Promise<void>((resolve) => {
      createRoot((dispose) => {
        const ws = useWorkspaceState();
        ws.setWorkspace({
          camera: { x: 1, y: 1 },
          zoom: 10,
          filters: [],
          selection: null,
        });
        vi.advanceTimersByTime(200);
        ws.setWorkspace({
          camera: { x: 2, y: 2 },
          zoom: 20,
          filters: [],
          selection: null,
        });
        vi.advanceTimersByTime(200);
        ws.setWorkspace({
          camera: { x: 3, y: 3 },
          zoom: 30,
          filters: [],
          selection: null,
        });
        vi.advanceTimersByTime(500);
        dispose();
        resolve();
      });
    });
    vi.useRealTimers();
    expect(putCalls).toHaveLength(1);
    const body = JSON.parse(putCalls[0].init?.body as string);
    expect(body.workspace.zoom).toBe(30);
    expect(body.workspace.camera.x).toBe(3);
  });

  it("fetchSource calls GET /api/source with file + line", async () => {
    let capturedUrl = "";
    __setFetchForTests(
      makeFetchMock((call) => {
        if (call.url.startsWith("/api/source?")) {
          capturedUrl = call.url;
          return jsonResponse({
            file: "src/main.rs",
            start_line: 8,
            total_lines: 20,
            content: ["line8", "line9", "line10", "line11", "line12"],
            truncated: false,
          });
        }
        return jsonResponse({ workspace: null, version: "1.0" });
      }),
    );
    let preview: Awaited<
      ReturnType<ReturnType<typeof useWorkspaceState>["fetchSource"]>
    > | null = null;
    await new Promise<void>((resolve) => {
      createRoot(async (dispose) => {
        const ws = useWorkspaceState();
        preview = await ws.fetchSource("src/main.rs", 10);
        dispose();
        resolve();
      });
    });
    // URLSearchParams URL-encodes the slashes — compare against the
    // decoded form to assert intent without coupling to encoding.
    const decoded = decodeURIComponent(capturedUrl);
    expect(decoded).toContain("file=src/main.rs");
    expect(decoded).toContain("line=10");
    expect(preview).not.toBeNull();
    expect(preview!.file).toBe("src/main.rs");
    expect(preview!.start_line).toBe(8);
    expect(preview!.total_lines).toBe(20);
    expect(preview!.content).toHaveLength(5);
    expect(preview!.truncated).toBe(false);
  });

  it("openInEditor POSTs /api/open-editor and returns true on 204", async () => {
    let capturedPost: FetchCall | null = null;
    __setFetchForTests(
      makeFetchMock((call) => {
        if (call.init?.method === "POST") {
          capturedPost = call;
          return new Response(null, { status: 204 });
        }
        return jsonResponse({ workspace: null, version: "1.0" });
      }),
    );
    let ok = false;
    await new Promise<void>((resolve) => {
      createRoot(async (dispose) => {
        const ws = useWorkspaceState();
        ok = await ws.openInEditor("src/main.rs", 42);
        dispose();
        resolve();
      });
    });
    expect(capturedPost).not.toBeNull();
    expect(capturedPost!.url).toBe("/api/open-editor");
    const body = JSON.parse(capturedPost!.init!.body as string);
    expect(body.file).toBe("src/main.rs");
    expect(body.line).toBe(42);
    expect(ok).toBe(true);
  });

  it("openInEditor returns false on 503 (no editor configured)", async () => {
    __setFetchForTests(
      makeFetchMock((call) => {
        if (call.init?.method === "POST") {
          return jsonResponse({ error: "no_editor_configured" }, 503);
        }
        return jsonResponse({ workspace: null, version: "1.0" });
      }),
    );
    let ok = true;
    await new Promise<void>((resolve) => {
      createRoot(async (dispose) => {
        const ws = useWorkspaceState();
        ok = await ws.openInEditor("src/main.rs", 1);
        dispose();
        resolve();
      });
    });
    expect(ok).toBe(false);
  });

  it("restoration swallows 404 (no workspace yet) silently", async () => {
    __setFetchForTests(
      makeFetchMock(() => new Response("not found", { status: 404 })),
    );
    let captured: number | null = null;
    await new Promise<void>((resolve) => {
      createRoot((dispose) => {
        const ws = useWorkspaceState();
        createEffect(() => {
          captured = ws.workspace().zoom;
        });
        setTimeout(() => {
          dispose();
          resolve();
        }, 100);
      });
    });
    expect(captured).toBe(50); // defaults
  });
});
