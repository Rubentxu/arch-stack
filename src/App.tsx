/**
 * App shell — top bar with bundle loader, main canvas, sidebar.
 *
 * The M17.0 MVP is intentionally minimal: load a bundle from a URL,
 * render the graph, click a node, see evidence. No toolbar, no
 * filters, no semantic zoom. Those come in M17.1+.
 */

import { createSignal, onCleanup, onMount, type Component } from "solid-js";
import { GraphRenderer } from "./renderer/g6";
import { loadBundle, type GraphBundle, type GraphNode } from "./bundle/loader";
import { Sidebar } from "./components/Sidebar";

const SAMPLE_BUNDLES: Array<{ label: string; url: string }> = [
  {
    label: "Sample call-graph (rust)",
    url: "/samples/call-graph.json",
  },
  {
    label: "Sample class-diagram (rust)",
    url: "/samples/class-diagram.json",
  },
];

export const App: Component = () => {
  let canvasRef: HTMLDivElement | undefined;
  let renderer: GraphRenderer | undefined;
  const [bundle, setBundle] = createSignal<GraphBundle | null>(null);
  const [selected, setSelected] = createSignal<GraphNode | null>(null);
  const [error, setError] = createSignal<string | null>(null);

  onMount(() => {
    if (!canvasRef) return;
    renderer = new GraphRenderer({
      container: canvasRef,
      width: canvasRef.clientWidth || 800,
      height: canvasRef.clientHeight || 600,
    });
  });

  onCleanup(() => {
    renderer?.destroy();
  });

  const handleLoad = async (url: string) => {
    setError(null);
    try {
      const b = await loadBundle(url);
      setBundle(b);
      renderer?.setData(b);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div class="app">
      <header class="topbar">
        <h1>archview</h1>
        <nav class="bundle-loader">
          {SAMPLE_BUNDLES.map((s) => (
            <button onClick={() => void handleLoad(s.url)}>{s.label}</button>
          ))}
          <input
            type="text"
            placeholder="bundle URL (file://… or http://…)"
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                const url = e.currentTarget.value.trim();
                if (url) void handleLoad(url);
              }
            }}
          />
        </nav>
      </header>

      <main class="main">
        <div class="canvas" ref={canvasRef}>
          {!bundle() && (
            <p class="empty-canvas">
              Load a bundle from the top bar to start exploring.
            </p>
          )}
        </div>
        <Sidebar
          node={selected()}
          bundleMeta={
            bundle()
              ? {
                  source: bundle()!.source,
                  schemaVersion: bundle()!.schemaVersion,
                  loadedAt: bundle()!.loadedAt,
                  rawKind: bundle()!.rawKind,
                }
              : null
          }
        />
      </main>

      {error() && <p class="error">{error()}</p>}
    </div>
  );
};

export default App;
