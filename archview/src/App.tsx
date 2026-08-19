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
import { EmptyState } from "./components/primitives";
import { resolveView, type CallGraphMode } from "./routing";
import { C4View } from "./views/C4View";
import { CallGraphView } from "./views/CallGraphView";
import { SequenceView } from "./views/SequenceView";
import { ClassDiagramView } from "./views/ClassDiagramView";
import { DriftView } from "./views/DriftView";
import { ImpactView } from "./views/ImpactView";
import { PackageView } from "./views/PackageView";
import { buildPackageNode } from "./views/PackageGraph";
import { useWorkspaceState, explainElement } from "./lib/workspace";
import { NavStack, zoomTargetFor, type NavEntry } from "./lib/navigation";

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
  {
    label: "Sample C4 semantic zoom (3 levels)",
    url: "/samples/c4-semantic-zoom.json",
  },
  {
    label: "Sample C4 stress (318 nodes, hub:core with 100 incoming edges)",
    url: "/samples/c4-stress-200.json",
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

  // ADR-062: cross-view navigation stack (breadcrumbs + back/forward).
  const [stack, setStack] = createSignal<NavStack>(new NavStack());

  const loadUrl = async (url: string, focusId?: string) => {
    setError(null);
    try {
      const b = await loadBundle(url);
      setBundle(b);
      setSelected(null);
      setStats(undefined);
      setCallGraphMode("impact");
      if (focusId) {
        // Identity survives view changes: keep the canonical element
        // selected when it is present in the destination bundle.
        setSelected(b.nodes.find((n) => n.id === focusId) ?? null);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleLoad = async (url: string, label?: string) => {
    setStack((s) => s.push({ url, label: label ?? url }));
    await loadUrl(url);
  };

  const navigateEntry = (entry: NavEntry) => {
    setStack((s) => s.push(entry));
    void loadUrl(entry.url, entry.elementId);
  };

  const goHistory = (dir: "back" | "forward") => {
    const next = dir === "back" ? stack().back() : stack().forward();
    if (next.index === stack().index) return;
    setStack(next);
    const entry = next.current();
    if (entry) void loadUrl(entry.url, entry.elementId);
  };

  const jumpTo = (i: number) => {
    const next = stack().jumpTo(i);
    if (next.index === stack().index) return;
    setStack(next);
    const entry = next.current();
    if (entry) void loadUrl(entry.url, entry.elementId);
  };

  const handleZoom = (dir: "in" | "out") => {
    const node = selected();
    if (!node) return;
    const t = zoomTargetFor(node, dir);
    if (t) {
      navigateEntry({
        url: t.url,
        label: t.label,
        elementId: t.elementId,
        level: t.level,
      });
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
          {/* M17.C2 / F1: 7 sample buttons used to take ~700px and
              pushed the nav history off-screen. A single <select>
              reclaims the width. Loading fires on change so there
              is no extra click. */}
          <select
            class="sample-select"
            disabled={driftMode()}
            onChange={(e) => {
              const url = e.currentTarget.value;
              if (url) void handleLoad(url);
              // Reset to placeholder so the same sample can be re-loaded
              // (otherwise selecting the current value is a no-op).
              e.currentTarget.value = "";
            }}
            aria-label="Open a sample bundle"
          >
            <option value="">Open sample…</option>
            <For each={SAMPLE_BUNDLES}>
              {(s) => <option value={s.url}>{s.label}</option>}
            </For>
          </select>
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

        <nav class="nav-history" aria-label="Navigation history">
          <button
            type="button"
            aria-label="Back"
            disabled={stack().index <= 0}
            onClick={() => goHistory("back")}
          >
            ←
          </button>
          <button
            type="button"
            aria-label="Forward"
            disabled={
              stack().length === 0 || stack().index >= stack().length - 1
            }
            onClick={() => goHistory("forward")}
          >
            →
          </button>
          <ol class="breadcrumbs">
            <For each={stack().all()}>
              {(entry, i) => (
                <li>
                  <button
                    type="button"
                    class={i() === stack().index ? "active" : ""}
                    title={entry.url}
                    onClick={() => jumpTo(i())}
                  >
                    {entry.label}
                  </button>
                </li>
              )}
            </For>
          </ol>
        </nav>
      </header>

      <main class="main">
        <Show
          when={driftMode() ? null : bundle()}
          fallback={
            driftMode() ? null : (
              <EmptyState
                icon={<EmptyStateIcon />}
                title="No bundle loaded"
                body="Pick a sample above to explore a call-graph, class diagram, C4 view, or sequence diagram. Or paste a bundle URL exported by archctl."
              />
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
          onZoom={handleZoom}
          onExplain={bundle()?.strict === true ? undefined : explainElement}
          edges={bundle()?.edges ?? []}
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

/** Inline icon for the empty state. A small graph glyph that
 *  hints at what the workbench is for without depending on an
 *  external icon font. */
function EmptyStateIcon() {
  return (
    <svg
      width="32"
      height="32"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <circle cx="6" cy="6" r="2.2" />
      <circle cx="18" cy="6" r="2.2" />
      <circle cx="12" cy="18" r="2.2" />
      <line x1="7.5" y1="7" x2="11" y2="16" />
      <line x1="16.5" y1="7" x2="13" y2="16" />
      <line x1="8" y1="6" x2="16" y2="6" />
    </svg>
  );
}

export default App;
