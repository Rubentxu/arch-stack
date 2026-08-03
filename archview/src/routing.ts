/**
 * Pure routing discriminator for the Archview manual router (R3).
 *
 * The workbench exposes exactly these specialized outcomes: C4,
 * Sequence, Class diagram, Call graph, Package, Impact, and Drift
 * (Drift is handled separately by `driftMode` in App). Bundle kind
 * triggers C4, Sequence, or Class diagram; a call-graph bundle is
 * routed through a selector that chooses Call graph, Package, or
 * Impact — with Impact as the default outcome. Unknown kinds fall
 * back to the generic graph view.
 *
 * Keeping this mapping pure makes the routing matrix unit-testable
 * without a DOM and guarantees exactly one outcome per input, which
 * prevents the nested-switch collisions of the previous router.
 */

/** Selector state for a loaded call-graph bundle. */
export type CallGraphMode = "impact" | "call-graph" | "package";

/** The specialized view App must render for a bundle. */
export type ResolvedView =
  | "c4"
  | "sequence"
  | "class-diagram"
  | "call-graph"
  | "package"
  | "impact"
  | "generic";

/**
 * Resolve which specialized view renders for a bundle `rawKind` and
 * the current call-graph selector mode. Returns exactly one outcome.
 */
export function resolveView(
  rawKind: string,
  callGraphMode: CallGraphMode,
): ResolvedView {
  switch (rawKind) {
    case "c4":
      return "c4";
    case "sequence":
      return "sequence";
    case "class-diagram":
      return "class-diagram";
    case "call-graph":
      // Impact remains the default call-graph outcome (R3).
      return callGraphMode;
    default:
      return "generic";
  }
}
