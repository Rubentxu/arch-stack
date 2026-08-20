/**
 * ImpactView — marks a node as "proposed for change" and shows the
 * blast radius: every node that transitively depends on it (callers)
 * or that it transitively depends on (callees).
 *
 * Visual: focus node + every node in the impact zone are drawn
 * in a G6 dagre horizontal graph; the focus is highlighted in
 * orange (proposed change), the impact zone in yellow, and
 * unreachable nodes are not drawn at all (they were noise
 * anyway).
 *
 * M17.1.4 replaced the previous path-tree render with a G6
 * graph. The pure helpers in ImpactGraph.ts are unchanged.
 */

import {
  For,
  Show,
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  onMount,
  untrack,
  type Component,
} from "solid-js";
import type { GraphEdge, GraphNode } from "../bundle/loader";
import type { SidebarStats } from "../components/Sidebar";
import { GraphRenderer } from "../renderer/g6";
import { LR_LAYERED } from "../renderer/layout-presets";
import {
  computeImpact,
  impactCount as countImpact,
  maxImpactDepth,
  type ImpactDirection,
  type ImpactEntry,
} from "./ImpactGraph";

export type { ImpactDirection, ImpactEntry };

export interface ImpactViewProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  initialFocusId?: string;
  onSelect: (id: string | null) => void;
  onStats?: (stats: SidebarStats) => void;
}

export const ImpactView: Component<ImpactViewProps> = (props) => {
  const [focusId, setFocusId] = createSignal<string | null>(
    untrack(() => props.initialFocusId ?? props.nodes[0]?.id ?? null),
  );
  const [direction, setDirection] = createSignal<ImpactDirection>("both");

  let containerRef!: HTMLDivElement;
  let renderer: GraphRenderer | undefined;

  const focus = createMemo(() => {
    const id = focusId();
    if (!id) return null;
    return props.nodes.find((n) => n.id === id) ?? null;
  });

  const impactEntries = createMemo<ImpactEntry[]>(() => {
    const id = focusId();
    if (!id) return [];
    return computeImpact(props.nodes, props.edges, id, direction());
  });

  const impactCount = createMemo<number>(() => countImpact(impactEntries()));

  /** Nodes the user should see: focus + impact zone. */
  const visibleNodes = createMemo<GraphNode[]>(() => {
    const f = focus();
    if (!f) return [];
    const ids = new Set<string>([f.id]);
    for (const e of impactEntries()) ids.add(e.nodeId);
    return props.nodes.filter((n) => ids.has(n.id));
  });

  /** Edges that connect two visible nodes. */
  const visibleEdges = createMemo<GraphEdge[]>(() => {
    const ids = new Set(visibleNodes().map((n) => n.id));
    return props.edges.filter((e) => ids.has(e.source) && ids.has(e.target));
  });

  // Build a per-node color override that highlights focus + zone.
  const nodeStyleConfig = createMemo(() => ({
    byLevel: { 1: "#f59e0b" /* focus */, 0: "#5b8def" },
    byKind: { function: "#f59e0b" },
    defaultFill: "#facc15" /* impact zone */,
    defaultStroke: "#a16207",
    selectedStroke: "#fb923c",
  }));

  createEffect(() => {
    // SidebarStats is shared with the call-graph view. We re-use
    // `blastRadius` (size of the impact zone) and map
    // `direction` to the call-graph vocabulary so the sidebar
    // shows consistent labels.
    const dir = direction();
    const sidebarDir: "callees" | "callers" | "both" =
      dir === "upstream"
        ? "callers"
        : dir === "downstream"
          ? "callees"
          : "both";
    props.onStats?.({
      blastRadius: impactCount(),
      depth: maxImpactDepth(impactEntries()),
      direction: sidebarDir,
    });
  });

  onMount(() => {
    renderer = new GraphRenderer({
      container: containerRef,
      width: containerRef?.clientWidth || 800,
      height: containerRef?.clientHeight || 600,
      // M19: ELK layered in Web Worker, left-to-right.
      layoutOptions: LR_LAYERED,
      onNodeClick: (id) => {
        setFocusId(id);
        props.onSelect(id);
      },
      // M21: Culling is opt-in per view. Default false until perf-gate
      // validates sustained FPS (bench/perf-cull.mjs). Set to true after
      // TTFP ≤ 5s and FPS ≥ 55 are confirmed on c4-stress-1k.json.
      enableCulling: false,
    });
    renderer.setData({
      schemaVersion: "0.0.0",
      source: "impact-view",
      loadedAt: "0",
      rawKind: "call-graph",
      nodes: visibleNodes(),
      edges: visibleEdges(),
    });
  });

  createEffect(() => {
    if (!renderer) return;
    renderer.setNodeStyle(nodeStyleConfig());
    renderer.setData({
      schemaVersion: "0.0.0",
      source: "impact-view",
      loadedAt: "0",
      rawKind: "call-graph",
      nodes: visibleNodes(),
      edges: visibleEdges(),
    });
  });

  createEffect(() => {
    const id = focusId();
    if (!renderer) return;
    if (id) {
      void renderer.focusNode(id);
    } else {
      void renderer.clearFocus();
    }
  });

  onMount(() => {
    if (typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry || !renderer) return;
      const { width, height } = entry.contentRect;
      if (width > 0 && height > 0) renderer.resize(width, height);
    });
    ro.observe(containerRef);
    onCleanup(() => ro.disconnect());
  });

  onCleanup(() => {
    renderer?.destroy();
    renderer = undefined;
  });

  return (
    <div class="impact-view">
      <header class="impact-header">
        <div class="impact-focus">
          <label>Focus (proposed change)</label>
          <select
            value={focusId() ?? ""}
            onChange={(e) => {
              setFocusId(e.currentTarget.value);
              props.onSelect(e.currentTarget.value);
            }}
          >
            <For each={props.nodes}>
              {(n) => <option value={n.id}>{n.label}</option>}
            </For>
          </select>
        </div>
        <div class="impact-direction">
          <label>Direction</label>
          <select
            value={direction()}
            onChange={(e) =>
              setDirection(e.currentTarget.value as ImpactDirection)
            }
          >
            <option value="upstream">callers (upstream)</option>
            <option value="downstream">callees (downstream)</option>
            <option value="both">both</option>
          </select>
        </div>
        <p class="impact-stats">
          <strong>{impactCount()}</strong> impacted · max depth{" "}
          <strong>{maxImpactDepth(impactEntries())}</strong> · {direction()}
        </p>
      </header>

      <Show
        when={focus()}
        fallback={
          <p class="empty">Pick a focus function to see its blast radius.</p>
        }
      >
        <div ref={containerRef} class="impact-canvas" />

        <section class="impact-legend">
          <span class="legend-chip legend-focus">Focus (proposed)</span>
          <span class="legend-chip legend-impact">Impact zone</span>
        </section>

        <Show when={impactEntries().length > 0}>
          <section class="impact-paths">
            <h3>Impact paths</h3>
            <ul>
              <For each={impactEntries()}>
                {(entry) => {
                  const node = props.nodes.find((n) => n.id === entry.nodeId);
                  if (!node) return null;
                  return (
                    <li>
                      <code>{node.label}</code>{" "}
                      <span class="muted">depth {entry.depth}</span>
                      <Show when={entry.path.length > 1}>
                        <span class="muted">
                          {" "}
                          via {entry.path.slice(0, -1).join(" → ")}
                        </span>
                      </Show>
                    </li>
                  );
                }}
              </For>
            </ul>
          </section>
        </Show>
      </Show>
    </div>
  );
};
