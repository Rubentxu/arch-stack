/**
 * CallGraphView — focuses on a single function and shows N levels
 * of callers/callees reachable from it, rendered as a G6 graph
 * (M17.1).
 *
 * - Focus node: the user-selected function (or first node by default)
 * - Direction: callers (incoming), callees (outgoing), or both
 * - Depth: BFS expansion in steps (1-N). Buttons add/remove levels.
 * - Async flow: edges tagged with `message_kind` (SyncCall/AsyncCall)
 *   are styled with a dashed stroke.
 * - Blast radius: count of unique functions reachable from the focus
 *   at the current depth (emitted via onStats for the sidebar).
 *
 * M17.1 — replaced the previous "list of levels" text view with a
 * dagre horizontal graph. The focus node is placed on the left,
 * callees expand to the right, callers expand to the left. The
 * BFS expansion itself is unchanged (CallGraphGraph.ts).
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
import {
  blastRadiusOf,
  expandLevels,
  MAX_DEPTH,
  type CallDirection,
  type LevelGroup,
} from "./CallGraphGraph";

export type { CallDirection };

export interface CallGraphViewProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  initialFocusId?: string;
  initialDepth?: number;
  onSelect: (id: string | null) => void;
  onStats?: (stats: SidebarStats) => void;
}

export const CallGraphView: Component<CallGraphViewProps> = (props) => {
  const [focusId, setFocusId] = createSignal<string | null>(
    untrack(() => props.initialFocusId ?? props.nodes[0]?.id ?? null),
  );
  const [depth, setDepth] = createSignal<number>(
    untrack(() => props.initialDepth ?? 1),
  );
  const [direction, setDirection] = createSignal<CallDirection>("callees");

  let containerRef!: HTMLDivElement;
  let renderer: GraphRenderer | undefined;

  const focus = createMemo(() => {
    const id = focusId();
    if (!id) return null;
    return props.nodes.find((n) => n.id === id) ?? null;
  });

  /** BFS from focus with direction + depth controls. */
  const levelGroups = createMemo<LevelGroup[]>(() => {
    const id = focusId();
    if (!id) return [];
    return expandLevels(props.nodes, props.edges, id, depth(), direction());
  });

  /** Blast radius: total unique functions reachable from focus. */
  const blastRadius = createMemo<number>(() => blastRadiusOf(levelGroups()));

  /** Flatten the per-level nodes into a single deduped list, focus first. */
  const visibleNodes = createMemo<GraphNode[]>(() => {
    const f = focus();
    if (!f) return [];
    const seen = new Set<string>([f.id]);
    const out: GraphNode[] = [f];
    for (const g of levelGroups()) {
      for (const n of g.nodes) {
        if (!seen.has(n.id)) {
          seen.add(n.id);
          out.push(n);
        }
      }
    }
    return out;
  });

  /** Flatten the per-level edges into a single deduped list. */
  const visibleEdges = createMemo<GraphEdge[]>(() => {
    const seen = new Set<string>();
    const out: GraphEdge[] = [];
    for (const g of levelGroups()) {
      for (const e of g.edges) {
        const k = `${e.source}->${e.target}`;
        if (!seen.has(k)) {
          seen.add(k);
          out.push(e);
        }
      }
    }
    return out;
  });

  // Emit stats to sidebar whenever focus/depth/direction change.
  createEffect(() => {
    props.onStats?.({
      blastRadius: blastRadius(),
      depth: depth(),
      direction: direction(),
    });
  });

  // Mount the renderer once the container is in the DOM.
  onMount(() => {
    renderer = new GraphRenderer({
      container: containerRef,
      width: containerRef.clientWidth || 800,
      height: containerRef.clientHeight || 600,
      layout: {
        type: "dagre",
        rankdir: direction() === "callers" ? "RL" : "LR",
        align: "UL",
        nodesep: 30,
        ranksep: 60,
      },
      onNodeClick: (id) => {
        setFocusId(id);
        props.onSelect(id);
      },
    });
    renderer.setData({
      schemaVersion: "0.0.0",
      source: "callgraph-view",
      loadedAt: "0",
      rawKind: "call-graph",
      nodes: visibleNodes(),
      edges: visibleEdges(),
    });
  });

  // Re-push data when focus/depth/direction/visible set change.
  createEffect(() => {
    const nodes = visibleNodes();
    const edges = visibleEdges();
    const dir = direction();
    if (!renderer) return;
    // Update layout direction on the fly if it changed.
    void renderer.setLayout({
      type: "dagre",
      rankdir: dir === "callers" ? "RL" : "LR",
      align: "UL",
      nodesep: 30,
      ranksep: 60,
    });
    renderer.setData({
      schemaVersion: "0.0.0",
      source: "callgraph-view",
      loadedAt: "0",
      rawKind: "call-graph",
      nodes,
      edges,
    });
  });

  // Highlight focus node.
  createEffect(() => {
    const id = focusId();
    if (!renderer) return;
    if (id) {
      void renderer.focusNode(id);
    } else {
      void renderer.clearFocus();
    }
  });

  // Resize observer.
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
    <div class="callgraph-view">
      <header class="callgraph-header">
        <div class="callgraph-focus">
          <label>Focus</label>
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

        <div class="callgraph-controls">
          <div class="control-group">
            <label>Direction</label>
            <select
              value={direction()}
              onChange={(e) =>
                setDirection(e.currentTarget.value as CallDirection)
              }
            >
              <option value="callees">callees (downstream)</option>
              <option value="callers">callers (upstream)</option>
              <option value="both">both</option>
            </select>
          </div>
          <div class="control-group">
            <label>Depth</label>
            <div class="depth-buttons">
              <button
                onClick={() => setDepth(Math.max(1, depth() - 1))}
                disabled={depth() <= 1}
                aria-label="Decrease depth"
              >
                −
              </button>
              <span class="depth-value">{depth()}</span>
              <button
                onClick={() => setDepth(Math.min(MAX_DEPTH, depth() + 1))}
                disabled={depth() >= MAX_DEPTH}
                aria-label="Increase depth"
              >
                +
              </button>
            </div>
          </div>
        </div>
      </header>

      <div class="callgraph-body">
        <Show when={focus()}>
          {(f) => (
            <section class="callgraph-focus-detail">
              <h2>{f().label}</h2>
              <Show when={f().file}>
                <code class="callgraph-file">
                  {f().file}:{f().line ?? "?"}
                </code>
              </Show>
              <div class="callgraph-stats">
                <span>
                  <strong>{blastRadius()}</strong> reachable functions
                </span>
                <span>·</span>
                <span>
                  depth <strong>{depth()}</strong> / {MAX_DEPTH}
                </span>
              </div>
            </section>
          )}
        </Show>

        <div ref={containerRef} class="callgraph-canvas" />
      </div>
    </div>
  );
};
