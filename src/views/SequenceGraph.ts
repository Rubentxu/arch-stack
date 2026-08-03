/**
 * Pure helpers for sequence diagram rendering.
 *
 * Extracted from SequenceView so they can be unit-tested without
 * pulling JSX into the test pipeline.
 */

import type { SequenceInteraction } from "../bundle/loader";

export interface Participant {
  key: string;
  name: string;
  file?: string;
}

/**
 * Build the participant list from interactions. A participant is
 * uniquely identified by file:name (matches the loader's key scheme).
 * Participants preserve first-appearance order.
 */
export function extractParticipants(
  interactions: SequenceInteraction[],
): Participant[] {
  const map = new Map<string, Participant>();
  for (const i of interactions) {
    for (const side of [
      { name: i.caller.name, file: i.caller.file },
      { name: i.callee.name, file: i.callee.file },
    ] as const) {
      if (!side.name) continue;
      const key = `${side.file ?? ""}:${side.name}`;
      if (!map.has(key)) {
        map.set(key, { key, name: side.name, file: side.file });
      }
    }
  }
  return [...map.values()];
}

/**
 * Sort interactions by `order` ascending (stable for ties).
 */
export function orderInteractions(
  interactions: SequenceInteraction[],
): SequenceInteraction[] {
  return [...interactions].sort((a, b) => a.order - b.order);
}

/**
 * Compute the column index for each participant key (0-based).
 * Unknown keys default to 0.
 */
export function participantColumns(
  participants: Participant[],
): Map<string, number> {
  const m = new Map<string, number>();
  participants.forEach((p, i) => m.set(p.key, i));
  return m;
}
