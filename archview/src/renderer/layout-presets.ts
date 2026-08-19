/**
 * Layout presets — ELK layered algorithm options for the standard
 * workbench directions.
 *
 * M19 replaces the M17.1 dagre layout (G6 built-in) with ELK layered
 * running in a Web Worker (elkjs spawns the worker internally via
 * its `workerUrl` option). These presets are the ELK equivalent of
 * the previous dagre configs that lived in each view:
 *
 *   - C4View, SequenceView, DriftView: top-to-bottom (TB) layered
 *   - CallGraphView, ClassDiagramView, PackageView, ImpactView: left-to-right (LR) layered
 *
 * The defaults are tuned to match the visual density of dagre
 * (nodeNode: 40px between siblings in a layer, 80px between layers).
 * Views can override by passing their own `LayoutOptions` to
 * `renderer.setData(bundle, options)`.
 *
 * ELK layered options reference (elkjs 0.12):
 *   - `elk.direction`: "DOWN" | "UP" | "RIGHT" | "LEFT"
 *   - `elk.layered.spacing.nodeNodeBetweenLayers`: pixels between layers
 *   - `elk.layered.spacing.nodeNode`: pixels between siblings in a layer
 *   - `elk.layered.crossingMinimization.semiInteractive`: true for stable
 *     layouts under frequent data changes (workbench default = true)
 *   - `elk.padding`: "[T,R,B,L]" inner padding of the root graph
 */

import type { LayoutOptions } from "./layout-client";

/** Top-to-bottom layered — for trees/sequences (C4, Sequence, Drift). */
export const TB_LAYERED: LayoutOptions = {
  "elk.algorithm": "layered",
  "elk.direction": "DOWN",
  "elk.layered.spacing.nodeNodeBetweenLayers": 80,
  "elk.layered.spacing.nodeNode": 40,
  "elk.layered.crossingMinimization.semiInteractive": true,
  "elk.padding": "[top=24,left=24,bottom=24,right=24]",
};

/** Left-to-right layered — for call/dependency flows (Call, Class, Package, Impact). */
export const LR_LAYERED: LayoutOptions = {
  "elk.algorithm": "layered",
  "elk.direction": "RIGHT",
  "elk.layered.spacing.nodeNodeBetweenLayers": 80,
  "elk.layered.spacing.nodeNode": 40,
  "elk.layered.crossingMinimization.semiInteractive": true,
  "elk.padding": "[top=24,left=24,bottom=24,right=24]",
};

/** Right-to-left layered — CallGraphView "callers" direction
 *  (focus on the right, callers expanding left). */
export const RL_LAYERED: LayoutOptions = {
  ...LR_LAYERED,
  "elk.direction": "LEFT",
};

/** Default fallback — TB layered. Used when a view does not pass options. */
export const DEFAULT_LAYOUT: LayoutOptions = TB_LAYERED;
