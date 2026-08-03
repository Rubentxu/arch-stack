/**
 * SequenceView — renders a sequence bundle as a UML sequence diagram.
 *
 * Layout (top-to-bottom):
 *   1. Participant header (lifeline labels, one per unique function/file)
 *   2. Interaction rows in time order (sorted by `order`)
 *   3. Each row: source participant → arrow → target participant
 *
 * M17.3 MVP is a text/grid layout. The arrow is rendered as a
 * styled span (no SVG) but the visual semantics match UML:
 *   - SyncCall: solid line, arrow head
 *   - AsyncCall: dashed line, open arrow head
 *   - Reply: dotted line, no arrow head
 *
 * M17.3.1 can upgrade to a full SVG lifeline render (vertical lines,
 * activation bars) without changing the data shape.
 */

import { For, Show, createMemo, type Component } from "solid-js";
import type { GraphNode, SequenceInteraction } from "../bundle/loader";
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
  /** Participant list (unique file:name, first-appearance order). */
  const participants = createMemo<Participant[]>(() =>
    extractParticipants(props.interactions),
  );

  /** Index: participant key → column index. */
  const colIndex = createMemo<Map<string, number>>(() =>
    participantColumns(participants()),
  );

  /** Interactions sorted by `order`. */
  const orderedInteractions = createMemo<SequenceInteraction[]>(() =>
    orderInteractions(props.interactions),
  );

  const handleParticipantClick = (p: Participant) => {
    const node = props.nodes.find(
      (n) => n.id === p.key || (n.file === p.file && n.label === p.name),
    );
    if (node) props.onSelect(node.id);
  };

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
        when={orderedInteractions().length > 0}
        fallback={<p class="empty">No interactions in this sequence bundle.</p>}
      >
        <div
          class="sequence-grid"
          style={`grid-template-columns: repeat(${participants().length}, 1fr);`}
        >
          <For each={participants()}>
            {(p) => (
              <div class="sequence-participant-header">
                <button
                  class="participant-label"
                  onClick={() => handleParticipantClick(p)}
                  title={p.file ? `${p.file}:${p.name}` : p.name}
                >
                  <span class="participant-name">{p.name}</span>
                  <Show when={p.file}>
                    <span class="participant-file">{p.file}</span>
                  </Show>
                </button>
              </div>
            )}
          </For>

          <For each={orderedInteractions()}>
            {(interaction, idx) => (
              <>
                <div class="sequence-row-meta" style={`grid-column: 1 / -1;`}>
                  <span class="sequence-step">{idx() + 1}</span>
                  <span class="sequence-order">order={interaction.order}</span>
                </div>
                <div class="sequence-row" style={`grid-column: 1 / -1;`}>
                  <SequenceArrow
                    interaction={interaction}
                    participants={participants()}
                    colIndex={colIndex()}
                  />
                </div>
              </>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
};

/**
 * Single interaction arrow. Renders a grid row that places the
 * source label, the arrow (line + arrowhead), and the target label.
 */
const SequenceArrow: Component<{
  interaction: SequenceInteraction;
  participants: Participant[];
  colIndex: Map<string, number>;
}> = (props) => {
  const kind = (): string => props.interaction.message_kind ?? "SyncCall";
  const sourceKey = (): string => {
    const c = props.interaction.caller;
    return `${c.file ?? ""}:${c.name ?? "?"}`;
  };
  const targetKey = (): string => {
    const c = props.interaction.callee;
    return `${c.file ?? ""}:${c.name ?? "?"}`;
  };
  const sourceCol = (): number => props.colIndex.get(sourceKey()) ?? 0;
  const targetCol = (): number => props.colIndex.get(targetKey()) ?? 0;

  return (
    <div class={`arrow-row kind-${kind()}`}>
      <div class="arrow-source" style={`grid-column: ${sourceCol() + 1};`}>
        <Show when={props.interaction.caller.name}>
          {props.interaction.caller.name}
        </Show>
      </div>
      <div
        class="arrow-line"
        style={`grid-column: ${Math.min(sourceCol(), targetCol()) + 1} / ${Math.max(sourceCol(), targetCol()) + 2};`}
      >
        <span class="arrow-line-stem" />
        <span class="arrow-head" />
        <span class="arrow-label">
          {props.interaction.label ??
            `${props.interaction.caller.name ?? "?"} → ${props.interaction.callee.name ?? "?"}`}
        </span>
      </div>
      <div class="arrow-target" style={`grid-column: ${targetCol() + 1};`}>
        <Show when={props.interaction.callee.name}>
          {props.interaction.callee.name}
        </Show>
      </div>
    </div>
  );
};
