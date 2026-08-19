/**
 * SequenceView — renders a sequence bundle as a G6 dagre
 * vertical graph (M17.1.3).
 *
 * Layout (top-to-bottom):
 *   - One node per participant (function), positioned by first
 *     appearance in the interaction order.
 *   - Edges ordered top-to-bottom by `order` (G6's dagre will
 *     lay them out in the same direction, so the visual reading
 *     matches the temporal reading).
 *
 * M17.1.3 replaced the previous CSS-grid lifelines render with
 * a G6 graph. The pure helpers in SequenceGraph.ts are unchanged.
 */

import {
  For,
  Show,
  createEffect,
  createMemo,
  onCleanup,
  onMount,
  type Component,
} from "solid-js";
import type { GraphNode, SequenceInteraction } from "../bundle/loader";
import { GraphRenderer } from "../renderer/g6";
import { TB_LAYERED } from "../renderer/layout-presets";
import {
  extractParticipants,
  orderInteractions,
  participantColumns,
  type Participant,
} from "./SequenceGraph";

export interface SequenceViewProps {
  nodes: GraphNode[];
  interactions: SequenceInteraction[];
  onSelect: (id: string | null) => void;
}

export const SequenceView: Component<SequenceViewProps> = (props) => {
  let containerRef!: HTMLDivElement;
  let renderer: GraphRenderer | undefined;

  const participants = createMemo<Participant[]>(() =>
    extractParticipants(props.interactions),
  );

  const orderedInteractions = createMemo<SequenceInteraction[]>(() =>
    orderInteractions(props.interactions),
  );

  /** Edges for the graph: one per ordered interaction, with
   *  caller/callee as source/target using the participants'
   *  canonical key. */
  const edges = createMemo(() => {
    const cols = participantColumns(participants());
    const ordered = orderedInteractions();
    return ordered
      .map((i, idx) => {
        const cKey = `${i.caller.file ?? ""}:${i.caller.name ?? ""}`;
        const dKey = `${i.callee.file ?? ""}:${i.callee.name ?? ""}`;
        if (!cols.has(cKey) || !cols.has(dKey)) return null;
        return {
          id: `s-${idx}`,
          source: cKey,
          target: dKey,
          kind: i.message_kind,
          label: i.label,
        };
      })
      .filter((e): e is NonNullable<typeof e> => e !== null);
  });

  onMount(() => {
    renderer = new GraphRenderer({
      container: containerRef,
      width: containerRef?.clientWidth || 800,
      height: containerRef?.clientHeight || 600,
      // M19: ELK layered in Web Worker, top-to-bottom.
      layoutOptions: TB_LAYERED,
      onNodeClick: (id) => {
        props.onSelect(id);
      },
    });
    renderer.setData({
      schemaVersion: "0.0.0",
      source: "sequence-view",
      loadedAt: "0",
      rawKind: "sequence",
      nodes: props.nodes,
      edges: edges(),
    });
  });

  createEffect(() => {
    if (!renderer) return;
    renderer.setData({
      schemaVersion: "0.0.0",
      source: "sequence-view",
      loadedAt: "0",
      rawKind: "sequence",
      nodes: props.nodes,
      edges: edges(),
    });
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
    <div class="sequence-view">
      <header class="sequence-header">
        <h2>Sequence diagram</h2>
        <p class="muted">
          {participants().length} participants · {orderedInteractions().length}{" "}
          interactions
        </p>
      </header>

      <Show
        when={participants().length > 0}
        fallback={<p class="empty">No participants in this bundle.</p>}
      >
        <div ref={containerRef} class="sequence-canvas" />

        <section class="sequence-interactions">
          <h3>Interactions (in order)</h3>
          <ol>
            <For each={orderedInteractions()}>
              {(i) => (
                <li>
                  <code>
                    {i.caller.name ?? "?"}
                    {i.caller.file ? ` (${i.caller.file})` : ""}
                  </code>
                  <span class="arrow"> → </span>
                  <code>
                    {i.callee.name ?? "?"}
                    {i.callee.file ? ` (${i.callee.file})` : ""}
                  </code>
                  <Show when={i.label}>
                    <span class="interaction-label"> · {i.label}</span>
                  </Show>
                </li>
              )}
            </For>
          </ol>
        </section>
      </Show>
    </div>
  );
};
