/**
 * App shell — top bar with bundle loader, main canvas, sidebar.
 *
 * Renders different views depending on the bundle shape:
 * - sequence → SequenceView (lifelines + arrows, M17.3)
 * - call-graph → ImpactView by default (blast radius, M17.7), with a
 *   selector to switch to CallGraphView (focus + BFS, M17.2) or
 *   PackageView (package diagram, M17.5)
 * - class-diagram → ClassDiagramView (UML compartments, M17.4)
 * - C4 → C4View (hierarchical with drill-down, M17.1)
 * - drift mode (M17.6): two C4 bundles side-by-side diff
 *
 * Routing decisions are delegated to `resolveView` (pure mapping,
 * R3) so exactly one specialized view renders per bundle kind.
 */

import {
  For,
  Match,
  Show,
  Switch,
  createSignal,
  type Component,
} from "solid-js";
import { GraphRenderer } from "./renderer/g6";
import { loadBundle, type GraphBundle, type GraphNode } from "./bundle/loader";
import { Sidebar, type SidebarStats } from "./components/Sidebar";
import { resolveView, type CallGraphMode } from "./routing";
import { C4View } from "./views/C4View";
import { CallGraphView } from "./views/CallGraphView";
import { SequenceView } from "./views/SequenceView";
import { ClassDiagramView } from "./views/ClassDiagramView";
import { DriftView } from "./views/DriftView";
import { ImpactView } from "./views/ImpactView";
import { PackageView } from "./views/PackageView";
import { buildPackageNode } from "./views/PackageGraph";
import { useWorkspaceState } from "./lib/workspace";

const SAMPLE_BUNDLES: Array<{ label: string; url: string }> = [
  {
    label: "Sample call-graph (rust)",
    url: "/samples/call-graph.json",
  },
  {
    label: "Sample call-graph (deep, 5 nodes)",
    url: "/samples/call-graph-deep.json",
  },
  {
    label: "Sample sequence diagram",
    url: "/samples/sequence.json",
  },
  {
    label: "Sample class-diagram (rust)",
    url: "/samples/class-diagram.json",
  },
  {
    label: "Sample class-diagram (rich, with traits + composition)",
    url: "/samples/class-diagram-rich.json",
  },
  {
    label: "Sample C4 context",
    url: "/samples/c4-context.json",
  },
  {
    label: "Sample C4 container (archctl)",
    url: "/samples/c4-container.json",
  },
];

export const App: Component = () => {
  const [bundle, setBundle] = createSignal<GraphBundle | null>(null);
  const [selected, setSelected] = createSignal<GraphNode | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [stats, setStats] = createSignal<SidebarStats | undefined>(undefined);
  const [driftMode, setDriftMode] = createSignal(false);
  const [declaredBundle, setDeclaredBundle] = createSignal<GraphBundle | null>(
    null,
  );
  const [actualBundle, setActualBundle] = createSignal<GraphBundle | null>(
    null,
  );
  /** Selector state for call-graph bundles: Impact (default) | Call graph | Package. */
  const [callGraphMode, setCallGraphMode] =
    createSignal<CallGraphMode>("impact");

  // H1 (ADR-041): durable workspace state — restore viewport on mount,
  // debounce PUT on every change, fetch source preview + open editor on
  // demand. Wired into the Sidebar (SourceDrawer) below.
  const ws = useWorkspaceState();

  const handleLoad = async (url: string) => {
    setError(null);
    try {
      const b = await loadBundle(url);
      setBundle(b);
      setSelected(null);
      setStats(undefined);
      setCallGraphMode("impact");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleDriftLoad = async (declaredUrl: string, actualUrl: string) => {
    setError(null);
    try {
      const [dec, act] = await Promise.all([
        loadBundle(declaredUrl),
        loadBundle(actualUrl),
      ]);
      setDeclaredBundle(dec);
      setActualBundle(act);
      setSelected(null);
      setStats(undefined);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div class="app">
      <header class="topbar">
        <h1>archview</h1>
        <nav class="bundle-loader">
          <For each={SAMPLE_BUNDLES}>
            {(s) => (
              <button onClick={() => void handleLoad(s.url)}>{s.label}</button>
            )}
          </For>
          <input
            type="text"
            placeholder={
              driftMode()
                ? "unused in drift mode"
                : "bundle URL (file://… or http://…)"
            }
            disabled={driftMode()}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !driftMode()) {
                const url = e.currentTarget.value.trim();
                if (url) void handleLoad(url);
              }
            }}
          />
          <button
            class={`drift-toggle ${driftMode() ? "active" : ""}`}
            onClick={() => setDriftMode(!driftMode())}
            title="Toggle drift detection mode (compare declared vs actual)"
          >
            {driftMode() ? "✓ drift mode" : "drift mode"}
          </button>
        </nav>

        <Show when={driftMode()}>
          <nav class="drift-loader">
            <input
              type="text"
              placeholder="declared C4 bundle URL"
              id="declared-url"
            />
            <input
              type="text"
              placeholder="actual C4 bundle URL"
              id="actual-url"
            />
            <button
              onClick={() => {
                const dec =
                  (
                    document.getElementById("declared-url") as HTMLInputElement
                  )?.value.trim() || "";
                const act =
                  (
                    document.getElementById("actual-url") as HTMLInputElement
                  )?.value.trim() || "";
                if (dec && act) void handleDriftLoad(dec, act);
              }}
            >
              compare
            </button>
          </nav>
        </Show>
      </header>

      <main class="main">
        <Show
          when={driftMode() ? null : bundle()}
          fallback={
            driftMode() ? null : (
              <p class="empty-canvas">
                Load a bundle from the top bar to start exploring.
              </p>
            )
          }
        >
          {(b) => {
            const kind = () => resolveView(b().rawKind, callGraphMode());
            return (
              <>
                <Show when={b().rawKind === "call-graph"}>
                  <nav class="view-selector" aria-label="View selector">
                    <button
                      class={callGraphMode() === "impact" ? "active" : ""}
                      onClick={() => setCallGraphMode("impact")}
                    >
                      Impact
                    </button>
                    <button
                      class={callGraphMode() === "call-graph" ? "active" : ""}
                      onClick={() => setCallGraphMode("call-graph")}
                    >
                      Call graph
                    </button>
                    <button
                      class={callGraphMode() === "package" ? "active" : ""}
                      onClick={() => setCallGraphMode("package")}
                    >
                      Package
                    </button>
                  </nav>
                </Show>

                <Switch>
                  <Match when={kind() === "c4"}>
                    <C4View
                      nodes={b().nodes}
                      edges={b().edges}
                      selectedId={selected()?.id ?? null}
                      onSelect={(id) => {
                        const node = id
                          ? (b().nodes.find((n) => n.id === id) ?? null)
                          : null;
                        setSelected(node);
                      }}
                    />
                  </Match>
                  <Match when={kind() === "sequence"}>
                    <SequenceView
                      nodes={b().nodes}
                      interactions={b().interactions ?? []}
                      onSelect={(id) => {
                        const node = id
                          ? (b().nodes.find((n) => n.id === id) ?? null)
                          : null;
                        setSelected(node);
                      }}
                    />
                  </Match>
                  <Match when={kind() === "call-graph"}>
                    <CallGraphView
                      nodes={b().nodes}
                      edges={b().edges}
                      onSelect={(id) => {
                        const node = id
                          ? (b().nodes.find((n) => n.id === id) ?? null)
                          : null;
                        setSelected(node);
                      }}
                      onStats={setStats}
                    />
                  </Match>
                  <Match when={kind() === "package"}>
                    <PackageView
                      nodes={b().nodes}
                      edges={b().edges}
                      onSelect={(id) => {
                        // Call-graph bundles have no package-kind nodes, so a
                        // package selection is synthesized (spec Option D).
                        setSelected(
                          id ? buildPackageNode(id, b().nodes) : null,
                        );
                      }}
                    />
                  </Match>
                  <Match when={kind() === "impact"}>
                    <ImpactView
                      nodes={b().nodes}
                      edges={b().edges}
                      onSelect={(id) => {
                        const node = id
                          ? (b().nodes.find((n) => n.id === id) ?? null)
                          : null;
                        setSelected(node);
                      }}
                      onStats={setStats}
                    />
                  </Match>
                  <Match when={kind() === "class-diagram"}>
                    <ClassDiagramView
                      nodes={b().nodes}
                      edges={b().edges}
                      onSelect={(id) => {
                        const node = id
                          ? (b().nodes.find((n) => n.id === id) ?? null)
                          : null;
                        setSelected(node);
                      }}
                    />
                  </Match>
                  <Match when={true}>
                    <GraphView
                      bundle={b()}
                      selectedId={selected()?.id ?? null}
                      onSelect={(node) => setSelected(node)}
                    />
                  </Match>
                </Switch>
              </>
            );
          }}
        </Show>

        <Show when={driftMode()}>
          <DriftView
            declared={declaredBundle()}
            actual={actualBundle()}
            onSelect={(id) => {
              const all = [
                ...(declaredBundle()?.nodes ?? []),
                ...(actualBundle()?.nodes ?? []),
              ];
              const node = id ? (all.find((n) => n.id === id) ?? null) : null;
              setSelected(node);
            }}
          />
        </Show>

        <Sidebar
          node={selected()}
          bundleMeta={
            !driftMode() && bundle()
              ? {
                  source: bundle()!.source,
                  schemaVersion: bundle()!.schemaVersion,
                  loadedAt: bundle()!.loadedAt,
                  rawKind: bundle()!.rawKind,
                  strict: bundle()!.strict,
                }
              : driftMode()
                ? declaredBundle() && actualBundle()
                  ? {
                      source: "drift comparison",
                      schemaVersion: `${declaredBundle()!.schemaVersion} vs ${actualBundle()!.schemaVersion}`,
                      loadedAt: actualBundle()!.loadedAt,
                      rawKind: "c4 (drift)",
                    }
                  : null
                : null
          }
          stats={stats()}
          onFetchSource={bundle()?.strict === true ? undefined : ws.fetchSource}
          onOpenInEditor={
            bundle()?.strict === true ? undefined : ws.openInEditor
          }
        />
      </main>

      {error() && <p class="error">{error()}</p>}
    </div>
  );
};

/**
 * GraphView — G6-based canvas for non-C4 bundles (call-graph, sequence,
 * class-diagram). The original M17.0 view, kept for those bundle shapes.
 */
const GraphView: Component<{
  bundle: GraphBundle;
  selectedId: string | null;
  onSelect: (node: GraphNode | null) => void;
}> = (props) => {
  let renderer: GraphRenderer | undefined;

  // SolidJS doesn't have onMount lifecycle for DOM ref initialization
  // in the same way React does; we use a createEffect to wire up.
  const mount = (el: HTMLDivElement) => {
    if (renderer) {
      renderer.destroy();
    }
    renderer = new GraphRenderer({
      container: el,
      width: el.clientWidth || 800,
      height: el.clientHeight || 600,
    });
    renderer.setData(props.bundle);
  };

  // Note: keeping it simple — no resize observer in M17.1.

  return <div class="canvas" ref={mount} />;
};

export default App;
